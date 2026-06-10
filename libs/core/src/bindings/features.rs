use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use pyo3::basic::CompareOp;
use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyString, PyTuple};

use crate::compiled::{MappedCompiledCorpus, MappedNodeValue, NodeFeatureEncoding};
use crate::corpus::{Boundary, Chunk, ChunkLengthKey, ChunkPositionKey, ComputedFeatureData};
use crate::feature::{EdgeFrequency, FeatureValue};

/// Parses a `nodeTypes` style argument (`None` / `str` / iterable of `str`)
/// into an owned `Vec<String>`. Mirrors TF's `freqList(nodeTypes=...)` which
/// accepts any iterable of node-type names (commonly a `set`).
pub(crate) fn node_types_from_py(
    arg: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<String>>> {
    let Some(arg) = arg else {
        return Ok(None);
    };
    if arg.is_none() {
        return Ok(None);
    }
    if let Ok(value) = arg.extract::<String>() {
        return Ok(Some(vec![value]));
    }
    let iter = arg.try_iter().map_err(|_| {
        PyTypeError::new_err("node type filter must be a str or an iterable of str")
    })?;
    let mut out = Vec::new();
    for item in iter {
        out.push(item?.extract::<String>()?);
    }
    Ok(Some(out))
}

fn as_str_refs(values: &Option<Vec<String>>) -> Option<Vec<&str>> {
    values
        .as_ref()
        .map(|values| values.iter().map(String::as_str).collect())
}

/// Parses a TF chunk `(node, (begin, end))` into the core [`Chunk`] type.
pub(crate) fn chunk_from_py(chunk: &Bound<'_, PyAny>) -> PyResult<Chunk> {
    let node: u32 = chunk
        .get_item(0)
        .map_err(|_| PyTypeError::new_err("chunk must be a (node, (begin, end)) tuple"))?
        .extract()?;
    let slots = chunk
        .get_item(1)
        .map_err(|_| PyTypeError::new_err("chunk must be a (node, (begin, end)) tuple"))?;
    let start: u32 = slots.get_item(0)?.extract()?;
    let end: u32 = slots.get_item(1)?.extract()?;
    Ok(Chunk { node, start, end })
}

/// Builds the canonical-position sort key for a chunk (`N.sortKeyChunk`).
pub(crate) fn sort_key_chunk_py(
    corpus: &MappedCompiledCorpus,
    chunk: &Bound<'_, PyAny>,
) -> PyResult<PyChunkPositionKey> {
    let chunk = chunk_from_py(chunk)?;
    Ok(PyChunkPositionKey {
        key: corpus.sort_key_chunk(chunk)?,
    })
}

/// Builds the canonical-length sort key for a chunk (`N.sortKeyChunkLength`).
pub(crate) fn sort_key_chunk_length_py(
    corpus: &MappedCompiledCorpus,
    chunk: &Bound<'_, PyAny>,
) -> PyResult<PyChunkLengthKey> {
    let chunk = chunk_from_py(chunk)?;
    Ok(PyChunkLengthKey {
        key: corpus.sort_key_chunk_length(chunk)?,
    })
}

/// Opaque, orderable key returned by `N.sortKeyChunk`. Comparisons delegate to
/// the core `ChunkPositionKey` ordering so `sorted(chunks, key=N.sortKeyChunk)`
/// produces TF's canonical chunk order.
#[pyclass(name = "ChunkPositionKey")]
pub(crate) struct PyChunkPositionKey {
    key: ChunkPositionKey,
}

#[pymethods]
impl PyChunkPositionKey {
    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        op.matches(self.key.cmp(&other.key))
    }
}

/// Opaque, orderable key returned by `N.sortKeyChunkLength`.
#[pyclass(name = "ChunkLengthKey")]
pub(crate) struct PyChunkLengthKey {
    key: ChunkLengthKey,
}

#[pymethods]
impl PyChunkLengthKey {
    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        op.matches(self.key.cmp(&other.key))
    }
}

pub(crate) fn feature_value_to_py(py: Python<'_>, value: &FeatureValue) -> PyResult<PyObject> {
    match value {
        FeatureValue::Str(value) => Ok(PyString::new(py, value).into()),
        FeatureValue::Int(value) => Ok(PyInt::new(py, *value).into()),
    }
}

fn mapped_value_to_py(py: Python<'_>, value: MappedNodeValue<'_>) -> PyResult<PyObject> {
    match value {
        MappedNodeValue::Str(value) => Ok(PyString::new(py, value).into()),
        MappedNodeValue::Int(value) => Ok(PyInt::new(py, value).into()),
    }
}

pub(crate) fn feature_value_from_py(value: &Bound<'_, PyAny>) -> PyResult<FeatureValue> {
    if let Ok(value) = value.extract::<i64>() {
        return Ok(FeatureValue::Int(value));
    }
    if let Ok(value) = value.extract::<String>() {
        return Ok(FeatureValue::string(value));
    }
    Err(PyTypeError::new_err("feature values must be str or int"))
}

fn mapped_value_from_feature(value: &FeatureValue) -> MappedNodeValue<'_> {
    match value {
        FeatureValue::Str(value) => MappedNodeValue::Str(value),
        FeatureValue::Int(value) => MappedNodeValue::Int(*value),
    }
}

/// Cached encoding of a node feature, resolved lazily on the long-lived
/// `PyNodeFeature` handle so the per-call `.v()` path can branch without a
/// metadata scan and dispatch straight to the concrete mmap accessor.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CachedKind {
    /// String-pool encoded: every distinct value is a single, stable byte range
    /// in the pool, so the returned `&str` slice is identity-stable per value and
    /// can be interned by `(ptr, len)`.
    StringPool,
    /// Mixed (string/int) encoded, or feature absent: values are read inline per
    /// row, so the slice pointer is per-node and not safe to intern.
    Mixed,
}

/// Upper bound on distinct interned Python strings per feature handle. A simple
/// "stop interning once full" policy keeps memory bounded (no LRU): at this cap
/// the table holds at most ~65k short `str` objects (a few MB), well under the
/// per-process budget. High-cardinality features (e.g. `g_word`) simply fall
/// back to building fresh strings once the table fills.
const INTERN_CAP: usize = 65_536;

/// Minimal multiplicative hasher for the integer intern keys. The default
/// `SipHash` is overkill (and ~10-20ns) for keys we already control; a single
/// Fibonacci-style multiply distributes pool pointers well enough and shaves the
/// hot-path cost on every `.v()` lookup.
#[derive(Default)]
struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut acc = self.0;
        for &byte in bytes {
            acc = (acc.rotate_left(5) ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
        }
        self.0 = acc;
    }

    fn write_u128(&mut self, value: u128) {
        let lo = value as u64;
        let hi = (value >> 64) as u64;
        self.0 = (hi ^ lo).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

type IdBuildHasher = std::hash::BuildHasherDefault<IdHasher>;

#[derive(Default)]
struct InternTable {
    /// Key is `((ptr as u128) << 64) | len`. The string-pool slice pointer alone
    /// would collide for an empty-string pool entry that shares a start offset
    /// with the following entry, so the length is folded in to make the key a
    /// true value identity.
    map: HashMap<u128, Py<PyString>, IdBuildHasher>,
    full: bool,
}

#[pyclass(name = "NodeFeature")]
pub(crate) struct PyNodeFeature {
    name: String,
    corpus: Arc<MappedCompiledCorpus>,
    /// Resolved once on first `.v()` access (mechanism 2: handle-owned decision).
    kind: OnceLock<CachedKind>,
    /// Pool-id (pointer-keyed) string interning cache (mechanism 1). Guarded by a
    /// `Mutex` so the handle stays `Sync` for pyo3; the GIL makes it uncontended.
    intern: Mutex<InternTable>,
}

impl PyNodeFeature {
    pub(crate) fn new(name: String, corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self {
            name,
            corpus,
            kind: OnceLock::new(),
            intern: Mutex::new(InternTable::default()),
        }
    }

    fn cached_kind(&self) -> CachedKind {
        *self.kind.get_or_init(|| {
            match self
                .corpus
                .metadata()
                .node_feature(&self.name)
                .map(|feature| feature.encoding)
            {
                Some(NodeFeatureEncoding::StringPool) => CachedKind::StringPool,
                _ => CachedKind::Mixed,
            }
        })
    }

    /// Returns the Python `str` for a string-pool value, reusing a cached
    /// `Py<PyString>` (INCREF) when the same pool entry was seen before so repeat
    /// values avoid re-encoding the bytes into a fresh CPython object.
    fn intern_str(&self, py: Python<'_>, value: &str) -> PyObject {
        let key = ((value.as_ptr() as u128) << 64) | (value.len() as u128);
        let mut table = self.intern.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(existing) = table.map.get(&key) {
            return existing.clone_ref(py).into_any();
        }
        let py_str = PyString::new(py, value);
        if !table.full {
            if table.map.len() >= INTERN_CAP {
                table.full = true;
            } else {
                table.map.insert(key, py_str.clone().unbind());
            }
        }
        py_str.into_any().unbind()
    }
}

#[pymethods]
impl PyNodeFeature {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    #[allow(non_snake_case)]
    fn metaData(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.corpus
            .metadata()
            .node_feature(&self.name)
            .map(|feature| feature.metadata.clone())
            .unwrap_or_default()
    }

    #[getter]
    fn meta(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.metaData()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = PyDict::new(py);
        if let Some(feature) = self.corpus.node_feature(&self.name)? {
            for (node, value) in feature.items()? {
                data.set_item(node, mapped_value_to_py(py, value)?)?;
            }
        }
        Ok(data.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn valueType(&self) -> Option<String> {
        self.corpus
            .metadata()
            .node_feature(&self.name)
            .and_then(|feature| feature.value_type().map(str::to_string))
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.corpus
            .metadata()
            .node_feature(&self.name)
            .and_then(|feature| feature.description().map(str::to_string))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn slotType(&self) -> Option<String> {
        (self.name == "otype")
            .then(|| self.corpus.slot_type().ok())
            .flatten()
    }

    #[getter]
    #[allow(non_snake_case)]
    fn maxSlot(&self) -> Option<u32> {
        (self.name == "otype")
            .then(|| self.corpus.max_slot().ok())
            .flatten()
    }

    #[getter]
    #[allow(non_snake_case)]
    fn maxNode(&self) -> Option<u32> {
        (self.name == "otype").then(|| self.corpus.max_node())
    }

    fn v(&self, py: Python<'_>, node: u32) -> PyResult<Option<PyObject>> {
        match self.cached_kind() {
            CachedKind::StringPool => {
                // Dispatch straight to the string-pool accessor, skipping the
                // generic `node_feature` dispatcher's extra metadata scan.
                let Some(view) = self.corpus.string_pool_node_feature(&self.name)? else {
                    return Ok(None);
                };
                let Some(value) = view.str_value(node)? else {
                    return Ok(None);
                };
                Ok(Some(self.intern_str(py, value)))
            }
            CachedKind::Mixed => self
                .corpus
                .mixed_node_feature(&self.name)?
                .map(|feature| feature.v(node))
                .transpose()?
                .flatten()
                .map(|value| mapped_value_to_py(py, value))
                .transpose(),
        }
    }

    /// pyo3 call-overhead floor: a positional-only method that does no work, used
    /// by the benchmark to subtract the binding's per-call cost from `.v()`.
    fn _noop(&self, node: u32) -> u32 {
        node
    }

    /// Pre-optimization `.v()` path (generic dispatcher + a fresh `PyString` per
    /// call, no interning). Kept private for the benchmark's before/after table so
    /// both numbers come from the same binary; not part of the public API.
    fn _v_legacy(&self, py: Python<'_>, node: u32) -> PyResult<Option<PyObject>> {
        self.corpus
            .node_feature(&self.name)?
            .map(|feature| feature.v(node))
            .transpose()?
            .flatten()
            .map(|value| mapped_value_to_py(py, value))
            .transpose()
    }

    fn s(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let value = feature_value_from_py(value)?;
        let nodes = self
            .corpus
            .node_feature(&self.name)?
            .map(|feature| feature.s(mapped_value_from_feature(&value)))
            .transpose()?
            .unwrap_or_default();
        Ok(PyTuple::new(py, nodes)?.into())
    }

    fn items(&self, py: Python<'_>) -> PyResult<PyObject> {
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
        Ok(PyTuple::new(py, rows)?.into())
    }

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
            Some(ComputedFeatureData::Levels(rows)) => {
                rows.into_iter().map(|(node_type, _, _, _)| node_type).collect()
            }
            _ => Vec::new(),
        };
        Ok(PyTuple::new(py, names)?.into())
    }

    #[pyo3(signature = (node_types=None))]
    fn freq_list(
        &self,
        py: Python<'_>,
        node_types: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node_types = node_types_from_py(node_types)?;
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

    #[allow(non_snake_case)]
    #[pyo3(signature = (nodeTypes=None))]
    fn freqList(
        &self,
        py: Python<'_>,
        nodeTypes: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.freq_list(py, nodeTypes)
    }
}

#[pyclass(name = "NodeFeatures")]
pub(crate) struct PyNodeFeatures {
    corpus: Arc<MappedCompiledCorpus>,
    /// Cache of resolved `PyNodeFeature` handles keyed by feature name. Returning
    /// the *same* handle for repeated `F.<feat>` accesses keeps the per-handle
    /// intern cache (mechanism 1) alive across `F.<feat>.v(n)` statements, which
    /// is what makes interning pay off in ordinary usage (TF's `sp = F.sp.v`
    /// idiom and bare `F.sp.v(n)` loops both benefit).
    handles: Mutex<HashMap<String, Py<PyNodeFeature>>>,
}

impl PyNodeFeatures {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self {
            corpus,
            handles: Mutex::new(HashMap::new()),
        }
    }
}

#[pymethods]
impl PyNodeFeatures {
    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyNodeFeature>> {
        if self.corpus.metadata().node_feature(name).is_none() {
            return Err(PyAttributeError::new_err(format!(
                "no node feature named {name}"
            )));
        }
        let mut handles = self.handles.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(existing) = handles.get(name) {
            return Ok(existing.clone_ref(py));
        }
        let handle = Py::new(
            py,
            PyNodeFeature::new(name.to_string(), Arc::clone(&self.corpus)),
        )?;
        handles.insert(name.to_string(), handle.clone_ref(py));
        Ok(handle)
    }

    fn __dir__(&self) -> Vec<String> {
        self.corpus.all_node_features(true)
    }
}

#[pyclass(name = "EdgeFeature")]
pub(crate) struct PyEdgeFeature {
    name: String,
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyEdgeFeature {
    pub(crate) fn new(name: String, corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self { name, corpus }
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
}

#[pymethods]
impl PyEdgeFeature {
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    #[allow(non_snake_case)]
    fn metaData(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .map(|feature| feature.metadata.clone())
            .unwrap_or_default()
    }

    #[getter]
    fn meta(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.metaData()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = PyDict::new(py);
        let Some(feature) = self.corpus.edge_feature(&self.name)? else {
            return Ok(data.into());
        };
        if feature.has_edge_values() {
            for (source, targets) in feature.items()? {
                let target_map = PyDict::new(py);
                for target in targets {
                    let value = feature
                        .edge_value(source, target)?
                        .map(|value| mapped_value_to_py(py, value))
                        .transpose()?
                        .unwrap_or_else(|| py.None());
                    target_map.set_item(target, value)?;
                }
                data.set_item(source, target_map)?;
            }
        } else {
            for (source, targets) in feature.items()? {
                data.set_item(source, PyTuple::new(py, targets)?)?;
            }
        }
        Ok(data.into())
    }

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
        if feature.has_edge_values() {
            for (target, mut sources) in inverse {
                sources.sort_unstable();
                let source_map = PyDict::new(py);
                for source in sources {
                    let value = feature
                        .edge_value(source, target)?
                        .map(|value| mapped_value_to_py(py, value))
                        .transpose()?
                        .unwrap_or_else(|| py.None());
                    source_map.set_item(source, value)?;
                }
                data.set_item(target, source_map)?;
            }
        } else {
            for (target, mut sources) in inverse {
                sources.sort_unstable();
                data.set_item(target, PyTuple::new(py, sources)?)?;
            }
        }
        Ok(data.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn valueType(&self) -> Option<String> {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .and_then(|feature| feature.value_type().map(str::to_string))
    }

    #[getter]
    fn description(&self) -> Option<String> {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .and_then(|feature| feature.description().map(str::to_string))
    }

    fn f(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        let feature = self
            .corpus
            .edge_feature(&self.name)?
            .ok_or_else(|| PyAttributeError::new_err(format!("no edge feature named {}", self.name)))?;
        if feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, feature.f_with_values(node)?)
        } else {
            self.nodes_to_tuple(py, feature.f(node)?)
        }
    }

    fn t(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        let feature = self
            .corpus
            .edge_feature(&self.name)?
            .ok_or_else(|| PyAttributeError::new_err(format!("no edge feature named {}", self.name)))?;
        if feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, feature.t_with_values(node)?)
        } else {
            self.nodes_to_tuple(py, feature.t(node)?)
        }
    }

    fn s(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        let feature = self
            .corpus
            .edge_feature(&self.name)?
            .ok_or_else(|| PyAttributeError::new_err(format!("no edge feature named {}", self.name)))?;
        self.nodes_to_tuple(py, feature.s(node)?)
    }

    fn b(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        let feature = self
            .corpus
            .edge_feature(&self.name)?
            .ok_or_else(|| PyAttributeError::new_err(format!("no edge feature named {}", self.name)))?;
        if feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, feature.b_with_values(node)?)
        } else {
            self.nodes_to_tuple(py, feature.b(node)?)
        }
    }

    fn items(&self, py: Python<'_>) -> PyResult<PyObject> {
        let rows = self
            .corpus
            .edge_feature(&self.name)?
            .map(|feature| feature.items())
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|(source, targets)| {
                let row: Vec<PyObject> = vec![
                    source.into_pyobject(py)?.into(),
                    PyTuple::new(py, targets)?.into(),
                ];
                PyTuple::new(py, row).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }

    #[pyo3(signature = (node_types_from=None, node_types_to=None))]
    fn freq_list(
        &self,
        py: Python<'_>,
        node_types_from: Option<&Bound<'_, PyAny>>,
        node_types_to: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node_types_from = node_types_from_py(node_types_from)?;
        let node_types_to = node_types_from_py(node_types_to)?;
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

    #[allow(non_snake_case)]
    #[pyo3(signature = (nodeTypesFrom=None, nodeTypesTo=None))]
    fn freqList(
        &self,
        py: Python<'_>,
        nodeTypesFrom: Option<&Bound<'_, PyAny>>,
        nodeTypesTo: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.freq_list(py, nodeTypesFrom, nodeTypesTo)
    }

    fn has_edge_values(&self) -> bool {
        self.corpus
            .metadata()
            .edge_feature(&self.name)
            .is_some_and(|feature| feature.edge_value_count > 0)
    }

    #[getter]
    #[allow(non_snake_case)]
    fn doValues(&self) -> bool {
        self.has_edge_values()
    }

    #[allow(non_snake_case)]
    fn hasEdgeValues(&self) -> bool {
        self.has_edge_values()
    }
}

#[pyclass(name = "EdgeFeatures")]
pub(crate) struct PyEdgeFeatures {
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyEdgeFeatures {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyEdgeFeatures {
    fn __getattr__(&self, name: &str) -> PyResult<PyEdgeFeature> {
        self.corpus
            .metadata()
            .edge_feature(name)
            .map(|_| PyEdgeFeature::new(name.to_string(), Arc::clone(&self.corpus)))
            .ok_or_else(|| PyAttributeError::new_err(format!("no edge feature named {name}")))
    }

    fn __dir__(&self) -> Vec<String> {
        self.corpus.all_edge_features(true)
    }
}

/// Lazy mapping that backs `C.levUp.data` / `C.levDown.data`.
///
/// SHAPE DIVERGENCE FROM TF: in text-fabric `C.levUp.data` is a flat tuple of
/// `maxNode` tuples where `data[n - 1]` are the embedders of node `n`, and
/// `C.levDown.data[n - maxSlot - 1]` are the embeddees of non-slot node `n`.
/// Materializing those (1.4M tuples on BHSA) eagerly defeats the mmap-first
/// design, so we expose a lazy object instead: index it by the node id directly
/// (`C.levUp.data[n]` -> embedders of `n`, `C.levDown.data[n]` -> embeddees of
/// `n`). Rows are read on demand from the `CFRLEVU1` / `CFRLEVD1` mmap CSRs.
#[pyclass(name = "LevView")]
pub(crate) struct PyLevView {
    corpus: Arc<MappedCompiledCorpus>,
    down: bool,
}

#[pymethods]
impl PyLevView {
    fn __getitem__(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        // Bound-check by node id so default Python iteration terminates rather
        // than looping forever (this is a mapping keyed by node, not a sequence).
        if node < 1 || node > self.corpus.max_node() {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "node {node} out of range"
            )));
        }
        let row = if self.down {
            self.corpus.lev_down_row(node)?
        } else {
            self.corpus.lev_up_row(node)?
        }
        .unwrap_or_default();
        Ok(PyTuple::new(py, row)?.into())
    }

    /// Embedders (`levUp`) or embeddees (`levDown`) of `node` as a tuple.
    fn row(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        self.__getitem__(py, node)
    }

    fn __len__(&self) -> usize {
        self.corpus.max_node() as usize
    }

    fn __contains__(&self, node: u32) -> bool {
        node >= 1 && node <= self.corpus.max_node()
    }
}

#[pyclass(name = "Computed")]
pub(crate) struct PyComputed {
    #[pyo3(get)]
    data: PyObject,
}

impl PyComputed {
    fn new(data: PyObject) -> Self {
        Self { data }
    }
}

#[pyclass(name = "Computeds")]
pub(crate) struct PyComputeds {
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyComputeds {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self { corpus }
    }

    fn boundary_to_py(py: Python<'_>, boundary: Boundary) -> PyResult<PyObject> {
        let first = boundary
            .first_slots
            .iter()
            .map(|nodes| PyTuple::new(py, nodes.iter().copied()).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        let last = boundary
            .last_slots
            .iter()
            .map(|nodes| PyTuple::new(py, nodes.iter().copied()).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        let row: Vec<PyObject> = vec![
            PyTuple::new(py, first)?.into(),
            PyTuple::new(py, last)?.into(),
        ];
        Ok(PyTuple::new(py, row)?.into())
    }

    pub(crate) fn get(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        match name {
            "levels" => Ok(self.levels(py)?.into_pyobject(py)?.into()),
            "order" => Ok(self.order(py)?.into_pyobject(py)?.into()),
            "rank" => Ok(self.rank(py)?.into_pyobject(py)?.into()),
            "boundary" => Ok(self.boundary(py)?.into_pyobject(py)?.into()),
            "levUp" => Ok(self.levUp(py)?.into_pyobject(py)?.into()),
            "levDown" => Ok(self.levDown(py)?.into_pyobject(py)?.into()),
            "sections" => Ok(self.sections(py)?.into_pyobject(py)?.into()),
            "characters" => Ok(self.characters(py)?.into_pyobject(py)?.into()),
            _ => Err(PyAttributeError::new_err(format!(
                "no computed feature named {name}"
            ))),
        }
    }

    fn lev_view(&self, py: Python<'_>, down: bool) -> PyResult<PyComputed> {
        let view = Py::new(
            py,
            PyLevView {
                corpus: Arc::clone(&self.corpus),
                down,
            },
        )?;
        Ok(PyComputed::new(view.into_any()))
    }

    fn sections_to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        // TF C.sections.data shape: dict(sec1=, sec2=, seqFromNode=, nodeFromSeq=).
        let data = PyDict::new(py);
        let Some(sections) = self.corpus.sections_data()? else {
            for key in ["sec1", "sec2", "seqFromNode", "nodeFromSeq"] {
                data.set_item(key, PyDict::new(py))?;
            }
            return Ok(data.into());
        };

        let sec1 = PyDict::new(py);
        for (node, inner) in &sections.sec1 {
            let inner_dict = PyDict::new(py);
            for (heading, target) in inner {
                inner_dict.set_item(heading, *target)?;
            }
            sec1.set_item(node, inner_dict)?;
        }

        let sec2 = PyDict::new(py);
        for (node, inner) in &sections.sec2 {
            let inner_dict = PyDict::new(py);
            for (heading1, inner2) in inner {
                let inner2_dict = PyDict::new(py);
                for (heading2, target) in inner2 {
                    inner2_dict.set_item(heading2, *target)?;
                }
                inner_dict.set_item(heading1, inner2_dict)?;
            }
            sec2.set_item(node, inner_dict)?;
        }

        let seq_from_node = PyDict::new(py);
        for (node, seq) in &sections.seq_from_node {
            seq_from_node.set_item(node, PyTuple::new(py, seq.iter().copied())?)?;
        }

        let node_from_seq = PyDict::new(py);
        for (seq, node) in &sections.node_from_seq {
            node_from_seq.set_item(PyTuple::new(py, seq.iter().copied())?, *node)?;
        }

        data.set_item("sec1", sec1)?;
        data.set_item("sec2", sec2)?;
        data.set_item("seqFromNode", seq_from_node)?;
        data.set_item("nodeFromSeq", node_from_seq)?;
        Ok(data.into())
    }

    fn characters_to_py(&self, py: Python<'_>) -> PyResult<PyObject> {
        // TF C.characters.data shape: dict{format: sorted [(char, count)]}.
        //
        // The mapped core exposes no `characters` accessor (it is not a
        // `computed_feature` arm and `precompute::characters` needs the in-memory
        // Corpus), so we reconstruct it here from the `otext` `fmt:` specs and the
        // referenced node-feature values. The feature names in a format spec are
        // extracted with a simplified `{...}` token scanner, which may diverge
        // from TF's full format compiler on exotic specs; on plain `{feature}`
        // specs (BHSA, mini fixtures) it matches.
        let result = PyDict::new(py);
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(result.into());
        };
        let meta = otext.metadata()?;

        // format name -> feature list
        let mut formats: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (key, value) in &meta {
            let Some(format_name) = key.strip_prefix("fmt:") else {
                continue;
            };
            let Some(spec) = value else {
                continue;
            };
            formats.insert(format_name.to_string(), extract_format_features(spec));
        }

        // char counts per feature (only computed once per feature)
        let mut counts_by_feature: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        for features in formats.values() {
            for feature in features {
                if counts_by_feature.contains_key(feature) {
                    continue;
                }
                let mut counts: BTreeMap<String, usize> = BTreeMap::new();
                if let Some(view) = self.corpus.node_feature(feature)? {
                    for row in view.items()? {
                        let (_, value) = row;
                        let text = match value {
                            MappedNodeValue::Str(value) => value.to_string(),
                            MappedNodeValue::Int(value) => value.to_string(),
                        };
                        for character in text.chars() {
                            *counts.entry(character.to_string()).or_default() += 1;
                        }
                    }
                }
                counts_by_feature.insert(feature.clone(), counts);
            }
        }

        for (format_name, features) in &formats {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for feature in features {
                if let Some(feature_counts) = counts_by_feature.get(feature) {
                    for (character, count) in feature_counts {
                        *counts.entry(character.clone()).or_default() += count;
                    }
                }
            }
            let rows = counts
                .into_iter()
                .map(|(character, count)| {
                    let row: Vec<PyObject> =
                        vec![PyString::new(py, &character).into(), count.into_pyobject(py)?.into()];
                    PyTuple::new(py, row).map(Into::into)
                })
                .collect::<PyResult<Vec<PyObject>>>()?;
            result.set_item(format_name, PyTuple::new(py, rows)?)?;
        }

        Ok(result.into())
    }
}

/// Extracts node-feature tokens from a TF `otext` format spec. Feature names are
/// the identifier-like runs that appear inside `{...}` groups; literals outside
/// braces (separators) are ignored. Tokens that are not real node features map
/// to no value and contribute nothing.
fn extract_format_features(spec: &str) -> Vec<String> {
    let mut features: Vec<String> = Vec::new();
    let mut depth: i32 = 0;
    let mut current = String::new();
    for ch in spec.chars() {
        match ch {
            '{' => {
                depth += 1;
            }
            '}' => {
                if !current.is_empty() && !features.contains(&current) {
                    features.push(current.clone());
                }
                current.clear();
                depth -= 1;
            }
            c if depth > 0 && (c.is_alphanumeric() || c == '_' || c == '@') => current.push(c),
            _ => {
                if depth > 0 && !current.is_empty() {
                    if !features.contains(&current) {
                        features.push(current.clone());
                    }
                    current.clear();
                }
            }
        }
    }
    features
}

#[pymethods]
impl PyComputeds {
    #[getter]
    fn levels(&self, py: Python<'_>) -> PyResult<PyComputed> {
        let rows = match self.corpus.computed_feature("levels")? {
            Some(ComputedFeatureData::Levels(rows)) => rows,
            _ => Vec::new(),
        }
        .into_iter()
        .map(|(node_type, level, first, last)| {
            let row: Vec<PyObject> = vec![
                PyString::new(py, &node_type).into(),
                level.into_pyobject(py)?.into(),
                first.into_pyobject(py)?.into(),
                last.into_pyobject(py)?.into(),
            ];
            PyTuple::new(py, row).map(Into::into)
        })
        .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyComputed::new(PyTuple::new(py, rows)?.into()))
    }

    #[getter]
    fn order(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(PyComputed::new(
            PyTuple::new(py, self.corpus.order()?)?.into(),
        ))
    }

    #[getter]
    fn rank(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(PyComputed::new(
            PyTuple::new(py, self.corpus.rank()?)?.into(),
        ))
    }

    #[getter]
    fn boundary(&self, py: Python<'_>) -> PyResult<PyComputed> {
        let boundary = match self.corpus.computed_feature("boundary")? {
            Some(ComputedFeatureData::Boundary(boundary)) => boundary,
            _ => Boundary {
                first_slots: Vec::new(),
                last_slots: Vec::new(),
            },
        };
        Ok(PyComputed::new(Self::boundary_to_py(py, boundary)?))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn levUp(&self, py: Python<'_>) -> PyResult<PyComputed> {
        self.lev_view(py, false)
    }

    #[getter]
    #[allow(non_snake_case)]
    fn levDown(&self, py: Python<'_>) -> PyResult<PyComputed> {
        self.lev_view(py, true)
    }

    #[getter]
    fn sections(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(PyComputed::new(self.sections_to_py(py)?))
    }

    #[getter]
    fn characters(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(PyComputed::new(self.characters_to_py(py)?))
    }

    fn __dir__(&self) -> Vec<&'static str> {
        vec![
            "levels",
            "order",
            "rank",
            "boundary",
            "levUp",
            "levDown",
            "sections",
            "characters",
        ]
    }

    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        self.get(py, name)
    }
}
