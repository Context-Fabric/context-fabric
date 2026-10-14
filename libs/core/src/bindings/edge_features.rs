//! The `E` API: `EdgeFeatures` and each `EdgeFeature`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyFrozenSet, PySet, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{
    as_str_refs, feature_value_to_py, mapped_value_to_py, node_number, node_types_from_py,
    warp_only,
};
use crate::bindings::visibility::FeatureVisibility;
use crate::config::OSLOTS;
use crate::mapped::{
    EdgeFeatureView, EdgeFrequency, MappedCompiledCorpus, MappedNodeValue, OwnedEdgeFeature,
};

/// An edge feature (TF `E.<feature>`): edges between nodes, optionally with
/// values. `E.oslots` also has the warp members `maxSlot` and `maxNode`.
#[pyclass(name = "EdgeFeature", module = "cfabric._core")]
pub(crate) struct PyEdgeFeature {
    name: String,
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Resolved on first use so per-call methods skip the corpus view cache.
    feature: OnceLock<OwnedEdgeFeature>,
}

impl PyEdgeFeature {
    pub(crate) fn new(name: String, corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self {
            name,
            corpus,
            api,
            feature: OnceLock::new(),
        }
    }

    fn edge(&self) -> PyResult<EdgeFeatureView<'_>> {
        if let Some(feature) = self.feature.get() {
            return Ok(feature.view());
        }
        let feature = self.corpus.owned_edge_feature(&self.name)?.ok_or_else(|| {
            PyAttributeError::new_err(format!("no edge feature named {}", self.name))
        })?;
        Ok(self.feature.get_or_init(|| feature).view())
    }

    fn has_edge_values(&self) -> bool {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .is_some_and(|feature| feature.edge_value_count > 0)
    }

    fn nodes_to_tuple(&self, py: Python<'_>, nodes: Vec<u32>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, nodes)?.into())
    }

    fn valued_nodes_to_tuple(
        &self,
        py: Python<'_>,
        nodes: Vec<(u32, Option<MappedNodeValue<'_>>)>,
    ) -> PyResult<PyObject> {
        let rows = nodes
            .into_iter()
            .map(|(node, value)| {
                let value = value
                    .map(|value| mapped_value_to_py(py, value))
                    .transpose()?
                    .unwrap_or_else(|| py.None());
                let row: Vec<PyObject> = vec![node.into_pyobject(py)?.into(), value];
                PyTuple::new(py, row).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }

    /// The value of edge `source -> target` (`None` when it has none).
    fn edge_value_to_py(
        &self,
        py: Python<'_>,
        feature: &crate::mapped::EdgeFeatureView<'_>,
        source: u32,
        target: u32,
    ) -> PyResult<PyObject> {
        Ok(feature
            .edge_value(source, target)?
            .map(|value| mapped_value_to_py(py, value))
            .transpose()?
            .unwrap_or_else(|| py.None()))
    }
}

#[pymethods]
impl PyEdgeFeature {
    /// The feature name.
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// The API (`Corpus`) this feature belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// The last slot node (`oslots` only).
    #[getter]
    #[allow(non_snake_case)]
    fn maxSlot(&self) -> PyResult<u32> {
        if self.name != OSLOTS {
            return Err(warp_only(&self.name, "maxSlot"));
        }
        Ok(self.corpus.max_slot()?)
    }

    /// The last node (`oslots` only).
    #[getter]
    #[allow(non_snake_case)]
    fn maxNode(&self) -> PyResult<u32> {
        if self.name != OSLOTS {
            return Err(warp_only(&self.name, "maxNode"));
        }
        Ok(self.corpus.max_node())
    }

    /// Metadata from the feature file header.
    #[getter]
    #[allow(non_snake_case)]
    fn metaData(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .map(|feature| feature.metadata.clone())
            .unwrap_or_default()
    }

    /// Metadata from the feature file header (TF name).
    #[getter]
    fn meta(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.metaData()
    }

    /// TF `E.f.data`: `{source: frozenset(targets)}`, or `{source: {target:
    /// value}}` for an edge feature with values, by ascending source. For
    /// `oslots`, TF's tuple of slot tuples indexed by `node - maxSlot - 1`.
    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let Some(feature) = self.corpus.edge_feature(&self.name)? else {
            return Ok(PyDict::new(py).into());
        };
        let mut rows = feature.items()?;
        rows.sort_unstable_by_key(|(source, _)| *source);
        if self.name == "oslots" {
            let slots = rows
                .into_iter()
                .map(|(_, slots)| PyTuple::new(py, slots).map(Into::into))
                .collect::<PyResult<Vec<PyObject>>>()?;
            return Ok(PyTuple::new(py, slots)?.into());
        }
        let data = PyDict::new(py);
        for (source, mut targets) in rows {
            if feature.has_edge_values() {
                targets.sort_unstable();
                let target_map = PyDict::new(py);
                for target in targets {
                    target_map
                        .set_item(target, self.edge_value_to_py(py, &feature, source, target)?)?;
                }
                data.set_item(source, target_map)?;
            } else {
                data.set_item(source, PyFrozenSet::new(py, targets)?)?;
            }
        }
        Ok(data.into())
    }

    /// TF `E.f.dataInv`: `{target: set(sources)}`, or `{target: {source:
    /// value}}` for an edge feature with values, by ascending target.
    #[getter]
    #[allow(non_snake_case)]
    fn dataInv(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = PyDict::new(py);
        let Some(feature) = self.corpus.edge_feature(&self.name)? else {
            return Ok(data.into());
        };
        let mut inverse = std::collections::BTreeMap::<u32, Vec<u32>>::new();
        for (source, targets) in feature.items()? {
            for target in targets {
                inverse.entry(target).or_default().push(source);
            }
        }
        for (target, mut sources) in inverse {
            sources.sort_unstable();
            if feature.has_edge_values() {
                let source_map = PyDict::new(py);
                for source in sources {
                    source_map
                        .set_item(source, self.edge_value_to_py(py, &feature, source, target)?)?;
                }
                data.set_item(target, source_map)?;
            } else {
                data.set_item(target, PySet::new(py, sources)?)?;
            }
        }
        Ok(data.into())
    }

    /// Value type of the edge values from the metadata.
    #[getter]
    #[allow(non_snake_case)]
    fn valueType(&self) -> Option<String> {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .and_then(|feature| feature.value_type().map(str::to_string))
    }

    /// Description from the metadata.
    #[getter]
    pub(crate) fn description(&self) -> Option<String> {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .and_then(|feature| feature.description().map(str::to_string))
    }

    /// Nodes reached by edges from node `n` (with values for valued edges).
    fn f(&self, py: Python<'_>, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<PyObject> {
        let node = n;
        let feature = self.edge()?;
        if feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, feature.f_with_values(node)?)
        } else {
            self.nodes_to_tuple(py, feature.f(node)?)
        }
    }

    /// Nodes with edges to node `n` (with values for valued edges).
    fn t(&self, py: Python<'_>, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<PyObject> {
        let node = n;
        let feature = self.edge()?;
        if feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, feature.t_with_values(node)?)
        } else {
            self.nodes_to_tuple(py, feature.t(node)?)
        }
    }

    /// Targets of node `n` without values (TF `E.oslots.s`: the slots of `n`).
    fn s(&self, py: Python<'_>, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<PyObject> {
        let feature = self.edge()?;
        self.nodes_to_tuple(py, feature.s(n)?)
    }

    /// Nodes connected to node `n` in either direction (with values for valued edges).
    fn b(&self, py: Python<'_>, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<PyObject> {
        let node = n;
        let feature = self.edge()?;
        if feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, feature.b_with_values(node)?)
        } else {
            self.nodes_to_tuple(py, feature.b(node)?)
        }
    }

    /// TF `E.f.items()`: `E.f.data.items()`; for `oslots`, an iterator of
    /// `(node, slots)` over the non-slot nodes.
    fn items(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = self.data(py)?;
        if self.name != "oslots" {
            return Ok(data.call_method0(py, "items")?);
        }
        let first = self.corpus.max_slot()? + 1;
        let rows = data
            .bind(py)
            .try_iter()?
            .zip(first..)
            .map(|(slots, node)| {
                let row: Vec<PyObject> = vec![node.into_pyobject(py)?.into(), slots?.unbind()];
                PyTuple::new(py, row).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.try_iter()?.into_any().unbind())
    }

    /// Frequency list of the edge values, most frequent first.
    ///
    /// Args:
    ///     nodeTypesFrom: Only count edges from nodes of these types.
    ///     nodeTypesTo: Only count edges to nodes of these types.
    ///
    /// Returns:
    ///     A tuple of `(value, count)` pairs for valued edges, otherwise the
    ///     number of edges.
    #[allow(non_snake_case)]
    #[pyo3(signature = (nodeTypesFrom=None, nodeTypesTo=None))]
    fn freqList(
        &self,
        py: Python<'_>,
        nodeTypesFrom: Option<&Bound<'_, PyAny>>,
        nodeTypesTo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node_types_from = node_types_from_py(nodeTypesFrom)?;
        let node_types_to = node_types_from_py(nodeTypesTo)?;
        let filter_from = as_str_refs(&node_types_from);
        let filter_to = as_str_refs(&node_types_to);
        match self.corpus.edge_frequency_list(
            &self.name,
            filter_from.as_deref(),
            filter_to.as_deref(),
        )? {
            EdgeFrequency::Count(count) => Ok(count.into_pyobject(py)?.into()),
            EdgeFrequency::Values(rows) => {
                let rows = rows
                    .into_iter()
                    .map(|(value, count)| {
                        let value = value
                            .as_ref()
                            .map(|value| feature_value_to_py(py, value))
                            .transpose()?
                            .unwrap_or_else(|| py.None());
                        let row: Vec<PyObject> = vec![value, count.into_pyobject(py)?.into()];
                        PyTuple::new(py, row).map(Into::into)
                    })
                    .collect::<PyResult<Vec<PyObject>>>()?;
                Ok(PyTuple::new(py, rows)?.into())
            }
        }
    }

    /// Whether the edges carry values.
    #[getter]
    #[allow(non_snake_case)]
    fn doValues(&self) -> bool {
        self.has_edge_values()
    }
}

/// The edge features of an API (TF `E`): like `F`, each `E.<feature>` handle
/// is stored in the instance `__dict__` on first access.
#[pyclass(name = "EdgeFeatures", module = "cfabric._core", dict)]
pub(crate) struct PyEdgeFeatures {
    corpus: Arc<MappedCompiledCorpus>,
    visibility: Arc<FeatureVisibility>,
    api: ApiRef,
    /// Cache of resolved handles, so `E.<feat>` is the same object each time.
    handles: Mutex<HashMap<String, Py<PyEdgeFeature>>>,
}

impl PyEdgeFeatures {
    pub(crate) fn new(
        corpus: Arc<MappedCompiledCorpus>,
        visibility: Arc<FeatureVisibility>,
        api: ApiRef,
    ) -> Self {
        Self {
            corpus,
            visibility,
            api,
            handles: Mutex::new(HashMap::new()),
        }
    }

    /// The cached feature object for loaded edge feature `name`, or
    /// `AttributeError` if it does not exist or is not loaded.
    pub(crate) fn handle(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyEdgeFeature>> {
        if self.corpus.metadata().edge_feature(name).is_none() || !self.visibility.exposes(name) {
            return Err(PyAttributeError::new_err(format!(
                "no loaded edge feature named {name}"
            )));
        }
        let mut handles = self.handles.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(existing) = handles.get(name) {
            return Ok(existing.clone_ref(py));
        }
        let handle = Py::new(
            py,
            PyEdgeFeature::new(
                name.to_string(),
                Arc::clone(&self.corpus),
                self.api.clone_ref(py),
            ),
        )?;
        handles.insert(name.to_string(), handle.clone_ref(py));
        Ok(handle)
    }
}

#[pymethods]
impl PyEdgeFeatures {
    fn __getattr__(slf: &Bound<'_, Self>, name: &str) -> PyResult<Py<PyEdgeFeature>> {
        let handle = slf.borrow().handle(slf.py(), name)?;
        slf.as_any().setattr(name, handle.clone_ref(slf.py()))?;
        Ok(handle)
    }

    fn __dir__(&self) -> Vec<String> {
        self.corpus
            .all_edge_features(true)
            .into_iter()
            .filter(|name| self.visibility.exposes(name))
            .collect()
    }
}
