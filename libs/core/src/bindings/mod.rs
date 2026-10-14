//! The `cfabric._core` Python extension module: one submodule per Python
//! class family, plus the conversions and helpers they share.

mod api;
mod api_ref;
mod computed;
mod convert;
mod edge_features;
mod errors;
mod fabric;
mod feature_file;
mod locality;
mod node_features;
mod nodes;
mod search;
mod text;
mod timestamp;
mod visibility;

use crate::cache::user_cache_dir;
use crate::config::{BANNER, NAME, VERSION};

/// The per-user cache directory: `CFABRIC_CACHE` when set, else the platform
/// cache directory for `cfabric`. Compiled caches of read-only corpora are
/// stored here.
#[pyfunction]
fn get_cache_dir() -> std::path::PathBuf {
    user_cache_dir()
}
use pyo3::prelude::*;

#[pymodule]
pub fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", VERSION)?;
    module.add("VERSION", VERSION)?;
    module.add("NAME", NAME)?;
    module.add("BANNER", BANNER)?;
    module.add_function(wrap_pyfunction!(get_cache_dir, module)?)?;
    module.add_class::<fabric::PyFabric>()?;
    module.add_class::<api::PyCorpus>()?;
    module.add_class::<feature_file::PyFeatureFile>()?;
    module.add_class::<timestamp::PyTimestamp>()?;
    module.add_class::<node_features::PyNodeFeature>()?;
    module.add_class::<node_features::PyNodeFeatures>()?;
    module.add_class::<edge_features::PyEdgeFeature>()?;
    module.add_class::<edge_features::PyEdgeFeatures>()?;
    module.add_class::<computed::PyComputed>()?;
    module.add_class::<computed::PyComputeds>()?;
    module.add_class::<computed::PyLevView>()?;
    module.add_class::<nodes::PyChunkPositionKey>()?;
    module.add_class::<nodes::PyChunkLengthKey>()?;
    module.add_class::<locality::PyLocality>()?;
    module.add_class::<nodes::PyNodes>()?;
    module.add_class::<text::PyText>()?;
    module.add_class::<search::PySearch>()?;
    module.add_class::<search::PySearchExe>()?;
    Ok(())
}
