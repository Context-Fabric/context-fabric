//! Row offsets for features whose rows have variable length (mixed node
//! features and edge features), stored in the `.cfr` section `CFRROWS1`.
//!
//! Finding row `i` of such a feature needs the byte offset of every row. Older
//! caches had to walk the whole payload on first access to build that table in
//! the heap (8 bytes per row, plus every payload page pulled into memory); the
//! `CFRROWS1` section stores it once at compile time as little-endian u32
//! offsets relative to the feature's payload start, read straight from the map.
//!
//! Section layout: `u32` entry count, then per entry a `u8` [`RowIndexKind`],
//! the feature name (`u32` length + UTF-8) and a `u32`-length-prefixed vector of
//! `u32` offsets, one per row.

use std::path::Path;
use std::sync::OnceLock;

use crate::compiled::{invalid_compiled, read_u32_from};
use crate::error::Result;

/// Which feature table a `CFRROWS1` entry belongs to (node and edge feature
/// names may coincide).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Absolute byte offsets of a feature's rows.
#[derive(Debug)]
pub enum RowOffsets {
    /// From `CFRROWS1`: offsets relative to `base` (the payload start), read
    /// from the map on demand. No heap, no payload walk.
    Mapped { base: usize, layout: RowIndexLayout },
    /// Caches without `CFRROWS1`: found by walking the payload once.
    Scanned(Vec<usize>),
}

impl RowOffsets {
    pub fn len(&self) -> usize {
        match self {
            Self::Mapped { layout, .. } => layout.len,
            Self::Scanned(offsets) => offsets.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Absolute offset of row `index`. Out-of-range rows map to `usize::MAX`,
    /// which every bounds-checked map read rejects as an error.
    pub fn get(&self, bytes: &[u8], index: usize) -> usize {
        match self {
            Self::Mapped { base, layout } => {
                if index >= layout.len {
                    return usize::MAX;
                }
                let at = layout.start + index * 4;
                bytes.get(at..at + 4).map_or(usize::MAX, |raw| {
                    base + u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize
                })
            }
            Self::Scanned(offsets) => offsets.get(index).copied().unwrap_or(usize::MAX),
        }
    }

    pub fn iter<'a>(&'a self, bytes: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
        (0..self.len()).map(move |index| self.get(bytes, index))
    }
}

/// Rows per entry of a [`RowLookup`] sample.
const ROW_SAMPLE_STRIDE: usize = 64;

/// Finds a node's row in a node-sorted row table living in the map.
///
/// The dense position `node - first_node` is tried first: an O(1) hit for any
/// feature defined on a contiguous node range (every word, every non-slot
/// node). Otherwise the node id of every 64th row, sampled into a small heap
/// array on the first miss, is bisected in cache, leaving a 64-row block to
/// bisect in the map instead of the whole table.
#[derive(Debug, Default)]
pub struct RowLookup {
    sample: OnceLock<Vec<u32>>,
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
        let sample = match self.sample.get() {
            Some(sample) => sample,
            None => {
                let sample = (0..row_count)
                    .step_by(ROW_SAMPLE_STRIDE)
                    .map(node_at)
                    .collect::<Result<Vec<_>>>()?;
                self.sample.get_or_init(|| sample)
            }
        };
        // The block whose first node is the last one <= `node`.
        let block = match sample.partition_point(|sampled| *sampled <= node) {
            0 => return Ok(None),
            after => after - 1,
        };
        let mut low = block * ROW_SAMPLE_STRIDE;
        if sample[block] == node {
            return Ok(Some(low));
        }
        low += 1;
        let mut high = ((block + 1) * ROW_SAMPLE_STRIDE).min(row_count);
        while low < high {
            let mid = low + (high - low) / 2;
            match node_at(mid)?.cmp(&node) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => return Ok(Some(mid)),
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::write_u32_vec;

    #[test]
    fn mapped_offsets_are_relative_to_the_payload_and_bounds_checked() {
        let path = Path::new("test.cfr");
        let mut bytes = vec![0xAA, 0xBB, 0xCC];
        write_u32_vec(&mut bytes, &[4, 17, 30], path).expect("write");
        let (layout, end) = RowIndexLayout::inspect(path, &bytes, 3).expect("inspect");
        assert_eq!(end, bytes.len());
        assert_eq!(layout.len(), 3);

        let offsets = RowOffsets::Mapped { base: 100, layout };
        assert_eq!(
            offsets.iter(&bytes).collect::<Vec<_>>(),
            vec![104, 117, 130]
        );
        assert_eq!(offsets.get(&bytes, 3), usize::MAX);

        let scanned = RowOffsets::Scanned(vec![104, 117, 130]);
        assert_eq!(
            scanned.iter(&bytes).collect::<Vec<_>>(),
            vec![104, 117, 130]
        );
        assert_eq!(scanned.get(&bytes, 9), usize::MAX);
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
    fn kind_tags_round_trip() {
        for kind in [RowIndexKind::Node, RowIndexKind::Edge] {
            assert_eq!(RowIndexKind::from_tag(kind as u8), Some(kind));
        }
        assert_eq!(RowIndexKind::from_tag(7), None);
    }
}
