//! The `cfabric._core` Python extension module.

mod accessors;
mod api_ref;
mod errors;
mod fabric;
mod feature_file;
mod features;
mod node_arg;
mod timestamp;
mod visibility;

use pyo3::prelude::*;

use crate::config::{BANNER, NAME, VERSION};
use accessors::{PyLocality, PyNodes, PySearch, PySearchExe, PyText};
use fabric::{PyCorpus, PyFabric};
use feature_file::PyFeatureFile;
use features::{
    PyChunkLengthKey, PyChunkPositionKey, PyComputed, PyComputeds, PyEdgeFeature, PyEdgeFeatures,
    PyLevView, PyNodeFeature, PyNodeFeatures,
};
use timestamp::PyTimestamp;

#[pymodule]
pub fn _core(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", VERSION)?;
    module.add("VERSION", VERSION)?;
    module.add("NAME", NAME)?;
    module.add("BANNER", BANNER)?;
    module.add_class::<PyFabric>()?;
    module.add_class::<PyCorpus>()?;
    module.add_class::<PyFeatureFile>()?;
    module.add_class::<PyTimestamp>()?;
    module.add_class::<PyNodeFeature>()?;
    module.add_class::<PyNodeFeatures>()?;
    module.add_class::<PyEdgeFeature>()?;
    module.add_class::<PyEdgeFeatures>()?;
    module.add_class::<PyComputed>()?;
    module.add_class::<PyComputeds>()?;
    module.add_class::<PyLevView>()?;
    module.add_class::<PyChunkPositionKey>()?;
    module.add_class::<PyChunkLengthKey>()?;
    module.add_class::<PyLocality>()?;
    module.add_class::<PyNodes>()?;
    module.add_class::<PyText>()?;
    module.add_class::<PySearch>()?;
    module.add_class::<PySearchExe>()?;
    Ok(())
}
