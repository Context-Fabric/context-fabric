//! Row offsets for features whose rows have variable length (mixed node
//! features and edge features), stored in the `.cfr` section `CFRROWS1`.
//!
//! Finding row `i` of such a feature needs the byte offset of every row. The
//! `CFRROWS1` section stores them once at compile time as little-endian u32
//! offsets relative to the feature's payload start, read straight from the map
//! (no heap table, no payload walk).
//!
//! Section layout: `u32` entry count, then per entry a `u8` [`RowIndexKind`],
//! the feature name (`u32` length + UTF-8) and a `u32`-length-prefixed vector of
//! `u32` offsets, one per row.

use std::path::Path;
use std::sync::OnceLock;

use crate::error::Result;
use crate::format::{invalid_compiled, read_u32_from};

/// Which feature table a `CFRROWS1` entry belongs to (node and edge feature
/// names may coincide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowIndexKind {
    Node = 0,
    Edge = 1,
}

impl RowIndexKind {
    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Self::Node),
            1 => Some(Self::Edge),
            _ => None,
        }
    }
}

/// Where one feature's offsets vector lives in the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowIndexLayout {
    len: usize,
    start: usize,
}

impl RowIndexLayout {
    /// Validates the length-prefixed offsets vector at `at` and returns its
    /// layout plus the offset just past it.
    pub fn inspect(path: &Path, bytes: &[u8], at: usize) -> Result<(Self, usize)> {
        let len = read_u32_from(path, bytes, at)? as usize;
        let start = at + 4;
        let end = len
            .checked_mul(4)
            .and_then(|byte_len| start.checked_add(byte_len))
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| invalid_compiled(path, "row index out of bounds"))?;
        Ok((Self { len, start }, end))
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

/// Absolute byte offsets of a feature's rows: its `CFRROWS1` offsets,
/// relative to `base` (the payload start), read from the map on demand.
#[derive(Debug)]
pub struct RowOffsets {
    base: usize,
    layout: RowIndexLayout,
}

impl RowOffsets {
    pub fn new(base: usize, layout: RowIndexLayout) -> Self {
        Self { base, layout }
    }

    pub fn len(&self) -> usize {
        self.layout.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Absolute offset of row `index`. Out-of-range rows map to `usize::MAX`,
    /// which every bounds-checked map read rejects as an error.
    pub fn get(&self, bytes: &[u8], index: usize) -> usize {
        if index >= self.layout.len {
            return usize::MAX;
        }
        let at = self.layout.start + index * 4;
        bytes.get(at..at + 4).map_or(usize::MAX, |raw| {
            self.base + u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize
        })
    }

    pub fn iter<'a>(&'a self, bytes: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        (0..self.len()).map(move |index| self.get(bytes, index))
    }
}

/// Tables at most this long are bisected directly; larger ones get a
/// [`NodeRank`] on their first lookup that misses the dense position.
const SMALL_TABLE_ROWS: usize = 1024;

/// Finds a node's row in a node-sorted row table living in the map (node ids
/// are unique per table).
///
/// The dense position `node - first_node` is tried first: an O(1) hit for any
/// feature defined on a contiguous node range (every word, every non-slot
/// node), which never builds anything. Other tables get a [`NodeRank`] built
/// on the first miss, which answers both hits and misses without touching the
/// map; small tables are bisected.
#[derive(Debug, Default)]
pub struct RowLookup {
    rank: OnceLock<NodeRank>,
}

impl RowLookup {
    /// Index of the row of `node` among `row_count` rows, where row `i`
    /// starts at `row_offset(i)` with its node id as leading u32.
    pub fn find(
        &self,
        path: &Path,
        bytes: &[u8],
        row_count: usize,
        row_offset: impl Fn(usize) -> usize,
        node: u32,
    ) -> Result<Option<usize>> {
        let node_at = |index: usize| read_u32_from(path, bytes, row_offset(index));
        if row_count == 0 {
            return Ok(None);
        }
        let first_node = node_at(0)?;
        if let Some(guess) = node.checked_sub(first_node).map(|delta| delta as usize)
            && guess < row_count
            && node_at(guess)? == node
        {
            return Ok(Some(guess));
        }
        if row_count <= SMALL_TABLE_ROWS {
            let (mut low, mut high) = (0, row_count);
            while low < high {
                let mid = low + (high - low) / 2;
                match node_at(mid)?.cmp(&node) {
                    std::cmp::Ordering::Less => low = mid + 1,
                    std::cmp::Ordering::Greater => high = mid,
                    std::cmp::Ordering::Equal => return Ok(Some(mid)),
                }
            }
            return Ok(None);
        }
        let rank = match self.rank.get() {
            Some(rank) => rank,
            None => {
                let rank = NodeRank::build((0..row_count).map(node_at)).ok_or_else(|| {
                    invalid_compiled(path, "feature rows are not sorted by node")
                })??;
                self.rank.get_or_init(|| rank)
            }
        };
        Ok(rank.row_of(node))
    }
}

/// Which nodes have a row, as a bitmap with the number of set bits before each
/// 64-bit word: the row of a present node is the count of present nodes below
/// it. 12 bytes per 64 node ids (about 270 KB for BHSA's 1.4M nodes).
#[derive(Debug)]
pub struct NodeRank {
    words: Vec<u64>,
    rows_before: Vec<u32>,
}

impl NodeRank {
    /// Builds from the rows' node ids in row order. `None` when they are not
    /// strictly ascending; the inner `Err` is a read error.
    fn build(nodes: impl Iterator<Item = Result<u32>>) -> Option<Result<Self>> {
        let mut words: Vec<u64> = Vec::new();
        let mut previous: Option<u32> = None;
        for node in nodes {
            let node = match node {
                Ok(node) => node,
                Err(error) => return Some(Err(error)),
            };
            if previous.is_some_and(|previous| previous >= node) {
                return None;
            }
            previous = Some(node);
            let word = node as usize / 64;
            if word >= words.len() {
                words.resize(word + 1, 0);
            }
            words[word] |= 1 << (node % 64);
        }
        let mut rows_before = Vec::with_capacity(words.len());
        let mut count = 0_u32;
        for word in &words {
            rows_before.push(count);
            count += word.count_ones();
        }
        Some(Ok(Self { words, rows_before }))
    }

    /// The row index of `node`, `None` when it has no row.
    pub fn row_of(&self, node: u32) -> Option<usize> {
        let word = node as usize / 64;
        let bits = *self.words.get(word)?;
        let bit = 1_u64 << (node % 64);
        (bits & bit != 0)
            .then(|| (self.rows_before[word] + (bits & (bit - 1)).count_ones()) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::write_u32_vec;

    #[test]
    fn mapped_offsets_are_relative_to_the_payload_and_bounds_checked() {
        let path = Path::new("test.cfr");
        let mut bytes = vec![0xAA, 0xBB, 0xCC];
        write_u32_vec(&mut bytes, &[4, 17, 30], path).expect("write");
        let (layout, end) = RowIndexLayout::inspect(path, &bytes, 3).expect("inspect");
        assert_eq!(end, bytes.len());
        assert_eq!(layout.len(), 3);

        let offsets = RowOffsets::new(100, layout);
        assert_eq!(
            offsets.iter(&bytes).collect::<Vec<_>>(),
            vec![104, 117, 130]
        );
        assert_eq!(offsets.get(&bytes, 3), usize::MAX);
    }

    #[test]
    fn rejects_a_vector_running_past_the_end() {
        let path = Path::new("test.cfr");
        let mut bytes = Vec::new();
        write_u32_vec(&mut bytes, &[1, 2], path).expect("write");
        bytes[0] = 9;
        assert!(RowIndexLayout::inspect(path, &bytes, 0).is_err());
    }

    fn find(lookup: &RowLookup, nodes: &[u32], node: u32) -> Option<usize> {
        let bytes: Vec<u8> = nodes.iter().flat_map(|node| node.to_le_bytes()).collect();
        lookup
            .find(Path::new("t"), &bytes, nodes.len(), |index| index * 4, node)
            .expect("in bounds")
    }

    #[test]
    fn row_lookup_finds_every_present_node_and_rejects_absent_ones() {
        let layouts: [Vec<u32>; 6] = [
            vec![],
            vec![7],
            (10..2_000).collect(),
            (1..400).map(|n| n * n).collect(),
            (1..300)
                .map(|n| if n < 150 { n } else { n * 1_000 })
                .collect(),
            [1, 2, 3, 1_000, 1_001, 50_000, 50_002, 4_000_000].to_vec(),
        ];
        for nodes in layouts {
            let present: std::collections::HashSet<u32> = nodes.iter().copied().collect();
            let lookup = RowLookup::default();
            for (index, node) in nodes.iter().enumerate() {
                assert_eq!(
                    find(&lookup, &nodes, *node),
                    Some(index),
                    "{node} in {nodes:?}"
                );
            }
            let probes = (0..nodes.last().copied().unwrap_or(10).min(5_000) + 2).chain([
                u32::MAX,
                4_000_001,
                299_001,
            ]);
            for node in probes.filter(|node| !present.contains(node)) {
                assert_eq!(find(&lookup, &nodes, node), None, "absent {node}");
            }
        }
    }

    #[test]
    fn row_lookup_rejects_unsorted_large_tables() {
        let mut nodes: Vec<u32> = (0..2_000).map(|n| n * 2).collect();
        nodes.swap(10, 11);
        let bytes: Vec<u8> = nodes.iter().flat_map(|node| node.to_le_bytes()).collect();
        let lookup = RowLookup::default();
        let missing_dense = 3;
        assert!(
            lookup
                .find(
                    Path::new("t"),
                    &bytes,
                    nodes.len(),
                    |index| index * 4,
                    missing_dense
                )
                .is_err()
        );
    }

    #[test]
    fn kind_tags_round_trip() {
        for kind in [RowIndexKind::Node, RowIndexKind::Edge] {
            assert_eq!(RowIndexKind::from_tag(kind as u8), Some(kind));
        }
        assert_eq!(RowIndexKind::from_tag(7), None);
    }
}
