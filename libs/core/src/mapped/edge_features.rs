//! Edge feature views over the mapped file.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::error::Result;
use crate::format::{
    CompiledEdgeFeature, bounded_capacity, invalid_compiled, read_mixed_node_value_from,
    read_u32_at, read_u32_from, skip_mixed_value,
};
use crate::index::edge_index::ReverseEdgeLayout;
use crate::index::row_index::{RowIndexLayout, RowLookup, RowOffsets};
use crate::mapped::order::mmap_rank;
use crate::mapped::{MappedCompiledCorpus, MappedNodeValue};

/// Owned, parsed metadata for an edge feature, including a lazily-built
/// `(source, target) -> value offset` index for O(log n) `edge_value` lookups.
pub(crate) struct EdgeParts {
    row_count: usize,
    pub(crate) row_offsets: RowOffsets,
    lookup: RowLookup,
    edge_value_count: usize,
    edge_values_start: Option<usize>,
    is_oslots: bool,
    reverse_index: ReverseEdgeLayout,
    reverse_lookup: RowLookup,
    /// Sorted `(source, target, value_payload_offset)` triples, built on first
    /// `edge_value` call by a single scan of the edge-values section.
    edge_value_index: OnceLock<Vec<(u32, u32, usize)>>,
}

/// Owned, long-lived handle for an edge feature.
pub struct OwnedEdgeFeature {
    pub(super) corpus: Arc<MappedCompiledCorpus>,
    pub(super) parts: Arc<EdgeParts>,
}

impl OwnedEdgeFeature {
    pub fn view(&self) -> EdgeFeatureView<'_> {
        EdgeFeatureView {
            path: &self.corpus.path,
            bytes: &self.corpus.mmap,
            parts: Arc::clone(&self.parts),
            rank_start: self.corpus.metadata.rank_start,
            rank_len: self.corpus.metadata.rank_len,
        }
    }
}

/// Row offsets served from a feature's `CFRROWS1` entry, checked to cover
/// exactly `row_count` rows.
pub(crate) fn mapped_row_offsets(
    path: &Path,
    layout: RowIndexLayout,
    payload_start: usize,
    row_count: usize,
) -> Result<RowOffsets> {
    if layout.len() != row_count {
        return Err(invalid_compiled(
            path,
            "row index length does not match the feature's row count",
        ));
    }
    Ok(RowOffsets::new(payload_start, layout))
}

pub(crate) fn build_edge_parts(
    path: &Path,
    bytes: &[u8],
    feature: &CompiledEdgeFeature,
) -> Result<EdgeParts> {
    let mut offset = feature.payload_start;
    let row_count = read_u32_at(path, bytes, &mut offset)? as usize;
    if row_count != feature.row_count {
        return Err(invalid_compiled(
            path,
            "edge row count does not match metadata",
        ));
    }
    let row_offsets =
        mapped_row_offsets(path, feature.row_index, feature.payload_start, row_count)?;
    Ok(EdgeParts {
        row_count,
        row_offsets,
        lookup: RowLookup::default(),
        edge_value_count: feature.edge_value_count,
        edge_values_start: feature.edge_values_start,
        is_oslots: feature.name == "oslots",
        reverse_index: feature.reverse_index,
        reverse_lookup: RowLookup::default(),
        edge_value_index: OnceLock::new(),
    })
}

pub struct EdgeFeatureView<'a> {
    pub(crate) path: &'a Path,
    pub(crate) bytes: &'a [u8],
    pub(crate) parts: Arc<EdgeParts>,
    // Rank is read per-node from the mmap (`rank_start`/`rank_len`) instead of
    // materializing the whole rank Vec on every `edge_feature()` call.
    pub(super) rank_start: usize,
    pub(super) rank_len: usize,
}

impl<'a> EdgeFeatureView<'a> {
    pub fn has_edge_values(&self) -> bool {
        self.parts.edge_value_count > 0
    }

    /// Byte offset of `source`'s row, if it has one.
    fn row_offset(&self, source: u32) -> Result<Option<usize>> {
        let row_offsets = &self.parts.row_offsets;
        let row_offset = |index| row_offsets.get(self.bytes, index);
        Ok(self
            .parts
            .lookup
            .find(
                self.path,
                self.bytes,
                self.parts.row_count,
                row_offset,
                source,
            )?
            .map(row_offset))
    }

    pub fn targets(&self, source: u32) -> Result<Option<EdgeTargets<'a>>> {
        let Some(row_offset) = self.row_offset(source)? else {
            return Ok(None);
        };
        let target_count = read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
        Ok(Some(EdgeTargets {
            path: self.path,
            bytes: self.bytes,
            offset: row_offset + 8,
            remaining: target_count,
        }))
    }

    /// First and last target of `source`'s row without materializing the list —
    /// one row lookup plus three u32 reads. Targets are stored ascending, so
    /// for `oslots` this is the node's slot interval. `None` when the source has
    /// no row (or an empty one).
    pub fn first_last_targets(&self, source: u32) -> Result<Option<(u32, u32)>> {
        let Some(row_offset) = self.row_offset(source)? else {
            return Ok(None);
        };
        let count = read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
        if count == 0 {
            return Ok(None);
        }
        let first = read_u32_from(self.path, self.bytes, row_offset + 8)?;
        let last = read_u32_from(self.path, self.bytes, row_offset + 8 + ((count - 1) * 4))?;
        Ok(Some((first, last)))
    }

    fn sorted_targets(&self, source: u32) -> Result<Vec<u32>> {
        let mut targets = self
            .targets(source)?
            .map(|targets| targets.collect::<Result<Vec<_>>>())
            .unwrap_or_else(|| Ok(Vec::new()))?;
        self.sort_nodes(&mut targets);
        Ok(targets)
    }

    pub fn s(&self, source: u32) -> Result<Vec<u32>> {
        // `oslots` slot-identity: a slot is its own slot, as in TF's
        // `E.oslots.s` (`tf/core/oslotsfeature.py`); node 0 and nodes past the
        // last row have no slots. Only `s` applies this; `forward`/`f` return
        // raw adjacency.
        if self.parts.is_oslots && self.is_slot(source)? {
            return Ok(vec![source]);
        }
        self.sorted_targets(source)
    }

    /// Whether `node` is a slot, for the `oslots` feature: slots are the
    /// nodes `1..first_source`, where `first_source` is the first (lowest)
    /// non-slot node, since `oslots` has a row for every non-slot node.
    fn is_slot(&self, node: u32) -> Result<bool> {
        if self.parts.row_offsets.is_empty() {
            return Ok(false);
        }
        let first_source = read_u32_from(
            self.path,
            self.bytes,
            self.parts.row_offsets.get(self.bytes, 0),
        )?;
        Ok(node >= 1 && node < first_source)
    }

    pub fn f(&self, source: u32) -> Result<Vec<u32>> {
        self.sorted_targets(source)
    }

    pub fn forward(&self, source: u32) -> Result<Vec<u32>> {
        self.sorted_targets(source)
    }

    pub fn forward_with_values(
        &self,
        source: u32,
    ) -> Result<Vec<(u32, Option<MappedNodeValue<'a>>)>> {
        self.forward(source)?
            .into_iter()
            .map(|target| Ok((target, self.edge_value(source, target)?)))
            .collect()
    }

    pub fn f_with_values(&self, source: u32) -> Result<Vec<(u32, Option<MappedNodeValue<'a>>)>> {
        self.forward_with_values(source)
    }

    /// Sources with an edge to `target`, in canonical order, served from the
    /// `CFRREVE1` reverse index.
    pub fn t(&self, target: u32) -> Result<Vec<u32>> {
        self.parts
            .reverse_index
            .sources(self.path, self.bytes, &self.parts.reverse_lookup, target)
    }

    pub fn backward(&self, target: u32) -> Result<Vec<u32>> {
        self.t(target)
    }

    pub fn backward_with_values(
        &self,
        target: u32,
    ) -> Result<Vec<(u32, Option<MappedNodeValue<'a>>)>> {
        self.backward(target)?
            .into_iter()
            .map(|source| Ok((source, self.edge_value(source, target)?)))
            .collect()
    }

    pub fn t_with_values(&self, target: u32) -> Result<Vec<(u32, Option<MappedNodeValue<'a>>)>> {
        self.backward_with_values(target)
    }

    pub fn both(&self, node: u32) -> Result<Vec<u32>> {
        let mut nodes = self.forward(node)?;
        nodes.extend(self.backward(node)?);
        self.sort_nodes(&mut nodes);
        nodes.dedup();
        Ok(nodes)
    }

    pub fn b(&self, node: u32) -> Result<Vec<u32>> {
        self.both(node)
    }

    pub fn both_with_values(&self, node: u32) -> Result<Vec<(u32, Option<MappedNodeValue<'a>>)>> {
        let mut by_node = HashMap::<u32, Option<MappedNodeValue<'a>>>::new();
        for (source, value) in self.backward_with_values(node)? {
            by_node.insert(source, value);
        }
        for (target, value) in self.forward_with_values(node)? {
            by_node.insert(target, value);
        }
        let mut rows: Vec<_> = by_node.into_iter().collect();
        rows.sort_unstable_by_key(|(node, _)| self.rank_key(*node));
        Ok(rows)
    }

    pub fn b_with_values(&self, node: u32) -> Result<Vec<(u32, Option<MappedNodeValue<'a>>)>> {
        self.both_with_values(node)
    }

    pub fn items(&self) -> Result<Vec<(u32, Vec<u32>)>> {
        let mut rows = Vec::with_capacity(self.parts.row_count);
        for row_offset in self.parts.row_offsets.iter(self.bytes) {
            let source = read_u32_from(self.path, self.bytes, row_offset)?;
            let target_count = read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
            let targets_start = row_offset + 8;
            let mut targets = Vec::with_capacity(bounded_capacity(
                target_count,
                self.bytes.len().saturating_sub(targets_start),
                4,
            ));
            for index in 0..target_count {
                targets.push(read_u32_from(
                    self.path,
                    self.bytes,
                    targets_start + (index * 4),
                )?);
            }
            self.sort_nodes(&mut targets);
            targets.dedup();
            rows.push((source, targets));
        }
        rows.sort_unstable_by_key(|(source, _)| self.rank_key(*source));
        Ok(rows)
    }

    fn sort_nodes(&self, nodes: &mut [u32]) {
        nodes.sort_unstable_by_key(|node| self.rank_key(*node));
    }

    fn rank_key(&self, node: u32) -> (u32, u32) {
        let rank =
            mmap_rank(self.path, self.bytes, self.rank_start, self.rank_len, node).unwrap_or(node);
        (rank, node)
    }

    /// Lazily-built, sorted `(source, target, value_offset)` index over the
    /// edge-values section, shared via the cached `EdgeParts`.
    fn edge_value_index(&self) -> Result<&[(u32, u32, usize)]> {
        if let Some(index) = self.parts.edge_value_index.get() {
            return Ok(index.as_slice());
        }
        let remaining = self
            .parts
            .edge_values_start
            .map_or(0, |start| self.bytes.len().saturating_sub(start));
        let mut index =
            Vec::with_capacity(bounded_capacity(self.parts.edge_value_count, remaining, 9));
        if let Some(mut offset) = self.parts.edge_values_start {
            for _ in 0..self.parts.edge_value_count {
                let source = read_u32_from(self.path, self.bytes, offset)?;
                let target = read_u32_from(self.path, self.bytes, offset + 4)?;
                offset += 8;
                index.push((source, target, offset));
                offset = skip_mixed_value(self.path, self.bytes, offset)?;
            }
        }
        // Values are written sorted by (source, target); keep the invariant so
        // `edge_value` can binary-search.
        let _ = self.parts.edge_value_index.set(index);
        Ok(self
            .parts
            .edge_value_index
            .get()
            .expect("edge value index just initialized")
            .as_slice())
    }

    pub fn edge_value(
        &self,
        expected_source: u32,
        expected_target: u32,
    ) -> Result<Option<MappedNodeValue<'a>>> {
        if self.parts.edge_values_start.is_none() || self.parts.edge_value_count == 0 {
            return Ok(None);
        }
        let index = self.edge_value_index()?;
        match index.binary_search_by(|(source, target, _)| {
            (*source, *target).cmp(&(expected_source, expected_target))
        }) {
            Ok(position) => {
                let (_, _, value_offset) = index[position];
                read_mixed_node_value_from(self.path, self.bytes, value_offset).map(Some)
            }
            Err(_) => Ok(None),
        }
    }
}

pub struct EdgeTargets<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
    remaining: usize,
}

impl Iterator for EdgeTargets<'_> {
    type Item = Result<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let result = read_u32_from(self.path, self.bytes, self.offset);
        self.offset += 4;
        self.remaining -= 1;
        Some(result)
    }
}
