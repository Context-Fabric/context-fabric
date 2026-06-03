use std::collections::{BTreeMap, BTreeSet};

pub use crate::feature::FeatureValue;

pub type Node = u32;
pub type NodeFeatureData = BTreeMap<Node, FeatureValue>;
pub type EdgeFeatureData = BTreeMap<Node, BTreeSet<Node>>;
pub type EdgeFeatureValueData = BTreeMap<Node, BTreeMap<Node, FeatureValue>>;
pub type MetaData = BTreeMap<String, Option<String>>;
pub type FeatureMetaData = BTreeMap<String, MetaData>;
pub type NodeArray = Vec<Node>;
pub type IndexArray = Vec<u32>;
pub type OffsetArray = Vec<u64>;
pub type SlotRange = (Node, Node);
pub type SearchResult = Vec<Node>;
pub type SectionSpec = Vec<String>;
pub type NodesByType = BTreeMap<String, Vec<Node>>;
