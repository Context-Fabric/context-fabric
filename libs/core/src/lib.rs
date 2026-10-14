//! Context-Fabric core: a memory-mapped corpus engine for Text-Fabric data.
//!
//! Layout, in the order data flows through it:
//!
//! - [`tf`]: Text-Fabric `.tf` source files (parse, write, explore).
//! - [`compile`]: loads `.tf` sources into an in-memory corpus, precomputes
//!   the indexes, and writes a `.cfr` file.
//! - [`index`]: the precomputed indexes (levels, `levUp`/`levDown`, boundary,
//!   reverse edges, row offsets, sections, structure).
//! - [`format`]: the `.cfr` file layout, reader and writer.
//! - [`cache`]: where compiled caches live and when they are fresh.
//! - [`mapped`]: the query engine over a memory-mapped `.cfr` file.
//! - [`search`]: search templates over the mapped engine.
//! - [`fabric`]: the `Fabric` facade tying locations, caches and loading
//!   together.
//! - `bindings` (feature `python`): the `cfabric._core` Python extension.

/// mimalloc returns freed pages to the OS (short purge delay), so RSS tracks
/// the live set instead of the per-query high-water mark — large search
/// transients (candidate sets, drivers, result rows) no longer pin hundreds of
/// MB after they are dropped, which the macOS system allocator does.
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod atomic_file;
#[cfg(feature = "python")]
pub mod bindings;
pub mod cache;
pub mod compile;
pub mod config;
pub mod error;
pub mod fabric;
pub mod format;
pub mod index;
pub mod mapped;
pub mod search;
pub mod tf;
pub mod value;

pub use compile::{Corpus, compile_features};
pub use config::{BANNER, CFR_VERSION, DEFAULT_FORMAT, NAME, OSLOTS, OTEXT, OTYPE, VERSION};
pub use error::{CfError, Result};
pub use fabric::{Fabric, FeatureSpec};
pub use format::{
    CompiledConfigFeature, CompiledEdgeFeature, CompiledMetadata, CompiledNodeFeature,
    NodeFeatureEncoding, inspect_compiled,
};
pub use index::Boundary;
pub use index::{
    SectionsData, StructureData, StructureHeading, StructureTree, apply_level_constraints,
};
pub use mapped::{
    Chunk, ChunkLengthKey, ChunkPositionKey, ComputedFeatureData, ConfigFeatureView,
    EdgeFeatureView, EdgeFrequency, MappedCompiledCorpus, MappedNodeFeatureView, MappedNodeValue,
    MappedSections, MixedNodeFeatureView, SectionOptions, SectionsContext,
    StringPoolNodeFeatureView, TextOptions, TextRenderer, WalkEvent,
};
pub use search::{MappedSearch, SearchSets, SearchStudy};
pub use tf::{
    EdgeFeature, FeatureInventory, NodeFeature, Otext, TfFeature, TfFeatureKind, TfFeatureMetadata,
    explore_feature_paths, parse_tf_file, parse_tf_file_metadata, tf_from_value, value_from_tf,
};
pub use value::FeatureValue;
