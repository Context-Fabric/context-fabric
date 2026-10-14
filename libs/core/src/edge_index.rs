//! Reverse (target -> sources) edge index, stored in the `.cfr` section
//! `CFRREVE1` so `E.<feature>.t` / `.b` and search's backward edge joins are a
//! binary search over the memory map instead of a scan of every forward row.
//!
//! Each edge feature gets one compact CSR keyed by the *distinct* targets
//! (not by node id, so sparse features stay small on disk):
//!
//! ```text
//! targets: u32 vec   ascending distinct target nodes
//! offsets: u32 vec   targets.len() + 1 entries into `sources`
//! sources: u32 vec   per target: its source nodes, canonical (rank) order, deduplicated
//! ```
//!
//! Every vector is written as `u32 length` followed by little-endian u32 values,
//! the same encoding as the rest of the file.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::compiled::{invalid_compiled, read_u32_from, write_u32_vec};
use crate::error::Result;
use crate::row_index::RowLookup;

/// The owned reverse CSR built at compile time from a feature's forward rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReverseEdgeCsr {
    targets: Vec<u32>,
    offsets: Vec<u32>,
    sources: Vec<u32>,
}

impl ReverseEdgeCsr {
    /// Inverts forward rows (`source -> targets`). Sources per target are put
    /// in canonical order (`rank[node - 1]`, ties broken by node id; nodes
    /// without a rank sort by their own id) and deduplicated, exactly the order
    /// `E.<feature>.t` returns.
    pub fn build(forward: &HashMap<u32, Vec<u32>>, rank: &[u32]) -> Self {
        let rank_key = |node: u32| -> (u32, u32) {
            let canonical = node
                .checked_sub(1)
                .and_then(|index| rank.get(index as usize).copied())
                .unwrap_or(node);
            (canonical, node)
        };
        let mut pairs: Vec<(u32, (u32, u32))> = forward
            .iter()
            .flat_map(|(source, targets)| {
                let key = rank_key(*source);
                targets.iter().map(move |target| (*target, key))
            })
            .collect();
        pairs.sort_unstable();
        pairs.dedup();

        let mut csr = Self {
            targets: Vec::new(),
            offsets: vec![0],
            sources: Vec::with_capacity(pairs.len()),
        };
        for (target, (_, source)) in pairs {
            if csr.targets.last() != Some(&target) {
                if !csr.targets.is_empty() {
                    csr.offsets.push(csr.sources.len() as u32);
                }
                csr.targets.push(target);
            }
            csr.sources.push(source);
        }
        if !csr.targets.is_empty() {
            csr.offsets.push(csr.sources.len() as u32);
        }
        csr
    }

    pub fn write<W: Write>(&self, writer: &mut W, path: &Path) -> Result<()> {
        write_u32_vec(writer, &self.targets, path)?;
        write_u32_vec(writer, &self.offsets, path)?;
        write_u32_vec(writer, &self.sources, path)
    }
}

/// Where one feature's reverse CSR lives inside the memory-mapped file. Plain
/// byte offsets: lookups read straight from the map, nothing is copied to the
/// heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReverseEdgeLayout {
    target_count: usize,
    targets_start: usize,
    offsets_start: usize,
    sources_start: usize,
}

impl ReverseEdgeLayout {
    /// Validates the three vectors starting at `start` and returns the layout
    /// plus the offset just past them.
    pub fn inspect(path: &Path, bytes: &[u8], start: usize) -> Result<(Self, usize)> {
        let vector = |at: usize| -> Result<(usize, usize, usize)> {
            let len = read_u32_from(path, bytes, at)? as usize;
            let data = at + 4;
            let end = len
                .checked_mul(4)
                .and_then(|byte_len| data.checked_add(byte_len))
                .filter(|end| *end <= bytes.len())
                .ok_or_else(|| invalid_compiled(path, "reverse edge index out of bounds"))?;
            Ok((len, data, end))
        };
        let (target_count, targets_start, after_targets) = vector(start)?;
        let (offset_count, offsets_start, after_offsets) = vector(after_targets)?;
        let (source_count, sources_start, end) = vector(after_offsets)?;
        if offset_count != target_count + 1 {
            return Err(invalid_compiled(
                path,
                "reverse edge index offsets do not match targets",
            ));
        }
        if read_u32_from(path, bytes, offsets_start + target_count * 4)? as usize != source_count {
            return Err(invalid_compiled(
                path,
                "reverse edge index offsets do not match sources",
            ));
        }
        Ok((
            Self {
                target_count,
                targets_start,
                offsets_start,
                sources_start,
            },
            end,
        ))
    }

    /// Sources of `target` in canonical order, read from the map: one lookup
    /// among the distinct targets (`lookup` caches its sample; keep one per
    /// layout) plus one read per source.
    pub fn sources(
        &self,
        path: &Path,
        bytes: &[u8],
        lookup: &RowLookup,
        target: u32,
    ) -> Result<Vec<u32>> {
        let targets_start = self.targets_start;
        let Some(index) = lookup.find(
            path,
            bytes,
            self.target_count,
            |index| targets_start + index * 4,
            target,
        )?
        else {
            return Ok(Vec::new());
        };
        let begin = read_u32_from(path, bytes, self.offsets_start + index * 4)? as usize;
        let end = read_u32_from(path, bytes, self.offsets_start + (index + 1) * 4)? as usize;
        (begin..end)
            .map(|position| read_u32_from(path, bytes, self.sources_start + position * 4))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forward(rows: &[(u32, &[u32])]) -> HashMap<u32, Vec<u32>> {
        rows.iter()
            .map(|(source, targets)| (*source, targets.to_vec()))
            .collect()
    }

    fn round_trip(csr: &ReverseEdgeCsr) -> (Vec<u8>, ReverseEdgeLayout) {
        let path = Path::new("test.cfr");
        // A leading pad byte proves the layout makes no alignment assumption.
        let mut bytes = vec![0xAA];
        csr.write(&mut bytes, path).expect("write reverse csr");
        let (layout, end) = ReverseEdgeLayout::inspect(path, &bytes, 1).expect("inspect");
        assert_eq!(end, bytes.len());
        (bytes, layout)
    }

    #[test]
    fn inverts_rows_in_rank_order_with_duplicates_removed() {
        // rank[node - 1]: node 5 sorts before node 4, node 6 has no rank.
        let rank = [1, 2, 3, 5, 4];
        let csr = ReverseEdgeCsr::build(
            &forward(&[(4, &[1, 2]), (5, &[1, 1]), (6, &[1]), (3, &[2])]),
            &rank,
        );
        let (bytes, layout) = round_trip(&csr);
        let path = Path::new("test.cfr");
        let expected: [(u32, &[u32]); 5] =
            [(0, &[]), (1, &[5, 4, 6]), (2, &[3, 4]), (3, &[]), (99, &[])];
        for (target, sources) in expected {
            assert_eq!(
                layout
                    .sources(path, &bytes, &RowLookup::default(), target)
                    .expect("lookup"),
                sources,
                "target {target}"
            );
        }
    }

    #[test]
    fn empty_feature_round_trips() {
        let csr = ReverseEdgeCsr::build(&HashMap::new(), &[]);
        let (bytes, layout) = round_trip(&csr);
        assert!(
            layout
                .sources(Path::new("test.cfr"), &bytes, &RowLookup::default(), 1)
                .expect("lookup")
                .is_empty()
        );
    }

    #[test]
    fn rejects_offsets_that_disagree_with_sources() {
        let csr = ReverseEdgeCsr::build(&forward(&[(2, &[1])]), &[]);
        let path = Path::new("test.cfr");
        let mut bytes = Vec::new();
        csr.write(&mut bytes, path).expect("write");
        // Corrupt the final offset (last 4 bytes of the offsets vector).
        let offsets_end = 4 + 4 + 4 + 2 * 4;
        bytes[offsets_end - 4..offsets_end].copy_from_slice(&7_u32.to_le_bytes());
        assert!(ReverseEdgeLayout::inspect(path, &bytes, 0).is_err());
    }
}
