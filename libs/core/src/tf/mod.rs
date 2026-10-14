//! Text-Fabric `.tf` source files: parsing, writing, exploration, and value
//! escaping.

pub(crate) mod explore;
pub(crate) mod feature;
pub(crate) mod parser;
mod writer;

pub use explore::{FeatureInventory, explore_feature_paths};
pub use feature::{EdgeFeature, NodeFeature};
pub use parser::{TfFeatureKind, TfFeatureMetadata, parse_tf_file, parse_tf_file_metadata};

use std::collections::BTreeMap;

use crate::error::{CfError, Result};
use crate::value::FeatureValue;

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
