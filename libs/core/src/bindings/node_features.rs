//! The `F` API: `NodeFeatures` and each `NodeFeature`.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

use numpy::IntoPyArray;
use pyo3::exceptions::{PyAttributeError, PyImportError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{
    as_str_refs, collect_nodes, feature_value_from_py, feature_value_to_py,
    mapped_value_from_feature, mapped_value_to_py, node_number, node_types_from_py, warp_only,
};
use crate::bindings::visibility::FeatureVisibility;
use crate::config::OTYPE;
use crate::mapped::{
    ComputedFeatureData, MappedCompiledCorpus, MappedNodeValue, OwnedNodeFeature,
    StringPoolNodeFeatureView,
};

/// Heuristic: for a large batch, one sequential pass over the (node-sorted)
/// feature rows into a dense node-indexed table beats `n` random binary searches
/// (sequential mmap reads are cache-friendly, and the per-row cost is paid once
/// instead of `log2(rows)` times per query). The crossover is roughly
/// `n_nodes * log2(rows) > rows`; the `* 8` factor keeps small/medium batches on
/// the binary-search path (whose cost is `n * log2(rows)` with no `O(maxNode)`
/// allocation).
fn use_dense_gather(n_nodes: usize, row_count: usize) -> bool {
    row_count > 0 && n_nodes.saturating_mul(8) >= row_count
}

/// Stores `value` for `node` in a dense table sized `maxNode + 1`. Node numbers
/// come from the mapped file; a row beyond `maxNode` (only possible in a
/// corrupted cache) is ignored rather than indexing out of bounds.
fn set_node_entry<T>(table: &mut [T], node: u32, value: T) {
    if let Some(entry) = table.get_mut(node as usize) {
        *entry = value;
    }
}

/// Upper bound on interned Python strings per feature handle: one slot per
/// string-pool id below this cap (8 bytes each, allocated on first `.v()`), so
/// memory stays bounded for high-cardinality features, whose rarer values
/// (ids at or above the cap) are built fresh per call.
const INTERN_CAP: usize = 65_536;

/// A node feature handle's per-corpus state, resolved on first use.
struct NodeFeatureState {
    /// `None` when the corpus lacks the feature.
    feature: Option<OwnedNodeFeature>,
    /// String-pool features only: the Python `str` for each pool id, created on
    /// first sight and then returned with a reference-count bump, so repeated
    /// values cost neither UTF-8 decoding nor a new object. Lock-free reads.
    interned: Box<[OnceLock<Py<PyString>>]>,
}

impl NodeFeatureState {
    fn resolve(corpus: &Arc<MappedCompiledCorpus>, name: &str) -> PyResult<Self> {
        let feature = corpus.owned_node_feature(name)?;
        let intern_slots = match &feature {
            Some(OwnedNodeFeature::StringPool(pool)) => pool.view().pool_count().min(INTERN_CAP),
            _ => 0,
        };
        Ok(Self {
            feature,
            interned: (0..intern_slots).map(|_| OnceLock::new()).collect(),
        })
    }

    /// The Python `str` for string-pool id `pool_id`.
    fn pool_value(
        &self,
        py: Python<'_>,
        view: &StringPoolNodeFeatureView<'_>,
        pool_id: u32,
    ) -> PyResult<PyObject> {
        let Some(slot) = self.interned.get(pool_id as usize) else {
            return Ok(PyString::new(py, view.pool_str(pool_id)?)
                .into_any()
                .unbind());
        };
        if let Some(value) = slot.get() {
            return Ok(value.clone_ref(py).into_any());
        }
        let value = PyString::new(py, view.pool_str(pool_id)?).unbind();
        Ok(slot.get_or_init(|| value).clone_ref(py).into_any())
    }
}

/// A node feature (TF `F.<feature>`): values per node. `F.otype` also has
/// the warp members `slotType`, `maxSlot`, `maxNode`, `all`, `support`, `sInterval`.
#[pyclass(name = "NodeFeature", module = "cfabric._core")]
pub(crate) struct PyNodeFeature {
    name: String,
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Resolved on first use: the feature view and its string intern table.
    state: OnceLock<NodeFeatureState>,
}

impl PyNodeFeature {
    pub(crate) fn new(name: String, corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self {
            name,
            corpus,
            api,
            state: OnceLock::new(),
        }
    }

    fn state(&self) -> PyResult<&NodeFeatureState> {
        if let Some(state) = self.state.get() {
            return Ok(state);
        }
        let state = NodeFeatureState::resolve(&self.corpus, &self.name)?;
        Ok(self.state.get_or_init(|| state))
    }
}

#[pymethods]
impl PyNodeFeature {
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

    /// Metadata from the feature file header.
    #[getter]
    #[allow(non_snake_case)]
    fn metaData(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.corpus
            .metadata()
            .node_feature(&self.name)
            .map(|feature| feature.metadata.clone())
            .unwrap_or_default()
    }

    /// Metadata from the feature file header (TF name).
    #[getter]
    fn meta(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.metaData()
    }

    /// TF `F.f.data`: `{node: value}`. For `otype`, TF's tuple of the node types
    /// of the non-slot nodes, indexed by `node - maxSlot - 1`.
    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let Some(feature) = self.corpus.node_feature(&self.name)? else {
            return Ok(PyDict::new(py).into());
        };
        if self.name == "otype" {
            let max_slot = self.corpus.max_slot()?;
            let types = feature
                .items()?
                .into_iter()
                .filter(|(node, _)| *node > max_slot)
                .map(|(_, value)| mapped_value_to_py(py, value))
                .collect::<PyResult<Vec<PyObject>>>()?;
            return Ok(PyTuple::new(py, types)?.into());
        }
        let data = PyDict::new(py);
        for (node, value) in feature.items()? {
            data.set_item(node, mapped_value_to_py(py, value)?)?;
        }
        Ok(data.into())
    }

    /// Value type from the metadata (`str` or `int`).
    #[getter]
    #[allow(non_snake_case)]
    fn valueType(&self) -> Option<String> {
        self.corpus
            .metadata()
            .node_feature(&self.name)
            .and_then(|feature| feature.value_type().map(str::to_string))
    }

    /// Description from the metadata.
    #[getter]
    pub(crate) fn description(&self) -> Option<String> {
        self.corpus
            .metadata()
            .node_feature(&self.name)
            .and_then(|feature| feature.description().map(str::to_string))
    }

    /// The slot type (`otype` only).
    #[getter]
    #[allow(non_snake_case)]
    fn slotType(&self) -> PyResult<String> {
        if self.name != OTYPE {
            return Err(warp_only(&self.name, "slotType"));
        }
        Ok(self.corpus.slot_type()?)
    }

    /// The last slot node (`otype` only).
    #[getter]
    #[allow(non_snake_case)]
    fn maxSlot(&self) -> PyResult<u32> {
        if self.name != OTYPE {
            return Err(warp_only(&self.name, "maxSlot"));
        }
        Ok(self.corpus.max_slot()?)
    }

    /// The last node (`otype` only).
    #[getter]
    #[allow(non_snake_case)]
    fn maxNode(&self) -> PyResult<u32> {
        if self.name != OTYPE {
            return Err(warp_only(&self.name, "maxNode"));
        }
        Ok(self.corpus.max_node())
    }

    /// Node interval per node type, `{type: (first, last)}` (`otype` only).
    #[getter]
    fn support(&self) -> PyResult<BTreeMap<String, (u32, u32)>> {
        if self.name != OTYPE {
            return Err(warp_only(&self.name, "support"));
        }
        Ok(match self.corpus.computed_feature("levels")? {
            Some(ComputedFeatureData::Levels(rows)) => rows
                .into_iter()
                .map(|(node_type, _, first, last)| (node_type, (first, last)))
                .collect(),
            _ => BTreeMap::new(),
        })
    }

    /// The `(first, last)` node interval of node type `val`, or `()` for an
    /// unknown type (`otype` only).
    #[allow(non_snake_case)]
    fn sInterval(&self, py: Python<'_>, val: &str) -> PyResult<PyObject> {
        Ok(match self.support()?.get(val) {
            Some(&(first, last)) => PyTuple::new(py, [first, last])?.into(),
            None => PyTuple::empty(py).into(),
        })
    }

    /// The value of the feature for node `n`, or `None` if it has none.
    fn v(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
    ) -> PyResult<Option<PyObject>> {
        let node = n;
        let state = self.state()?;
        match &state.feature {
            Some(OwnedNodeFeature::StringPool(feature)) => {
                let view = feature.view();
                match view.pool_id(node)? {
                    Some(pool_id) => Ok(Some(state.pool_value(py, &view, pool_id)?)),
                    None => Ok(None),
                }
            }
            Some(OwnedNodeFeature::Mixed(feature)) => feature
                .view()
                .value(node)?
                .map(|value| mapped_value_to_py(py, value))
                .transpose(),
            None => Ok(None),
        }
    }

    /// pyo3 call-overhead floor: a positional-only method that does no work, used
    /// by the benchmark to subtract the binding's per-call cost from `.v()`.
    fn _noop(&self, #[pyo3(from_py_with = node_number)] node: u32) -> u32 {
        node
    }

    /// Pre-optimization `.v()` path (generic dispatcher + a fresh `PyString` per
    /// call, no interning). Kept private for the benchmark's before/after table so
    /// both numbers come from the same binary; not part of the public API.
    fn _v_legacy(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] node: u32,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .node_feature(&self.name)?
            .map(|feature| feature.v(node))
            .transpose()?
            .flatten()
            .map(|value| mapped_value_to_py(py, value))
            .transpose()
    }

    /// Batch value lookup (ADDITIVE; W4). Accepts any iterable of node ids
    /// (`list` / `tuple` / `range` / `set` / numpy array) and returns a `list`
    /// whose elements match `.v()` per node: the interned/decoded value, or
    /// `None` when the feature is absent on that node. The feature view (and the
    /// string-pool intern cache) is resolved once for the whole batch, and large
    /// batches switch to a single sequential row scan (see `use_dense_gather`)
    /// instead of one binary search per node.
    fn vs(&self, py: Python<'_>, nodes: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let nodes = collect_nodes(nodes)?;
        let max_node = self.corpus.max_node() as usize;
        let state = self.state()?;
        let values: Vec<PyObject> = match &state.feature {
            None => nodes.iter().map(|_| py.None()).collect(),
            Some(OwnedNodeFeature::StringPool(feature)) => {
                let view = feature.view();
                if use_dense_gather(nodes.len(), view.row_count()) {
                    let mut table: Vec<Option<u32>> = vec![None; max_node + 1];
                    view.for_each_id_row(&mut |node, pool_id| {
                        if let Some(entry) = table.get_mut(node as usize) {
                            *entry = Some(pool_id);
                        }
                        Ok(true)
                    })?;
                    nodes
                        .iter()
                        .map(|&node| match table.get(node as usize).copied().flatten() {
                            Some(pool_id) => state.pool_value(py, &view, pool_id),
                            None => Ok(py.None()),
                        })
                        .collect::<PyResult<Vec<PyObject>>>()?
                } else {
                    nodes
                        .iter()
                        .map(|&node| match view.pool_id(node)? {
                            Some(pool_id) => state.pool_value(py, &view, pool_id),
                            None => Ok(py.None()),
                        })
                        .collect::<PyResult<Vec<PyObject>>>()?
                }
            }
            Some(OwnedNodeFeature::Mixed(feature)) => {
                let view = feature.view();
                if use_dense_gather(nodes.len(), view.row_count()) {
                    let mut table: Vec<Option<MappedNodeValue<'_>>> = vec![None; max_node + 1];
                    for row in view.rows() {
                        let (node, value) = row?;
                        set_node_entry(&mut table, node, Some(value));
                    }
                    nodes
                        .iter()
                        .map(|&node| match table.get(node as usize).copied().flatten() {
                            Some(value) => mapped_value_to_py(py, value),
                            None => Ok(py.None()),
                        })
                        .collect::<PyResult<Vec<PyObject>>>()?
                } else {
                    nodes
                        .iter()
                        .map(|&node| match view.v(node)? {
                            Some(value) => mapped_value_to_py(py, value),
                            None => Ok(py.None()),
                        })
                        .collect::<PyResult<Vec<PyObject>>>()?
                }
            }
        };
        Ok(PyList::new(py, values)?.into())
    }

    /// Batch lookup for int-valued features into a NumPy `int64` array (ADDITIVE;
    /// W4). One boundary crossing and zero per-element Python objects.
    ///
    /// Contract: TF's per-node "absent => None" cannot live in a dense numeric
    /// array, so absent nodes take `fill` (default `0`). Pass an explicit `fill`
    /// (e.g. a sentinel) to detect gaps. `int64` is used so the full `i64` value
    /// range and any chosen sentinel always fit. Raises `TypeError` if the
    /// feature is not int-valued.
    #[pyo3(signature = (nodes, fill=0))]
    fn vs_array(&self, py: Python<'_>, nodes: &Bound<'_, PyAny>, fill: i64) -> PyResult<PyObject> {
        // NumPy is an optional dependency. Import it up front: the numpy crate
        // panics (instead of raising) when its C API capsule cannot be loaded.
        py.import("numpy").map_err(|error| {
            let import_error = PyImportError::new_err(
                "vs_array requires NumPy; install it with: pip install 'context-fabric[numpy]'",
            );
            import_error.set_cause(py, Some(error));
            import_error
        })?;
        if self.valueType().as_deref() != Some("int") {
            return Err(PyTypeError::new_err(format!(
                "vs_array requires an int-valued feature; '{}' is not int-valued",
                self.name
            )));
        }
        let nodes = collect_nodes(nodes)?;
        let mut out: Vec<i64> = Vec::with_capacity(nodes.len());
        match self.corpus.mixed_node_feature(&self.name)? {
            None => out.resize(nodes.len(), fill),
            Some(view) if use_dense_gather(nodes.len(), view.row_count()) => {
                // One sequential scan into a dense node-indexed table, then O(1)
                // gather, instead of one binary search per node.
                let max_node = self.corpus.max_node() as usize;
                let mut table: Vec<i64> = vec![fill; max_node + 1];
                for row in view.rows() {
                    let (node, value) = row?;
                    if let MappedNodeValue::Int(value) = value {
                        set_node_entry(&mut table, node, value);
                    }
                }
                out.extend(
                    nodes
                        .iter()
                        .map(|&node| table.get(node as usize).copied().unwrap_or(fill)),
                );
            }
            Some(view) => {
                for &node in &nodes {
                    out.push(match view.v(node)? {
                        Some(MappedNodeValue::Int(value)) => value,
                        _ => fill,
                    });
                }
            }
        }
        Ok(out.into_pyarray(py).into_any().unbind())
    }

    /// All nodes that have value `val`, in canonical order.
    fn s(&self, py: Python<'_>, val: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let value = feature_value_from_py(val)?;
        let nodes = self
            .corpus
            .node_feature(&self.name)?
            .map(|feature| feature.s(mapped_value_from_feature(&value)))
            .transpose()?
            .unwrap_or_default();
        Ok(PyTuple::new(py, nodes)?.into())
    }

    /// TF `F.f.items()`: `F.f.data.items()`; for `otype`, an iterator of
    /// `(node, type)` over all nodes.
    fn items(&self, py: Python<'_>) -> PyResult<PyObject> {
        if self.name != "otype" {
            return self.data(py)?.call_method0(py, "items");
        }
        let rows = self
            .corpus
            .node_feature(&self.name)?
            .map(|feature| feature.items())
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|(node, value)| {
                let row: Vec<PyObject> = vec![
                    node.into_pyobject(py)?.into(),
                    mapped_value_to_py(py, value)?,
                ];
                PyTuple::new(py, row).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.try_iter()?.into_any().unbind())
    }

    /// All node types, from the biggest to the slot type (`otype` only).
    #[getter]
    #[allow(non_snake_case)]
    fn all(&self, py: Python<'_>) -> PyResult<PyObject> {
        if self.name != "otype" {
            return Err(PyAttributeError::new_err(format!(
                "node feature {} has no attribute 'all'",
                self.name
            )));
        }
        let names: Vec<String> = match self.corpus.computed_feature("levels")? {
            Some(ComputedFeatureData::Levels(rows)) => rows
                .into_iter()
                .map(|(node_type, _, _, _)| node_type)
                .collect(),
            _ => Vec::new(),
        };
        Ok(PyTuple::new(py, names)?.into())
    }

    /// Frequency list of the values, most frequent first.
    ///
    /// Args:
    ///     nodeTypes: Only count values on nodes of these types (a str or an
    ///         iterable of str).
    ///
    /// Returns:
    ///     A tuple of `(value, count)` pairs.
    #[allow(non_snake_case)]
    #[pyo3(signature = (nodeTypes=None))]
    fn freqList(&self, py: Python<'_>, nodeTypes: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let node_types = node_types_from_py(nodeTypes)?;
        let filter = as_str_refs(&node_types);
        let rows = self
            .corpus
            .node_frequency_list(&self.name, filter.as_deref())?
            .into_iter()
            .map(|(value, count)| {
                let row: Vec<PyObject> = vec![
                    feature_value_to_py(py, &value)?,
                    count.into_pyobject(py)?.into(),
                ];
                PyTuple::new(py, row).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }
}

/// The node features of an API (TF `F`): `F.<feature>` resolves to one
/// long-lived `NodeFeature` handle, stored in the instance `__dict__` on first
/// access so later `F.<feature>` lookups are plain attribute hits (no
/// `__getattr__` fallback) and every access shares the handle's intern table.
#[pyclass(name = "NodeFeatures", module = "cfabric._core", dict)]
pub(crate) struct PyNodeFeatures {
    corpus: Arc<MappedCompiledCorpus>,
    visibility: Arc<FeatureVisibility>,
    api: ApiRef,
    /// Resolved handles keyed by feature name, shared by `F.<feature>` and
    /// `Fs(name)`.
    handles: Mutex<HashMap<String, Py<PyNodeFeature>>>,
}

impl PyNodeFeatures {
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

    /// The cached feature object for loaded node feature `name`, or
    /// `AttributeError` if it does not exist or is not loaded.
    pub(crate) fn handle(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyNodeFeature>> {
        if self.corpus.metadata().node_feature(name).is_none() || !self.visibility.exposes(name) {
            return Err(PyAttributeError::new_err(format!(
                "no loaded node feature named {name}"
            )));
        }
        let mut handles = self.handles.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(existing) = handles.get(name) {
            return Ok(existing.clone_ref(py));
        }
        let handle = Py::new(
            py,
            PyNodeFeature::new(
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
impl PyNodeFeatures {
    fn __getattr__(slf: &Bound<'_, Self>, name: &str) -> PyResult<Py<PyNodeFeature>> {
        let handle = slf.borrow().handle(slf.py(), name)?;
        slf.as_any().setattr(name, handle.clone_ref(slf.py()))?;
        Ok(handle)
    }

    fn __dir__(&self) -> Vec<String> {
        self.corpus
            .all_node_features(true)
            .into_iter()
            .filter(|name| self.visibility.exposes(name))
            .collect()
    }
}
