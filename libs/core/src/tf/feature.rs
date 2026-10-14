//! Node and edge features as parsed from `.tf` files.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::value::FeatureValue;

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
