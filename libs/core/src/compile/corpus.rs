//! The in-memory corpus loaded from `.tf` sources: the input of compilation.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::compile::is_superset;
use crate::error::{CfError, Result};
use crate::tf::{EdgeFeature, NodeFeature, Otext, TfFeature, parse_tf_file};
use crate::value::FeatureValue;

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
        let level_constraints = config_features
            .get("otext")
            .and_then(|otext| Otext::from_metadata(otext).level_constraints);
        let type_ranks = Self::compute_type_ranks(
            &slot_type,
            &nodes_by_type,
            &slot_counts,
            level_constraints.as_deref(),
        );

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
        level_constraints: Option<&str>,
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

        // Honor `@levelConstraints` (TF `prepare.py`): adjust the size-based order
        // before deriving ranks, so co-extensive nodes break ties identically.
        if let Some(spec) = level_constraints {
            crate::index::apply_level_constraints(&mut levels, spec, |level| level.0.as_str());
        }

        let mut type_ranks = HashMap::new();
        for (rank, (node_type, _)) in levels.into_iter().rev().enumerate() {
            type_ranks.insert(node_type, rank as u32);
        }
        type_ranks
    }

    pub fn node_feature(&self, name: &str) -> Option<&NodeFeature> {
        self.node_features.get(name)
    }

    pub fn edge_feature(&self, name: &str) -> Option<&EdgeFeature> {
        self.edge_features.get(name)
    }

    pub fn config_feature(&self, name: &str) -> Option<&BTreeMap<String, Option<String>>> {
        self.config_features.get(name)
    }

    /// The `otext` config feature (empty when absent).
    pub fn otext(&self) -> Otext {
        self.config_feature("otext")
            .map(Otext::from_metadata)
            .unwrap_or_default()
    }

    pub fn nodes_of_type(&self, node_type: &str) -> &[u32] {
        self.nodes_by_type
            .get(node_type)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn max_slot(&self) -> u32 {
        self.max_slot
    }

    pub fn max_node(&self) -> u32 {
        self.max_node
    }

    pub fn slots_of(&self, node: u32) -> Option<&[u32]> {
        self.edge_feature("oslots")
            .and_then(|feature| feature.targets(node))
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

    pub fn up(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        match node_type {
            Some(node_type) => self.up_types(node, Some(&[node_type])),
            None => self.up_types(node, None),
        }
    }

    pub fn u(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.up(node, node_type)
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

fn min_difference(left: &[u32], right: &[u32]) -> Option<u32> {
    left.iter()
        .copied()
        .find(|value| right.binary_search(value).is_err())
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
