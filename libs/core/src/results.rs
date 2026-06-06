use std::collections::BTreeMap;

use serde::Serialize;

use crate::compiled::MappedCompiledCorpus;
use crate::corpus::{
    Corpus, CorpusDescription, FeatureCatalogEntry, FeatureKind, NodeTypeOverview,
};
use crate::error::Result;
use crate::feature::FeatureValue;
use crate::mapped_sections::MappedSections;
use crate::mapped_text::MappedText;

pub const DEFAULT_MAX_TEXT_SLOTS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeInfo {
    pub node: u32,
    pub node_type: String,
    pub text: String,
    pub section_ref: String,
    pub slots: Option<Vec<u32>>,
    pub features: Option<BTreeMap<String, FeatureValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInfoOptions {
    pub include_slots: bool,
    pub include_features: Vec<String>,
    pub max_text_slots: Option<usize>,
}

impl Default for NodeInfoOptions {
    fn default() -> Self {
        Self {
            include_slots: false,
            include_features: Vec::new(),
            max_text_slots: Some(DEFAULT_MAX_TEXT_SLOTS),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeList {
    pub nodes: Vec<NodeInfo>,
    pub total_count: usize,
    pub query: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchResult {
    pub results: Vec<Vec<NodeInfo>>,
    pub total_count: usize,
    pub template: String,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeatureInfo {
    pub name: String,
    pub kind: FeatureKind,
    pub value_type: String,
    pub description: String,
    pub has_values: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CorpusInfo {
    pub name: String,
    pub path: String,
    pub node_types: Vec<CorpusNodeTypeInfo>,
    pub node_features: Vec<String>,
    pub edge_features: Vec<String>,
    pub slot_type: String,
    pub max_slot: u32,
    pub max_node: u32,
    pub section_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CorpusNodeTypeInfo {
    pub node_type: String,
    pub count: usize,
    pub average_slots: f64,
    pub min_node: u32,
    pub max_node: u32,
}

impl NodeInfo {
    pub fn to_dict(&self) -> serde_json::Value {
        let mut result = serde_json::Map::new();
        result.insert("node".to_string(), serde_json::json!(self.node));
        result.insert("otype".to_string(), serde_json::json!(self.node_type));
        result.insert("text".to_string(), serde_json::json!(self.text));
        if !self.section_ref.is_empty() {
            result.insert(
                "section_ref".to_string(),
                serde_json::json!(self.section_ref),
            );
        }
        if let Some(slots) = &self.slots {
            if !slots.is_empty() {
                result.insert("slots".to_string(), serde_json::json!(slots));
            }
        }
        if let Some(features) = &self.features {
            if !features.is_empty() {
                result.insert("features".to_string(), serde_json::json!(features));
            }
        }
        serde_json::Value::Object(result)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_dict()).expect("NodeInfo should serialize")
    }

    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        self.to_json()
    }

    pub fn from_corpus(corpus: &Corpus, node: u32, options: &NodeInfoOptions) -> Self {
        let node_type = corpus.node_type(node).unwrap_or_default().to_string();
        let node_slots = if node_type != corpus.slot_type {
            Some(corpus.slots(node))
        } else {
            None
        };
        let slots = if options.include_slots && node_type != corpus.slot_type {
            let slots = node_slots.clone().unwrap_or_default();
            (!slots.is_empty()).then_some(slots)
        } else {
            None
        };
        let features = if options.include_features.is_empty() {
            None
        } else {
            let mut values = BTreeMap::new();
            for feature_name in &options.include_features {
                if let Some(value) = corpus
                    .node_feature(feature_name)
                    .and_then(|feature| feature.v(node))
                {
                    values.insert(feature_name.clone(), value.clone());
                }
            }
            (!values.is_empty()).then_some(values)
        };
        Self {
            node,
            node_type,
            text: render_corpus_node_text(corpus, node, node_slots.as_deref(), options),
            section_ref: corpus.section_ref(node),
            slots,
            features,
        }
    }

    pub fn from_mapped(
        text: &MappedText<'_>,
        sections: &MappedSections<'_>,
        node: u32,
        options: &NodeInfoOptions,
    ) -> Result<Self> {
        let node_type = sections.node_type(node)?.unwrap_or_default().to_string();
        let is_slot = sections.is_slot(node)?;
        let node_slots = if is_slot {
            None
        } else {
            Some(sections.slots(node)?)
        };
        let slots = if options.include_slots && !is_slot {
            let slots = node_slots.clone().unwrap_or_default();
            (!slots.is_empty()).then_some(slots)
        } else {
            None
        };
        let features = if options.include_features.is_empty() {
            None
        } else {
            let mut values = BTreeMap::new();
            for feature_name in &options.include_features {
                if let Some(value) = sections.node_feature_value(feature_name, node)? {
                    values.insert(feature_name.clone(), value);
                }
            }
            (!values.is_empty()).then_some(values)
        };
        Ok(Self {
            node,
            node_type,
            text: render_mapped_node_text(text, node, node_slots.as_deref(), options)?,
            section_ref: sections.section_ref(node)?,
            slots,
            features,
        })
    }
}

fn render_corpus_node_text(
    corpus: &Corpus,
    node: u32,
    node_slots: Option<&[u32]>,
    options: &NodeInfoOptions,
) -> String {
    if let Some(slots) = node_slots {
        if let Some(max_text_slots) = options.max_text_slots {
            if slots.len() > max_text_slots {
                return format!("[{} slots - text omitted]", slots.len());
            }
        }
    }
    corpus.text(node, None)
}

fn render_mapped_node_text(
    text: &MappedText<'_>,
    node: u32,
    node_slots: Option<&[u32]>,
    options: &NodeInfoOptions,
) -> Result<String> {
    if let Some(slots) = node_slots {
        if let Some(max_text_slots) = options.max_text_slots {
            if slots.len() > max_text_slots {
                return Ok(format!("[{} slots - text omitted]", slots.len()));
            }
        }
    }
    text.text(node, None)
}

impl NodeList {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.iter().map(NodeInfo::to_dict).collect::<Vec<_>>(),
            "total_count": self.total_count,
            "query": self.query,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_dict()).expect("NodeList should serialize")
    }

    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        self.to_json()
    }

    pub fn from_nodes(
        corpus: &Corpus,
        nodes: &[u32],
        limit: Option<usize>,
        query: Option<String>,
        options: &NodeInfoOptions,
    ) -> Self {
        let total_count = nodes.len();
        let take_count = limit.unwrap_or(total_count);
        let displayed_nodes: Vec<u32> = nodes.iter().take(take_count).copied().collect();
        let text = corpus.text_nodes(&displayed_nodes, None);
        let nodes = displayed_nodes
            .iter()
            .map(|node| NodeInfo::from_corpus(corpus, *node, options))
            .collect();
        Self {
            nodes,
            total_count,
            query,
            text,
        }
    }

    pub fn from_mapped_nodes(
        text: &MappedText<'_>,
        sections: &MappedSections<'_>,
        nodes: &[u32],
        limit: Option<usize>,
        query: Option<String>,
        options: &NodeInfoOptions,
    ) -> Result<Self> {
        let total_count = nodes.len();
        let take_count = limit.unwrap_or(total_count);
        let displayed_nodes: Vec<u32> = nodes.iter().take(take_count).copied().collect();
        let rendered_text = text.text_nodes(&displayed_nodes, None)?;
        let nodes = displayed_nodes
            .iter()
            .map(|node| NodeInfo::from_mapped(text, sections, *node, options))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            nodes,
            total_count,
            query,
            text: rendered_text,
        })
    }
}

impl SearchResult {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "results": self
                .results
                .iter()
                .map(|row| row.iter().map(NodeInfo::to_dict).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            "total_count": self.total_count,
            "template": self.template,
            "plan": self.plan,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_dict()).expect("SearchResult should serialize")
    }

    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        self.to_json()
    }

    pub fn from_search(
        corpus: &Corpus,
        results: &[Vec<u32>],
        template: impl Into<String>,
        limit: Option<usize>,
        options: &NodeInfoOptions,
    ) -> Self {
        let total_count = results.len();
        let take_count = limit.unwrap_or(total_count);
        let results = results
            .iter()
            .take(take_count)
            .map(|row| {
                row.iter()
                    .map(|node| NodeInfo::from_corpus(corpus, *node, options))
                    .collect()
            })
            .collect();
        Self {
            results,
            total_count,
            template: template.into(),
            plan: None,
        }
    }

    pub fn from_mapped_search(
        text: &MappedText<'_>,
        sections: &MappedSections<'_>,
        results: &[Vec<u32>],
        template: impl Into<String>,
        limit: Option<usize>,
        options: &NodeInfoOptions,
    ) -> Result<Self> {
        let total_count = results.len();
        let take_count = limit.unwrap_or(total_count);
        let results = results
            .iter()
            .take(take_count)
            .map(|row| {
                row.iter()
                    .map(|node| NodeInfo::from_mapped(text, sections, *node, options))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            results,
            total_count,
            template: template.into(),
            plan: None,
        })
    }
}

impl FeatureInfo {
    pub fn to_dict(&self) -> serde_json::Value {
        let mut result = serde_json::Map::new();
        result.insert("name".to_string(), serde_json::json!(self.name));
        result.insert("kind".to_string(), serde_json::json!(self.kind));
        result.insert("value_type".to_string(), serde_json::json!(self.value_type));
        result.insert(
            "description".to_string(),
            serde_json::json!(self.description),
        );
        if let Some(has_values) = self.has_values {
            result.insert("has_values".to_string(), serde_json::json!(has_values));
        }
        serde_json::Value::Object(result)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_dict()).expect("FeatureInfo should serialize")
    }

    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        self.to_json()
    }

    pub fn from_corpus(corpus: &Corpus, name: &str, kind: FeatureKind) -> Option<Self> {
        match kind {
            FeatureKind::Node => {
                let feature = corpus.node_feature(name)?;
                Some(Self {
                    name: feature.name.clone(),
                    kind,
                    value_type: feature.value_type().unwrap_or_default().to_string(),
                    description: feature.description().unwrap_or_default().to_string(),
                    has_values: None,
                })
            }
            FeatureKind::Edge => {
                let feature = corpus.edge_feature(name)?;
                Some(Self {
                    name: feature.name.clone(),
                    kind,
                    value_type: feature.value_type().unwrap_or_default().to_string(),
                    description: feature.description().unwrap_or_default().to_string(),
                    has_values: Some(feature.has_edge_values()),
                })
            }
        }
    }

    pub fn from_mapped(
        corpus: &MappedCompiledCorpus,
        name: &str,
        kind: FeatureKind,
    ) -> Option<Self> {
        match kind {
            FeatureKind::Node => {
                let feature = corpus.metadata().node_feature(name)?;
                Some(Self {
                    name: feature.name.clone(),
                    kind,
                    value_type: feature.value_type().unwrap_or_default().to_string(),
                    description: feature.description().unwrap_or_default().to_string(),
                    has_values: None,
                })
            }
            FeatureKind::Edge => {
                let feature = corpus.metadata().edge_feature(name)?;
                Some(Self {
                    name: feature.name.clone(),
                    kind,
                    value_type: feature.value_type().unwrap_or_default().to_string(),
                    description: feature.description().unwrap_or_default().to_string(),
                    has_values: Some(feature.metadata.contains_key("edgeValues")),
                })
            }
        }
    }
}

impl CorpusDescription {
    pub fn from_mapped(
        corpus: &MappedCompiledCorpus,
        text: &MappedText<'_>,
        name: impl Into<String>,
    ) -> Result<Self> {
        let name = name.into();
        let info = CorpusInfo::from_mapped(corpus, name.clone(), "")?;
        let slot_type = info.slot_type.clone();
        let node_types = info
            .node_types
            .into_iter()
            .map(|node_type| NodeTypeOverview {
                is_slot_type: node_type.node_type == slot_type,
                node_type: node_type.node_type,
                count: node_type.count,
            })
            .collect();
        Ok(Self {
            name,
            node_types,
            section_types: info.section_types,
            text_representations: text.text_representations()?,
            node_features: mapped_feature_catalog(corpus, FeatureKind::Node),
            edge_features: mapped_feature_catalog(corpus, FeatureKind::Edge),
        })
    }
}

fn mapped_feature_catalog(
    corpus: &MappedCompiledCorpus,
    kind: FeatureKind,
) -> Vec<FeatureCatalogEntry> {
    let feature_names: Vec<String> = match kind {
        FeatureKind::Node => corpus
            .metadata()
            .node_features
            .iter()
            .map(|feature| feature.name.clone())
            .collect(),
        FeatureKind::Edge => corpus
            .metadata()
            .edge_features
            .iter()
            .map(|feature| feature.name.clone())
            .collect(),
    };
    feature_names
        .iter()
        .filter_map(|name| FeatureInfo::from_mapped(corpus, name, kind))
        .map(|info| FeatureCatalogEntry {
            name: info.name,
            kind: info.kind,
            value_type: info.value_type,
            description: info.description,
        })
        .collect()
}

impl CorpusInfo {
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "path": self.path,
            "node_types": self
                .node_types
                .iter()
                .map(|node_type| serde_json::json!({
                    "type": node_type.node_type,
                    "count": node_type.count,
                    "avg_slots": node_type.average_slots,
                    "min_node": node_type.min_node,
                    "max_node": node_type.max_node,
                }))
                .collect::<Vec<_>>(),
            "node_features": self.node_features,
            "edge_features": self.edge_features,
            "slot_type": self.slot_type,
            "max_slot": self.max_slot,
            "max_node": self.max_node,
            "section_types": self.section_types,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_dict()).expect("CorpusInfo should serialize")
    }

    #[allow(non_snake_case)]
    pub fn toJson(&self) -> String {
        self.to_json()
    }

    pub fn from_corpus(corpus: &Corpus, name: impl Into<String>, path: impl Into<String>) -> Self {
        let node_types = corpus
            .levels()
            .into_iter()
            .map(
                |(node_type, average_slots, min_node, max_node)| CorpusNodeTypeInfo {
                    node_type: node_type.to_string(),
                    count: max_node.saturating_sub(min_node) as usize + 1,
                    average_slots,
                    min_node,
                    max_node,
                },
            )
            .collect();
        Self {
            name: name.into(),
            path: path.into(),
            node_types,
            node_features: corpus
                .node_feature_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
            edge_features: corpus
                .edge_feature_names()
                .into_iter()
                .map(str::to_string)
                .collect(),
            slot_type: corpus.slot_type.clone(),
            max_slot: corpus.max_slot,
            max_node: corpus.max_node,
            section_types: corpus.section_types(),
        }
    }

    pub fn from_mapped(
        corpus: &MappedCompiledCorpus,
        name: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<Self> {
        let otype = corpus
            .string_pool_node_feature("otype")?
            .ok_or_else(|| crate::error::CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .edge_feature("oslots")?
            .ok_or_else(|| crate::error::CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype.str_value(1)?.unwrap_or_default().to_string();
        let max_node = corpus.metadata().rank_len as u32;
        let max_slot = otype
            .value_interval(&slot_type)?
            .map(|(_, last)| last)
            .unwrap_or_default();

        let mut rows = BTreeMap::<String, (usize, usize, u32, u32)>::new();
        for row in otype.rows() {
            let (node, node_type) = row?;
            let slot_count = if node_type == slot_type {
                1
            } else {
                match oslots.targets(node)? {
                    Some(targets) => targets.collect::<Result<Vec<_>>>()?.len(),
                    None => 0,
                }
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

        let mut node_types = rows
            .into_iter()
            .map(
                |(node_type, (count, total_slots, min_node, max_node))| CorpusNodeTypeInfo {
                    node_type,
                    count,
                    average_slots: total_slots as f64 / count.max(1) as f64,
                    min_node,
                    max_node,
                },
            )
            .collect::<Vec<_>>();
        node_types.sort_by(|left, right| {
            right
                .average_slots
                .total_cmp(&left.average_slots)
                .then_with(|| right.min_node.cmp(&left.min_node))
        });

        let sections = MappedSections::new(corpus)?;
        Ok(Self {
            name: name.into(),
            path: path.into(),
            node_types,
            node_features: corpus
                .metadata()
                .node_features
                .iter()
                .map(|feature| feature.name.clone())
                .collect(),
            edge_features: corpus
                .metadata()
                .edge_features
                .iter()
                .map(|feature| feature.name.clone())
                .collect(),
            slot_type,
            max_slot,
            max_node,
            section_types: sections.section_types().to_vec(),
        })
    }
}
