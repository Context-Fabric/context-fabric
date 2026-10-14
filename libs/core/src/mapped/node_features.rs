//! Node feature views (string-pool and mixed encodings) over the mapped file.

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use crate::error::Result;
use crate::format::{
    CompiledNodeFeature, bounded_capacity, invalid_compiled, metadata_value,
    read_mixed_node_value_from, read_string_range_at, read_u32_at, read_u32_from,
};
use crate::index::row_index::{RowLookup, RowOffsets};
use crate::mapped::MappedCompiledCorpus;
use crate::mapped::edge_features::mapped_row_offsets;
use crate::mapped::order::sort_nodes_by_mmap_rank;
use crate::value::FeatureValue;

/// Owned, parsed metadata for a string-pool node feature. Built once per
/// feature and shared (behind an `Arc`) across every view, so repeated
/// `string_pool_node_feature()` calls avoid re-scanning the mmap header.
pub(crate) struct StringPoolParts {
    metadata: BTreeMap<String, Option<String>>,
    pub(crate) row_count: usize,
    pub(crate) rows_start: usize,
    pub(crate) pool_ranges: Vec<Range<usize>>,
    lookup: RowLookup,
}

/// Owned, parsed metadata for a mixed (string/int) node feature.
pub(crate) struct MixedParts {
    metadata: BTreeMap<String, Option<String>>,
    row_count: usize,
    row_offsets: RowOffsets,
    lookup: RowLookup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappedNodeValue<'a> {
    Str(&'a str),
    Int(i64),
}

pub enum MappedNodeFeatureView<'a> {
    StringPool(StringPoolNodeFeatureView<'a>),
    Mixed(MixedNodeFeatureView<'a>),
}

impl<'a> MappedNodeFeatureView<'a> {
    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        match self {
            Self::StringPool(feature) => feature.metadata_value(key),
            Self::Mixed(feature) => feature.metadata_value(key),
        }
    }

    pub fn value(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        match self {
            Self::StringPool(feature) => feature.v(node),
            Self::Mixed(feature) => feature.v(node),
        }
    }

    pub fn v(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        self.value(node)
    }

    pub fn s(&self, expected: MappedNodeValue<'_>) -> Result<Vec<u32>> {
        match (self, expected) {
            (Self::StringPool(feature), MappedNodeValue::Str(expected)) => feature.s(expected),
            (Self::StringPool(_), MappedNodeValue::Int(_)) => Ok(Vec::new()),
            (Self::Mixed(feature), expected) => feature.s(expected),
        }
    }

    pub fn items(&self) -> Result<Vec<(u32, MappedNodeValue<'a>)>> {
        match self {
            Self::StringPool(feature) => feature.items(),
            Self::Mixed(feature) => feature.items(),
        }
    }

    /// Visits `(node, value)` rows in node order while `visit` returns `true`.
    pub fn for_each_row(
        &self,
        visit: &mut impl FnMut(u32, MappedNodeValue<'a>) -> Result<bool>,
    ) -> Result<()> {
        match self {
            Self::StringPool(feature) => {
                for row in feature.rows() {
                    let (node, value) = row?;
                    if !visit(node, MappedNodeValue::Str(value))? {
                        break;
                    }
                }
            }
            Self::Mixed(feature) => {
                for row in feature.rows() {
                    let (node, value) = row?;
                    if !visit(node, value)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn build_string_pool_parts(
    path: &Path,
    bytes: &[u8],
    feature: &CompiledNodeFeature,
) -> Result<StringPoolParts> {
    let mut offset = feature.payload_start;
    let pool_count = read_u32_at(path, bytes, &mut offset)? as usize;
    let mut pool_ranges = Vec::with_capacity(bounded_capacity(
        pool_count,
        feature.payload_end.saturating_sub(offset),
        4,
    ));
    for _ in 0..pool_count {
        pool_ranges.push(read_string_range_at(path, bytes, &mut offset)?);
    }
    let row_count = read_u32_at(path, bytes, &mut offset)? as usize;
    if row_count != feature.row_count {
        return Err(invalid_compiled(
            path,
            "string-pool row count does not match metadata",
        ));
    }
    Ok(StringPoolParts {
        metadata: feature.metadata.clone(),
        row_count,
        rows_start: offset,
        pool_ranges,
        lookup: RowLookup::default(),
    })
}

/// Owned, long-lived handle for a string-pool node feature. Holds the corpus
/// `Arc` plus the parsed `Arc<StringPoolParts>` so a binding object can resolve
/// the feature once and then build a thin borrowing `view()` per access without
/// re-touching the corpus view cache (`RwLock` + `HashMap`). `view()` is an
/// `Arc` pointer clone plus a few field copies.
pub struct OwnedStringPoolFeature {
    pub(super) corpus: Arc<MappedCompiledCorpus>,
    pub(super) parts: Arc<StringPoolParts>,
}

impl OwnedStringPoolFeature {
    pub fn corpus(&self) -> &MappedCompiledCorpus {
        &self.corpus
    }

    pub fn view(&self) -> StringPoolNodeFeatureView<'_> {
        StringPoolNodeFeatureView {
            path: &self.corpus.path,
            bytes: &self.corpus.mmap,
            parts: Arc::clone(&self.parts),
            rank_start: self.corpus.metadata.rank_start,
            rank_len: self.corpus.metadata.rank_len,
        }
    }

    /// Direct value read without an intermediate `view()` binding.
    pub fn str_value(&self, node: u32) -> Result<Option<String>> {
        Ok(self.view().str_value(node)?.map(str::to_string))
    }
}

/// Owned, long-lived handle for a mixed (string/int) node feature.
pub struct OwnedMixedFeature {
    pub(super) corpus: Arc<MappedCompiledCorpus>,
    pub(super) parts: Arc<MixedParts>,
}

impl OwnedMixedFeature {
    pub fn view(&self) -> MixedNodeFeatureView<'_> {
        MixedNodeFeatureView {
            path: &self.corpus.path,
            bytes: &self.corpus.mmap,
            parts: Arc::clone(&self.parts),
            rank_start: self.corpus.metadata.rank_start,
            rank_len: self.corpus.metadata.rank_len,
        }
    }
}

/// Owned, long-lived handle for a node feature of either encoding. Resolve it
/// once with [`MappedCompiledCorpus::owned_node_feature`] and read values per
/// node with no view-cache traffic.
pub enum OwnedNodeFeature {
    StringPool(OwnedStringPoolFeature),
    Mixed(OwnedMixedFeature),
}

impl OwnedNodeFeature {
    /// The node's value, borrowed from the memory map.
    pub fn value(&self, node: u32) -> Result<Option<MappedNodeValue<'_>>> {
        match self {
            Self::StringPool(feature) => feature.view().v(node),
            Self::Mixed(feature) => feature.view().value(node),
        }
    }
}

pub struct StringPoolNodeFeatureView<'a> {
    pub(super) path: &'a Path,
    pub(super) bytes: &'a [u8],
    pub(crate) parts: Arc<StringPoolParts>,
    pub(super) rank_start: usize,
    pub(super) rank_len: usize,
}

impl<'a> StringPoolNodeFeatureView<'a> {
    pub fn row_count(&self) -> usize {
        self.parts.row_count
    }

    fn sort_by_rank(&self, nodes: &mut [u32]) {
        sort_nodes_by_mmap_rank(self.path, self.bytes, self.rank_start, self.rank_len, nodes);
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.parts.metadata, key)
    }

    pub fn str_value(&self, node: u32) -> Result<Option<&'a str>> {
        Ok(self.str_value_id(node)?.map(|(_, value)| value))
    }

    /// Row index of `node` in this feature's node-sorted rows.
    fn find_row(&self, node: u32) -> Result<Option<usize>> {
        let rows_start = self.parts.rows_start;
        self.parts.lookup.find(
            self.path,
            self.bytes,
            self.parts.row_count,
            |index| rows_start + index * 8,
            node,
        )
    }

    /// The pooled string with id `pool_id`.
    pub fn pool_str(&self, pool_id: u32) -> Result<&'a str> {
        let Some(range) = self.parts.pool_ranges.get(pool_id as usize) else {
            return Err(invalid_compiled(self.path, "string pool id out of bounds"));
        };
        std::str::from_utf8(&self.bytes[range.clone()])
            .map_err(|_| invalid_compiled(self.path, "invalid utf-8 string"))
    }

    /// The highest node with a value (rows are sorted by node).
    pub fn last_node(&self) -> Result<Option<u32>> {
        match self.parts.row_count.checked_sub(1) {
            Some(last) => Ok(Some(read_u32_from(
                self.path,
                self.bytes,
                self.parts.rows_start + last * 8,
            )?)),
            None => Ok(None),
        }
    }

    /// Number of distinct pooled strings (pool ids are `0..pool_count`).
    pub fn pool_count(&self) -> usize {
        self.parts.pool_ranges.len()
    }

    /// Like [`str_value`](Self::str_value) but also returns the node's
    /// string-pool id. The pool id is a stable, dense handle for the distinct
    /// string value (callers can intern by id, group rows by id, or compare two
    /// nodes' values without touching the bytes). Single mmap binary search.
    pub fn str_value_id(&self, node: u32) -> Result<Option<(u32, &'a str)>> {
        match self.pool_id(node)? {
            Some(pool_id) => Ok(Some((pool_id, self.pool_str(pool_id)?))),
            None => Ok(None),
        }
    }

    pub fn v(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        self.str_value(node)
            .map(|value| value.map(MappedNodeValue::Str))
    }

    pub fn s(&self, expected: &str) -> Result<Vec<u32>> {
        let Some(expected_id) = self.pool_id_of(expected) else {
            return Ok(Vec::new());
        };
        // Compare pool ids, not decoded strings: one u32 compare per row.
        let mut nodes = Vec::new();
        self.for_each_id_row(&mut |node, pool_id| {
            if pool_id == expected_id {
                nodes.push(node);
            }
            Ok(true)
        })?;
        // Canonical (rank) order, matching the in-memory `NodeFeature::s`.
        self.sort_by_rank(&mut nodes);
        Ok(nodes)
    }

    /// The pool id of `value` (pooled strings are distinct), compared as raw
    /// bytes without decoding the pool.
    pub fn pool_id_of(&self, value: &str) -> Option<u32> {
        self.parts
            .pool_ranges
            .iter()
            .position(|range| self.bytes.get(range.clone()) == Some(value.as_bytes()))
            .map(|index| index as u32)
    }

    /// The node's pool id without decoding the pooled string: one binary search,
    /// two u32 reads. `None` when the node has no value.
    pub fn pool_id(&self, node: u32) -> Result<Option<u32>> {
        match self.find_row(node)? {
            Some(index) => Ok(Some(read_u32_from(
                self.path,
                self.bytes,
                self.parts.rows_start + index * 8 + 4,
            )?)),
            None => Ok(None),
        }
    }

    /// Visit every `(node, pool_id)` row without decoding the pooled strings —
    /// two u32 reads per row. `visit` returns `false` to stop early.
    pub fn for_each_id_row(&self, visit: &mut impl FnMut(u32, u32) -> Result<bool>) -> Result<()> {
        for index in 0..self.parts.row_count {
            let row_offset = self.parts.rows_start + (index * 8);
            let node = read_u32_from(self.path, self.bytes, row_offset)?;
            let pool_id = read_u32_from(self.path, self.bytes, row_offset + 4)?;
            if !visit(node, pool_id)? {
                break;
            }
        }
        Ok(())
    }

    /// The distinct values this feature can take, read straight from the string
    /// pool (one entry per distinct value) — O(distinct) with no per-node scan.
    /// For `otype` this is exactly the set of node types. Note a pooled value may
    /// in principle be unreferenced by any row, so this is a *superset* of the
    /// live values; for the warp `otype` feature the two coincide.
    pub fn distinct_values(&self) -> Result<Vec<&'a str>> {
        self.parts
            .pool_ranges
            .iter()
            .map(|range| {
                std::str::from_utf8(&self.bytes[range.clone()])
                    .map_err(|_| invalid_compiled(self.path, "invalid utf-8 string"))
            })
            .collect()
    }

    pub fn rows(&self) -> StringPoolNodeRows<'a, '_> {
        StringPoolNodeRows {
            feature: self,
            index: 0,
        }
    }

    pub fn items(&self) -> Result<Vec<(u32, MappedNodeValue<'a>)>> {
        self.rows()
            .map(|row| row.map(|(node, value)| (node, MappedNodeValue::Str(value))))
            .collect()
    }
}

pub struct StringPoolNodeRows<'a, 'view> {
    feature: &'view StringPoolNodeFeatureView<'a>,
    index: usize,
}

pub(crate) fn build_mixed_parts(
    path: &Path,
    bytes: &[u8],
    feature: &CompiledNodeFeature,
) -> Result<MixedParts> {
    let mut offset = feature.payload_start;
    let row_count = read_u32_at(path, bytes, &mut offset)? as usize;
    if row_count != feature.row_count {
        return Err(invalid_compiled(
            path,
            "mixed row count does not match metadata",
        ));
    }
    let layout = feature
        .row_index
        .ok_or_else(|| invalid_compiled(path, "mixed feature without row offsets"))?;
    let row_offsets = mapped_row_offsets(path, layout, feature.payload_start, row_count)?;
    Ok(MixedParts {
        metadata: feature.metadata.clone(),
        row_count,
        row_offsets,
        lookup: RowLookup::default(),
    })
}

pub struct MixedNodeFeatureView<'a> {
    pub(super) path: &'a Path,
    pub(super) bytes: &'a [u8],
    pub(super) parts: Arc<MixedParts>,
    pub(super) rank_start: usize,
    pub(super) rank_len: usize,
}

impl<'a> MixedNodeFeatureView<'a> {
    pub fn row_count(&self) -> usize {
        self.parts.row_count
    }

    fn sort_by_rank(&self, nodes: &mut [u32]) {
        sort_nodes_by_mmap_rank(self.path, self.bytes, self.rank_start, self.rank_len, nodes);
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.parts.metadata, key)
    }

    pub fn value(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        let row_offsets = &self.parts.row_offsets;
        let row_offset = |index| row_offsets.get(self.bytes, index);
        match self.parts.lookup.find(
            self.path,
            self.bytes,
            self.parts.row_count,
            row_offset,
            node,
        )? {
            Some(index) => {
                read_mixed_node_value_from(self.path, self.bytes, row_offset(index) + 4).map(Some)
            }
            None => Ok(None),
        }
    }

    pub fn v(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        self.value(node)
    }

    pub fn s(&self, expected: MappedNodeValue<'_>) -> Result<Vec<u32>> {
        let mut nodes = Vec::new();
        for row in self.rows() {
            let (node, value) = row?;
            if mapped_node_values_equal(value, expected) {
                nodes.push(node);
            }
        }
        // Canonical (rank) order, matching the in-memory `NodeFeature::s`.
        self.sort_by_rank(&mut nodes);
        Ok(nodes)
    }

    pub fn rows(&self) -> MixedNodeRows<'a, '_> {
        MixedNodeRows {
            feature: self,
            index: 0,
        }
    }

    pub fn items(&self) -> Result<Vec<(u32, MappedNodeValue<'a>)>> {
        self.rows().collect()
    }
}

pub struct MixedNodeRows<'a, 'view> {
    feature: &'view MixedNodeFeatureView<'a>,
    index: usize,
}

impl<'a> Iterator for MixedNodeRows<'a, '_> {
    type Item = Result<(u32, MappedNodeValue<'a>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.feature.parts.row_offsets.len() {
            return None;
        }
        let row_offset = self
            .feature
            .parts
            .row_offsets
            .get(self.feature.bytes, self.index);
        self.index += 1;
        let node = match read_u32_from(self.feature.path, self.feature.bytes, row_offset) {
            Ok(node) => node,
            Err(error) => return Some(Err(error)),
        };
        Some(
            read_mixed_node_value_from(self.feature.path, self.feature.bytes, row_offset + 4)
                .map(|value| (node, value)),
        )
    }
}

impl<'a> Iterator for StringPoolNodeRows<'a, '_> {
    type Item = Result<(u32, &'a str)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.feature.parts.row_count {
            return None;
        }
        let row_offset = self.feature.parts.rows_start + (self.index * 8);
        self.index += 1;
        let node = match read_u32_from(self.feature.path, self.feature.bytes, row_offset) {
            Ok(node) => node,
            Err(error) => return Some(Err(error)),
        };
        let pool_id = match read_u32_from(self.feature.path, self.feature.bytes, row_offset + 4) {
            Ok(pool_id) => pool_id as usize,
            Err(error) => return Some(Err(error)),
        };
        let Some(range) = self.feature.parts.pool_ranges.get(pool_id) else {
            return Some(Err(invalid_compiled(
                self.feature.path,
                "string pool id out of bounds",
            )));
        };
        Some(
            std::str::from_utf8(&self.feature.bytes[range.clone()])
                .map(|value| (node, value))
                .map_err(|_| invalid_compiled(self.feature.path, "invalid utf-8 string")),
        )
    }
}

fn mapped_node_values_equal(left: MappedNodeValue<'_>, right: MappedNodeValue<'_>) -> bool {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => left == right,
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => left == right,
        _ => false,
    }
}

impl From<MappedNodeValue<'_>> for FeatureValue {
    fn from(value: MappedNodeValue<'_>) -> Self {
        match value {
            MappedNodeValue::Str(value) => FeatureValue::string(value),
            MappedNodeValue::Int(value) => FeatureValue::Int(value),
        }
    }
}
