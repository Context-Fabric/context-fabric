//! Context-Fabric core: a memory-mapped corpus engine for Text-Fabric data.

/// mimalloc returns freed pages to the OS (short purge delay), so RSS tracks
/// the live set instead of the per-query high-water mark — large search
/// transients (candidate sets, drivers, result rows) no longer pin hundreds of
/// MB after they are dropped, which the macOS system allocator does.
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod atomic_file;
#[cfg(feature = "python")]
pub mod bindings;
mod cache;
mod compiled;
mod config;
mod corpus;
mod downloader;
mod edge_index;
mod error;
mod explore;
mod fabric;
mod feature;
mod io;
mod mapped_search;
mod mapped_sections;
mod mapped_text;
mod parser;
mod precompute;
mod row_index;
mod search;
mod utils;

pub use compiled::{
    CompiledConfigFeature, CompiledEdgeFeature, CompiledMetadata, CompiledNodeFeature,
    ConfigFeatureView, EdgeFeatureView, MappedCompiledCorpus, MappedNodeFeatureView,
    MappedNodeValue, MappedU32s, MixedNodeFeatureView, NodeFeatureEncoding,
    StringPoolNodeFeatureView, compile_features, inspect_compiled,
};
pub use config::{BANNER, CFR_VERSION, DEFAULT_FORMAT, NAME, OSLOTS, OTEXT, OTYPE, VERSION};
pub use corpus::{
    Boundary, Chunk, ChunkLengthKey, ChunkPositionKey, ComputedFeatureData, Corpus, SectionOptions,
    StructureTree, TextOptions, WalkEvent,
};
pub use error::{CfError, Result};
pub use explore::{FeatureInventory, explore_feature_paths};
pub use fabric::{Fabric, FeatureSpec};
pub use feature::{
    EdgeFeature, EdgeFrequency, FeatureValue, NodeFeature, TfFeature, tf_from_value, value_from_tf,
};
pub use mapped_search::MappedSearch;
pub use mapped_sections::{MappedSections, SectionsContext};
pub use mapped_text::{MappedText, TextContext};
pub use parser::{TfFeatureKind, TfFeatureMetadata, parse_tf_file, parse_tf_file_metadata};
pub use precompute::{SectionsData, StructureData, StructureHeading, apply_level_constraints};
pub use search::{SearchSets, SearchStudy};
