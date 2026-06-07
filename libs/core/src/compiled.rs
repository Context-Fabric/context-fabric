use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use memmap2::Mmap;

use crate::corpus::{
    Chunk, ChunkLengthKey, ChunkPositionKey, ComputedFeatureData, Corpus, FeatureCatalogEntry,
    FeatureDescription, FeatureKind, FeatureValueSample, LoadedFeatureInfo, LoadedFeatureKind,
    StructureTree,
};
use crate::error::{CfError, Result};
use crate::feature::{EdgeFeature, EdgeFrequency, FeatureValue, NodeFeature};
use crate::precompute::{self, StructureData, StructureHeading};

const MAGIC: &[u8; 8] = b"CFRUST02";
const FEATURE_METADATA_MAGIC: &[u8; 8] = b"CFRMETA1";
const EDGE_VALUES_MAGIC: &[u8; 8] = b"CFREDGE1";
const STRUCTURE_MAGIC: &[u8; 8] = b"CFRSTRC2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMetadata {
    pub byte_len: usize,
    pub node_features: Vec<CompiledNodeFeature>,
    pub edge_features: Vec<CompiledEdgeFeature>,
    pub config_features: Vec<CompiledConfigFeature>,
    pub order_len: usize,
    pub rank_len: usize,
    pub order_start: Option<usize>,
    pub rank_start: Option<usize>,
    pub structure_start: Option<usize>,
}

impl CompiledMetadata {
    pub fn node_feature(&self, name: &str) -> Option<&CompiledNodeFeature> {
        self.node_features
            .iter()
            .find(|feature| feature.name == name)
    }

    pub fn edge_feature(&self, name: &str) -> Option<&CompiledEdgeFeature> {
        self.edge_features
            .iter()
            .find(|feature| feature.name == name)
    }

    pub fn config_feature(&self, name: &str) -> Option<&CompiledConfigFeature> {
        self.config_features
            .iter()
            .find(|feature| feature.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNodeFeature {
    pub name: String,
    pub encoding: NodeFeatureEncoding,
    pub row_count: usize,
    pub string_pool_count: Option<usize>,
    pub payload_start: usize,
    pub payload_end: usize,
    pub metadata: BTreeMap<String, Option<String>>,
}

impl CompiledNodeFeature {
    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.metadata, key)
    }

    pub fn value_type(&self) -> Option<&str> {
        self.metadata_value("valueType")
    }

    pub fn description(&self) -> Option<&str> {
        self.metadata_value("description")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeFeatureEncoding {
    StringPool,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledEdgeFeature {
    pub name: String,
    pub row_count: usize,
    pub payload_start: usize,
    pub payload_end: usize,
    pub edge_value_count: usize,
    pub edge_values_start: Option<usize>,
    pub metadata: BTreeMap<String, Option<String>>,
}

impl CompiledEdgeFeature {
    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.metadata, key)
    }

    pub fn value_type(&self) -> Option<&str> {
        self.metadata_value("valueType")
    }

    pub fn description(&self) -> Option<&str> {
        self.metadata_value("description")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledConfigFeature {
    pub name: String,
    pub metadata_count: usize,
    pub payload_start: usize,
    pub payload_end: usize,
}

pub struct MappedCompiledCorpus {
    path: PathBuf,
    mmap: Mmap,
    #[cfg(target_os = "macos")]
    _shared_mmap_hint: Mmap,
    metadata: CompiledMetadata,
}

impl std::fmt::Debug for MappedCompiledCorpus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MappedCompiledCorpus")
            .field("path", &self.path)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

fn metadata_value<'a>(
    metadata: &'a BTreeMap<String, Option<String>>,
    key: &str,
) -> Option<&'a str> {
    metadata.get(key).and_then(Option::as_deref).or_else(|| {
        if key == "valueType" {
            metadata.get("value_type").and_then(Option::as_deref)
        } else {
            None
        }
    })
}

fn remove_value_type_metadata(metadata: &mut BTreeMap<String, Option<String>>) -> String {
    metadata
        .remove("valueType")
        .or_else(|| metadata.remove("value_type"))
        .flatten()
        .unwrap_or_default()
}

struct MappedLevel {
    node_type: String,
    average_slots: f64,
    min_node: u32,
    max_node: u32,
}

impl MappedCompiledCorpus {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mmap = map_file(path)?;
        let metadata = inspect_compiled_bytes(path, &mmap)?;
        #[cfg(target_os = "macos")]
        let shared_mmap_hint = map_file(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            mmap,
            #[cfg(target_os = "macos")]
            _shared_mmap_hint: shared_mmap_hint,
            metadata,
        })
    }

    pub fn metadata(&self) -> &CompiledMetadata {
        &self.metadata
    }

    pub fn all_node_features(&self, warp: bool) -> Vec<String> {
        self.metadata
            .node_features
            .iter()
            .filter(|feature| warp || feature.name != "otype")
            .map(|feature| feature.name.clone())
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn Fall(&self, warp: bool) -> Vec<String> {
        self.all_node_features(warp)
    }

    pub fn all_edge_features(&self, warp: bool) -> Vec<String> {
        self.metadata
            .edge_features
            .iter()
            .filter(|feature| warp || feature.name != "oslots")
            .map(|feature| feature.name.clone())
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn Eall(&self, warp: bool) -> Vec<String> {
        self.all_edge_features(warp)
    }

    pub fn all_computed_features(&self) -> Vec<String> {
        self.computed_names()
            .iter()
            .map(|name| name.to_string())
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn Call(&self) -> Vec<String> {
        self.all_computed_features()
    }

    pub fn computed_feature(&self, name: &str) -> Result<Option<ComputedFeatureData>> {
        match name {
            "levels" => Ok(Some(ComputedFeatureData::Levels(
                self.mapped_levels()?
                    .into_iter()
                    .map(|level| {
                        (
                            level.node_type,
                            level.average_slots,
                            level.min_node,
                            level.max_node,
                        )
                    })
                    .collect(),
            ))),
            "order" => self
                .order()
                .map(|order| Some(ComputedFeatureData::Order(order))),
            "rank" => self
                .rank()
                .map(|rank| Some(ComputedFeatureData::Rank(rank))),
            "boundary" => crate::mapped_sections::MappedSections::new(self)?
                .boundary()
                .map(|boundary| Some(ComputedFeatureData::Boundary(boundary))),
            _ => Ok(None),
        }
    }

    pub fn otype_rank(&self) -> Result<BTreeMap<String, u32>> {
        Ok(self
            .mapped_levels()?
            .into_iter()
            .rev()
            .enumerate()
            .map(|(rank, level)| (level.node_type, rank as u32))
            .collect())
    }

    #[allow(non_snake_case)]
    pub fn otypeRank(&self) -> Result<BTreeMap<String, u32>> {
        self.otype_rank()
    }

    pub fn max_node(&self) -> u32 {
        self.metadata.rank_len as u32
    }

    #[allow(non_snake_case)]
    pub fn maxNode(&self) -> u32 {
        self.max_node()
    }

    pub fn slot_type(&self) -> Result<String> {
        let otype = self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        Ok(otype.str_value(1)?.unwrap_or_default().to_string())
    }

    pub fn search(&self) -> crate::mapped_search::MappedSearch<'_> {
        crate::mapped_search::MappedSearch::new(self)
    }

    pub fn nodes_of_type(&self, node_type: &str) -> Result<Vec<u32>> {
        Ok(self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?
            .s(node_type)?)
    }

    pub fn text(&self, node: u32, format: Option<&str>) -> Result<String> {
        crate::mapped_text::MappedText::new(self)?.text(node, format)
    }

    pub fn structure_data(&self) -> Result<Option<StructureData>> {
        let Some(structure_start) = self.metadata.structure_start else {
            return Ok(None);
        };
        let mut reader = Reader {
            path: &self.path,
            bytes: &self.mmap,
            offset: structure_start,
        };
        reader.read_structure_section()
    }

    #[allow(non_snake_case)]
    pub fn structureData(&self) -> Result<Option<StructureData>> {
        self.structure_data()
    }

    pub fn structure(&self, node: Option<u32>) -> Result<Option<StructureTree>> {
        let Some(data) = self.structure_data()? else {
            return Ok(None);
        };
        Ok(match node {
            Some(node) => structure_tree_from_data(node, &data),
            None => Some(StructureTree::Forest(
                data.top
                    .iter()
                    .filter_map(|node| structure_tree_from_data(*node, &data))
                    .collect(),
            )),
        })
    }

    pub fn top(&self) -> Result<Option<Vec<u32>>> {
        Ok(self.structure_data()?.map(|data| data.top))
    }

    pub fn heading_from_node(&self, node: u32) -> Result<Option<Vec<StructureHeading>>> {
        Ok(self
            .structure_data()?
            .and_then(|data| data.heading_from_node.get(&node).cloned()))
    }

    #[allow(non_snake_case)]
    pub fn headingFromNode(&self, node: u32) -> Result<Option<Vec<StructureHeading>>> {
        self.heading_from_node(node)
    }

    pub fn node_from_heading(&self, heading: &[StructureHeading]) -> Result<Option<u32>> {
        Ok(self
            .structure_data()?
            .and_then(|data| data.node_from_heading.get(heading).copied()))
    }

    #[allow(non_snake_case)]
    pub fn nodeFromHeading(&self, heading: &[StructureHeading]) -> Result<Option<u32>> {
        self.node_from_heading(heading)
    }

    pub fn structure_pretty(
        &self,
        node: Option<u32>,
        full_heading: bool,
    ) -> Result<Option<String>> {
        let Some(data) = self.structure_data()? else {
            return Ok(None);
        };
        let tree = match node {
            Some(node) => structure_tree_from_data(node, &data),
            None => Some(StructureTree::Forest(
                data.top
                    .iter()
                    .filter_map(|node| structure_tree_from_data(*node, &data))
                    .collect(),
            )),
        };
        let Some(tree) = tree else {
            return Ok(None);
        };
        let mut lines = Vec::new();
        structure_pretty_lines(&tree, &data, full_heading, "  ", &mut lines);
        Ok(Some(lines.join("\n")))
    }

    #[allow(non_snake_case)]
    pub fn structurePretty(
        &self,
        node: Option<u32>,
        full_heading: bool,
    ) -> Result<Option<String>> {
        self.structure_pretty(node, full_heading)
    }

    #[allow(non_snake_case)]
    pub fn slotType(&self) -> Result<String> {
        self.slot_type()
    }

    pub fn max_slot(&self) -> Result<u32> {
        let slot_type = self.slot_type()?;
        Ok(self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?
            .value_interval(&slot_type)?
            .map(|(_, max_slot)| max_slot)
            .unwrap_or_default())
    }

    #[allow(non_snake_case)]
    pub fn maxSlot(&self) -> Result<u32> {
        self.max_slot()
    }

    #[allow(non_snake_case)]
    pub fn Cs(&self, name: &str) -> Result<Option<ComputedFeatureData>> {
        self.computed_feature(name)
    }

    pub fn is_loaded(
        &self,
        features: Option<&[&str]>,
    ) -> Result<BTreeMap<String, Option<LoadedFeatureInfo>>> {
        let feature_names = match features {
            Some(features) => features.iter().map(|name| (*name).to_string()).collect(),
            None => self.loaded_feature_names(),
        };
        let mut loaded = BTreeMap::new();
        for name in feature_names {
            let info = self.loaded_feature_info(&name)?;
            loaded.insert(name, info);
        }
        Ok(loaded)
    }

    #[allow(non_snake_case)]
    pub fn isLoaded(
        &self,
        features: Option<&[&str]>,
    ) -> Result<BTreeMap<String, Option<LoadedFeatureInfo>>> {
        self.is_loaded(features)
    }

    pub fn node_feature_types(&self, feature_name: &str) -> Result<Vec<String>> {
        if self.metadata.node_feature(feature_name).is_none() {
            return Ok(Vec::new());
        }
        let nodes = self.node_feature_row_nodes(feature_name)?;
        let mut node_types = Vec::new();
        for level in self.mapped_levels()? {
            if nodes
                .iter()
                .any(|node| (level.min_node..=level.max_node).contains(node))
            {
                node_types.push(level.node_type);
            }
        }
        Ok(node_types)
    }

    pub fn edge_feature_source_types(&self, feature_name: &str) -> Result<Vec<String>> {
        let Some(feature) = self.edge_feature(feature_name)? else {
            return Ok(Vec::new());
        };
        let mut source_nodes = Vec::new();
        for row_offset in &feature.row_offsets {
            source_nodes.push(read_u32_from(feature.path, feature.bytes, *row_offset)?);
        }
        let mut node_types = Vec::new();
        for level in self.mapped_levels()? {
            if source_nodes
                .iter()
                .any(|node| (level.min_node..=level.max_node).contains(node))
            {
                node_types.push(level.node_type);
            }
        }
        Ok(node_types)
    }

    pub fn edge_feature_target_types(&self, feature_name: &str) -> Result<Vec<String>> {
        let Some(feature) = self.edge_feature(feature_name)? else {
            return Ok(Vec::new());
        };
        let mut target_nodes = Vec::new();
        for row_offset in &feature.row_offsets {
            let target_count = read_u32_from(feature.path, feature.bytes, row_offset + 4)? as usize;
            let targets_start = row_offset + 8;
            for index in 0..target_count {
                target_nodes.push(read_u32_from(
                    feature.path,
                    feature.bytes,
                    targets_start + (index * 4),
                )?);
            }
        }
        let mut node_types = Vec::new();
        for level in self.mapped_levels()? {
            if target_nodes
                .iter()
                .any(|node| (level.min_node..=level.max_node).contains(node))
            {
                node_types.push(level.node_type);
            }
        }
        Ok(node_types)
    }

    pub fn all_node_feature_types(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut rows = BTreeMap::new();
        for feature in &self.metadata.node_features {
            rows.insert(
                feature.name.clone(),
                self.node_feature_types(&feature.name)?,
            );
        }
        Ok(rows)
    }

    pub fn all_edge_feature_source_types(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut rows = BTreeMap::new();
        for feature in &self.metadata.edge_features {
            rows.insert(
                feature.name.clone(),
                self.edge_feature_source_types(&feature.name)?,
            );
        }
        Ok(rows)
    }

    pub fn all_edge_feature_target_types(&self) -> Result<BTreeMap<String, Vec<String>>> {
        let mut rows = BTreeMap::new();
        for feature in &self.metadata.edge_features {
            rows.insert(
                feature.name.clone(),
                self.edge_feature_target_types(&feature.name)?,
            );
        }
        Ok(rows)
    }

    pub fn feature_catalog(
        &self,
        kind: Option<FeatureKind>,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<FeatureCatalogEntry>> {
        let mut rows = Vec::new();
        if kind.is_none_or(|kind| kind == FeatureKind::Node) {
            for feature in &self.metadata.node_features {
                let applies_to_requested_type = match node_types {
                    Some(requested) => {
                        let feature_types = self.node_feature_types(&feature.name)?;
                        requested
                            .iter()
                            .any(|node_type| feature_types.iter().any(|item| item == node_type))
                    }
                    None => true,
                };
                if applies_to_requested_type {
                    rows.push(FeatureCatalogEntry {
                        name: feature.name.clone(),
                        kind: FeatureKind::Node,
                        value_type: feature.value_type().unwrap_or("str").to_string(),
                        description: feature.description().unwrap_or_default().to_string(),
                    });
                }
            }
        }
        if kind.is_none_or(|kind| kind == FeatureKind::Edge) {
            for feature in &self.metadata.edge_features {
                let applies_to_requested_type = match node_types {
                    Some(requested) => {
                        let feature_types = self.edge_feature_source_types(&feature.name)?;
                        requested
                            .iter()
                            .any(|node_type| feature_types.iter().any(|item| item == node_type))
                    }
                    None => true,
                };
                if applies_to_requested_type {
                    rows.push(FeatureCatalogEntry {
                        name: feature.name.clone(),
                        kind: FeatureKind::Edge,
                        value_type: feature.value_type().unwrap_or("str").to_string(),
                        description: feature.description().unwrap_or_default().to_string(),
                    });
                }
            }
        }
        Ok(rows)
    }

    pub fn describe_feature(
        &self,
        feature_name: &str,
        sample_limit: usize,
    ) -> Result<FeatureDescription> {
        if let Some(feature) = self.metadata.node_feature(feature_name) {
            let frequency_list = self.node_frequency_list(feature_name, None)?;
            return Ok(FeatureDescription {
                name: feature.name.clone(),
                kind: Some(FeatureKind::Node),
                value_type: feature.value_type().unwrap_or("str").to_string(),
                description: feature.description().unwrap_or_default().to_string(),
                node_types: self.node_feature_types(feature_name)?,
                unique_values: frequency_list.len(),
                sample_values: frequency_list
                    .into_iter()
                    .take(sample_limit)
                    .map(|(value, count)| FeatureValueSample {
                        value: Some(value),
                        count,
                    })
                    .collect(),
                has_values: None,
                error: None,
            });
        }

        if let Some(feature) = self.metadata.edge_feature(feature_name) {
            let has_values = feature.edge_value_count > 0;
            let (unique_values, sample_values) = if has_values {
                match self.edge_frequency_list(feature_name, None, None)? {
                    EdgeFrequency::Values(frequency_list) => {
                        let unique_values = frequency_list.len();
                        let sample_values = frequency_list
                            .into_iter()
                            .take(sample_limit)
                            .map(|(value, count)| FeatureValueSample { value, count })
                            .collect();
                        (unique_values, sample_values)
                    }
                    EdgeFrequency::Count(_) => (0, Vec::new()),
                }
            } else {
                (0, Vec::new())
            };
            return Ok(FeatureDescription {
                name: feature.name.clone(),
                kind: Some(FeatureKind::Edge),
                value_type: feature.value_type().unwrap_or("str").to_string(),
                description: feature.description().unwrap_or_default().to_string(),
                node_types: self.edge_feature_source_types(feature_name)?,
                unique_values,
                sample_values,
                has_values: Some(has_values),
                error: None,
            });
        }

        Ok(FeatureDescription {
            name: feature_name.to_string(),
            kind: None,
            value_type: String::new(),
            description: String::new(),
            node_types: Vec::new(),
            unique_values: 0,
            sample_values: Vec::new(),
            has_values: None,
            error: Some(format!("Feature '{feature_name}' not found")),
        })
    }

    pub fn describe_features(
        &self,
        feature_names: &[&str],
        sample_limit: usize,
    ) -> Result<BTreeMap<String, FeatureDescription>> {
        let mut descriptions = BTreeMap::new();
        for feature_name in feature_names {
            descriptions.insert(
                (*feature_name).to_string(),
                self.describe_feature(feature_name, sample_limit)?,
            );
        }
        Ok(descriptions)
    }

    pub fn node_frequency_list(
        &self,
        feature_name: &str,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<(FeatureValue, usize)>> {
        let feature = self
            .metadata
            .node_feature(feature_name)
            .ok_or_else(|| CfError::MissingFeature(feature_name.to_string()))?;
        let levels = self.mapped_levels()?;
        let mut counts = HashMap::<FeatureValue, usize>::new();
        match feature.encoding {
            NodeFeatureEncoding::StringPool => {
                let view = self
                    .string_pool_node_feature(feature_name)?
                    .expect("metadata existence checked above");
                for row in view.rows() {
                    let (node, value) = row?;
                    if mapped_node_type_allowed(node, node_types, &levels) {
                        *counts.entry(FeatureValue::string(value)).or_default() += 1;
                    }
                }
            }
            NodeFeatureEncoding::Mixed => {
                let view = self
                    .mixed_node_feature(feature_name)?
                    .expect("metadata existence checked above");
                for row in view.rows() {
                    let (node, value) = row?;
                    if mapped_node_type_allowed(node, node_types, &levels) {
                        *counts
                            .entry(mapped_value_to_feature_value(value))
                            .or_default() += 1;
                    }
                }
            }
        }
        Ok(sorted_feature_value_counts(counts))
    }

    pub fn node_freq_list(
        &self,
        feature_name: &str,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<(FeatureValue, usize)>> {
        self.node_frequency_list(feature_name, node_types)
    }

    #[allow(non_snake_case)]
    pub fn node_freqList(
        &self,
        feature_name: &str,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<(FeatureValue, usize)>> {
        self.node_frequency_list(feature_name, node_types)
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
            for row_offset in &feature.row_offsets {
                let source = read_u32_from(feature.path, feature.bytes, *row_offset)?;
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
        for row_offset in &feature.row_offsets {
            let source = read_u32_from(feature.path, feature.bytes, *row_offset)?;
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
                let value = feature
                    .edge_value(source, target)?
                    .map(mapped_value_to_feature_value);
                *counts.entry(value).or_default() += 1;
            }
        }
        Ok(EdgeFrequency::Values(sorted_optional_feature_value_counts(
            counts,
        )))
    }

    pub fn edge_freq_list(
        &self,
        feature_name: &str,
        node_types_from: Option<&[&str]>,
        node_types_to: Option<&[&str]>,
    ) -> Result<EdgeFrequency> {
        self.edge_frequency_list(feature_name, node_types_from, node_types_to)
    }

    #[allow(non_snake_case)]
    pub fn edge_freqList(
        &self,
        feature_name: &str,
        node_types_from: Option<&[&str]>,
        node_types_to: Option<&[&str]>,
    ) -> Result<EdgeFrequency> {
        self.edge_frequency_list(feature_name, node_types_from, node_types_to)
    }

    pub fn string_pool_node_feature(
        &self,
        name: &str,
    ) -> Result<Option<StringPoolNodeFeatureView<'_>>> {
        let Some(feature) = self.metadata.node_feature(name) else {
            return Ok(None);
        };
        if feature.encoding != NodeFeatureEncoding::StringPool {
            return Ok(None);
        }
        StringPoolNodeFeatureView::new(&self.path, &self.mmap, feature).map(Some)
    }

    pub fn mixed_node_feature(&self, name: &str) -> Result<Option<MixedNodeFeatureView<'_>>> {
        let Some(feature) = self.metadata.node_feature(name) else {
            return Ok(None);
        };
        if feature.encoding != NodeFeatureEncoding::Mixed {
            return Ok(None);
        }
        MixedNodeFeatureView::new(&self.path, &self.mmap, feature).map(Some)
    }

    pub fn node_feature(&self, name: &str) -> Result<Option<MappedNodeFeatureView<'_>>> {
        let Some(feature) = self.metadata.node_feature(name) else {
            return Ok(None);
        };
        match feature.encoding {
            NodeFeatureEncoding::StringPool => self
                .string_pool_node_feature(name)
                .map(|feature| feature.map(MappedNodeFeatureView::StringPool)),
            NodeFeatureEncoding::Mixed => self
                .mixed_node_feature(name)
                .map(|feature| feature.map(MappedNodeFeatureView::Mixed)),
        }
    }

    #[allow(non_snake_case)]
    pub fn Fs(&self, name: &str) -> Result<Option<MappedNodeFeatureView<'_>>> {
        self.node_feature(name)
    }

    pub fn edge_feature(&self, name: &str) -> Result<Option<EdgeFeatureView<'_>>> {
        let Some(feature) = self.metadata.edge_feature(name) else {
            return Ok(None);
        };
        EdgeFeatureView::new(&self.path, &self.mmap, feature, self.rank()?).map(Some)
    }

    #[allow(non_snake_case)]
    pub fn Es(&self, name: &str) -> Result<Option<EdgeFeatureView<'_>>> {
        self.edge_feature(name)
    }

    pub fn config_feature(&self, name: &str) -> Result<Option<ConfigFeatureView<'_>>> {
        let Some(feature) = self.metadata.config_feature(name) else {
            return Ok(None);
        };
        ConfigFeatureView::new(&self.path, &self.mmap, feature).map(Some)
    }

    fn computed_names(&self) -> [&'static str; 4] {
        ["levels", "order", "rank", "boundary"]
    }

    fn loaded_feature_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        names.extend(
            self.metadata
                .node_features
                .iter()
                .map(|feature| feature.name.clone()),
        );
        names.extend(
            self.metadata
                .edge_features
                .iter()
                .map(|feature| feature.name.clone()),
        );
        names.extend(
            self.metadata
                .config_features
                .iter()
                .map(|feature| feature.name.clone()),
        );
        names.extend(self.computed_names().iter().map(|name| name.to_string()));
        names.into_iter().collect()
    }

    fn loaded_feature_info(&self, name: &str) -> Result<Option<LoadedFeatureInfo>> {
        if let Some(feature) = self.metadata.node_feature(name) {
            let mut metadata = feature.metadata.clone();
            let value_type = remove_value_type_metadata(&mut metadata);
            return Ok(Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Node,
                value_type,
                metadata,
                edge_values: None,
            }));
        }
        if let Some(feature) = self.metadata.edge_feature(name) {
            let mut metadata = feature.metadata.clone();
            let value_type = remove_value_type_metadata(&mut metadata);
            return Ok(Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Edge,
                value_type,
                metadata,
                edge_values: Some(name != "oslots" && feature.edge_value_count > 0),
            }));
        }
        if self.metadata.config_feature(name).is_some() {
            let metadata = self
                .config_feature(name)?
                .expect("metadata existence checked above")
                .metadata()?;
            let mut metadata = metadata;
            let value_type = remove_value_type_metadata(&mut metadata);
            return Ok(Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Config,
                value_type,
                metadata,
                edge_values: None,
            }));
        }
        if self.computed_names().contains(&name) {
            return Ok(Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Computed,
                value_type: String::new(),
                metadata: BTreeMap::new(),
                edge_values: None,
            }));
        }
        Ok(None)
    }

    fn node_feature_row_nodes(&self, feature_name: &str) -> Result<Vec<u32>> {
        let Some(feature) = self.metadata.node_feature(feature_name) else {
            return Ok(Vec::new());
        };
        match feature.encoding {
            NodeFeatureEncoding::StringPool => self
                .string_pool_node_feature(feature_name)?
                .expect("metadata existence checked above")
                .rows()
                .map(|row| row.map(|(node, _)| node))
                .collect(),
            NodeFeatureEncoding::Mixed => self
                .mixed_node_feature(feature_name)?
                .expect("metadata existence checked above")
                .rows()
                .map(|row| row.map(|(node, _)| node))
                .collect(),
        }
    }

    fn mapped_levels(&self) -> Result<Vec<MappedLevel>> {
        let otype = self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = self
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype.str_value(1)?.unwrap_or_default().to_string();
        let mut rows = BTreeMap::<String, (usize, usize, u32, u32)>::new();

        for row in otype.rows() {
            let (node, node_type) = row?;
            let slot_count = if node_type == slot_type {
                1
            } else {
                oslots.s(node)?.len()
            };
            rows.entry(node_type.to_string())
                .and_modify(|(count, total_slots, min_node, max_node)| {
                    *count += 1;
                    *total_slots += slot_count;
                    *min_node = (*min_node).min(node);
                    *max_node = (*max_node).max(node);
                })
                .or_insert((1, slot_count, node, node));
        }

        let mut levels = rows
            .into_iter()
            .map(
                |(node_type, (count, total_slots, min_node, max_node))| MappedLevel {
                    node_type,
                    average_slots: total_slots as f64 / count.max(1) as f64,
                    min_node,
                    max_node,
                },
            )
            .collect::<Vec<_>>();
        levels.sort_by(|left, right| {
            right
                .average_slots
                .total_cmp(&left.average_slots)
                .then_with(|| right.min_node.cmp(&left.min_node))
        });
        Ok(levels)
    }

    pub fn order(&self) -> Result<Vec<u32>> {
        let Some(order_start) = self.metadata.order_start else {
            return Ok(Vec::new());
        };
        (0..self.metadata.order_len)
            .map(|index| read_u32_from(&self.path, &self.mmap, order_start + (index * 4)))
            .collect()
    }

    pub fn rank(&self) -> Result<Vec<u32>> {
        let Some(rank_start) = self.metadata.rank_start else {
            return Ok(Vec::new());
        };
        (0..self.metadata.rank_len)
            .map(|index| read_u32_from(&self.path, &self.mmap, rank_start + (index * 4)))
            .collect()
    }

    pub fn sort_key(&self, node: u32) -> Result<Option<u32>> {
        let Some(rank_start) = self.metadata.rank_start else {
            return Ok(None);
        };
        if node == 0 || node as usize > self.metadata.rank_len {
            return Ok(None);
        }
        read_u32_from(
            &self.path,
            &self.mmap,
            rank_start + ((node as usize - 1) * 4),
        )
        .map(Some)
    }

    #[allow(non_snake_case)]
    pub fn sortKey(&self, node: u32) -> Result<Option<u32>> {
        self.sort_key(node)
    }

    pub fn sort_key_tuple(&self, nodes: &[u32]) -> Result<Vec<Option<u32>>> {
        nodes.iter().map(|node| self.sort_key(*node)).collect()
    }

    #[allow(non_snake_case)]
    pub fn sortKeyTuple(&self, nodes: &[u32]) -> Result<Vec<Option<u32>>> {
        self.sort_key_tuple(nodes)
    }

    pub fn sort_nodes(&self, nodes: &mut [u32]) -> Result<()> {
        let Some(rank_start) = self.metadata.rank_start else {
            nodes.sort_unstable();
            return Ok(());
        };
        nodes.sort_unstable_by_key(|node| {
            if *node == 0 || *node as usize > self.metadata.rank_len {
                return u32::MAX;
            }
            read_u32_from(
                &self.path,
                &self.mmap,
                rank_start + ((*node as usize - 1) * 4),
            )
            .unwrap_or(u32::MAX)
        });
        Ok(())
    }

    pub fn sorted_nodes(&self, nodes: impl IntoIterator<Item = u32>) -> Result<Vec<u32>> {
        let mut nodes = nodes.into_iter().collect::<Vec<_>>();
        self.sort_nodes(&mut nodes)?;
        Ok(nodes)
    }

    #[allow(non_snake_case)]
    pub fn sortNodes(&self, nodes: impl IntoIterator<Item = u32>) -> Result<Vec<u32>> {
        self.sorted_nodes(nodes)
    }

    pub fn sort_key_chunk(&self, chunk: Chunk) -> Result<ChunkPositionKey> {
        Ok(ChunkPositionKey::new(
            chunk.start,
            self.type_rank(chunk.node)?,
            chunk.end,
            chunk.node,
        ))
    }

    #[allow(non_snake_case)]
    pub fn sortKeyChunk(&self, chunk: Chunk) -> Result<ChunkPositionKey> {
        self.sort_key_chunk(chunk)
    }

    pub fn sort_key_chunk_length(&self, chunk: Chunk) -> Result<ChunkLengthKey> {
        Ok(ChunkLengthKey::new(
            chunk.end.saturating_sub(chunk.start),
            chunk.start,
            self.type_rank(chunk.node)?,
            chunk.node,
        ))
    }

    #[allow(non_snake_case)]
    pub fn sortKeyChunkLength(&self, chunk: Chunk) -> Result<ChunkLengthKey> {
        self.sort_key_chunk_length(chunk)
    }

    fn type_rank(&self, node: u32) -> Result<u32> {
        let node_type = self
            .string_pool_node_feature("otype")?
            .and_then(|otype| otype.str_value(node).transpose())
            .transpose()?
            .map(str::to_string);
        let levels = self.mapped_levels()?;
        let fallback = levels
            .last()
            .map(|level| level.node_type.as_str())
            .unwrap_or_default();
        let node_type = node_type.as_deref().unwrap_or(fallback);
        Ok(levels
            .iter()
            .rev()
            .position(|level| level.node_type == node_type)
            .map(|rank| rank as u32)
            .unwrap_or(u32::MAX))
    }
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
    pub fn row_count(&self) -> usize {
        match self {
            Self::StringPool(feature) => feature.row_count(),
            Self::Mixed(feature) => feature.row_count(),
        }
    }

    pub fn meta(&self) -> &BTreeMap<String, Option<String>> {
        match self {
            Self::StringPool(feature) => feature.meta(),
            Self::Mixed(feature) => feature.meta(),
        }
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        match self {
            Self::StringPool(feature) => feature.metadata_value(key),
            Self::Mixed(feature) => feature.metadata_value(key),
        }
    }

    pub fn value_type(&self) -> Option<&str> {
        self.metadata_value("valueType")
    }

    #[allow(non_snake_case)]
    pub fn valueType(&self) -> Option<&str> {
        self.value_type()
    }

    pub fn description(&self) -> Option<&str> {
        self.metadata_value("description")
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

    pub fn nodes_with_value(&self, expected: MappedNodeValue<'_>) -> Result<Vec<u32>> {
        self.s(expected)
    }

    pub fn select(&self, expected: MappedNodeValue<'_>) -> Result<Vec<u32>> {
        self.nodes_with_value(expected)
    }

    pub fn filter_by_value(
        &self,
        nodes: &[u32],
        expected: MappedNodeValue<'_>,
    ) -> Result<Vec<u32>> {
        match (self, expected) {
            (Self::StringPool(feature), MappedNodeValue::Str(expected)) => {
                feature.filter_by_value(nodes, expected)
            }
            (Self::StringPool(_), MappedNodeValue::Int(_)) => Ok(Vec::new()),
            (Self::Mixed(feature), expected) => feature.filter_by_value(nodes, expected),
        }
    }

    pub fn filter_by_values(
        &self,
        nodes: &[u32],
        expected: &[MappedNodeValue<'_>],
    ) -> Result<Vec<u32>> {
        match self {
            Self::StringPool(feature) => {
                let strings: Vec<_> = expected
                    .iter()
                    .filter_map(|value| match value {
                        MappedNodeValue::Str(value) => Some(*value),
                        MappedNodeValue::Int(_) => None,
                    })
                    .collect();
                feature.filter_by_values(nodes, &strings)
            }
            Self::Mixed(feature) => feature.filter_by_values(nodes, expected),
        }
    }

    pub fn filter_has_value(&self, nodes: &[u32]) -> Result<Vec<u32>> {
        match self {
            Self::StringPool(feature) => feature.filter_has_value(nodes),
            Self::Mixed(feature) => feature.filter_has_value(nodes),
        }
    }

    pub fn filter_missing_value(&self, nodes: &[u32]) -> Result<Vec<u32>> {
        match self {
            Self::StringPool(feature) => feature.filter_missing_value(nodes),
            Self::Mixed(feature) => feature.filter_missing_value(nodes),
        }
    }

    pub fn filter_less_than(
        &self,
        nodes: &[u32],
        expected: MappedNodeValue<'_>,
    ) -> Result<Vec<u32>> {
        match (self, expected) {
            (Self::StringPool(feature), MappedNodeValue::Str(expected)) => {
                feature.filter_less_than(nodes, expected)
            }
            (Self::StringPool(_), MappedNodeValue::Int(_)) => Ok(Vec::new()),
            (Self::Mixed(feature), expected) => feature.filter_less_than(nodes, expected),
        }
    }

    pub fn filter_greater_than(
        &self,
        nodes: &[u32],
        expected: MappedNodeValue<'_>,
    ) -> Result<Vec<u32>> {
        match (self, expected) {
            (Self::StringPool(feature), MappedNodeValue::Str(expected)) => {
                feature.filter_greater_than(nodes, expected)
            }
            (Self::StringPool(_), MappedNodeValue::Int(_)) => Ok(Vec::new()),
            (Self::Mixed(feature), expected) => feature.filter_greater_than(nodes, expected),
        }
    }

    pub fn value_interval(&self, expected: MappedNodeValue<'_>) -> Result<Option<(u32, u32)>> {
        match (self, expected) {
            (Self::StringPool(feature), MappedNodeValue::Str(expected)) => {
                feature.value_interval(expected)
            }
            (Self::StringPool(_), MappedNodeValue::Int(_)) => Ok(None),
            (Self::Mixed(feature), expected) => feature.value_interval(expected),
        }
    }

    #[allow(non_snake_case)]
    pub fn sInterval(&self, expected: MappedNodeValue<'_>) -> Result<Option<(u32, u32)>> {
        self.value_interval(expected)
    }

    pub fn items(&self) -> Result<Vec<(u32, MappedNodeValue<'a>)>> {
        match self {
            Self::StringPool(feature) => feature.items(),
            Self::Mixed(feature) => feature.items(),
        }
    }
}

pub struct StringPoolNodeFeatureView<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: BTreeMap<String, Option<String>>,
    row_count: usize,
    rows_start: usize,
    pool_ranges: Vec<Range<usize>>,
}

impl<'a> StringPoolNodeFeatureView<'a> {
    fn new(path: &'a Path, bytes: &'a [u8], feature: &CompiledNodeFeature) -> Result<Self> {
        let mut offset = feature.payload_start;
        let pool_count = read_u32_at(path, bytes, &mut offset)? as usize;
        let mut pool_ranges = Vec::with_capacity(pool_count);
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
        Ok(Self {
            path,
            bytes,
            metadata: feature.metadata.clone(),
            row_count,
            rows_start: offset,
            pool_ranges,
        })
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn meta(&self) -> &BTreeMap<String, Option<String>> {
        &self.metadata
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.metadata, key)
    }

    pub fn value_type(&self) -> Option<&str> {
        self.metadata_value("valueType")
    }

    #[allow(non_snake_case)]
    pub fn valueType(&self) -> Option<&str> {
        self.value_type()
    }

    pub fn description(&self) -> Option<&str> {
        self.metadata_value("description")
    }

    pub fn str_value(&self, node: u32) -> Result<Option<&'a str>> {
        let mut low = 0_usize;
        let mut high = self.row_count;
        while low < high {
            let mid = (low + high) / 2;
            let row_offset = self.rows_start + (mid * 8);
            let row_node = read_u32_from(self.path, self.bytes, row_offset)?;
            match row_node.cmp(&node) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => {
                    let pool_id = read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
                    let Some(range) = self.pool_ranges.get(pool_id) else {
                        return Err(invalid_compiled(self.path, "string pool id out of bounds"));
                    };
                    return std::str::from_utf8(&self.bytes[range.clone()])
                        .map(Some)
                        .map_err(|_| invalid_compiled(self.path, "invalid utf-8 string"));
                }
            }
        }
        Ok(None)
    }

    pub fn v(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        self.str_value(node)
            .map(|value| value.map(MappedNodeValue::Str))
    }

    pub fn s(&self, expected: &str) -> Result<Vec<u32>> {
        let mut nodes = Vec::new();
        for row in self.rows() {
            let (node, value) = row?;
            if value == expected {
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    pub fn nodes_with_value(&self, expected: &str) -> Result<Vec<u32>> {
        self.s(expected)
    }

    pub fn select(&self, expected: &str) -> Result<Vec<u32>> {
        self.nodes_with_value(expected)
    }

    pub fn filter_by_value(&self, nodes: &[u32], expected: &str) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.str_value(*node)? == Some(expected) {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_by_values(&self, nodes: &[u32], expected: &[&str]) -> Result<Vec<u32>> {
        if expected.is_empty() {
            return Ok(Vec::new());
        }
        let mut matches = Vec::new();
        for node in nodes {
            if self
                .str_value(*node)?
                .is_some_and(|value| expected.contains(&value))
            {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_has_value(&self, nodes: &[u32]) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.str_value(*node)?.is_some() {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_missing_value(&self, nodes: &[u32]) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.str_value(*node)?.is_none() {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_less_than(&self, nodes: &[u32], expected: &str) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.str_value(*node)?.is_some_and(|value| value < expected) {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_greater_than(&self, nodes: &[u32], expected: &str) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.str_value(*node)?.is_some_and(|value| value > expected) {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn value_interval(&self, expected: &str) -> Result<Option<(u32, u32)>> {
        let mut interval: Option<(u32, u32)> = None;
        for row in self.rows() {
            let (node, value) = row?;
            if value == expected {
                interval = Some(match interval {
                    Some((first, _)) => (first, node),
                    None => (node, node),
                });
            }
        }
        Ok(interval)
    }

    #[allow(non_snake_case)]
    pub fn sInterval(&self, expected: &str) -> Result<Option<(u32, u32)>> {
        self.value_interval(expected)
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

pub struct MixedNodeFeatureView<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: BTreeMap<String, Option<String>>,
    row_count: usize,
    row_offsets: Vec<usize>,
}

impl<'a> MixedNodeFeatureView<'a> {
    fn new(path: &'a Path, bytes: &'a [u8], feature: &CompiledNodeFeature) -> Result<Self> {
        let mut offset = feature.payload_start;
        let row_count = read_u32_at(path, bytes, &mut offset)? as usize;
        if row_count != feature.row_count {
            return Err(invalid_compiled(
                path,
                "mixed row count does not match metadata",
            ));
        }
        let mut row_offsets = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            row_offsets.push(offset);
            offset = skip_mixed_node_row(path, bytes, offset)?;
        }
        Ok(Self {
            path,
            bytes,
            metadata: feature.metadata.clone(),
            row_count,
            row_offsets,
        })
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn meta(&self) -> &BTreeMap<String, Option<String>> {
        &self.metadata
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.metadata, key)
    }

    pub fn value_type(&self) -> Option<&str> {
        self.metadata_value("valueType")
    }

    #[allow(non_snake_case)]
    pub fn valueType(&self) -> Option<&str> {
        self.value_type()
    }

    pub fn description(&self) -> Option<&str> {
        self.metadata_value("description")
    }

    pub fn value(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        let mut low = 0_usize;
        let mut high = self.row_count;
        while low < high {
            let mid = (low + high) / 2;
            let row_offset = self.row_offsets[mid];
            let row_node = read_u32_from(self.path, self.bytes, row_offset)?;
            match row_node.cmp(&node) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => {
                    return read_mixed_node_value_from(self.path, self.bytes, row_offset + 4)
                        .map(Some);
                }
            }
        }
        Ok(None)
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
        Ok(nodes)
    }

    pub fn nodes_with_value(&self, expected: MappedNodeValue<'_>) -> Result<Vec<u32>> {
        self.s(expected)
    }

    pub fn select(&self, expected: MappedNodeValue<'_>) -> Result<Vec<u32>> {
        self.nodes_with_value(expected)
    }

    pub fn filter_by_value(
        &self,
        nodes: &[u32],
        expected: MappedNodeValue<'_>,
    ) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self
                .value(*node)?
                .is_some_and(|value| mapped_node_values_equal(value, expected))
            {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_by_values(
        &self,
        nodes: &[u32],
        expected: &[MappedNodeValue<'_>],
    ) -> Result<Vec<u32>> {
        if expected.is_empty() {
            return Ok(Vec::new());
        }
        let mut matches = Vec::new();
        for node in nodes {
            if self.value(*node)?.is_some_and(|value| {
                expected
                    .iter()
                    .any(|expected| mapped_node_values_equal(value, *expected))
            }) {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_has_value(&self, nodes: &[u32]) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.value(*node)?.is_some() {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_missing_value(&self, nodes: &[u32]) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self.value(*node)?.is_none() {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_less_than(
        &self,
        nodes: &[u32],
        expected: MappedNodeValue<'_>,
    ) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self
                .value(*node)?
                .and_then(|value| compare_mapped_node_values(value, expected))
                .is_some_and(|ordering| ordering.is_lt())
            {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn filter_greater_than(
        &self,
        nodes: &[u32],
        expected: MappedNodeValue<'_>,
    ) -> Result<Vec<u32>> {
        let mut matches = Vec::new();
        for node in nodes {
            if self
                .value(*node)?
                .and_then(|value| compare_mapped_node_values(value, expected))
                .is_some_and(|ordering| ordering.is_gt())
            {
                matches.push(*node);
            }
        }
        Ok(matches)
    }

    pub fn value_interval(&self, expected: MappedNodeValue<'_>) -> Result<Option<(u32, u32)>> {
        let mut interval: Option<(u32, u32)> = None;
        for row in self.rows() {
            let (node, value) = row?;
            if mapped_node_values_equal(value, expected) {
                interval = Some(match interval {
                    Some((first, _)) => (first, node),
                    None => (node, node),
                });
            }
        }
        Ok(interval)
    }

    #[allow(non_snake_case)]
    pub fn sInterval(&self, expected: MappedNodeValue<'_>) -> Result<Option<(u32, u32)>> {
        self.value_interval(expected)
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
        let row_offset = *self.feature.row_offsets.get(self.index)?;
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
        if self.index >= self.feature.row_count {
            return None;
        }
        let row_offset = self.feature.rows_start + (self.index * 8);
        self.index += 1;
        let node = match read_u32_from(self.feature.path, self.feature.bytes, row_offset) {
            Ok(node) => node,
            Err(error) => return Some(Err(error)),
        };
        let pool_id = match read_u32_from(self.feature.path, self.feature.bytes, row_offset + 4) {
            Ok(pool_id) => pool_id as usize,
            Err(error) => return Some(Err(error)),
        };
        let Some(range) = self.feature.pool_ranges.get(pool_id) else {
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

pub struct EdgeFeatureView<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata: BTreeMap<String, Option<String>>,
    row_count: usize,
    row_offsets: Vec<usize>,
    edge_value_count: usize,
    edge_values_start: Option<usize>,
    rank: Vec<u32>,
}

impl<'a> EdgeFeatureView<'a> {
    fn new(
        path: &'a Path,
        bytes: &'a [u8],
        feature: &CompiledEdgeFeature,
        rank: Vec<u32>,
    ) -> Result<Self> {
        let mut offset = feature.payload_start;
        let row_count = read_u32_at(path, bytes, &mut offset)? as usize;
        if row_count != feature.row_count {
            return Err(invalid_compiled(
                path,
                "edge row count does not match metadata",
            ));
        }
        let mut row_offsets = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            row_offsets.push(offset);
            offset = skip_edge_row(path, bytes, offset)?;
        }
        Ok(Self {
            path,
            bytes,
            metadata: feature.metadata.clone(),
            row_count,
            row_offsets,
            edge_value_count: feature.edge_value_count,
            edge_values_start: feature.edge_values_start,
            rank,
        })
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }

    pub fn row_offset_count(&self) -> usize {
        self.row_offsets.len()
    }

    pub fn meta(&self) -> &BTreeMap<String, Option<String>> {
        &self.metadata
    }

    pub fn metadata_value(&self, key: &str) -> Option<&str> {
        metadata_value(&self.metadata, key)
    }

    pub fn value_type(&self) -> Option<&str> {
        self.metadata_value("valueType")
    }

    #[allow(non_snake_case)]
    pub fn valueType(&self) -> Option<&str> {
        self.value_type()
    }

    pub fn description(&self) -> Option<&str> {
        self.metadata_value("description")
    }

    pub fn edge_value_count(&self) -> usize {
        self.edge_value_count
    }

    pub fn has_edge_values(&self) -> bool {
        self.edge_value_count > 0
    }

    #[allow(non_snake_case)]
    pub fn hasEdgeValues(&self) -> bool {
        self.has_edge_values()
    }

    pub fn targets(&self, source: u32) -> Result<Option<EdgeTargets<'a>>> {
        let mut low = 0_usize;
        let mut high = self.row_count;
        while low < high {
            let mid = (low + high) / 2;
            let row_offset = self.row_offsets[mid];
            let row_source = read_u32_from(self.path, self.bytes, row_offset)?;
            match row_source.cmp(&source) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Greater => high = mid,
                std::cmp::Ordering::Equal => {
                    let target_count =
                        read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
                    let targets_start = row_offset + 8;
                    return Ok(Some(EdgeTargets {
                        path: self.path,
                        bytes: self.bytes,
                        offset: targets_start,
                        remaining: target_count,
                    }));
                }
            }
        }
        Ok(None)
    }

    pub fn s(&self, source: u32) -> Result<Vec<u32>> {
        let mut targets = self
            .targets(source)?
            .map(|targets| targets.collect::<Result<Vec<_>>>())
            .unwrap_or_else(|| Ok(Vec::new()))?;
        self.sort_nodes(&mut targets);
        Ok(targets)
    }

    pub fn f(&self, source: u32) -> Result<Vec<u32>> {
        self.s(source)
    }

    pub fn forward(&self, source: u32) -> Result<Vec<u32>> {
        self.s(source)
    }

    pub fn all_targets<I>(&self, sources: I) -> Result<BTreeSet<u32>>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut targets = BTreeSet::new();
        for source in sources {
            targets.extend(self.forward(source)?);
        }
        Ok(targets)
    }

    pub fn get_all_targets<I>(&self, sources: I) -> Result<BTreeSet<u32>>
    where
        I: IntoIterator<Item = u32>,
    {
        self.all_targets(sources)
    }

    pub fn filter_sources_with_targets_in<I, J>(
        &self,
        sources: I,
        targets: J,
    ) -> Result<(BTreeSet<u32>, BTreeSet<u32>)>
    where
        I: IntoIterator<Item = u32>,
        J: IntoIterator<Item = u32>,
    {
        let target_set: BTreeSet<u32> = targets.into_iter().collect();
        if target_set.is_empty() {
            return Ok((BTreeSet::new(), BTreeSet::new()));
        }
        let mut matching_sources = BTreeSet::new();
        let mut matching_targets = BTreeSet::new();
        for source in sources {
            for target in self.forward(source)? {
                if target_set.contains(&target) {
                    matching_sources.insert(source);
                    matching_targets.insert(target);
                }
            }
        }
        Ok((matching_sources, matching_targets))
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

    pub fn t(&self, target: u32) -> Result<Vec<u32>> {
        let mut sources = Vec::new();
        for row_offset in &self.row_offsets {
            let source = read_u32_from(self.path, self.bytes, *row_offset)?;
            let target_count = read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
            let targets_start = row_offset + 8;
            for index in 0..target_count {
                let candidate = read_u32_from(self.path, self.bytes, targets_start + (index * 4))?;
                if candidate == target {
                    sources.push(source);
                    break;
                }
            }
        }
        self.sort_nodes(&mut sources);
        sources.dedup();
        Ok(sources)
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

    pub fn edge_count(&self) -> Result<usize> {
        let mut count = 0_usize;
        for row_offset in &self.row_offsets {
            count += read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
        }
        Ok(count)
    }

    pub fn items(&self) -> Result<Vec<(u32, Vec<u32>)>> {
        let mut rows = Vec::with_capacity(self.row_count);
        for row_offset in &self.row_offsets {
            let source = read_u32_from(self.path, self.bytes, *row_offset)?;
            let target_count = read_u32_from(self.path, self.bytes, row_offset + 4)? as usize;
            let targets_start = row_offset + 8;
            let mut targets = Vec::with_capacity(target_count);
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
        let rank = self
            .rank
            .get(node.saturating_sub(1) as usize)
            .copied()
            .unwrap_or(node);
        (rank, node)
    }

    pub fn edge_value(
        &self,
        expected_source: u32,
        expected_target: u32,
    ) -> Result<Option<MappedNodeValue<'a>>> {
        let Some(mut offset) = self.edge_values_start else {
            return Ok(None);
        };
        for _ in 0..self.edge_value_count {
            let source = read_u32_from(self.path, self.bytes, offset)?;
            let target = read_u32_from(self.path, self.bytes, offset + 4)?;
            offset += 8;
            if source == expected_source && target == expected_target {
                return read_mixed_node_value_from(self.path, self.bytes, offset).map(Some);
            }
            offset = skip_mixed_value(self.path, self.bytes, offset)?;
        }
        Ok(None)
    }
}

pub struct EdgeTargets<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
    remaining: usize,
}

pub struct ConfigFeatureView<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    metadata_count: usize,
    payload_start: usize,
}

impl<'a> ConfigFeatureView<'a> {
    fn new(path: &'a Path, bytes: &'a [u8], feature: &CompiledConfigFeature) -> Result<Self> {
        let mut offset = feature.payload_start;
        let metadata_count = read_u32_at(path, bytes, &mut offset)? as usize;
        if metadata_count != feature.metadata_count {
            return Err(invalid_compiled(
                path,
                "config metadata count does not match metadata",
            ));
        }
        Ok(Self {
            path,
            bytes,
            metadata_count,
            payload_start: feature.payload_start,
        })
    }

    pub fn metadata_count(&self) -> usize {
        self.metadata_count
    }

    pub fn get(&self, expected_key: &str) -> Result<Option<Option<&'a str>>> {
        for row in self.items() {
            let (key, value) = row?;
            if key == expected_key {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }

    pub fn metadata_value(&self, key: &str) -> Result<Option<&'a str>> {
        let value = self.get(key)?.flatten();
        if value.is_some() || key != "valueType" {
            return Ok(value);
        }
        Ok(self.get("value_type")?.flatten())
    }

    pub fn metadata(&self) -> Result<BTreeMap<String, Option<String>>> {
        let mut metadata = BTreeMap::new();
        for row in self.items() {
            let (key, value) = row?;
            metadata.insert(key.to_string(), value.map(str::to_string));
        }
        Ok(metadata)
    }

    pub fn meta(&self) -> Result<BTreeMap<String, Option<String>>> {
        self.metadata()
    }

    pub fn items(&self) -> ConfigMetadataItems<'a> {
        let offset = self.payload_start + 4;
        ConfigMetadataItems {
            path: self.path,
            bytes: self.bytes,
            offset,
            remaining: self.metadata_count,
        }
    }
}

pub struct ConfigMetadataItems<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
    remaining: usize,
}

impl<'a> Iterator for ConfigMetadataItems<'a> {
    type Item = Result<(&'a str, Option<&'a str>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;

        let key_range = match read_string_range_at(self.path, self.bytes, &mut self.offset) {
            Ok(range) => range,
            Err(error) => return Some(Err(error)),
        };
        let key = match std::str::from_utf8(&self.bytes[key_range]) {
            Ok(key) => key,
            Err(_) => return Some(Err(invalid_compiled(self.path, "invalid utf-8 string"))),
        };
        let tag = match self.bytes.get(self.offset) {
            Some(tag) => *tag,
            None => {
                return Some(Err(invalid_compiled(
                    self.path,
                    "unexpected end of config metadata value tag",
                )));
            }
        };
        self.offset += 1;
        match tag {
            0 => Some(Ok((key, None))),
            1 => {
                let value_range =
                    match read_string_range_at(self.path, self.bytes, &mut self.offset) {
                        Ok(range) => range,
                        Err(error) => return Some(Err(error)),
                    };
                Some(
                    std::str::from_utf8(&self.bytes[value_range])
                        .map(|value| (key, Some(value)))
                        .map_err(|_| invalid_compiled(self.path, "invalid utf-8 string")),
                )
            }
            _ => Some(Err(invalid_compiled(
                self.path,
                "invalid config metadata value tag",
            ))),
        }
    }
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

pub fn compile_features(
    tf_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    features: &[&str],
) -> Result<()> {
    let corpus = if features.is_empty() {
        Corpus::load(tf_path)?
    } else {
        Corpus::load_features(tf_path, features)?
    }
    .with_rank_arrays();
    write_compiled(&corpus, output_path.as_ref())
}

pub fn compile_loaded_corpus(corpus: &Corpus, output_path: impl AsRef<Path>) -> Result<()> {
    write_compiled(corpus, output_path.as_ref())
}

pub fn inspect_compiled(path: impl AsRef<Path>) -> Result<CompiledMetadata> {
    let path = path.as_ref();
    let mmap = map_file(path)?;
    inspect_compiled_bytes(path, &mmap)
}

fn inspect_compiled_bytes(path: &Path, bytes: &[u8]) -> Result<CompiledMetadata> {
    let mut reader = Reader::new(path, bytes);
    reader.expect_magic()?;

    let mut node_features = Vec::new();
    let node_count = reader.read_u32()? as usize;
    for _ in 0..node_count {
        node_features.push(reader.inspect_node_feature()?);
    }

    let mut edge_features = Vec::new();
    let edge_count = reader.read_u32()? as usize;
    for _ in 0..edge_count {
        edge_features.push(reader.inspect_edge_feature()?);
    }

    let (order_len, order_start) = if reader.is_done() {
        (0, None)
    } else {
        reader.inspect_u32_vec()?
    };
    let (rank_len, rank_start) = if reader.is_done() {
        (0, None)
    } else {
        reader.inspect_u32_vec()?
    };
    let config_features = if reader.is_done() {
        Vec::new()
    } else {
        reader.inspect_config_features()?
    };
    if !reader.is_done() {
        let (node_metadata, edge_metadata) = reader.read_feature_metadata_section()?;
        for feature in &mut node_features {
            if let Some(metadata) = node_metadata.get(&feature.name) {
                feature.metadata = metadata.clone();
            }
        }
        for feature in &mut edge_features {
            if let Some(metadata) = edge_metadata.get(&feature.name) {
                feature.metadata = metadata.clone();
            }
        }
    }
    if !reader.is_done() {
        let edge_values = reader.inspect_edge_values_section()?;
        for feature in &mut edge_features {
            if let Some((edge_value_count, edge_values_start)) = edge_values.get(&feature.name) {
                feature.edge_value_count = *edge_value_count;
                feature.edge_values_start = Some(*edge_values_start);
            }
        }
    }
    let structure_start = if reader.is_done() {
        None
    } else {
        reader.inspect_structure_section()?
    };

    Ok(CompiledMetadata {
        byte_len: bytes.len(),
        node_features,
        edge_features,
        config_features,
        order_len,
        rank_len,
        order_start,
        rank_start,
        structure_start,
    })
}

fn map_file(path: &Path) -> Result<Mmap> {
    let file = File::open(path).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    // The map is only read while this function parses the cache, and the parser
    // does not retain borrowed slices after returning the materialized Corpus.
    Ok(unsafe {
        Mmap::map(&file).map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?
    })
}

fn read_string_range_at(path: &Path, bytes: &[u8], offset: &mut usize) -> Result<Range<usize>> {
    let len = read_u32_at(path, bytes, offset)? as usize;
    let start = *offset;
    let end = start
        .checked_add(len)
        .ok_or_else(|| invalid_compiled(path, "compiled string offset overflow"))?;
    if end > bytes.len() {
        return Err(invalid_compiled(
            path,
            "unexpected end of compiled corpus string",
        ));
    }
    *offset = end;
    Ok(start..end)
}

fn read_u32_at(path: &Path, bytes: &[u8], offset: &mut usize) -> Result<u32> {
    let value = read_u32_from(path, bytes, *offset)?;
    *offset = offset
        .checked_add(4)
        .ok_or_else(|| invalid_compiled(path, "compiled u32 offset overflow"))?;
    Ok(value)
}

fn read_u32_from(path: &Path, bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| invalid_compiled(path, "compiled u32 offset overflow"))?;
    let Some(raw) = bytes.get(offset..end) else {
        return Err(invalid_compiled(
            path,
            "unexpected end of compiled corpus u32",
        ));
    };
    let mut value = [0_u8; 4];
    value.copy_from_slice(raw);
    Ok(u32::from_le_bytes(value))
}

fn read_i64_from(path: &Path, bytes: &[u8], offset: usize) -> Result<i64> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| invalid_compiled(path, "compiled i64 offset overflow"))?;
    let Some(raw) = bytes.get(offset..end) else {
        return Err(invalid_compiled(
            path,
            "unexpected end of compiled corpus i64",
        ));
    };
    let mut value = [0_u8; 8];
    value.copy_from_slice(raw);
    Ok(i64::from_le_bytes(value))
}

fn read_mixed_node_value_from<'a>(
    path: &Path,
    bytes: &'a [u8],
    offset: usize,
) -> Result<MappedNodeValue<'a>> {
    let tag = *bytes
        .get(offset)
        .ok_or_else(|| invalid_compiled(path, "unexpected end of mixed feature value tag"))?;
    match tag {
        0 => {
            let mut string_offset = offset + 1;
            let range = read_string_range_at(path, bytes, &mut string_offset)?;
            std::str::from_utf8(&bytes[range])
                .map(MappedNodeValue::Str)
                .map_err(|_| invalid_compiled(path, "invalid utf-8 string"))
        }
        1 => read_i64_from(path, bytes, offset + 1).map(MappedNodeValue::Int),
        _ => Err(invalid_compiled(path, "invalid feature value tag")),
    }
}

fn mapped_node_values_equal(left: MappedNodeValue<'_>, right: MappedNodeValue<'_>) -> bool {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => left == right,
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => left == right,
        _ => false,
    }
}

fn compare_mapped_node_values(
    left: MappedNodeValue<'_>,
    right: MappedNodeValue<'_>,
) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => Some(left.cmp(right)),
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => Some(left.cmp(&right)),
        _ => None,
    }
}

fn mapped_value_to_feature_value(value: MappedNodeValue<'_>) -> FeatureValue {
    match value {
        MappedNodeValue::Str(value) => FeatureValue::string(value),
        MappedNodeValue::Int(value) => FeatureValue::Int(value),
    }
}

fn sorted_feature_value_counts(counts: HashMap<FeatureValue, usize>) -> Vec<(FeatureValue, usize)> {
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_unstable_by_key(|(value, count)| (Reverse(*count), value.sort_token()));
    rows
}

fn sorted_optional_feature_value_counts(
    counts: HashMap<Option<FeatureValue>, usize>,
) -> Vec<(Option<FeatureValue>, usize)> {
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_unstable_by_key(|(value, count)| {
        (Reverse(*count), optional_feature_value_sort_token(value))
    });
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

fn optional_feature_value_sort_token(value: &Option<FeatureValue>) -> String {
    match value {
        Some(value) => value.sort_token(),
        None => "n:".to_string(),
    }
}

fn structure_tree_from_data(node: u32, data: &StructureData) -> Option<StructureTree> {
    if !data.heading_from_node.contains_key(&node) {
        return None;
    }
    Some(StructureTree::Node {
        node,
        children: data
            .down
            .get(&node)
            .into_iter()
            .flatten()
            .filter_map(|child| structure_tree_from_data(*child, data))
            .collect(),
    })
}

fn structure_heading_repr(heading: &[StructureHeading]) -> String {
    heading
        .iter()
        .map(|item| format!("{}:{}", item.node_type, item.heading))
        .collect::<Vec<_>>()
        .join("-")
}

fn structure_pretty_lines(
    tree: &StructureTree,
    data: &StructureData,
    full_heading: bool,
    indent: &str,
    lines: &mut Vec<String>,
) {
    match tree {
        StructureTree::Forest(children) => {
            for child in children {
                structure_pretty_lines(child, data, full_heading, indent, lines);
            }
        }
        StructureTree::Node { node, children } => {
            if let Some(heading) = data.heading_from_node.get(node) {
                let parts = if full_heading {
                    heading.as_slice()
                } else {
                    heading.last().map(std::slice::from_ref).unwrap_or(&[])
                };
                lines.push(format!("{indent}{}", structure_heading_repr(parts)));
            }
            let child_indent = format!("{indent}    ");
            for child in children {
                structure_pretty_lines(child, data, full_heading, &child_indent, lines);
            }
        }
    }
}

fn skip_mixed_node_row(path: &Path, bytes: &[u8], offset: usize) -> Result<usize> {
    let tag_offset = offset
        .checked_add(4)
        .ok_or_else(|| invalid_compiled(path, "mixed feature row offset overflow"))?;
    let tag = *bytes
        .get(tag_offset)
        .ok_or_else(|| invalid_compiled(path, "unexpected end of mixed feature value tag"))?;
    match tag {
        0 => {
            let mut string_offset = tag_offset + 1;
            read_string_range_at(path, bytes, &mut string_offset)?;
            Ok(string_offset)
        }
        1 => tag_offset
            .checked_add(9)
            .ok_or_else(|| invalid_compiled(path, "mixed int row offset overflow")),
        _ => Err(invalid_compiled(path, "invalid feature value tag")),
    }
}

fn skip_mixed_value(path: &Path, bytes: &[u8], offset: usize) -> Result<usize> {
    let tag = *bytes
        .get(offset)
        .ok_or_else(|| invalid_compiled(path, "unexpected end of mixed feature value tag"))?;
    match tag {
        0 => {
            let mut string_offset = offset + 1;
            read_string_range_at(path, bytes, &mut string_offset)?;
            Ok(string_offset)
        }
        1 => offset
            .checked_add(9)
            .ok_or_else(|| invalid_compiled(path, "mixed int row offset overflow")),
        _ => Err(invalid_compiled(path, "invalid feature value tag")),
    }
}

fn skip_edge_row(path: &Path, bytes: &[u8], offset: usize) -> Result<usize> {
    let target_count = read_u32_from(path, bytes, offset + 4)? as usize;
    offset
        .checked_add(8)
        .and_then(|offset| offset.checked_add(target_count.checked_mul(4)?))
        .ok_or_else(|| invalid_compiled(path, "compiled edge row offset overflow"))
}

fn invalid_compiled(path: &Path, message: impl Into<String>) -> CfError {
    CfError::Parse {
        path: path.to_path_buf(),
        line: 0,
        message: message.into(),
    }
}

fn write_compiled(corpus: &Corpus, output_path: &Path) -> Result<()> {
    let file = File::create(output_path).map_err(|source| CfError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    writer.write_all(MAGIC).map_err(|source| CfError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;

    write_u32(&mut writer, corpus.node_features.len() as u32, output_path)?;
    for feature in corpus.node_features.values() {
        write_node_feature(&mut writer, feature, output_path)?;
    }

    write_u32(&mut writer, corpus.edge_features.len() as u32, output_path)?;
    for feature in corpus.edge_features.values() {
        write_edge_feature(&mut writer, feature, output_path)?;
    }
    write_u32_vec(
        &mut writer,
        corpus.order.as_deref().unwrap_or(&[]),
        output_path,
    )?;
    write_u32_vec(
        &mut writer,
        corpus.rank.as_deref().unwrap_or(&[]),
        output_path,
    )?;
    write_config_features(&mut writer, &corpus.config_features, output_path)?;
    write_feature_metadata_section(
        &mut writer,
        &corpus.node_features,
        &corpus.edge_features,
        output_path,
    )?;
    write_edge_values_section(&mut writer, &corpus.edge_features, output_path)?;
    write_structure_section(&mut writer, corpus, output_path)?;
    writer.flush().map_err(|source| CfError::Io {
        path: output_path.to_path_buf(),
        source,
    })
}

fn write_config_features<W: Write>(
    writer: &mut W,
    features: &BTreeMap<String, BTreeMap<String, Option<String>>>,
    path: &Path,
) -> Result<()> {
    write_u32(writer, features.len() as u32, path)?;
    for (name, metadata) in features {
        write_string(writer, name, path)?;
        write_u32(writer, metadata.len() as u32, path)?;
        for (key, value) in metadata {
            write_string(writer, key, path)?;
            match value {
                Some(value) => {
                    writer.write_all(&[1]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    write_string(writer, value, path)?;
                }
                None => {
                    writer.write_all(&[0]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn write_feature_metadata_section<W: Write>(
    writer: &mut W,
    node_features: &BTreeMap<String, NodeFeature>,
    edge_features: &BTreeMap<String, EdgeFeature>,
    path: &Path,
) -> Result<()> {
    writer
        .write_all(FEATURE_METADATA_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32(writer, node_features.len() as u32, path)?;
    for feature in node_features.values() {
        write_string(writer, &feature.name, path)?;
        write_metadata_map(writer, &feature.metadata, path)?;
    }
    write_u32(writer, edge_features.len() as u32, path)?;
    for feature in edge_features.values() {
        write_string(writer, &feature.name, path)?;
        write_metadata_map(writer, &feature.metadata, path)?;
    }
    Ok(())
}

fn write_edge_values_section<W: Write>(
    writer: &mut W,
    edge_features: &BTreeMap<String, EdgeFeature>,
    path: &Path,
) -> Result<()> {
    writer
        .write_all(EDGE_VALUES_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let valued_features = edge_features
        .values()
        .filter(|feature| !feature.edge_values.is_empty())
        .collect::<Vec<_>>();
    write_u32(writer, valued_features.len() as u32, path)?;
    for feature in valued_features {
        write_string(writer, &feature.name, path)?;
        write_u32(writer, feature.edge_values.len() as u32, path)?;
        let mut rows = feature.edge_values.iter().collect::<Vec<_>>();
        rows.sort_unstable_by_key(|((source, target), _)| (*source, *target));
        for ((source, target), value) in rows {
            write_u32(writer, *source, path)?;
            write_u32(writer, *target, path)?;
            write_feature_value(writer, value, path)?;
        }
    }
    Ok(())
}

fn write_structure_section<W: Write>(writer: &mut W, corpus: &Corpus, path: &Path) -> Result<()> {
    let Some(data) = precompute::structure(corpus) else {
        return Ok(());
    };
    writer
        .write_all(STRUCTURE_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32_vec(writer, &data.top, path)?;
    write_u32(writer, data.heading_from_node.len() as u32, path)?;
    for (node, heading) in &data.heading_from_node {
        write_u32(writer, *node, path)?;
        write_heading(writer, heading, path)?;
    }
    write_u32(writer, data.node_from_heading.len() as u32, path)?;
    for (heading, node) in &data.node_from_heading {
        write_heading(writer, heading, path)?;
        write_u32(writer, *node, path)?;
    }
    write_u32(writer, data.multiple.len() as u32, path)?;
    for (heading, nodes) in &data.multiple {
        write_heading(writer, heading, path)?;
        write_u32_vec(writer, nodes, path)?;
    }
    write_u32(writer, data.up.len() as u32, path)?;
    for (node, parent) in &data.up {
        write_u32(writer, *node, path)?;
        write_u32(writer, *parent, path)?;
    }
    write_u32(writer, data.down.len() as u32, path)?;
    for (parent, children) in &data.down {
        write_u32(writer, *parent, path)?;
        write_u32_vec(writer, children, path)?;
    }
    Ok(())
}

fn write_heading<W: Write>(
    writer: &mut W,
    heading: &[StructureHeading],
    path: &Path,
) -> Result<()> {
    write_u32(writer, heading.len() as u32, path)?;
    for item in heading {
        write_string(writer, &item.node_type, path)?;
        write_string(writer, &item.heading, path)?;
    }
    Ok(())
}

fn write_metadata_map<W: Write>(
    writer: &mut W,
    metadata: &BTreeMap<String, Option<String>>,
    path: &Path,
) -> Result<()> {
    write_u32(writer, metadata.len() as u32, path)?;
    for (key, value) in metadata {
        write_string(writer, key, path)?;
        match value {
            Some(value) => {
                writer.write_all(&[1]).map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
                write_string(writer, value, path)?;
            }
            None => {
                writer.write_all(&[0]).map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
            }
        }
    }
    Ok(())
}

fn write_node_feature<W: Write>(writer: &mut W, feature: &NodeFeature, path: &Path) -> Result<()> {
    write_string(writer, &feature.name, path)?;
    let all_strings = feature
        .values
        .values()
        .all(|value| matches!(value, FeatureValue::Str(_)));
    writer
        .write_all(&[if all_strings { 0 } else { 1 }])
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32(writer, feature.values.len() as u32, path)?;

    let mut payload = Vec::new();
    if all_strings {
        let mut pool: Vec<Arc<str>> = Vec::new();
        let mut pool_ids: HashMap<Arc<str>, u32> = HashMap::new();
        for value in feature.values.values() {
            let FeatureValue::Str(value) = value else {
                unreachable!();
            };
            if !pool_ids.contains_key(value) {
                let id = pool.len() as u32;
                pool.push(value.clone());
                pool_ids.insert(value.clone(), id);
            }
        }
        write_u32(writer, pool.len() as u32, path)?;
        write_u32(&mut payload, pool.len() as u32, path)?;
        for value in &pool {
            write_string(&mut payload, value, path)?;
        }

        write_u32(&mut payload, feature.values.len() as u32, path)?;
        let mut rows: Vec<_> = feature.values.iter().collect();
        rows.sort_unstable_by_key(|(node, _)| **node);
        for (node, value) in rows {
            let FeatureValue::Str(value) = value else {
                unreachable!();
            };
            write_u32(&mut payload, *node, path)?;
            write_u32(&mut payload, pool_ids[value], path)?;
        }
    } else {
        write_u32(writer, 0, path)?;
        write_u32(&mut payload, feature.values.len() as u32, path)?;
        let mut rows: Vec<_> = feature.values.iter().collect();
        rows.sort_unstable_by_key(|(node, _)| **node);
        for (node, value) in rows {
            write_u32(&mut payload, *node, path)?;
            match value {
                FeatureValue::Str(value) => {
                    payload.write_all(&[0]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    write_string(&mut payload, value, path)?;
                }
                FeatureValue::Int(value) => {
                    payload.write_all(&[1]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    payload
                        .write_all(&value.to_le_bytes())
                        .map_err(|source| CfError::Io {
                            path: path.to_path_buf(),
                            source,
                        })?;
                }
            }
        }
    }
    write_u32(writer, payload.len() as u32, path)?;
    writer.write_all(&payload).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn write_edge_feature<W: Write>(writer: &mut W, feature: &EdgeFeature, path: &Path) -> Result<()> {
    write_string(writer, &feature.name, path)?;
    write_u32(writer, feature.values.len() as u32, path)?;
    let mut payload = Vec::new();
    write_u32(&mut payload, feature.values.len() as u32, path)?;
    let mut rows: Vec<_> = feature.values.iter().collect();
    rows.sort_unstable_by_key(|(node, _)| **node);
    for (node, targets) in rows {
        write_u32(&mut payload, *node, path)?;
        write_u32(&mut payload, targets.len() as u32, path)?;
        for target in targets {
            write_u32(&mut payload, *target, path)?;
        }
    }
    write_u32(writer, payload.len() as u32, path)?;
    writer.write_all(&payload).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn write_feature_value<W: Write>(writer: &mut W, value: &FeatureValue, path: &Path) -> Result<()> {
    match value {
        FeatureValue::Str(value) => {
            writer.write_all(&[0]).map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            write_string(writer, value, path)?;
        }
        FeatureValue::Int(value) => {
            writer.write_all(&[1]).map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            writer
                .write_all(&value.to_le_bytes())
                .map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
    }
    Ok(())
}

fn write_string<W: Write>(writer: &mut W, value: &str, path: &Path) -> Result<()> {
    write_u32(writer, value.len() as u32, path)?;
    writer
        .write_all(value.as_bytes())
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })
}

fn write_u32<W: Write>(writer: &mut W, value: u32, path: &Path) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })
}

fn write_u32_vec<W: Write>(writer: &mut W, values: &[u32], path: &Path) -> Result<()> {
    write_u32(writer, values.len() as u32, path)?;
    for value in values {
        write_u32(writer, *value, path)?;
    }
    Ok(())
}

struct Reader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(path: &'a Path, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            offset: 0,
        }
    }

    fn expect_magic(&mut self) -> Result<()> {
        let magic = self.take(MAGIC.len())?;
        if magic != MAGIC {
            return Err(self.invalid("invalid compiled corpus magic"));
        }
        Ok(())
    }

    fn inspect_node_feature(&mut self) -> Result<CompiledNodeFeature> {
        let name = self.read_string()?;
        let encoding_tag = self.read_u8()?;
        let row_count = self.read_u32()? as usize;
        let aux_count = self.read_u32()? as usize;
        let payload_len = self.read_u32()? as usize;
        let payload_start = self.offset;
        self.skip_bytes(payload_len)?;
        let (encoding, string_pool_count) = match encoding_tag {
            0 => (NodeFeatureEncoding::StringPool, Some(aux_count)),
            1 => (NodeFeatureEncoding::Mixed, None),
            _ => return Err(self.invalid("invalid node feature encoding")),
        };
        Ok(CompiledNodeFeature {
            name,
            encoding,
            row_count,
            string_pool_count,
            payload_start,
            payload_end: self.offset,
            metadata: BTreeMap::new(),
        })
    }

    fn inspect_config_features(&mut self) -> Result<Vec<CompiledConfigFeature>> {
        let feature_count = self.read_u32()? as usize;
        let mut features = Vec::with_capacity(feature_count);
        for _ in 0..feature_count {
            let name = self.read_string()?;
            let payload_start = self.offset;
            let metadata_count = self.read_u32()? as usize;
            for _ in 0..metadata_count {
                self.skip_string()?;
                match self.read_u8()? {
                    0 => {}
                    1 => self.skip_string()?,
                    _ => return Err(self.invalid("invalid config metadata value tag")),
                }
            }
            features.push(CompiledConfigFeature {
                name,
                metadata_count,
                payload_start,
                payload_end: self.offset,
            });
        }
        Ok(features)
    }

    fn read_feature_metadata_section(
        &mut self,
    ) -> Result<(
        BTreeMap<String, BTreeMap<String, Option<String>>>,
        BTreeMap<String, BTreeMap<String, Option<String>>>,
    )> {
        let magic = self.take(FEATURE_METADATA_MAGIC.len())?;
        if magic != FEATURE_METADATA_MAGIC {
            return Err(self.invalid("invalid feature metadata section magic"));
        }
        let node_count = self.read_u32()? as usize;
        let mut node_metadata = BTreeMap::new();
        for _ in 0..node_count {
            let name = self.read_string()?;
            node_metadata.insert(name, self.read_metadata_map()?);
        }
        let edge_count = self.read_u32()? as usize;
        let mut edge_metadata = BTreeMap::new();
        for _ in 0..edge_count {
            let name = self.read_string()?;
            edge_metadata.insert(name, self.read_metadata_map()?);
        }
        Ok((node_metadata, edge_metadata))
    }

    fn inspect_edge_values_section(&mut self) -> Result<HashMap<String, (usize, usize)>> {
        let magic = self.take(EDGE_VALUES_MAGIC.len())?;
        if magic != EDGE_VALUES_MAGIC {
            return Err(self.invalid("invalid edge value metadata magic"));
        }
        let feature_count = self.read_u32()? as usize;
        let mut edge_values = HashMap::with_capacity(feature_count);
        for _ in 0..feature_count {
            let name = self.read_string()?;
            let value_count = self.read_u32()? as usize;
            let values_start = self.offset;
            for _ in 0..value_count {
                self.skip_bytes(8)?;
                self.skip_feature_value()?;
            }
            edge_values.insert(name, (value_count, values_start));
        }
        Ok(edge_values)
    }

    fn inspect_structure_section(&mut self) -> Result<Option<usize>> {
        let start = self.offset;
        self.read_structure_section()?;
        Ok(Some(start))
    }

    fn read_structure_section(&mut self) -> Result<Option<StructureData>> {
        let magic = self.take(STRUCTURE_MAGIC.len())?;
        if magic != STRUCTURE_MAGIC {
            return Err(self.invalid("invalid structure section magic"));
        }
        let top = self.read_u32_vec()?;

        let heading_count = self.read_u32()? as usize;
        let mut heading_from_node = BTreeMap::new();
        for _ in 0..heading_count {
            let node = self.read_u32()?;
            heading_from_node.insert(node, self.read_heading()?);
        }

        let node_heading_count = self.read_u32()? as usize;
        let mut node_from_heading = BTreeMap::new();
        for _ in 0..node_heading_count {
            let heading = self.read_heading()?;
            let node = self.read_u32()?;
            node_from_heading.insert(heading, node);
        }

        let multiple_count = self.read_u32()? as usize;
        let mut multiple = BTreeMap::new();
        for _ in 0..multiple_count {
            let heading = self.read_heading()?;
            multiple.insert(heading, self.read_u32_vec()?);
        }

        let up_count = self.read_u32()? as usize;
        let mut up = BTreeMap::new();
        for _ in 0..up_count {
            let node = self.read_u32()?;
            let parent = self.read_u32()?;
            up.insert(node, parent);
        }

        let down_count = self.read_u32()? as usize;
        let mut down = BTreeMap::new();
        for _ in 0..down_count {
            let parent = self.read_u32()?;
            down.insert(parent, self.read_u32_vec()?);
        }

        Ok(Some(StructureData {
            heading_from_node,
            node_from_heading,
            multiple,
            top,
            up,
            down,
        }))
    }

    fn read_metadata_map(&mut self) -> Result<BTreeMap<String, Option<String>>> {
        let metadata_count = self.read_u32()? as usize;
        let mut metadata = BTreeMap::new();
        for _ in 0..metadata_count {
            let key = self.read_string()?;
            let value = match self.read_u8()? {
                0 => None,
                1 => Some(self.read_string()?),
                _ => return Err(self.invalid("invalid feature metadata value tag")),
            };
            metadata.insert(key, value);
        }
        Ok(metadata)
    }

    fn inspect_edge_feature(&mut self) -> Result<CompiledEdgeFeature> {
        let name = self.read_string()?;
        let row_count = self.read_u32()? as usize;
        let payload_len = self.read_u32()? as usize;
        let payload_start = self.offset;
        self.skip_bytes(payload_len)?;
        Ok(CompiledEdgeFeature {
            name,
            row_count,
            payload_start,
            payload_end: self.offset,
            edge_value_count: 0,
            edge_values_start: None,
            metadata: BTreeMap::new(),
        })
    }

    fn skip_feature_value(&mut self) -> Result<()> {
        match self.read_u8()? {
            0 => self.skip_string(),
            1 => self.skip_bytes(8),
            _ => Err(self.invalid("invalid feature value tag")),
        }
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_string)
            .map_err(|_| self.invalid("invalid utf-8 string"))
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u32_vec(&mut self) -> Result<Vec<u32>> {
        let len = self.read_u32()? as usize;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(self.read_u32()?);
        }
        Ok(values)
    }

    fn read_heading(&mut self) -> Result<Vec<StructureHeading>> {
        let len = self.read_u32()? as usize;
        let mut heading = Vec::with_capacity(len);
        for _ in 0..len {
            heading.push(StructureHeading {
                node_type: self.read_string()?,
                heading: self.read_string()?,
            });
        }
        Ok(heading)
    }

    fn inspect_u32_vec(&mut self) -> Result<(usize, Option<usize>)> {
        let len = self.read_u32()? as usize;
        let start = self.offset;
        self.skip_bytes(
            len.checked_mul(4)
                .ok_or_else(|| self.invalid("compiled u32 vector byte count overflow"))?,
        )?;
        Ok((len, Some(start)))
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| self.invalid("compiled corpus offset overflow"))?;
        if end > self.bytes.len() {
            return Err(self.invalid("unexpected end of compiled corpus"));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn skip_bytes(&mut self, len: usize) -> Result<()> {
        self.take(len).map(|_| ())
    }

    fn skip_string(&mut self) -> Result<()> {
        let len = self.read_u32()? as usize;
        self.skip_bytes(len)
    }

    fn is_done(&self) -> bool {
        self.offset >= self.bytes.len()
    }

    fn invalid(&self, message: impl Into<String>) -> CfError {
        CfError::Parse {
            path: self.path.to_path_buf(),
            line: 0,
            message: message.into(),
        }
    }
}
