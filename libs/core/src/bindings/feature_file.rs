//! Entries of `TF.features`: one record per feature known to the `Fabric`.
//!
//! Text-Fabric's `TF.features` maps every feature name to a `tf.core.data.Data`
//! object. Context-Fabric keeps feature data memory-mapped instead of loading it
//! per feature, so the entries here are lightweight records carrying the same
//! descriptive attributes (`path`, `dirName`, `metaData`, `isEdge`, ...) without
//! a `data` payload; feature data is read through the API (`F`, `E`, `C`).

use std::collections::BTreeMap;
use std::path::Path;

use pyo3::prelude::*;

/// Description of one feature file (or precomputed feature) of a dataset.
#[pyclass(name = "FeatureFile", module = "cfabric._core", frozen)]
pub(crate) struct PyFeatureFile {
    /// Directory holding the feature file.
    #[pyo3(get, name = "dirName")]
    dir_name: String,
    /// Feature name (the file stem).
    #[pyo3(get, name = "fileName")]
    file_name: String,
    /// File extension, including the dot (`.tf`).
    #[pyo3(get)]
    extension: String,
    /// Full path of the feature file.
    #[pyo3(get)]
    path: String,
    /// Metadata from the feature file header.
    #[pyo3(get, name = "metaData")]
    meta_data: BTreeMap<String, Option<String>>,
    /// Whether this is an edge feature.
    #[pyo3(get, name = "isEdge")]
    is_edge: bool,
    /// Whether this is a configuration (metadata-only) feature.
    #[pyo3(get, name = "isConfig")]
    is_config: bool,
    /// Whether this is an edge feature whose edges carry values.
    #[pyo3(get, name = "edgeValues")]
    edge_values: bool,
    /// Value type of the feature (`str`, `int`, or empty for configs).
    #[pyo3(get, name = "dataType")]
    data_type: String,
    /// Whether the feature is exposed by the Fabric's current API.
    #[pyo3(get, name = "dataLoaded")]
    data_loaded: bool,
    /// Name of the precomputed block (`levels`, `order`, ...) for computed
    /// features, `None` for features backed by a `.tf` file.
    #[pyo3(get)]
    method: Option<String>,
}

/// What kind of feature a [`PyFeatureFile`] describes.
pub(crate) enum FeatureKind {
    Node,
    Edge,
    Config,
    /// A precomputed block, named without the surrounding underscores.
    Computed(String),
}

impl PyFeatureFile {
    pub(crate) fn new(
        path: &Path,
        kind: FeatureKind,
        meta_data: BTreeMap<String, Option<String>>,
        data_loaded: bool,
    ) -> Self {
        let edge_values = matches!(kind, FeatureKind::Edge) && meta_data.contains_key("edgeValues");
        let data_type = meta_data
            .get("valueType")
            .cloned()
            .flatten()
            .unwrap_or_else(|| {
                if matches!(kind, FeatureKind::Node | FeatureKind::Edge) {
                    "str".to_string()
                } else {
                    String::new()
                }
            });
        Self {
            dir_name: path
                .parent()
                .map(|dir| dir.display().to_string())
                .unwrap_or_default(),
            file_name: path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default(),
            extension: path
                .extension()
                .map(|ext| format!(".{}", ext.to_string_lossy()))
                .unwrap_or_default(),
            path: path.display().to_string(),
            is_edge: matches!(kind, FeatureKind::Edge),
            is_config: matches!(kind, FeatureKind::Config),
            edge_values,
            data_type,
            data_loaded,
            method: match kind {
                FeatureKind::Computed(name) => Some(name),
                _ => None,
            },
            meta_data,
        }
    }
}

#[pymethods]
impl PyFeatureFile {
    fn __repr__(&self) -> String {
        format!("FeatureFile({:?})", self.path)
    }
}
