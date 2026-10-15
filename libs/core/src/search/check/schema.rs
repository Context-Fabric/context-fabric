//! What template validation needs to know about a corpus: its node types,
//! its features with their value types, and the custom sets of a search.

use std::collections::{HashMap, HashSet};

use crate::config::{OSLOTS, OTYPE};
use crate::error::{CfError, Result};
use crate::mapped::MappedCompiledCorpus;
use crate::search::SearchSets;

/// A feature's value type (TF `dataType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueType {
    Int,
    Str,
}

impl ValueType {
    fn from_metadata(value_type: Option<&str>) -> Self {
        if value_type == Some("int") {
            Self::Int
        } else {
            Self::Str
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Str => "str",
        }
    }
}

/// An edge feature as validation sees it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EdgeSchema {
    pub(crate) value_type: ValueType,
    /// Whether edges carry values (TF `edgeValues`).
    pub(crate) has_values: bool,
    /// Whether the feature can be a search relation: not `oslots` and not an
    /// `omap` feature (TF `tf/search/relations.py::basicRelations`).
    pub(crate) searchable: bool,
}

/// The node types of a corpus.
pub(crate) struct NodeTypes<'s> {
    known: HashSet<String>,
    /// The node types in level order (TF `C.levels`), for messages only:
    /// computing the levels scans the corpus.
    level_order: Box<dyn Fn() -> Vec<String> + 's>,
}

impl<'s> NodeTypes<'s> {
    /// Node types listed in level order.
    #[cfg(test)]
    pub(crate) fn listed(types: Vec<String>) -> Self {
        Self {
            known: types.iter().cloned().collect(),
            level_order: Box::new(move || types.clone()),
        }
    }

    pub(crate) fn contains(&self, node_type: &str) -> bool {
        self.known.contains(node_type)
    }

    pub(crate) fn in_level_order(&self) -> Vec<String> {
        (self.level_order)()
    }
}

/// The corpus facts a template is validated against (TF's `C.levels`,
/// `TF.featureSets` and `TF.features`), and the search's custom sets.
pub(crate) struct TemplateSchema<'s> {
    pub(crate) node_types: NodeTypes<'s>,
    pub(crate) node_features: HashMap<&'s str, ValueType>,
    pub(crate) edge_features: HashMap<&'s str, EdgeSchema>,
    /// Custom set names, when the search has `sets`.
    pub(crate) set_names: Option<Vec<String>>,
}

impl<'s> TemplateSchema<'s> {
    pub(crate) fn new(
        corpus: &'s MappedCompiledCorpus,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Self> {
        let metadata = corpus.metadata();
        let node_features = metadata
            .node_features
            .iter()
            .map(|feature| {
                (
                    feature.name.as_str(),
                    ValueType::from_metadata(feature.value_type()),
                )
            })
            .collect();
        let edge_features = metadata
            .edge_features
            .iter()
            .map(|feature| {
                let schema = EdgeSchema {
                    value_type: ValueType::from_metadata(feature.value_type()),
                    has_values: feature.metadata.contains_key("edgeValues"),
                    searchable: feature.name != OSLOTS && !feature.name.starts_with("omap"),
                };
                (feature.name.as_str(), schema)
            })
            .collect();
        let set_names = sets.map(|sets| {
            let mut names: Vec<String> = sets.keys().map(|name| (*name).to_string()).collect();
            names.sort();
            names
        });
        let known: HashSet<String> = corpus
            .string_pool_node_feature(OTYPE)?
            .ok_or_else(|| CfError::MissingFeature(OTYPE.to_string()))?
            .distinct_values()?
            .into_iter()
            .map(str::to_string)
            .collect();
        let fallback: Vec<String> = known.iter().cloned().collect();
        let level_order = Box::new(move || match corpus.mapped_levels() {
            Ok(levels) => levels.into_iter().map(|level| level.node_type).collect(),
            Err(_) => fallback.clone(),
        });
        Ok(Self {
            node_types: NodeTypes { known, level_order },
            node_features,
            edge_features,
            set_names,
        })
    }

    pub(crate) fn is_set(&self, name: &str) -> bool {
        self.set_names
            .as_ref()
            .is_some_and(|names| names.iter().any(|set| set == name))
    }
}
