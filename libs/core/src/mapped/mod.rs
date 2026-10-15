//! The query engine over a memory-mapped `.cfr` file: features, canonical
//! order, locality and sections, structure, and text.

mod config_features;
mod corpus;
mod edge_features;
mod frequency;
mod indexes;
mod node_features;
mod order;
pub(crate) mod sections;
mod structure;
pub(crate) mod text;

pub use config_features::{ConfigFeatureView, ConfigMetadataItems};
pub use corpus::{ComputedFeatureData, MappedCompiledCorpus};
pub use edge_features::{EdgeFeatureView, EdgeTargets, OwnedEdgeFeature};
pub use frequency::EdgeFrequency;
pub use node_features::{
    MappedNodeFeatureView, MappedNodeValue, MixedNodeFeatureView, MixedNodeRows, OwnedMixedFeature,
    OwnedNodeFeature, OwnedStringPoolFeature, StringPoolNodeFeatureView, StringPoolNodeRows,
};
pub use order::{Chunk, ChunkLengthKey, ChunkPositionKey};
pub use sections::{MappedSections, SectionOptions, SectionsContext, WalkEvent};
pub use text::{FormatTemplate, TemplatePart, TextOptions, TextRenderer};
