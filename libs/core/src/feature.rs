use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Deref;
use std::sync::Arc;

use serde::{Serialize, Serializer};

use crate::error::{CfError, Result};
use crate::precompute::{LevDownData, LevUpData, OrderData, RankData};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    pub(crate) fn sort_token(&self) -> String {
        match self {
            Self::Str(value) => format!("s:{value}"),
            Self::Int(value) => format!("i:{value:020}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeFeature {
    pub name: String,
    pub metadata: BTreeMap<String, Option<String>>,
    pub values: HashMap<u32, FeatureValue>,
    pub by_value: HashMap<FeatureValue, Vec<u32>>,
    rank: Option<Arc<Vec<u32>>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeFeatures;

pub type OtypeFeature = NodeFeature;

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

    pub fn data(&self) -> &HashMap<u32, FeatureValue> {
        &self.values
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

    pub fn v(&self, node: u32) -> Option<&FeatureValue> {
        self.value(node)
    }

    pub fn str_value(&self, node: u32) -> Option<&str> {
        self.value(node).and_then(FeatureValue::as_str)
    }

    pub fn nodes_with_value(&self, value: &FeatureValue) -> &[u32] {
        self.by_value.get(value).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn s(&self, value: &FeatureValue) -> &[u32] {
        self.nodes_with_value(value)
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

    pub fn select(&self, value: &FeatureValue) -> Vec<u32> {
        self.nodes_with_value(value).to_vec()
    }

    pub fn filter_by_value(&self, nodes: &[u32], expected: &FeatureValue) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.value(*node)
                    .is_some_and(|value| feature_values_match(value, expected))
            })
            .collect()
    }

    pub fn filter_by_values(&self, nodes: &[u32], expected: &[FeatureValue]) -> Vec<u32> {
        if expected.is_empty() {
            return Vec::new();
        }
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.value(*node).is_some_and(|value| {
                    expected
                        .iter()
                        .any(|expected| feature_values_match(value, expected))
                })
            })
            .collect()
    }

    pub fn filter_has_value(&self, nodes: &[u32]) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.value(*node).is_some())
            .collect()
    }

    pub fn filter_missing_value(&self, nodes: &[u32]) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.value(*node).is_none())
            .collect()
    }

    pub fn filter_less_than(&self, nodes: &[u32], expected: &FeatureValue) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.value(*node)
                    .and_then(|value| compare_feature_values(value, expected))
                    .is_some_and(|ordering| ordering.is_lt())
            })
            .collect()
    }

    pub fn filter_greater_than(&self, nodes: &[u32], expected: &FeatureValue) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.value(*node)
                    .and_then(|value| compare_feature_values(value, expected))
                    .is_some_and(|ordering| ordering.is_gt())
            })
            .collect()
    }

    pub fn frequency_list(&self) -> Vec<(FeatureValue, usize)> {
        let mut counts: HashMap<FeatureValue, usize> = HashMap::new();
        for value in self.values.values() {
            *counts.entry(value.clone()).or_default() += 1;
        }
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_unstable_by_key(|(value, count)| (Reverse(*count), value.sort_token()));
        rows
    }

    pub fn freq_list(&self) -> Vec<(FeatureValue, usize)> {
        self.frequency_list()
    }

    #[allow(non_snake_case)]
    pub fn freqList(&self) -> Vec<(FeatureValue, usize)> {
        self.frequency_list()
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EdgeFeatures;

pub type OslotsFeature = EdgeFeature;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeFrequency {
    Count(usize),
    Values(Vec<(Option<FeatureValue>, usize)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Computed<T> {
    pub data: T,
}

impl<T> Computed<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn into_data(self) -> T {
        self.data
    }
}

impl<T> Deref for Computed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Computeds;

pub type RankComputed = Computed<RankData>;
pub type OrderComputed = Computed<OrderData>;
pub type LevUpComputed = Computed<LevUpData>;
pub type LevDownComputed = Computed<LevDownData>;

impl EdgeFeature {
    pub fn new(
        name: String,
        metadata: BTreeMap<String, Option<String>>,
        values: HashMap<u32, Vec<u32>>,
    ) -> Self {
        Self::new_with_values(name, metadata, values, HashMap::new())
    }

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

    pub fn data(&self) -> &HashMap<u32, Vec<u32>> {
        &self.values
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

    pub fn s(&self, node: u32) -> Vec<u32> {
        let targets = self.forward(node);
        if targets.is_empty() && self.name == "oslots" {
            vec![node]
        } else {
            targets
        }
    }

    pub fn f(&self, node: u32) -> Vec<u32> {
        self.forward(node)
    }

    pub fn t(&self, node: u32) -> Vec<u32> {
        self.backward(node)
    }

    pub fn forward(&self, node: u32) -> Vec<u32> {
        self.targets(node).map(<[u32]>::to_vec).unwrap_or_default()
    }

    pub fn all_targets<I>(&self, sources: I) -> BTreeSet<u32>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut targets = BTreeSet::new();
        for source in sources {
            if let Some(source_targets) = self.targets(source) {
                targets.extend(source_targets.iter().copied());
            }
        }
        targets
    }

    pub fn get_all_targets<I>(&self, sources: I) -> BTreeSet<u32>
    where
        I: IntoIterator<Item = u32>,
    {
        self.all_targets(sources)
    }

    pub fn filter_sources_with_targets_in<I, J>(
        &self,
        sources: I,
        targets: J,
    ) -> (BTreeSet<u32>, BTreeSet<u32>)
    where
        I: IntoIterator<Item = u32>,
        J: IntoIterator<Item = u32>,
    {
        let target_set: BTreeSet<u32> = targets.into_iter().collect();
        if target_set.is_empty() {
            return (BTreeSet::new(), BTreeSet::new());
        }
        let mut matching_sources = BTreeSet::new();
        let mut matching_targets = BTreeSet::new();
        for source in sources {
            if let Some(source_targets) = self.targets(source) {
                for target in source_targets {
                    if target_set.contains(target) {
                        matching_sources.insert(source);
                        matching_targets.insert(*target);
                    }
                }
            }
        }
        (matching_sources, matching_targets)
    }

    pub fn forward_with_values(&self, node: u32) -> Vec<(u32, Option<FeatureValue>)> {
        self.forward(node)
            .into_iter()
            .map(|target| (target, self.edge_value(node, target).cloned()))
            .collect()
    }

    pub fn f_with_values(&self, node: u32) -> Vec<(u32, Option<FeatureValue>)> {
        self.forward_with_values(node)
    }

    pub fn edge_value(&self, source: u32, target: u32) -> Option<&FeatureValue> {
        self.edge_values.get(&(source, target))
    }

    pub fn has_edge_values(&self) -> bool {
        self.metadata.contains_key("edgeValues")
    }

    #[allow(non_snake_case)]
    pub fn hasEdgeValues(&self) -> bool {
        self.has_edge_values()
    }

    pub fn edge_value_count(&self) -> usize {
        self.edge_values.len()
    }

    pub fn inverse_map(&self) -> HashMap<u32, Vec<u32>> {
        let mut inverse: HashMap<u32, Vec<u32>> = HashMap::new();
        for (source, targets) in &self.values {
            for target in targets {
                inverse.entry(*target).or_default().push(*source);
            }
        }
        for sources in inverse.values_mut() {
            sources.sort_unstable();
            sources.dedup();
        }
        inverse
    }

    pub fn data_inv(&self) -> HashMap<u32, Vec<u32>> {
        self.inverse_map()
    }

    #[allow(non_snake_case)]
    pub fn dataInv(&self) -> HashMap<u32, Vec<u32>> {
        self.data_inv()
    }

    pub fn data_with_values(&self) -> HashMap<u32, Vec<(u32, Option<FeatureValue>)>> {
        self.values
            .keys()
            .map(|source| (*source, self.forward_with_values(*source)))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn dataWithValues(&self) -> HashMap<u32, Vec<(u32, Option<FeatureValue>)>> {
        self.data_with_values()
    }

    pub fn data_inv_with_values(&self) -> HashMap<u32, Vec<(u32, Option<FeatureValue>)>> {
        let mut inverse: HashMap<u32, Vec<(u32, Option<FeatureValue>)>> = HashMap::new();
        for (source, targets) in &self.values {
            for target in targets {
                inverse
                    .entry(*target)
                    .or_default()
                    .push((*source, self.edge_value(*source, *target).cloned()));
            }
        }
        for sources in inverse.values_mut() {
            sources.sort_unstable_by_key(|(source, _)| *source);
            sources.dedup();
        }
        inverse
    }

    #[allow(non_snake_case)]
    pub fn dataInvWithValues(&self) -> HashMap<u32, Vec<(u32, Option<FeatureValue>)>> {
        self.data_inv_with_values()
    }

    pub fn backward(&self, node: u32) -> Vec<u32> {
        let mut sources = Vec::new();
        for (source, targets) in &self.values {
            if targets.contains(&node) {
                sources.push(*source);
            }
        }
        self.sort_nodes(&mut sources);
        sources.dedup();
        sources
    }

    pub fn backward_with_values(&self, node: u32) -> Vec<(u32, Option<FeatureValue>)> {
        self.backward(node)
            .into_iter()
            .map(|source| (source, self.edge_value(source, node).cloned()))
            .collect()
    }

    pub fn t_with_values(&self, node: u32) -> Vec<(u32, Option<FeatureValue>)> {
        self.backward_with_values(node)
    }

    pub fn both(&self, node: u32) -> Vec<u32> {
        let mut nodes = self.forward(node);
        nodes.extend(self.backward(node));
        nodes.sort_unstable();
        nodes.dedup();
        nodes
    }

    pub fn b(&self, node: u32) -> Vec<u32> {
        self.both(node)
    }

    pub fn both_with_values(&self, node: u32) -> Vec<(u32, Option<FeatureValue>)> {
        let mut by_node: HashMap<u32, Option<FeatureValue>> = HashMap::new();
        for (source, value) in self.backward_with_values(node) {
            by_node.insert(source, value);
        }
        for (target, value) in self.forward_with_values(node) {
            by_node.insert(target, value);
        }
        let mut rows: Vec<_> = by_node.into_iter().collect();
        rows.sort_unstable_by_key(|(node, _)| rank_key(*node, self.rank.as_deref()));
        rows
    }

    pub fn b_with_values(&self, node: u32) -> Vec<(u32, Option<FeatureValue>)> {
        self.both_with_values(node)
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

    pub fn items_with_values(&self) -> Vec<(u32, Vec<(u32, Option<FeatureValue>)>)> {
        let mut rows: Vec<_> = self
            .values
            .keys()
            .map(|source| (*source, self.forward_with_values(*source)))
            .collect();
        rows.sort_unstable_by_key(|(source, _)| rank_key(*source, self.rank.as_deref()));
        rows
    }

    #[allow(non_snake_case)]
    pub fn itemsWithValues(&self) -> Vec<(u32, Vec<(u32, Option<FeatureValue>)>)> {
        self.items_with_values()
    }

    pub fn frequency_list(&self) -> EdgeFrequency {
        if !self.has_edge_values() {
            return EdgeFrequency::Count(self.edge_count());
        }

        let mut counts: HashMap<Option<FeatureValue>, usize> = HashMap::new();
        for (source, targets) in &self.values {
            for target in targets {
                let value = self.edge_value(*source, *target).cloned();
                *counts.entry(value).or_default() += 1;
            }
        }
        let mut rows: Vec<_> = counts.into_iter().collect();
        rows.sort_unstable_by_key(|(value, count)| {
            (Reverse(*count), optional_feature_value_sort_token(value))
        });
        EdgeFrequency::Values(rows)
    }

    pub fn freq_list(&self) -> EdgeFrequency {
        self.frequency_list()
    }

    #[allow(non_snake_case)]
    pub fn freqList(&self) -> EdgeFrequency {
        self.frequency_list()
    }

    pub fn edge_count(&self) -> usize {
        self.values.values().map(Vec::len).sum()
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

fn optional_feature_value_sort_token(value: &Option<FeatureValue>) -> String {
    match value {
        Some(value) => value.sort_token(),
        None => "n:".to_string(),
    }
}

fn feature_values_match(actual: &FeatureValue, expected: &FeatureValue) -> bool {
    if actual == expected {
        return true;
    }
    match (actual, expected) {
        (FeatureValue::Int(actual), FeatureValue::Str(expected)) => expected
            .parse::<i64>()
            .is_ok_and(|expected| *actual == expected),
        _ => false,
    }
}

fn compare_feature_values(
    actual: &FeatureValue,
    expected: &FeatureValue,
) -> Option<std::cmp::Ordering> {
    match (actual, expected) {
        (FeatureValue::Int(actual), FeatureValue::Int(expected)) => Some(actual.cmp(expected)),
        (FeatureValue::Int(actual), FeatureValue::Str(expected)) => expected
            .parse::<i64>()
            .ok()
            .map(|expected| actual.cmp(&expected)),
        (FeatureValue::Str(actual), FeatureValue::Str(expected)) => Some(actual.cmp(expected)),
        (FeatureValue::Str(actual), FeatureValue::Int(expected)) => actual
            .parse::<i64>()
            .ok()
            .map(|actual| actual.cmp(expected)),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TfFeature {
    Node(NodeFeature),
    Edge(EdgeFeature),
    Config {
        name: String,
        metadata: BTreeMap<String, Option<String>>,
    },
}

impl TfFeature {
    pub fn name(&self) -> &str {
        match self {
            Self::Node(feature) => &feature.name,
            Self::Edge(feature) => &feature.name,
            Self::Config { name, .. } => name,
        }
    }
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

#[allow(non_snake_case)]
pub fn valueFromTf(raw: &str) -> String {
    value_from_tf(raw)
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

#[allow(non_snake_case)]
pub fn tfFromValue(value: &FeatureValue) -> String {
    tf_from_value(value)
}
