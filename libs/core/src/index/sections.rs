//! The sections index (`T.nodeFromSection` lookup tables).

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::value::FeatureValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionsData {
    pub sec1: BTreeMap<u32, BTreeMap<String, u32>>,
    pub sec2: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>>,
    pub seq_from_node: BTreeMap<u32, Vec<u32>>,
    pub node_from_seq: BTreeMap<Vec<u32>, u32>,
}

/// The key a section heading value has in the `sec1`/`sec2` tables: its text.
pub(crate) fn section_key(value: &FeatureValue) -> Cow<'_, str> {
    match value {
        FeatureValue::Str(text) => Cow::Borrowed(text),
        FeatureValue::Int(number) => Cow::Owned(number.to_string()),
    }
}
