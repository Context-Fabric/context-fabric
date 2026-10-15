//! [`FeatureValue`]: the value of a node or edge feature.

use std::sync::Arc;

use serde::{Serialize, Serializer};

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
