use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use serde::Serialize;

use crate::error::{CfError, Result};
use crate::feature::{EdgeFeature, EdgeFrequency, FeatureValue, NodeFeature, TfFeature};
use crate::io::{TfData, TfDataContent};
use crate::parser::parse_tf_file;
use crate::precompute::{self, StructureData, StructureHeading};
use crate::search::Search;

#[derive(Debug)]
pub struct Corpus {
    pub node_features: BTreeMap<String, NodeFeature>,
    pub edge_features: BTreeMap<String, EdgeFeature>,
    pub config_features: BTreeMap<String, BTreeMap<String, Option<String>>>,
    pub max_node: u32,
    pub max_slot: u32,
    pub slot_type: String,
    pub nodes_by_type: HashMap<String, Vec<u32>>,
    pub type_ranks: HashMap<String, u32>,
    pub order: Option<Vec<u32>>,
    pub rank: Option<Vec<u32>>,
    order_cache: OnceLock<Vec<u32>>,
    rank_cache: OnceLock<Vec<u32>>,
    boundary_cache: OnceLock<Boundary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Boundary {
    pub first_slots: Vec<Vec<u32>>,
    pub last_slots: Vec<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusOverview {
    pub name: String,
    pub node_types: Vec<NodeTypeOverview>,
    pub section_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CorpusDescription {
    pub name: String,
    pub node_types: Vec<NodeTypeOverview>,
    pub section_types: Vec<String>,
    pub text_representations: TextRepresentationInfo,
    pub node_features: Vec<FeatureCatalogEntry>,
    pub edge_features: Vec<FeatureCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeTypeOverview {
    pub node_type: String,
    pub count: usize,
    pub is_slot_type: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureCatalogEntry {
    pub name: String,
    pub kind: FeatureKind,
    pub value_type: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureDescription {
    pub name: String,
    pub kind: Option<FeatureKind>,
    pub value_type: String,
    pub description: String,
    pub node_types: Vec<String>,
    pub unique_values: usize,
    pub sample_values: Vec<FeatureValueSample>,
    pub has_values: Option<bool>,
    pub error: Option<String>,
}

fn remove_value_type_metadata(metadata: &mut BTreeMap<String, Option<String>>) -> String {
    metadata
        .remove("valueType")
        .or_else(|| metadata.remove("value_type"))
        .flatten()
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextFormatSample {
    pub original: String,
    pub transliterated: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextFormatInfo {
    pub name: String,
    pub original_spec: String,
    pub transliteration_spec: String,
    pub samples: Vec<TextFormatSample>,
    pub unique_characters: usize,
    pub total_samples: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextRepresentationInfo {
    pub description: String,
    pub formats: Vec<TextFormatInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextOptions {
    pub format: Option<String>,
    pub descend: Option<bool>,
}

impl TextOptions {
    pub fn new(format: Option<impl Into<String>>, descend: Option<bool>) -> Self {
        Self {
            format: format.map(Into::into),
            descend,
        }
    }
}

struct ResolvedTextFormat {
    target_type: String,
    spec: String,
    implicit_node_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum StructureTree {
    Forest(Vec<StructureTree>),
    Node {
        node: u32,
        children: Vec<StructureTree>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StructureInfo {
    pub headings: Vec<(String, String)>,
    pub node_count: usize,
    pub multiple_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureValueSample {
    pub value: Option<FeatureValue>,
    pub count: usize,
}

impl CorpusOverview {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "node_types": self
                .node_types
                .iter()
                .map(NodeTypeOverview::to_dict)
                .collect::<Vec<_>>(),
            "sections": {
                "levels": self.section_types,
            },
        })
    }
}

impl CorpusDescription {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "node_types": self
                .node_types
                .iter()
                .map(NodeTypeOverview::to_dict)
                .collect::<Vec<_>>(),
            "sections": {
                "levels": self.section_types,
            },
            "text_representations": self.text_representations.to_dict(),
            "features": self
                .node_features
                .iter()
                .map(FeatureCatalogEntry::to_dict)
                .collect::<Vec<_>>(),
            "edge_features": self
                .edge_features
                .iter()
                .map(FeatureCatalogEntry::to_dict)
                .collect::<Vec<_>>(),
        })
    }
}

impl NodeTypeOverview {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "type": self.node_type,
            "count": self.count,
            "is_slot_type": self.is_slot_type,
        })
    }
}

impl FeatureCatalogEntry {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "kind": self.kind,
            "value_type": self.value_type,
            "description": self.description,
        })
    }
}

impl FeatureDescription {
    pub fn to_dict(&self) -> serde_json::Value {
        let kind = self
            .kind
            .map(|kind| serde_json::to_value(kind).expect("FeatureKind should serialize"))
            .unwrap_or_else(|| serde_json::json!("unknown"));
        let mut result = serde_json::json!({
            "name": self.name,
            "kind": kind,
            "value_type": self.value_type,
            "description": self.description,
        });
        if let Some(error) = &self.error {
            result["error"] = serde_json::json!(error);
            return result;
        }
        result["node_types"] = serde_json::json!(self.node_types);
        result["unique_values"] = serde_json::json!(self.unique_values);
        result["sample_values"] = serde_json::json!(
            self.sample_values
                .iter()
                .map(FeatureValueSample::to_dict)
                .collect::<Vec<_>>()
        );
        if let Some(has_values) = self.has_values {
            result["has_values"] = serde_json::json!(has_values);
        }
        result
    }
}

impl TextFormatSample {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "original": self.original,
            "transliterated": self.transliterated,
        })
    }
}

impl TextFormatInfo {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "original_script": self.original_spec,
            "transliteration": self.transliteration_spec,
            "samples": self
                .samples
                .iter()
                .map(TextFormatSample::to_dict)
                .collect::<Vec<_>>(),
            "unique_characters": self.unique_characters,
            "total_samples": self.total_samples,
        })
    }
}

impl TextRepresentationInfo {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "description": self.description,
            "formats": self
                .formats
                .iter()
                .map(TextFormatInfo::to_dict)
                .collect::<Vec<_>>(),
        })
    }
}

impl FeatureValueSample {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "value": self.value,
            "count": self.count,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionOptions {
    pub last_slot: bool,
    pub fillup: bool,
    pub level: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FeatureKind {
    Node,
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadedFeatureKind {
    Node,
    Edge,
    Config,
    Computed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoadedFeatureInfo {
    pub kind: LoadedFeatureKind,
    pub value_type: String,
    pub metadata: BTreeMap<String, Option<String>>,
    pub edge_values: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ComputedFeatureData {
    Levels(Vec<(String, f64, u32, u32)>),
    Order(Vec<u32>),
    Rank(Vec<u32>),
    Boundary(Boundary),
    LevUp(Vec<Vec<u32>>),
    LevDown(Vec<Vec<u32>>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkEvent {
    Start(u32),
    End(u32),
    Slot(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub node: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkPositionKey {
    start: u32,
    reverse_type_rank: std::cmp::Reverse<u32>,
    reverse_end: std::cmp::Reverse<u32>,
    node: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkLengthKey {
    reverse_size: std::cmp::Reverse<u32>,
    start: u32,
    type_rank: u32,
    node: u32,
}

impl ChunkPositionKey {
    pub(crate) fn new(start: u32, type_rank: u32, end: u32, node: u32) -> Self {
        Self {
            start,
            reverse_type_rank: std::cmp::Reverse(type_rank),
            reverse_end: std::cmp::Reverse(end),
            node,
        }
    }
}

impl ChunkLengthKey {
    pub(crate) fn new(size: u32, start: u32, type_rank: u32, node: u32) -> Self {
        Self {
            reverse_size: std::cmp::Reverse(size),
            start,
            type_rank,
            node,
        }
    }
}

impl Corpus {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_inner(&[path.as_ref().to_path_buf()], None)
    }

    pub fn load_features(path: impl AsRef<Path>, features: &[&str]) -> Result<Self> {
        Self::load_inner(&[path.as_ref().to_path_buf()], Some(features))
    }

    pub fn load_paths<I, P>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let paths = paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        Self::load_inner(&paths, None)
    }

    pub fn load_features_from_paths<I, P>(paths: I, features: &[&str]) -> Result<Self>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let paths = paths
            .into_iter()
            .map(|path| path.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        Self::load_inner(&paths, Some(features))
    }

    pub fn add_features_from(&mut self, path: impl AsRef<Path>, features: &[&str]) -> Result<bool> {
        let added = Self::load_features(path, features)?;
        self.merge_loaded_features(added)
    }

    pub fn add_features_from_paths<I, P>(&mut self, paths: I, features: &[&str]) -> Result<bool>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let added = Self::load_features_from_paths(paths, features)?;
        self.merge_loaded_features(added)
    }

    fn merge_loaded_features(&mut self, added: Self) -> Result<bool> {
        let mut node_features = std::mem::take(&mut self.node_features);
        let mut edge_features = std::mem::take(&mut self.edge_features);
        let mut config_features = std::mem::take(&mut self.config_features);
        let order = self.order.clone();
        let rank = self.rank.clone();

        node_features.extend(added.node_features);
        edge_features.extend(added.edge_features);
        config_features.extend(added.config_features);

        let mut rebuilt =
            Self::from_feature_maps_with_configs(node_features, edge_features, config_features)?;
        rebuilt.order = order;
        rebuilt.rank = rank;
        rebuilt.attach_feature_rank();
        *self = rebuilt;
        Ok(true)
    }

    #[allow(non_snake_case)]
    pub fn addFeaturesFrom(&mut self, path: impl AsRef<Path>, features: &[&str]) -> Result<bool> {
        self.add_features_from(path, features)
    }

    pub fn save(&self, location: impl AsRef<Path>) -> Result<bool> {
        let location = location.as_ref();
        fs::create_dir_all(location).map_err(|source| CfError::Io {
            path: location.to_path_buf(),
            source,
        })?;
        for feature in self.node_features.values() {
            let mut data = TfData::new(location.join(format!("{}.tf", feature.name)));
            data.data = Some(TfDataContent::Node(feature.clone()));
            data.save_result()?;
        }
        for feature in self.edge_features.values() {
            let mut data = TfData::new(location.join(format!("{}.tf", feature.name)));
            data.data = Some(TfDataContent::Edge(feature.clone()));
            data.save_result()?;
        }
        for (name, metadata) in &self.config_features {
            let mut data = TfData::new(location.join(format!("{name}.tf")));
            data.data = Some(TfDataContent::Config {
                name: name.clone(),
                metadata: metadata.clone(),
            });
            data.save_result()?;
        }
        Ok(true)
    }

    pub(crate) fn from_feature_maps_with_configs(
        node_features: BTreeMap<String, NodeFeature>,
        edge_features: BTreeMap<String, EdgeFeature>,
        config_features: BTreeMap<String, BTreeMap<String, Option<String>>>,
    ) -> Result<Self> {
        let otype = node_features
            .get("otype")
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        if !edge_features.contains_key("oslots") {
            return Err(CfError::MissingFeature("oslots".to_string()));
        }

        let slot_type = otype
            .str_value(1)
            .ok_or_else(|| CfError::MissingFeature("otype slot type".to_string()))?
            .to_string();
        let mut max_slot = 0_u32;
        let mut max_node = 0_u32;
        let mut nodes_by_type: HashMap<String, Vec<u32>> = HashMap::new();
        let mut slot_counts: HashMap<String, usize> = HashMap::new();

        for (node, value) in &otype.values {
            max_node = max_node.max(*node);
            if let FeatureValue::Str(node_type) = value {
                if node_type.as_ref() == slot_type {
                    max_slot = max_slot.max(*node);
                }
                nodes_by_type
                    .entry(node_type.to_string())
                    .or_default()
                    .push(*node);
            }
        }

        let oslots = edge_features
            .get("oslots")
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        for (node_type, nodes) in &nodes_by_type {
            let mut total = 0_usize;
            for node in nodes {
                total += if *node <= max_slot {
                    1
                } else {
                    oslots.targets(*node).map(<[u32]>::len).unwrap_or_default()
                };
            }
            slot_counts.insert(node_type.clone(), total);
        }

        for nodes in nodes_by_type.values_mut() {
            nodes.sort_unstable();
        }
        let type_ranks = Self::compute_type_ranks(&slot_type, &nodes_by_type, &slot_counts);

        Ok(Self {
            node_features,
            edge_features,
            config_features,
            max_node,
            max_slot,
            slot_type,
            nodes_by_type,
            type_ranks,
            order: None,
            rank: None,
            order_cache: OnceLock::new(),
            rank_cache: OnceLock::new(),
            boundary_cache: OnceLock::new(),
        }
        .with_rank_arrays())
    }

    pub(crate) fn with_rank_arrays(mut self) -> Self {
        let order = self.compute_order();
        let mut rank = vec![0_u32; self.max_node as usize];
        for (index, node) in order.iter().enumerate() {
            if let Some(slot) = rank.get_mut((*node - 1) as usize) {
                *slot = index as u32;
            }
        }
        self.order = Some(order);
        self.rank = Some(rank);
        self.attach_feature_rank();
        self
    }

    fn attach_feature_rank(&mut self) {
        let Some(rank) = self.rank.clone() else {
            return;
        };
        let rank = Arc::new(rank);
        for feature in self.node_features.values_mut() {
            let updated = feature.clone().with_rank(rank.clone());
            *feature = updated;
        }
        for feature in self.edge_features.values_mut() {
            let updated = feature.clone().with_rank(rank.clone());
            *feature = updated;
        }
    }

    fn load_inner(paths: &[PathBuf], features: Option<&[&str]>) -> Result<Self> {
        let mut node_features = BTreeMap::new();
        let mut edge_features = BTreeMap::new();
        let mut config_features = BTreeMap::new();
        for path in paths {
            for entry in fs::read_dir(path).map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })? {
                let entry = entry.map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
                let feature_path = entry.path();
                if feature_path.extension().and_then(|ext| ext.to_str()) != Some("tf") {
                    continue;
                }
                let Some(feature_name) = feature_path.file_stem().and_then(|stem| stem.to_str())
                else {
                    continue;
                };
                if let Some(features) = features {
                    if !features.contains(&feature_name)
                        && !matches!(feature_name, "otype" | "oslots")
                        && !is_config_feature_file(&feature_path)?
                    {
                        continue;
                    }
                }
                let parsed = parse_tf_file(&feature_path)?;
                if let Some(features) = features {
                    if matches!(&parsed, TfFeature::Config { .. }) {
                        // Config features are lightweight metadata and keep selective loads usable.
                    } else if !features.contains(&feature_name)
                        && !matches!(feature_name, "otype" | "oslots")
                    {
                        continue;
                    }
                }
                match parsed {
                    TfFeature::Node(feature) => {
                        node_features.insert(feature.name.clone(), feature);
                    }
                    TfFeature::Edge(feature) => {
                        edge_features.insert(feature.name.clone(), feature);
                    }
                    TfFeature::Config { name, metadata } => {
                        config_features.insert(name, metadata);
                    }
                }
            }
        }

        Self::from_feature_maps_with_configs(node_features, edge_features, config_features)
    }

    fn compute_type_ranks(
        slot_type: &str,
        nodes_by_type: &HashMap<String, Vec<u32>>,
        slot_counts: &HashMap<String, usize>,
    ) -> HashMap<String, u32> {
        let mut levels: Vec<(String, f64)> = nodes_by_type
            .iter()
            .filter(|(node_type, _)| node_type.as_str() != slot_type)
            .map(|(node_type, nodes)| {
                let slot_count = slot_counts.get(node_type).copied().unwrap_or_default();
                (
                    node_type.clone(),
                    slot_count as f64 / nodes.len().max(1) as f64,
                )
            })
            .collect();
        levels.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        levels.push((slot_type.to_string(), 1.0));

        let mut type_ranks = HashMap::new();
        for (rank, (node_type, _)) in levels.into_iter().rev().enumerate() {
            type_ranks.insert(node_type, rank as u32);
        }
        type_ranks
    }

    pub fn search(&self) -> Search<'_> {
        Search::new(self)
    }

    pub fn node_feature(&self, name: &str) -> Option<&NodeFeature> {
        self.node_features.get(name)
    }

    #[allow(non_snake_case)]
    pub fn Fs(&self, name: &str) -> Option<&NodeFeature> {
        self.node_feature(name)
    }

    pub fn edge_feature(&self, name: &str) -> Option<&EdgeFeature> {
        self.edge_features.get(name)
    }

    #[allow(non_snake_case)]
    pub fn Es(&self, name: &str) -> Option<&EdgeFeature> {
        self.edge_feature(name)
    }

    pub fn config_feature(&self, name: &str) -> Option<&BTreeMap<String, Option<String>>> {
        self.config_features.get(name)
    }

    pub fn node_feature_names(&self) -> Vec<&str> {
        self.node_features.keys().map(String::as_str).collect()
    }

    pub fn edge_feature_names(&self) -> Vec<&str> {
        self.edge_features.keys().map(String::as_str).collect()
    }

    pub fn all_node_features(&self, warp: bool) -> Vec<String> {
        self.node_features
            .keys()
            .filter(|name| warp || name.as_str() != "otype")
            .cloned()
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn Fall(&self, warp: bool) -> Vec<String> {
        self.all_node_features(warp)
    }

    pub fn all_edge_features(&self, warp: bool) -> Vec<String> {
        self.edge_features
            .keys()
            .filter(|name| warp || name.as_str() != "oslots")
            .cloned()
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

    pub fn computed_feature(&self, name: &str) -> Option<ComputedFeatureData> {
        match name {
            "levels" => Some(ComputedFeatureData::Levels(
                self.levels()
                    .into_iter()
                    .map(|(node_type, average_slots, min_node, max_node)| {
                        (node_type.to_string(), average_slots, min_node, max_node)
                    })
                    .collect(),
            )),
            "order" => Some(ComputedFeatureData::Order(self.order())),
            "rank" => Some(ComputedFeatureData::Rank(self.rank())),
            "boundary" => Some(ComputedFeatureData::Boundary(self.boundary())),
            _ => None,
        }
    }

    #[allow(non_snake_case)]
    pub fn Cs(&self, name: &str) -> Option<ComputedFeatureData> {
        self.computed_feature(name)
    }

    pub fn is_loaded(
        &self,
        features: Option<&[&str]>,
    ) -> BTreeMap<String, Option<LoadedFeatureInfo>> {
        let feature_names = match features {
            Some(features) => features.iter().map(|name| (*name).to_string()).collect(),
            None => self.loaded_feature_names(),
        };
        feature_names
            .into_iter()
            .map(|name| {
                let info = self.loaded_feature_info(&name);
                (name, info)
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn isLoaded(
        &self,
        features: Option<&[&str]>,
    ) -> BTreeMap<String, Option<LoadedFeatureInfo>> {
        self.is_loaded(features)
    }

    pub fn footprint(&self) -> BTreeMap<String, usize> {
        BTreeMap::from([
            ("configs".to_string(), self.config_features.len()),
            ("edges".to_string(), self.edge_features.len()),
            ("nodes".to_string(), self.node_features.len()),
            ("computed".to_string(), self.computed_names().len()),
            ("maxNode".to_string(), self.max_node as usize),
        ])
    }

    pub fn overview(&self, name: impl Into<String>) -> CorpusOverview {
        let node_types = self
            .levels()
            .into_iter()
            .map(|(node_type, _, min_node, max_node)| NodeTypeOverview {
                node_type: node_type.to_string(),
                count: max_node.saturating_sub(min_node) as usize + 1,
                is_slot_type: node_type == self.slot_type,
            })
            .collect();
        CorpusOverview {
            name: name.into(),
            node_types,
            section_types: self.section_types(),
        }
    }

    pub fn describe_corpus(&self, name: impl Into<String>) -> CorpusDescription {
        CorpusDescription {
            name: name.into(),
            node_types: self.overview("").node_types,
            section_types: self.section_types(),
            text_representations: self.text_representations(),
            node_features: self.feature_catalog(Some(FeatureKind::Node), None),
            edge_features: self.feature_catalog(Some(FeatureKind::Edge), None),
        }
    }

    pub fn section_types(&self) -> Vec<String> {
        self.config_feature("otext")
            .and_then(|metadata| metadata.get("sectionTypes"))
            .and_then(Option::as_deref)
            .map(parse_csv_config)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn sectionTypes(&self) -> Vec<String> {
        self.section_types()
    }

    pub fn section_features(&self) -> Vec<String> {
        self.config_feature("otext")
            .and_then(|metadata| metadata.get("sectionFeatures"))
            .and_then(Option::as_deref)
            .map(parse_csv_config)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn sectionFeatures(&self) -> Vec<String> {
        self.section_features()
    }

    pub fn structure_types(&self) -> Vec<String> {
        self.config_feature("otext")
            .and_then(|metadata| metadata.get("structureTypes"))
            .and_then(Option::as_deref)
            .map(parse_csv_config)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn structureTypes(&self) -> Vec<String> {
        self.structure_types()
    }

    pub fn structure_features(&self) -> Vec<String> {
        self.config_feature("otext")
            .and_then(|metadata| metadata.get("structureFeatures"))
            .and_then(Option::as_deref)
            .map(parse_csv_config)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn structureFeatures(&self) -> Vec<String> {
        self.structure_features()
    }

    pub fn structure_data(&self) -> Option<StructureData> {
        precompute::structure(self)
    }

    #[allow(non_snake_case)]
    pub fn structureData(&self) -> Option<StructureData> {
        self.structure_data()
    }

    pub fn structure(&self, node: Option<u32>) -> Option<StructureTree> {
        let data = self.structure_data()?;
        match node {
            Some(node) => self.structure_tree_from_data(node, &data),
            None => Some(StructureTree::Forest(
                data.top
                    .iter()
                    .filter_map(|node| self.structure_tree_from_data(*node, &data))
                    .collect(),
            )),
        }
    }

    pub fn structure_pretty(&self, node: Option<u32>, full_heading: bool) -> Option<String> {
        let data = self.structure_data()?;
        let tree = self.structure(node)?;
        let mut lines = Vec::new();
        self.structure_pretty_lines(&tree, &data, full_heading, "  ", &mut lines);
        Some(lines.join("\n"))
    }

    #[allow(non_snake_case)]
    pub fn structurePretty(&self, node: Option<u32>, full_heading: bool) -> Option<String> {
        self.structure_pretty(node, full_heading)
    }

    pub fn structure_info(&self) -> Option<StructureInfo> {
        let data = self.structure_data()?;
        Some(StructureInfo {
            headings: self
                .structure_types()
                .into_iter()
                .zip(self.structure_features())
                .collect(),
            node_count: data.heading_from_node.len(),
            multiple_count: data.multiple.len(),
        })
    }

    #[allow(non_snake_case)]
    pub fn structureInfo(&self) -> Option<StructureInfo> {
        self.structure_info()
    }

    pub fn top(&self) -> Option<Vec<u32>> {
        Some(self.structure_data()?.top)
    }

    pub fn structure_up(&self, node: u32) -> Option<u32> {
        self.structure_data()?.up.get(&node).copied()
    }

    pub fn structure_down(&self, node: u32) -> Option<Vec<u32>> {
        Some(
            self.structure_data()?
                .down
                .get(&node)
                .cloned()
                .unwrap_or_default(),
        )
    }

    pub fn heading_from_node(&self, node: u32) -> Option<Vec<StructureHeading>> {
        self.structure_data()?.heading_from_node.get(&node).cloned()
    }

    #[allow(non_snake_case)]
    pub fn headingFromNode(&self, node: u32) -> Option<Vec<StructureHeading>> {
        self.heading_from_node(node)
    }

    pub fn node_from_heading(&self, heading: &[StructureHeading]) -> Option<u32> {
        self.structure_data()?
            .node_from_heading
            .get(heading)
            .copied()
    }

    #[allow(non_snake_case)]
    pub fn nodeFromHeading(&self, heading: &[StructureHeading]) -> Option<u32> {
        self.node_from_heading(heading)
    }

    fn structure_tree_from_data(&self, node: u32, data: &StructureData) -> Option<StructureTree> {
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
                .filter_map(|child| self.structure_tree_from_data(*child, data))
                .collect(),
        })
    }

    fn structure_pretty_lines(
        &self,
        tree: &StructureTree,
        data: &StructureData,
        full_heading: bool,
        indent: &str,
        lines: &mut Vec<String>,
    ) {
        match tree {
            StructureTree::Forest(children) => {
                for child in children {
                    self.structure_pretty_lines(child, data, full_heading, indent, lines);
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
                    self.structure_pretty_lines(child, data, full_heading, &child_indent, lines);
                }
            }
        }
    }

    pub fn section_tuple(&self, node: u32, options: &SectionOptions) -> Vec<Option<u32>> {
        let section_types = self.section_types();
        if section_types.is_empty() || node == 0 || node > self.max_node {
            return Vec::new();
        }

        let Some(node_type) = self.node_type(node) else {
            return Vec::new();
        };
        let reference_slot = if node <= self.max_slot {
            node
        } else {
            let slots = self.slots(node);
            let Some(slot) = (if options.last_slot {
                slots.last()
            } else {
                slots.first()
            }) else {
                return Vec::new();
            };
            *slot
        };

        let mut sections = Vec::new();
        for (index, section_type) in section_types.iter().enumerate() {
            let section_node = if node_type == section_type {
                Some(node)
            } else {
                self.up(reference_slot, Some(section_type))
                    .into_iter()
                    .next()
            };
            if section_node.is_some() || options.fillup || index == 0 {
                sections.push(section_node);
                if node_type == section_type && !options.fillup {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(level) = options.level {
            sections.truncate(level);
        }
        sections
    }

    #[allow(non_snake_case)]
    pub fn sectionTuple(&self, node: u32, options: &SectionOptions) -> Vec<Option<u32>> {
        self.section_tuple(node, options)
    }

    pub fn section_from_node(
        &self,
        node: u32,
        options: &SectionOptions,
    ) -> Vec<Option<FeatureValue>> {
        self.section_from_node_lang(node, options, "en")
    }

    pub fn section_from_node_lang(
        &self,
        node: u32,
        options: &SectionOptions,
        lang: &str,
    ) -> Vec<Option<FeatureValue>> {
        let section_features = self.section_features();
        self.section_tuple(node, options)
            .into_iter()
            .enumerate()
            .map(|(index, section_node)| {
                let section_node = section_node?;
                let feature_name = if index == 0 {
                    self.section_0_feature_for_lang(lang)
                        .or_else(|| section_features.get(index).cloned())?
                } else {
                    section_features.get(index)?.clone()
                };
                self.node_feature(&feature_name)
                    .and_then(|feature| feature.v(section_node))
                    .cloned()
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn sectionFromNode(
        &self,
        node: u32,
        options: &SectionOptions,
    ) -> Vec<Option<FeatureValue>> {
        self.section_from_node(node, options)
    }

    pub fn section_ref(&self, node: u32) -> String {
        self.section_from_node(node, &SectionOptions::default())
            .into_iter()
            .filter_map(|value| value.map(section_value_to_string))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn node_from_section(&self, section: &[FeatureValue]) -> Option<u32> {
        self.node_from_section_lang(section, "en")
    }

    pub fn node_from_section_lang(&self, section: &[FeatureValue], lang: &str) -> Option<u32> {
        let section_types = self.section_types();
        let section_features = self.section_features();
        if section.is_empty()
            || section.len() > section_types.len()
            || section.len() > section_features.len()
        {
            return None;
        }

        let target_index = section.len() - 1;
        self.nodes_of_type(&section_types[target_index])
            .iter()
            .copied()
            .find(|node| {
                section.iter().enumerate().all(|(index, expected)| {
                    let options = SectionOptions {
                        fillup: true,
                        level: Some(index + 1),
                        ..SectionOptions::default()
                    };
                    self.section_from_node_lang(*node, &options, lang)
                        .get(index)
                        .and_then(Option::as_ref)
                        == Some(expected)
                })
            })
    }

    #[allow(non_snake_case)]
    pub fn nodeFromSection(&self, section: &[FeatureValue]) -> Option<u32> {
        self.node_from_section(section)
    }

    pub fn languages(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.section_0_language_features()
            .into_iter()
            .filter_map(|(code, feature_name)| {
                let feature = self.node_feature(&feature_name)?;
                let mut metadata = BTreeMap::new();
                metadata.insert(
                    "language".to_string(),
                    feature
                        .metadata
                        .get("language")
                        .and_then(Option::clone)
                        .unwrap_or_else(|| "default".to_string()),
                );
                metadata.insert(
                    "languageEnglish".to_string(),
                    feature
                        .metadata
                        .get("languageEnglish")
                        .and_then(Option::clone)
                        .unwrap_or_else(|| "default".to_string()),
                );
                Some((code, metadata))
            })
            .collect()
    }

    pub fn name_from_node(&self, lang: &str) -> BTreeMap<u32, String> {
        let Some(feature_name) = self.section_0_feature_for_lang(lang) else {
            return BTreeMap::new();
        };
        let Some(feature) = self.node_feature(&feature_name) else {
            return BTreeMap::new();
        };
        feature
            .values
            .iter()
            .filter_map(|(node, value)| value.as_str().map(|value| (*node, value.to_string())))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn nameFromNode(&self, lang: &str) -> BTreeMap<u32, String> {
        self.name_from_node(lang)
    }

    pub fn node_from_name(&self, lang: &str) -> BTreeMap<(String, String), u32> {
        let section_0_type = self.section_types().into_iter().next().unwrap_or_default();
        let names = self.name_from_node(lang);
        self.nodes_of_type(&section_0_type)
            .into_iter()
            .filter_map(|node| {
                names
                    .get(node)
                    .map(|name| ((section_0_type.clone(), name.clone()), *node))
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn nodeFromName(&self, lang: &str) -> BTreeMap<(String, String), u32> {
        self.node_from_name(lang)
    }

    pub fn section_0_name(&self, node: u32, lang: &str) -> Option<String> {
        let section_0_type = self.section_types().into_iter().next()?;
        let section_0_node = if self.node_type(node)? == section_0_type {
            node
        } else {
            self.u(node, Some(&section_0_type)).into_iter().next()?
        };
        self.name_from_node(lang).get(&section_0_node).cloned()
    }

    pub fn section_0_node(&self, name: &str, lang: &str) -> Option<u32> {
        let section_0_type = self.section_types().into_iter().next().unwrap_or_default();
        self.node_from_name(lang)
            .get(&(section_0_type, name.to_string()))
            .copied()
    }

    #[allow(non_snake_case)]
    pub fn bookName(&self, node: u32, lang: &str) -> Option<String> {
        self.section_0_name(node, lang)
    }

    #[allow(non_snake_case)]
    pub fn bookNode(&self, name: &str, lang: &str) -> Option<u32> {
        self.section_0_node(name, lang)
    }

    fn section_0_language_features(&self) -> BTreeMap<String, String> {
        let Some(section_0_type) = self.section_types().into_iter().next() else {
            return BTreeMap::new();
        };
        self.node_features
            .iter()
            .filter_map(|(name, feature)| {
                let code = feature
                    .metadata
                    .get("languageCode")
                    .and_then(Option::as_deref)?;
                let has_section_0_values = self
                    .nodes_of_type(&section_0_type)
                    .iter()
                    .any(|node| feature.v(*node).is_some());
                has_section_0_values.then(|| (code.to_string(), name.clone()))
            })
            .collect()
    }

    fn section_0_feature_for_lang(&self, lang: &str) -> Option<String> {
        let language_features = self.section_0_language_features();
        if let Some(feature) = language_features.get(lang) {
            return Some(feature.clone());
        }
        if let Some(feature) = language_features.get("") {
            return Some(feature.clone());
        }
        self.section_features().into_iter().next()
    }

    pub fn text(&self, node: u32, format: Option<&str>) -> String {
        self.text_with_options(node, &TextOptions::new(format.map(str::to_string), None))
    }

    pub fn text_with_options(&self, node: u32, options: &TextOptions) -> String {
        if node == 0 || node > self.max_node {
            return String::new();
        }
        let Some(node_type) = self.node_type(node) else {
            return String::new();
        };
        let Some(resolved) = self.resolve_text_format(node_type, options.format.as_deref()) else {
            return String::new();
        };

        let down_type = match options.descend {
            Some(true) => Some(resolved.target_type.as_str()),
            Some(false) => None,
            None => {
                if resolved.implicit_node_default {
                    None
                } else {
                    Some(resolved.target_type.as_str())
                }
            }
        }
        .filter(|down_type| *down_type != node_type);

        let nodes = match down_type {
            Some(down_type) if down_type == self.slot_type => self.slots(node),
            Some(down_type) => self.d(node, Some(down_type)),
            None => vec![node],
        };
        nodes
            .into_iter()
            .map(|text_node| self.text_for_node(text_node, &resolved.spec))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn textWithOptions(&self, node: u32, options: &TextOptions) -> String {
        self.text_with_options(node, options)
    }

    fn resolve_text_format(
        &self,
        node_type: &str,
        format: Option<&str>,
    ) -> Option<ResolvedTextFormat> {
        let otext = self.config_feature("otext")?;
        let (format_name, implicit_node_default) = match format {
            Some(format) => (
                format.strip_prefix("fmt:").unwrap_or(format).to_string(),
                false,
            ),
            None => {
                let node_default = format!("{node_type}-default");
                if otext.contains_key(&format!("fmt:{node_default}")) {
                    (node_default, true)
                } else {
                    ("text-orig-full".to_string(), false)
                }
            }
        };
        let format_key = format!("fmt:{format_name}");
        let Some(spec) = self
            .config_feature("otext")
            .and_then(|metadata| metadata.get(&format_key))
            .and_then(Option::as_deref)
        else {
            return None;
        };
        let (target_type, spec) = self.split_format(spec);
        Some(ResolvedTextFormat {
            target_type,
            spec,
            implicit_node_default,
        })
    }

    pub fn text_nodes(&self, nodes: &[u32], format: Option<&str>) -> String {
        nodes
            .iter()
            .map(|node| self.text(*node, format))
            .collect::<String>()
    }

    pub fn split_format(&self, template: &str) -> (String, String) {
        let mut parts = template.splitn(2, '#');
        let first = parts.next().unwrap_or_default();
        let Some(rest) = parts.next() else {
            return (self.slot_type.clone(), template.to_string());
        };
        if self.nodes_by_type.contains_key(first) {
            (first.to_string(), rest.to_string())
        } else {
            (self.slot_type.clone(), template.to_string())
        }
    }

    #[allow(non_snake_case)]
    pub fn splitFormat(&self, template: &str) -> (String, String) {
        self.split_format(template)
    }

    pub fn split_default_format(&self, template: &str) -> Option<String> {
        let (node_type, suffix) = template.rsplit_once('-')?;
        (suffix == "default" && self.nodes_by_type.contains_key(node_type))
            .then(|| node_type.to_string())
    }

    #[allow(non_snake_case)]
    pub fn splitDefaultFormat(&self, template: &str) -> Option<String> {
        self.split_default_format(template)
    }

    fn text_for_node(&self, node: u32, spec: &str) -> String {
        let mut rendered = String::new();
        let mut rest = spec;
        while let Some(start) = rest.find('{') {
            rendered.push_str(&render_format_literal(&rest[..start]));
            let after_start = &rest[(start + 1)..];
            let Some(end) = after_start.find('}') else {
                rendered.push_str(&render_format_literal(&rest[start..]));
                return rendered;
            };
            let placeholder = &after_start[..end];
            rendered.push_str(&self.placeholder_value(node, placeholder));
            rest = &after_start[(end + 1)..];
        }
        rendered.push_str(&render_format_literal(rest));
        rendered
    }

    fn placeholder_value(&self, slot: u32, placeholder: &str) -> String {
        let (features, default) = placeholder
            .split_once(':')
            .map(|(features, default)| (features, Some(default)))
            .unwrap_or((placeholder, None));
        features
            .split('/')
            .find_map(|feature_name| {
                let value = self
                    .node_feature(feature_name)
                    .and_then(|feature| feature.str_value(slot))
                    .unwrap_or_default();
                (!value.is_empty()).then(|| value.to_string())
            })
            .unwrap_or_else(|| default.map(render_format_literal).unwrap_or_default())
    }

    pub fn node_feature_types(&self, feature_name: &str) -> Vec<&str> {
        let Some(feature) = self.node_feature(feature_name) else {
            return Vec::new();
        };
        self.levels()
            .into_iter()
            .filter_map(|(node_type, _, min_node, max_node)| {
                (min_node..=max_node)
                    .any(|node| feature.v(node).is_some())
                    .then_some(node_type)
            })
            .collect()
    }

    pub fn edge_feature_source_types(&self, feature_name: &str) -> Vec<&str> {
        let Some(feature) = self.edge_feature(feature_name) else {
            return Vec::new();
        };
        self.levels()
            .into_iter()
            .filter_map(|(node_type, _, min_node, max_node)| {
                (min_node..=max_node)
                    .any(|node| {
                        feature
                            .targets(node)
                            .is_some_and(|targets| !targets.is_empty())
                    })
                    .then_some(node_type)
            })
            .collect()
    }

    pub fn edge_feature_target_types(&self, feature_name: &str) -> Vec<&str> {
        let Some(feature) = self.edge_feature(feature_name) else {
            return Vec::new();
        };
        let mut targets_by_type = Vec::new();
        for (node_type, _, min_node, max_node) in self.levels() {
            let has_target = feature.items().into_iter().any(|(_, targets)| {
                targets
                    .into_iter()
                    .any(|target| (min_node..=max_node).contains(&target))
            });
            if has_target {
                targets_by_type.push(node_type);
            }
        }
        targets_by_type
    }

    pub fn feature_catalog(
        &self,
        kind: Option<FeatureKind>,
        node_types: Option<&[&str]>,
    ) -> Vec<FeatureCatalogEntry> {
        let mut rows = Vec::new();
        if kind.is_none_or(|kind| kind == FeatureKind::Node) {
            for feature in self.node_features.values() {
                let applies_to_requested_type = node_types.is_none_or(|requested| {
                    let feature_types = self.node_feature_types(&feature.name);
                    requested
                        .iter()
                        .any(|node_type| feature_types.contains(node_type))
                });
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
            for feature in self.edge_features.values() {
                let applies_to_requested_type = node_types.is_none_or(|requested| {
                    let feature_types = self.edge_feature_source_types(&feature.name);
                    requested
                        .iter()
                        .any(|node_type| feature_types.contains(node_type))
                });
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
        rows
    }

    pub fn describe_feature(&self, feature_name: &str, sample_limit: usize) -> FeatureDescription {
        if let Some(feature) = self.node_feature(feature_name) {
            let frequency_list = feature.frequency_list();
            return FeatureDescription {
                name: feature.name.clone(),
                kind: Some(FeatureKind::Node),
                value_type: feature.value_type().unwrap_or("str").to_string(),
                description: feature.description().unwrap_or_default().to_string(),
                node_types: self
                    .node_feature_types(&feature.name)
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
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
            };
        }

        if let Some(feature) = self.edge_feature(feature_name) {
            let has_values = feature.has_edge_values();
            let (unique_values, sample_values) = match feature.frequency_list() {
                EdgeFrequency::Count(_) => (0, Vec::new()),
                EdgeFrequency::Values(rows) => {
                    let unique_values = rows.len();
                    let sample_values = rows
                        .into_iter()
                        .take(sample_limit)
                        .map(|(value, count)| FeatureValueSample { value, count })
                        .collect();
                    (unique_values, sample_values)
                }
            };
            return FeatureDescription {
                name: feature.name.clone(),
                kind: Some(FeatureKind::Edge),
                value_type: feature.value_type().unwrap_or("str").to_string(),
                description: feature.description().unwrap_or_default().to_string(),
                node_types: self
                    .edge_feature_source_types(&feature.name)
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                unique_values,
                sample_values,
                has_values: Some(has_values),
                error: None,
            };
        }

        FeatureDescription {
            name: feature_name.to_string(),
            kind: None,
            value_type: String::new(),
            description: String::new(),
            node_types: Vec::new(),
            unique_values: 0,
            sample_values: Vec::new(),
            has_values: None,
            error: Some(format!("Feature '{feature_name}' not found")),
        }
    }

    pub fn describe_features(
        &self,
        feature_names: &[&str],
        sample_limit: usize,
    ) -> BTreeMap<String, FeatureDescription> {
        feature_names
            .iter()
            .map(|feature_name| {
                (
                    (*feature_name).to_string(),
                    self.describe_feature(feature_name, sample_limit),
                )
            })
            .collect()
    }

    pub fn all_node_feature_types(&self) -> BTreeMap<String, Vec<String>> {
        self.node_features
            .keys()
            .map(|feature_name| {
                (
                    feature_name.clone(),
                    self.node_feature_types(feature_name)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                )
            })
            .collect()
    }

    pub fn all_edge_feature_source_types(&self) -> BTreeMap<String, Vec<String>> {
        self.edge_features
            .keys()
            .map(|feature_name| {
                (
                    feature_name.clone(),
                    self.edge_feature_source_types(feature_name)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                )
            })
            .collect()
    }

    pub fn all_edge_feature_target_types(&self) -> BTreeMap<String, Vec<String>> {
        self.edge_features
            .keys()
            .map(|feature_name| {
                (
                    feature_name.clone(),
                    self.edge_feature_target_types(feature_name)
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                )
            })
            .collect()
    }

    pub fn text_representations(&self) -> TextRepresentationInfo {
        let pairs = self.text_format_pairs();
        if pairs.is_empty() {
            return TextRepresentationInfo {
                description: "No text format metadata available or no orig/trans pairs defined"
                    .to_string(),
                formats: Vec::new(),
            };
        }

        let formats = pairs
            .into_iter()
            .filter_map(
                |(
                    base_name,
                    original_name,
                    transliterated_name,
                    original_spec,
                    transliterated_spec,
                )| {
                    let (samples, unique_characters) =
                        self.text_format_samples(&original_name, &transliterated_name);
                    (!samples.is_empty()).then_some(TextFormatInfo {
                        name: base_name,
                        original_spec,
                        transliteration_spec: transliterated_spec,
                        total_samples: samples.len(),
                        samples,
                        unique_characters,
                    })
                },
            )
            .collect();

        TextRepresentationInfo {
            description: "Shows how text values are encoded in this corpus. Samples provide exhaustive character coverage for understanding the relationship between original script and transliterated forms.".to_string(),
            formats,
        }
    }

    fn text_format_pairs(&self) -> Vec<(String, String, String, String, String)> {
        let Some(otext) = self.config_feature("otext") else {
            return Vec::new();
        };
        let mut originals = BTreeMap::<String, (String, String)>::new();
        let mut transliterations = BTreeMap::<String, (String, String)>::new();
        for (key, value) in otext {
            let Some(format_name) = key.strip_prefix("fmt:") else {
                continue;
            };
            let Some(spec) = value.as_deref() else {
                continue;
            };
            if format_name.contains("-orig-") {
                originals.insert(
                    format_name.replace("-orig-", "-"),
                    (format_name.to_string(), spec.to_string()),
                );
            } else if format_name.contains("-trans-") {
                transliterations.insert(
                    format_name.replace("-trans-", "-"),
                    (format_name.to_string(), spec.to_string()),
                );
            }
        }

        originals
            .into_iter()
            .filter_map(|(base_name, (original_name, original_spec))| {
                let (transliterated_name, transliterated_spec) =
                    transliterations.get(&base_name)?.clone();
                Some((
                    base_name,
                    original_name,
                    transliterated_name,
                    original_spec,
                    transliterated_spec,
                ))
            })
            .collect()
    }

    fn text_format_samples(
        &self,
        original_name: &str,
        transliterated_name: &str,
    ) -> (Vec<TextFormatSample>, usize) {
        let mut samples = Vec::new();
        let mut covered_chars = HashSet::new();
        let mut seen_originals = HashSet::new();
        for slot in 1..=self.max_slot {
            let original = self.text(slot, Some(original_name)).trim().to_string();
            if original.is_empty() || seen_originals.contains(&original) {
                continue;
            }
            let new_chars = original
                .chars()
                .filter(|ch| !covered_chars.contains(ch))
                .collect::<Vec<_>>();
            if new_chars.is_empty() {
                continue;
            }
            let transliterated = self
                .text(slot, Some(transliterated_name))
                .trim()
                .to_string();
            if transliterated.is_empty() {
                continue;
            }
            covered_chars.extend(new_chars);
            seen_originals.insert(original.clone());
            samples.push(TextFormatSample {
                original,
                transliterated,
            });
        }
        (samples, covered_chars.len())
    }

    pub fn computed_names(&self) -> [&'static str; 4] {
        ["levels", "order", "rank", "boundary"]
    }

    fn loaded_feature_names(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        names.extend(self.node_features.keys().cloned());
        names.extend(self.edge_features.keys().cloned());
        names.extend(self.config_features.keys().cloned());
        names.extend(self.computed_names().iter().map(|name| name.to_string()));
        names.into_iter().collect()
    }

    fn loaded_feature_info(&self, name: &str) -> Option<LoadedFeatureInfo> {
        if let Some(feature) = self.node_feature(name) {
            let mut metadata = feature.meta().clone();
            let value_type = remove_value_type_metadata(&mut metadata);
            return Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Node,
                value_type,
                metadata,
                edge_values: None,
            });
        }
        if let Some(feature) = self.edge_feature(name) {
            let mut metadata = feature.meta().clone();
            let value_type = remove_value_type_metadata(&mut metadata);
            return Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Edge,
                value_type,
                metadata,
                edge_values: Some(name != "oslots" && feature.has_edge_values()),
            });
        }
        if let Some(metadata) = self.config_feature(name) {
            let mut metadata = metadata.clone();
            let value_type = remove_value_type_metadata(&mut metadata);
            return Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Config,
                value_type,
                metadata,
                edge_values: None,
            });
        }
        if self.computed_names().contains(&name) {
            return Some(LoadedFeatureInfo {
                kind: LoadedFeatureKind::Computed,
                value_type: String::new(),
                metadata: BTreeMap::new(),
                edge_values: None,
            });
        }
        None
    }

    pub fn levels(&self) -> Vec<(&str, f64, u32, u32)> {
        let mut rows: Vec<_> = self
            .nodes_by_type
            .iter()
            .map(|(node_type, nodes)| {
                let slot_count: usize = nodes
                    .iter()
                    .map(|node| {
                        if *node <= self.max_slot {
                            1
                        } else {
                            self.slots_of(*node).map(<[u32]>::len).unwrap_or_default()
                        }
                    })
                    .sum();
                let average_slots = slot_count as f64 / nodes.len().max(1) as f64;
                let min_node = nodes.first().copied().unwrap_or_default();
                let max_node = nodes.last().copied().unwrap_or_default();
                (node_type.as_str(), average_slots, min_node, max_node)
            })
            .collect();
        rows.sort_unstable_by_key(|(node_type, _, _, _)| {
            std::cmp::Reverse(self.type_ranks.get(*node_type).copied().unwrap_or_default())
        });
        rows
    }

    pub fn otype_rank(&self) -> BTreeMap<String, u32> {
        self.type_ranks
            .iter()
            .map(|(node_type, rank)| (node_type.clone(), *rank))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn otypeRank(&self) -> BTreeMap<String, u32> {
        self.otype_rank()
    }

    pub fn nodes_of_type(&self, node_type: &str) -> &[u32] {
        self.nodes_by_type
            .get(node_type)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn node_type_items(&self) -> Vec<(u32, String)> {
        (1..=self.max_node)
            .filter_map(|node| {
                self.node_type(node)
                    .map(|node_type| (node, node_type.to_string()))
            })
            .collect()
    }

    pub fn otype_items(&self) -> Vec<(u32, String)> {
        self.node_type_items()
    }

    #[allow(non_snake_case)]
    pub fn otypeItems(&self) -> Vec<(u32, String)> {
        self.node_type_items()
    }

    pub fn max_slot(&self) -> u32 {
        self.max_slot
    }

    #[allow(non_snake_case)]
    pub fn maxSlot(&self) -> u32 {
        self.max_slot()
    }

    pub fn max_node(&self) -> u32 {
        self.max_node
    }

    #[allow(non_snake_case)]
    pub fn maxNode(&self) -> u32 {
        self.max_node()
    }

    pub fn slot_type(&self) -> &str {
        &self.slot_type
    }

    #[allow(non_snake_case)]
    pub fn slotType(&self) -> &str {
        self.slot_type()
    }

    pub fn node_type_interval(&self, node_type: &str) -> Option<(u32, u32)> {
        let nodes = self.nodes_by_type.get(node_type)?;
        Some((*nodes.first()?, *nodes.last()?))
    }

    pub fn s_interval(&self, node_type: &str) -> Option<(u32, u32)> {
        self.node_type_interval(node_type)
    }

    #[allow(non_snake_case)]
    pub fn sInterval(&self, node_type: &str) -> Option<(u32, u32)> {
        self.node_type_interval(node_type)
    }

    pub fn feature_nodes(&self) -> Vec<u32> {
        let Some(otype) = self.node_feature("otype") else {
            return Vec::new();
        };
        let mut nodes = otype.values.keys().copied().collect::<Vec<_>>();
        nodes.sort_unstable();
        nodes
    }

    pub fn node_frequency_list(
        &self,
        feature_name: &str,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<(FeatureValue, usize)>> {
        let feature = self
            .node_feature(feature_name)
            .ok_or_else(|| CfError::MissingFeature(feature_name.to_string()))?;
        let mut counts: HashMap<FeatureValue, usize> = HashMap::new();
        for (node, value) in &feature.values {
            if self.node_type_allowed(*node, node_types) {
                *counts.entry(value.clone()).or_default() += 1;
            }
        }
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_unstable_by_key(|(value, count)| (std::cmp::Reverse(*count), value.sort_token()));
        Ok(rows)
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
        let feature = self
            .edge_feature(feature_name)
            .ok_or_else(|| CfError::MissingFeature(feature_name.to_string()))?;
        if !feature.has_edge_values() {
            let mut count = 0_usize;
            for (source, targets) in &feature.values {
                if !self.node_type_allowed(*source, node_types_from) {
                    continue;
                }
                count += targets
                    .iter()
                    .filter(|target| self.node_type_allowed(**target, node_types_to))
                    .count();
            }
            return Ok(EdgeFrequency::Count(count));
        }

        let mut counts: HashMap<Option<FeatureValue>, usize> = HashMap::new();
        for (source, targets) in &feature.values {
            if !self.node_type_allowed(*source, node_types_from) {
                continue;
            }
            for target in targets {
                if !self.node_type_allowed(*target, node_types_to) {
                    continue;
                }
                let value = feature.edge_value(*source, *target).cloned();
                *counts.entry(value).or_default() += 1;
            }
        }
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_unstable_by_key(|(value, count)| {
            (
                std::cmp::Reverse(*count),
                optional_feature_value_sort_token(value),
            )
        });
        Ok(EdgeFrequency::Values(rows))
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

    pub fn slots_of(&self, node: u32) -> Option<&[u32]> {
        self.edge_feature("oslots")
            .and_then(|feature| feature.targets(node))
    }

    pub fn slots(&self, node: u32) -> Vec<u32> {
        if node == 0 || node > self.max_node {
            return Vec::new();
        }
        if node <= self.max_slot {
            return vec![node];
        }
        self.slots_of(node).map(<[u32]>::to_vec).unwrap_or_default()
    }

    pub fn oslots_items(&self) -> Vec<(u32, Vec<u32>)> {
        let mut rows = Vec::new();
        for node in (self.max_slot + 1)..=self.max_node {
            if let Some(slots) = self.slots_of(node) {
                rows.push((node, slots.to_vec()));
            }
        }
        rows
    }

    pub fn contains(&self, parent: u32, child: u32) -> bool {
        if child <= self.max_slot {
            return self
                .slots_of(parent)
                .is_some_and(|slots| slots.binary_search(&child).is_ok());
        }
        let Some(child_slots) = self.slots_of(child) else {
            return false;
        };
        self.slots_of(parent).is_some_and(|parent_slots| {
            child_slots
                .iter()
                .all(|slot| parent_slots.binary_search(slot).is_ok())
        })
    }

    pub fn all_nodes(&self) -> Vec<u32> {
        let mut nodes: Vec<_> = (1..=self.max_node).collect();
        self.sort_nodes(&mut nodes);
        nodes
    }

    #[allow(non_snake_case)]
    pub fn allNodes(&self) -> Vec<u32> {
        self.all_nodes()
    }

    pub fn walk(&self, nodes: Option<&[u32]>) -> Vec<u32> {
        match nodes {
            Some(nodes) => {
                let mut nodes = nodes.to_vec();
                self.sort_nodes(&mut nodes);
                nodes
            }
            None => self.order(),
        }
    }

    #[allow(non_snake_case)]
    pub fn walkEvents(&self, nodes: Option<&[u32]>) -> Vec<WalkEvent> {
        self.walk_events(nodes)
    }

    pub fn walk_events(&self, nodes: Option<&[u32]>) -> Vec<WalkEvent> {
        let walk_nodes = self.walk(nodes);
        let walk_node_set: Option<HashSet<u32>> =
            nodes.map(|nodes| nodes.iter().copied().collect());
        let boundary = self.boundary_ref();
        let mut events = Vec::new();

        for node in walk_nodes {
            if self.node_type(node) == Some(self.slot_type.as_str()) {
                events.push(WalkEvent::Slot(node));
                if let Some(ending_nodes) = boundary.last_slots.get((node - 1) as usize) {
                    for ending_node in ending_nodes.iter().rev() {
                        if walk_node_set
                            .as_ref()
                            .is_none_or(|node_set| node_set.contains(ending_node))
                        {
                            events.push(WalkEvent::End(*ending_node));
                        }
                    }
                }
            } else {
                events.push(WalkEvent::Start(node));
            }
        }

        events
    }

    pub fn order(&self) -> Vec<u32> {
        self.order_array().to_vec()
    }

    pub fn rank(&self) -> Vec<u32> {
        self.rank_array().to_vec()
    }

    fn order_array(&self) -> &[u32] {
        if let Some(order) = &self.order {
            return order;
        }
        self.order_cache.get_or_init(|| self.compute_order())
    }

    fn rank_array(&self) -> &[u32] {
        if let Some(rank) = &self.rank {
            return rank;
        }
        self.rank_cache.get_or_init(|| self.compute_rank())
    }

    fn compute_rank(&self) -> Vec<u32> {
        let order = self.order_array();
        let mut rank = vec![0_u32; self.max_node as usize];
        for (index, node) in order.iter().enumerate() {
            if let Some(slot) = rank.get_mut((*node - 1) as usize) {
                *slot = index as u32;
            }
        }
        rank
    }

    pub fn boundary(&self) -> Boundary {
        self.boundary_ref().clone()
    }

    fn boundary_ref(&self) -> &Boundary {
        self.boundary_cache.get_or_init(|| self.compute_boundary())
    }

    fn compute_boundary(&self) -> Boundary {
        let rank = self.rank_array();
        let mut first_slots = vec![Vec::new(); self.max_slot as usize];
        let mut last_slots = vec![Vec::new(); self.max_slot as usize];

        for node in (self.max_slot + 1)..=self.max_node {
            let Some(slots) = self.slots_of(node) else {
                continue;
            };
            let (Some(first), Some(last)) = (slots.first(), slots.last()) else {
                continue;
            };
            if let Some(nodes) = first_slots.get_mut((*first - 1) as usize) {
                nodes.push(node);
            }
            if let Some(nodes) = last_slots.get_mut((*last - 1) as usize) {
                nodes.push(node);
            }
        }

        for nodes in &mut first_slots {
            nodes.sort_unstable_by_key(|node| {
                std::cmp::Reverse(
                    rank.get((*node).saturating_sub(1) as usize)
                        .copied()
                        .unwrap_or_default(),
                )
            });
        }
        for nodes in &mut last_slots {
            nodes.sort_unstable_by_key(|node| {
                rank.get((*node).saturating_sub(1) as usize)
                    .copied()
                    .unwrap_or(u32::MAX)
            });
        }

        Boundary {
            first_slots,
            last_slots,
        }
    }

    pub fn sort_nodes(&self, nodes: &mut [u32]) {
        let rank = self.rank_array();
        nodes.sort_unstable_by_key(|node| {
            rank.get((*node).saturating_sub(1) as usize)
                .copied()
                .unwrap_or(u32::MAX)
        });
    }

    pub fn sorted_nodes(&self, nodes: impl IntoIterator<Item = u32>) -> Vec<u32> {
        let mut nodes = nodes.into_iter().collect::<Vec<_>>();
        self.sort_nodes(&mut nodes);
        nodes
    }

    #[allow(non_snake_case)]
    pub fn sortNodes(&self, nodes: impl IntoIterator<Item = u32>) -> Vec<u32> {
        self.sorted_nodes(nodes)
    }

    pub fn sort_key(&self, node: u32) -> usize {
        self.rank_array()
            .get(node.saturating_sub(1) as usize)
            .copied()
            .map(|rank| rank as usize)
            .unwrap_or_else(|| self.order_rank(node))
    }

    #[allow(non_snake_case)]
    pub fn sortKey(&self, node: u32) -> usize {
        self.sort_key(node)
    }

    pub fn sort_key_tuple(&self, nodes: &[u32]) -> Vec<usize> {
        nodes.iter().map(|node| self.sort_key(*node)).collect()
    }

    #[allow(non_snake_case)]
    pub fn sortKeyTuple(&self, nodes: &[u32]) -> Vec<usize> {
        self.sort_key_tuple(nodes)
    }

    pub fn sort_key_chunk(&self, chunk: Chunk) -> ChunkPositionKey {
        ChunkPositionKey::new(
            chunk.start,
            self.type_rank(chunk.node),
            chunk.end,
            chunk.node,
        )
    }

    #[allow(non_snake_case)]
    pub fn sortKeyChunk(&self, chunk: Chunk) -> ChunkPositionKey {
        self.sort_key_chunk(chunk)
    }

    pub fn sort_key_chunk_length(&self, chunk: Chunk) -> ChunkLengthKey {
        ChunkLengthKey::new(
            chunk.end.saturating_sub(chunk.start),
            chunk.start,
            self.type_rank(chunk.node),
            chunk.node,
        )
    }

    #[allow(non_snake_case)]
    pub fn sortKeyChunkLength(&self, chunk: Chunk) -> ChunkLengthKey {
        self.sort_key_chunk_length(chunk)
    }

    pub fn order_rank(&self, node: u32) -> usize {
        (1..=self.max_node)
            .filter(|candidate| self.compare_nodes(*candidate, node) == Ordering::Less)
            .count()
    }

    pub fn up(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        match node_type {
            Some(node_type) => self.up_types(node, Some(&[node_type])),
            None => self.up_types(node, None),
        }
    }

    pub fn u(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.up(node, node_type)
    }

    pub fn intersecting(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        match node_type {
            Some(node_type) => self.intersecting_types(node, Some(&[node_type])),
            None => self.intersecting_types(node, None),
        }
    }

    pub fn i(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.intersecting(node, node_type)
    }

    pub fn intersecting_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        if node <= self.max_slot || node > self.max_node {
            return Vec::new();
        }
        let slots = self.slots(node);
        if slots.is_empty() {
            return Vec::new();
        }
        let slot_set = slots.iter().copied().collect::<HashSet<_>>();
        let mut result = Vec::new();
        if self.node_type_allowed(1, node_types) {
            result.extend(slots.iter().copied());
        }
        for candidate in (self.max_slot + 1)..=self.max_node {
            if candidate == node || !self.node_type_allowed(candidate, node_types) {
                continue;
            }
            if self.slots_of(candidate).is_some_and(|candidate_slots| {
                candidate_slots.iter().any(|slot| slot_set.contains(slot))
            }) {
                result.push(candidate);
            }
        }
        self.sort_nodes(&mut result);
        result.reverse();
        result
    }

    pub fn up_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        let mut result = Vec::new();
        for candidate in (self.max_slot + 1)..=self.max_node {
            if candidate == node || !self.contains(candidate, node) {
                continue;
            }
            if !self.node_type_allowed(candidate, node_types) {
                continue;
            }
            result.push(candidate);
        }
        self.sort_nodes(&mut result);
        result.reverse();
        result
    }

    pub fn u_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.up_types(node, node_types)
    }

    pub fn down(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        match node_type {
            Some(node_type) => self.down_types(node, Some(&[node_type])),
            None => self.down_types(node, None),
        }
    }

    pub fn d(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.down(node, node_type)
    }

    pub fn down_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        let Some(slots) = self.slots_of(node) else {
            return Vec::new();
        };
        let mut result: Vec<u32> = Vec::new();
        if node_types.is_none_or(|expected| expected.contains(&self.slot_type.as_str())) {
            result.extend_from_slice(slots);
        }
        for candidate in (self.max_slot + 1)..=self.max_node {
            if candidate == node || !self.contains(node, candidate) {
                continue;
            }
            if !self.node_type_allowed(candidate, node_types) {
                continue;
            }
            result.push(candidate);
        }
        self.sort_nodes(&mut result);
        result
    }

    pub fn d_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.down_types(node, node_types)
    }

    pub fn next(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        match node_type {
            Some(node_type) => self.next_types(node, Some(&[node_type])),
            None => self.next_types(node, None),
        }
    }

    pub fn n(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.next(node, node_type)
    }

    pub fn next_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        if node == 0 || node == self.max_slot || node > self.max_node {
            return Vec::new();
        }
        let Some(next_slot) = self.adjacent_slot(node, true) else {
            return Vec::new();
        };
        if next_slot > self.max_slot {
            return Vec::new();
        }

        let boundary = self.boundary_ref();
        let mut result = vec![next_slot];
        if let Some(nodes) = boundary.first_slots.get((next_slot - 1) as usize) {
            result.extend_from_slice(nodes);
        }
        self.filter_locality_nodes(result, node_types)
    }

    pub fn previous(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        match node_type {
            Some(node_type) => self.previous_types(node, Some(&[node_type])),
            None => self.previous_types(node, None),
        }
    }

    pub fn p(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.previous(node, node_type)
    }

    pub fn previous_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        if node <= 1 || node > self.max_node {
            return Vec::new();
        }
        let Some(previous_slot) = self.adjacent_slot(node, false) else {
            return Vec::new();
        };
        if previous_slot == 0 {
            return Vec::new();
        }

        let boundary = self.boundary_ref();
        let mut result = boundary
            .last_slots
            .get((previous_slot - 1) as usize)
            .cloned()
            .unwrap_or_default();
        result.push(previous_slot);
        self.filter_locality_nodes(result, node_types)
    }

    fn adjacent_slot(&self, node: u32, forward: bool) -> Option<u32> {
        if node <= self.max_slot {
            return if forward {
                node.checked_add(1)
            } else {
                node.checked_sub(1)
            };
        }
        let slots = self.slots_of(node)?;
        if forward {
            slots.last().and_then(|slot| slot.checked_add(1))
        } else {
            slots.first().and_then(|slot| slot.checked_sub(1))
        }
    }

    fn filter_locality_nodes(&self, nodes: Vec<u32>, node_types: Option<&[&str]>) -> Vec<u32> {
        let Some(node_types) = node_types else {
            return nodes;
        };
        nodes
            .into_iter()
            .filter(|node| self.node_type_allowed(*node, Some(node_types)))
            .collect()
    }

    fn node_type_allowed(&self, node: u32, node_types: Option<&[&str]>) -> bool {
        node_types.is_none_or(|node_types| {
            self.node_type(node)
                .is_some_and(|node_type| node_types.contains(&node_type))
        })
    }

    pub fn node_type(&self, node: u32) -> Option<&str> {
        self.node_feature("otype")
            .and_then(|otype| otype.str_value(node))
    }

    fn type_rank(&self, node: u32) -> u32 {
        let node_type = self.node_type(node).unwrap_or(&self.slot_type);
        self.type_ranks.get(node_type).copied().unwrap_or(u32::MAX)
    }

    fn node_slots(&self, node: u32) -> Vec<u32> {
        if node <= self.max_slot {
            vec![node]
        } else {
            self.slots_of(node).map(<[u32]>::to_vec).unwrap_or_default()
        }
    }

    fn compare_nodes(&self, left: u32, right: u32) -> Ordering {
        if left == right {
            return Ordering::Equal;
        }
        let left_slots = self.node_slots(left);
        let right_slots = self.node_slots(right);
        let left_rank = self.type_rank(left);
        let right_rank = self.type_rank(right);

        if left_slots == right_slots {
            return if left_rank == right_rank {
                left.cmp(&right)
            } else if left_rank > right_rank {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        if is_superset(&left_slots, &right_slots) {
            return Ordering::Less;
        }
        if is_superset(&right_slots, &left_slots) {
            return Ordering::Greater;
        }

        let left_min = min_difference(&left_slots, &right_slots).unwrap_or(u32::MAX);
        let right_min = min_difference(&right_slots, &left_slots).unwrap_or(u32::MAX);
        left_min.cmp(&right_min)
    }

    fn compute_order(&self) -> Vec<u32> {
        let mut nodes: Vec<_> = (1..=self.max_node).collect();
        nodes.sort_unstable_by(|left, right| self.compare_nodes(*left, *right));
        nodes
    }
}

fn is_superset(left: &[u32], right: &[u32]) -> bool {
    right.iter().all(|value| left.binary_search(value).is_ok())
}

fn min_difference(left: &[u32], right: &[u32]) -> Option<u32> {
    left.iter()
        .copied()
        .find(|value| right.binary_search(value).is_err())
}

fn optional_feature_value_sort_token(value: &Option<FeatureValue>) -> String {
    match value {
        Some(value) => value.sort_token(),
        None => "n:".to_string(),
    }
}

fn render_format_literal(raw: &str) -> String {
    let mut rendered = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            rendered.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => rendered.push('\t'),
            Some('n') => rendered.push('\n'),
            Some(other) => {
                rendered.push('\\');
                rendered.push(other);
            }
            None => rendered.push('\\'),
        }
    }
    rendered
}

fn is_config_feature_file(path: &Path) -> Result<bool> {
    let file = File::open(path).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut lines = BufReader::new(file).lines();
    let Some(first) = lines.next() else {
        return Ok(false);
    };
    let first = first.map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(first.trim_end() == "@config")
}

fn parse_csv_config(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|item| {
            let item = item.trim();
            (!item.is_empty()).then(|| item.to_string())
        })
        .collect()
}

fn section_value_to_string(value: FeatureValue) -> String {
    match value {
        FeatureValue::Str(value) => value.to_string(),
        FeatureValue::Int(value) => value.to_string(),
    }
}

fn structure_heading_repr(heading: &[StructureHeading]) -> String {
    heading
        .iter()
        .map(|part| format!("{}:{}", part.node_type, part.heading))
        .collect::<Vec<_>>()
        .join("-")
}
