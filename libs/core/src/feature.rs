use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use serde::{Serialize, Serializer};

use crate::error::{CfError, Result};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureValue {
    Str(Arc<str>),
    Int(i64),
}

impl Serialize for FeatureValue {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Str(value) => serializer.serialize_str(value),
            Self::Int(value) => serializer.serialize_i64(*value),
        }
    }
}

impl FeatureValue {
    pub fn string(value: impl AsRef<str>) -> Self {
        Self::Str(Arc::from(value.as_ref()))
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(value) => Some(value),
            Self::Int(_) => None,
        }
    }
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
pub struct NodeFeature {
    pub name: String,
    pub metadata: BTreeMap<String, Option<String>>,
    pub values: HashMap<u32, FeatureValue>,
    pub by_value: HashMap<FeatureValue, Vec<u32>>,
    rank: Option<Arc<Vec<u32>>>,
}

impl NodeFeature {
    pub fn new(
        name: String,
        metadata: BTreeMap<String, Option<String>>,
        values: HashMap<u32, FeatureValue>,
    ) -> Self {
        Self::new_with_indexing(name, metadata, values, true)
    }

    pub fn new_with_indexing(
        name: String,
        metadata: BTreeMap<String, Option<String>>,
        values: HashMap<u32, FeatureValue>,
        build_index: bool,
    ) -> Self {
        let mut by_value: HashMap<FeatureValue, Vec<u32>> = HashMap::new();
        if build_index {
            for (node, value) in &values {
                by_value.entry(value.clone()).or_default().push(*node);
            }
            for nodes in by_value.values_mut() {
                nodes.sort_unstable();
            }
        }
        Self {
            name,
            metadata,
            values,
            by_value,
            rank: None,
        }
    }

    pub fn with_rank(mut self, rank: Arc<Vec<u32>>) -> Self {
        self.rank = Some(rank);
        self.reindex_by_value();
        self
    }

    fn reindex_by_value(&mut self) {
        self.by_value.clear();
        for (node, value) in &self.values {
            self.by_value.entry(value.clone()).or_default().push(*node);
        }
        let rank = self.rank.clone();
        for nodes in self.by_value.values_mut() {
            sort_nodes_by_rank(nodes, rank.as_deref());
        }
    }

    pub fn value(&self, node: u32) -> Option<&FeatureValue> {
        self.values.get(&node)
    }

    pub fn v(&self, node: u32) -> Option<&FeatureValue> {
        self.value(node)
    }

    pub fn str_value(&self, node: u32) -> Option<&str> {
        self.value(node).and_then(FeatureValue::as_str)
    }

    pub fn items(&self) -> Vec<(u32, FeatureValue)> {
        let mut rows: Vec<_> = self
            .values
            .iter()
            .map(|(node, value)| (*node, value.clone()))
            .collect();
        let rank = self.rank.as_deref();
        rows.sort_unstable_by_key(|(node, _)| rank_key(*node, rank));
        rows
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeFeature {
    pub name: String,
    pub metadata: BTreeMap<String, Option<String>>,
    pub values: HashMap<u32, Vec<u32>>,
    pub edge_values: HashMap<(u32, u32), FeatureValue>,
    rank: Option<Arc<Vec<u32>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeFrequency {
    Count(usize),
    Values(Vec<(Option<FeatureValue>, usize)>),
}

impl EdgeFeature {
    pub fn new_with_values(
        name: String,
        metadata: BTreeMap<String, Option<String>>,
        mut values: HashMap<u32, Vec<u32>>,
        edge_values: HashMap<(u32, u32), FeatureValue>,
    ) -> Self {
        for targets in values.values_mut() {
            targets.sort_unstable();
            targets.dedup();
        }
        Self {
            name,
            metadata,
            values,
            edge_values,
            rank: None,
        }
    }

    pub fn with_rank(mut self, rank: Arc<Vec<u32>>) -> Self {
        self.rank = Some(rank.clone());
        for targets in self.values.values_mut() {
            sort_nodes_by_rank(targets, Some(&rank));
            targets.dedup();
        }
        self
    }

    pub fn targets(&self, node: u32) -> Option<&[u32]> {
        self.values.get(&node).map(Vec::as_slice)
    }

    pub fn edge_value(&self, source: u32, target: u32) -> Option<&FeatureValue> {
        self.edge_values.get(&(source, target))
    }

    pub fn has_edge_values(&self) -> bool {
        self.metadata.contains_key("edgeValues")
    }

    pub fn items(&self) -> Vec<(u32, Vec<u32>)> {
        let mut rows: Vec<_> = self
            .values
            .iter()
            .map(|(source, targets)| {
                let mut targets = targets.clone();
                self.sort_nodes(&mut targets);
                targets.dedup();
                (*source, targets)
            })
            .collect();
        rows.sort_unstable_by_key(|(source, _)| rank_key(*source, self.rank.as_deref()));
        rows
    }

    fn sort_nodes(&self, nodes: &mut [u32]) {
        sort_nodes_by_rank(nodes, self.rank.as_deref());
    }
}

fn sort_nodes_by_rank(nodes: &mut [u32], rank: Option<&Vec<u32>>) {
    nodes.sort_unstable_by_key(|node| rank_key(*node, rank));
}

fn rank_key(node: u32, rank: Option<&Vec<u32>>) -> (u32, u32) {
    let canonical = rank
        .and_then(|rank| rank.get(node.saturating_sub(1) as usize).copied())
        .unwrap_or(node);
    (canonical, node)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TfFeature {
    Node(NodeFeature),
    Edge(EdgeFeature),
    Config {
        name: String,
        metadata: BTreeMap<String, Option<String>>,
    },
}

pub(crate) fn parse_value(
    raw: &str,
    value_type: &str,
    path: &std::path::Path,
    line: usize,
) -> Result<Option<FeatureValue>> {
    if value_type == "int" {
        if raw.is_empty() {
            return Ok(None);
        }
        let parsed = raw.parse::<i64>().map_err(|_| CfError::Parse {
            path: path.to_path_buf(),
            line,
            message: format!("expected integer value, got {raw:?}"),
        })?;
        Ok(Some(FeatureValue::Int(parsed)))
    } else {
        Ok(Some(FeatureValue::string(value_from_tf(raw))))
    }
}

pub fn value_from_tf(raw: &str) -> String {
    raw.split("\\\\")
        .map(|part| part.replace("\\t", "\t").replace("\\n", "\n"))
        .collect::<Vec<_>>()
        .join("\\")
}

pub fn tf_from_value(value: &FeatureValue) -> String {
    match value {
        FeatureValue::Str(value) => value
            .split('\\')
            .map(|part| part.replace('\t', "\\t").replace('\n', "\\n"))
            .collect::<Vec<_>>()
            .join("\\\\"),
        FeatureValue::Int(value) => value.to_string(),
    }
}
