//! Frequency lists (`freqList`) of node and edge features.

use std::collections::HashMap;

use crate::error::{CfError, Result};
use crate::format::{NodeFeatureEncoding, invalid_compiled, read_u32_from};
use crate::mapped::MappedCompiledCorpus;
use crate::mapped::corpus::MappedLevel;
use crate::value::FeatureValue;

impl MappedCompiledCorpus {
    pub fn node_frequency_list(
        &self,
        feature_name: &str,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<(FeatureValue, usize)>> {
        let feature = self
            .metadata
            .node_feature(feature_name)
            .ok_or_else(|| CfError::MissingFeature(feature_name.to_string()))?;
        // Levels exist only to answer the `nodeTypes=` filter; the unfiltered
        // call must not pay for the full-corpus scan behind `mapped_levels`.
        let levels = match node_types {
            Some(_) => Some(self.mapped_levels()?),
            None => None,
        };
        let node_allowed = |node: u32| match &levels {
            Some(levels) => mapped_node_type_allowed(node, node_types, levels),
            None => true,
        };
        let mut counts = HashMap::<FeatureValue, usize>::new();
        match feature.encoding {
            NodeFeatureEncoding::StringPool => {
                let view = self
                    .string_pool_node_feature(feature_name)?
                    .expect("metadata existence checked above");
                // Count by pool id: one u32 read per row, no per-row string
                // decode, hash, or allocation. Each distinct value materializes
                // once at the end.
                let mut histogram = vec![0_usize; view.parts.pool_ranges.len()];
                for index in 0..view.parts.row_count {
                    let row_offset = view.parts.rows_start + (index * 8);
                    if levels.is_some() {
                        let node = read_u32_from(&self.path, &self.mmap, row_offset)?;
                        if !node_allowed(node) {
                            continue;
                        }
                    }
                    let pool_id = read_u32_from(&self.path, &self.mmap, row_offset + 4)? as usize;
                    let Some(slot) = histogram.get_mut(pool_id) else {
                        return Err(invalid_compiled(&self.path, "string pool id out of bounds"));
                    };
                    *slot += 1;
                }
                for (pool_id, count) in histogram.into_iter().enumerate() {
                    if count == 0 {
                        continue;
                    }
                    let range = view.parts.pool_ranges[pool_id].clone();
                    let value = std::str::from_utf8(&self.mmap[range])
                        .map_err(|_| invalid_compiled(&self.path, "invalid utf-8 string"))?;
                    *counts.entry(FeatureValue::string(value)).or_default() += count;
                }
            }
            NodeFeatureEncoding::Mixed => {
                let view = self
                    .mixed_node_feature(feature_name)?
                    .expect("metadata existence checked above");
                for row in view.rows() {
                    let (node, value) = row?;
                    if node_allowed(node) {
                        *counts.entry(FeatureValue::from(value)).or_default() += 1;
                    }
                }
            }
        }
        Ok(sorted_feature_value_counts(counts))
    }

    pub fn edge_frequency_list(
        &self,
        feature_name: &str,
        node_types_from: Option<&[&str]>,
        node_types_to: Option<&[&str]>,
    ) -> Result<EdgeFrequency> {
        let Some(feature_metadata) = self.metadata.edge_feature(feature_name) else {
            return Err(CfError::MissingFeature(feature_name.to_string()));
        };
        let feature = self
            .edge_feature(feature_name)?
            .expect("metadata existence checked above");
        let levels = self.mapped_levels()?;
        if feature_metadata.edge_value_count == 0 {
            let mut count = 0_usize;
            for row_offset in feature.parts.row_offsets.iter(feature.bytes) {
                let source = read_u32_from(feature.path, feature.bytes, row_offset)?;
                if !mapped_node_type_allowed(source, node_types_from, &levels) {
                    continue;
                }
                let target_count =
                    read_u32_from(feature.path, feature.bytes, row_offset + 4)? as usize;
                let targets_start = row_offset + 8;
                for index in 0..target_count {
                    let target =
                        read_u32_from(feature.path, feature.bytes, targets_start + (index * 4))?;
                    if mapped_node_type_allowed(target, node_types_to, &levels) {
                        count += 1;
                    }
                }
            }
            return Ok(EdgeFrequency::Count(count));
        }

        let mut counts = HashMap::<Option<FeatureValue>, usize>::new();
        for row_offset in feature.parts.row_offsets.iter(feature.bytes) {
            let source = read_u32_from(feature.path, feature.bytes, row_offset)?;
            if !mapped_node_type_allowed(source, node_types_from, &levels) {
                continue;
            }
            let target_count = read_u32_from(feature.path, feature.bytes, row_offset + 4)? as usize;
            let targets_start = row_offset + 8;
            for index in 0..target_count {
                let target =
                    read_u32_from(feature.path, feature.bytes, targets_start + (index * 4))?;
                if !mapped_node_type_allowed(target, node_types_to, &levels) {
                    continue;
                }
                let value = feature.edge_value(source, target)?.map(FeatureValue::from);
                *counts.entry(value).or_default() += 1;
            }
        }
        Ok(EdgeFrequency::Values(sorted_optional_feature_value_counts(
            counts,
        )))
    }
}

fn sorted_feature_value_counts(counts: HashMap<FeatureValue, usize>) -> Vec<(FeatureValue, usize)> {
    let mut rows: Vec<_> = counts.into_iter().collect();
    sort_frequency_rows(&mut rows);
    rows
}

fn sorted_optional_feature_value_counts(
    counts: HashMap<Option<FeatureValue>, usize>,
) -> Vec<(Option<FeatureValue>, usize)> {
    let mut rows: Vec<_> = counts.into_iter().collect();
    sort_optional_frequency_rows(&mut rows);
    rows
}

fn mapped_node_type_allowed(
    node: u32,
    node_types: Option<&[&str]>,
    levels: &[MappedLevel],
) -> bool {
    let Some(node_types) = node_types else {
        return true;
    };
    levels.iter().any(|level| {
        (level.min_node..=level.max_node).contains(&node)
            && node_types
                .iter()
                .any(|node_type| *node_type == level.node_type)
    })
}

/// Sorts `(value, count)` rows in Text-Fabric's `freqList` order
/// (`tf/core/nodefeature.py`): highest count first, ties by ascending value.
/// Values of one feature share a type, so the derived `Ord` compares ints
/// numerically and strings by code point, like Python.
pub(crate) fn sort_frequency_rows(rows: &mut [(FeatureValue, usize)]) {
    rows.sort_unstable_by(|(left, left_count), (right, right_count)| {
        right_count.cmp(left_count).then_with(|| left.cmp(right))
    });
}

/// [`sort_frequency_rows`] for edge values, where a missing value (`None`) sorts
/// after every value (Text-Fabric cannot order `None` against values at all).
pub(crate) fn sort_optional_frequency_rows(rows: &mut [(Option<FeatureValue>, usize)]) {
    rows.sort_unstable_by(|(left, left_count), (right, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left.is_none().cmp(&right.is_none()))
            .then_with(|| left.cmp(right))
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeFrequency {
    Count(usize),
    Values(Vec<(Option<FeatureValue>, usize)>),
}
