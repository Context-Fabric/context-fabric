//! The sections index (`T.nodeFromSection` lookup tables).

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionsData {
    pub sec1: BTreeMap<u32, BTreeMap<String, u32>>,
    pub sec2: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>>,
    pub seq_from_node: BTreeMap<u32, Vec<u32>>,
    pub node_from_seq: BTreeMap<Vec<u32>, u32>,
}
