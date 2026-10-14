//! The table of contents of a `.cfr` file: where each feature and section
//! lives, read once when the file is opened.

use std::collections::BTreeMap;

use crate::index::edge_index::ReverseEdgeLayout;
use crate::index::row_index::RowIndexLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledMetadata {
    pub byte_len: usize,
    pub node_features: Vec<CompiledNodeFeature>,
    pub edge_features: Vec<CompiledEdgeFeature>,
    pub config_features: Vec<CompiledConfigFeature>,
    pub order_len: usize,
    pub rank_len: usize,
    pub order_start: usize,
    pub rank_start: usize,
    /// The structure section, `None` when no usable structure is configured.
    pub structure_start: Option<usize>,
    /// CSR payload starts (just after each section magic).
    pub lev_up_start: usize,
    pub lev_down_start: usize,
    pub boundary_first_start: usize,
    pub boundary_last_start: usize,
    /// The `CFRSECT1` payload, `None` when no sections are configured.
    pub sections_start: Option<usize>,
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
    /// The `CFRROWS1` row offsets: always present for mixed features (checked
    /// on open), `None` for string-pool features, whose rows are fixed-size.
    pub row_index: Option<RowIndexLayout>,
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
    /// The `CFRREVE1` reverse index.
    pub reverse_index: ReverseEdgeLayout,
    /// The `CFRROWS1` row offsets.
    pub row_index: RowIndexLayout,
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

pub(crate) fn metadata_value<'a>(
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
