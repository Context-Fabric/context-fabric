//! The `N`, `L`, `T` and `S` APIs: nodes, locality, text and search.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyIterator, PyList, PySet, PyString, PyTuple};

use super::api_ref::ApiRef;
use super::features::feature_value_from_py;
use super::node_arg::{node_number, optional_node_number};
use super::timestamp::{Silent, write_console};
use crate::config::DEFAULT_FORMAT;
use crate::index::{StructureHeading, StructureTree};
use crate::mapped::{
    MappedCompiledCorpus, MappedSections, SectionOptions, SectionsContext, TextOptions,
    TextRenderer, WalkEvent,
};
use crate::search::{MappedSearch, SEARCH_FAIL_FACTOR, SearchSets};
use crate::value::FeatureValue;

fn nodes_to_tuple(py: Python<'_>, nodes: Vec<u32>) -> PyResult<PyObject> {
    Ok(PyTuple::new(py, nodes)?.into())
}

fn nodes_from_py(items: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Vec<u32>>> {
    let Some(items) = items else {
        return Ok(None);
    };
    if items.is_none() {
        return Ok(None);
    }
    let iter = PyIterator::from_object(items)?;
    iter.map(|item| item?.extract::<u32>())
        .collect::<PyResult<Vec<_>>>()
        .map(Some)
}

fn type_filter_from_py(value: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Vec<String>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(text) = value.extract::<String>() {
        return Ok(Some(vec![text]));
    }
    let iter = PyIterator::from_object(value)?;
    iter.map(|item| item?.extract::<String>())
        .collect::<PyResult<Vec<_>>>()
        .map(Some)
}

/// Convert the Python `sets=` argument (a dict mapping a set name to an
/// iterable of node ids) into owned `(name, nodes)` pairs. The engine's
/// `SearchSets` borrows the names, so the owned storage must outlive the call.
fn search_sets_from_py(
    sets: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<(String, Vec<u32>)>>> {
    let Some(sets) = sets else {
        return Ok(None);
    };
    if sets.is_none() {
        return Ok(None);
    }
    let dict = sets.downcast::<PyDict>().map_err(|_| {
        PyTypeError::new_err("sets must be a dict mapping set names to iterables of node ids")
    })?;
    let mut owned = Vec::with_capacity(dict.len());
    for (key, value) in dict.iter() {
        let name = key
            .extract::<String>()
            .map_err(|_| PyTypeError::new_err("sets keys must be strings (custom set names)"))?;
        let nodes = PyIterator::from_object(&value)?
            .map(|item| item?.extract::<u32>())
            .collect::<PyResult<Vec<_>>>()?;
        owned.push((name, nodes));
    }
    Ok(Some(owned))
}

/// Borrow owned set storage as the engine's `SearchSets` view.
fn borrow_search_sets(owned: &Option<Vec<(String, Vec<u32>)>>) -> Option<SearchSets<'_>> {
    owned.as_ref().map(|pairs| {
        pairs
            .iter()
            .map(|(name, nodes)| (name.as_str(), nodes.clone()))
            .collect::<HashMap<_, _>>()
    })
}

/// The Python `shallow=` argument, normalized to TF semantics
/// (`tf/search/searchexe.py:55`): `0 if not shallow else 1 if shallow is True
/// else shallow`. Holds the projection width: 0 = full tuples, 1 = first
/// component only, k>1 = k-prefix tuples.
#[derive(Clone, Copy)]
struct Shallow(usize);

impl Shallow {
    /// `shallow=False`: full result tuples.
    const FULL: Self = Self(0);
}

impl<'py> FromPyObject<'py> for Shallow {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.is_none() {
            return Ok(Self::FULL);
        }
        // `bool` is a subclass of `int`, so it must be checked before integers.
        if let Ok(flag) = obj.downcast::<PyBool>() {
            return Ok(Self(usize::from(flag.is_true())));
        }
        if let Ok(value) = obj.extract::<i64>() {
            return Ok(Self(if value <= 0 { 0 } else { value as usize }));
        }
        Err(PyTypeError::new_err(
            "shallow must be a bool, an int, or None",
        ))
    }
}

fn type_filter_refs(values: &Option<Vec<String>>) -> Option<Vec<&str>> {
    values
        .as_ref()
        .map(|values| values.iter().map(String::as_str).collect())
}

fn feature_value_to_py(py: Python<'_>, value: &FeatureValue) -> PyResult<PyObject> {
    match value {
        FeatureValue::Str(value) => Ok(PyString::new(py, value).into()),
        FeatureValue::Int(value) => Ok(value.into_pyobject(py)?.into()),
    }
}

fn option_feature_value_to_py(py: Python<'_>, value: Option<&FeatureValue>) -> PyResult<PyObject> {
    match value {
        Some(value) => feature_value_to_py(py, value),
        None => Ok(py.None()),
    }
}

/// TF `T.structure` shape: a node is `(node, (child, ...))` and the whole
/// corpus a tuple of those, all tuples.
fn structure_tree_to_py(py: Python<'_>, tree: &StructureTree) -> PyResult<PyObject> {
    let children_to_py = |children: &[StructureTree]| -> PyResult<PyObject> {
        let items = children
            .iter()
            .map(|child| structure_tree_to_py(py, child))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyTuple::new(py, items)?.into())
    };
    match tree {
        StructureTree::Forest(children) => children_to_py(children),
        StructureTree::Node { node, children } => {
            let row: Vec<PyObject> =
                vec![node.into_pyobject(py)?.into(), children_to_py(children)?];
            Ok(PyTuple::new(py, row)?.into())
        }
    }
}

pub(crate) fn heading_to_py(py: Python<'_>, heading: &[StructureHeading]) -> PyResult<PyObject> {
    let rows = heading
        .iter()
        .map(|item| {
            let row: Vec<PyObject> = vec![
                PyString::new(py, &item.node_type).into(),
                option_feature_value_to_py(py, item.heading.as_ref())?,
            ];
            PyTuple::new(py, row).map(Into::into)
        })
        .collect::<PyResult<Vec<PyObject>>>()?;
    Ok(PyTuple::new(py, rows)?.into())
}

/// A heading as passed to `T.nodeFromHeading`: `((nodeType, value), ...)` where a
/// value is an int, a str, or `None`. Values keep their type, so (as in TF) a
/// str `"1"` does not find a heading whose feature value is the int `1`.
fn heading_from_py(items: &Bound<'_, PyAny>) -> PyResult<Vec<StructureHeading>> {
    let rows = items.extract::<Vec<(String, Bound<'_, PyAny>)>>()?;
    rows.into_iter()
        .map(|(node_type, heading)| {
            let heading = if heading.is_none() {
                None
            } else {
                Some(feature_value_from_py(&heading)?)
            };
            Ok(StructureHeading { node_type, heading })
        })
        .collect()
}

fn section_from_py(items: &Bound<'_, PyAny>) -> PyResult<Vec<FeatureValue>> {
    let values = items.extract::<Vec<Bound<'_, PyAny>>>()?;
    values
        .iter()
        .map(feature_value_from_py)
        .collect::<PyResult<Vec<_>>>()
}

/// Locality API (TF `L`): embedders, embeddees, and neighbours of nodes.
#[pyclass(name = "Locality", module = "cfabric._core")]
pub(crate) struct PyLocality {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Fully-resolved sections context, built once on first locality call. After
    /// that, `L.u/d/n/p/i` are pure CSR-slice + otype-filter + tuple-build with no
    /// per-call view-cache lookup or `otext` re-parse.
    context: OnceLock<SectionsContext>,
}

impl PyLocality {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self {
            corpus,
            api,
            context: OnceLock::new(),
        }
    }

    fn context(&self) -> PyResult<&SectionsContext> {
        if let Some(context) = self.context.get() {
            return Ok(context);
        }
        let context = SectionsContext::new(&self.corpus)?;
        let _ = self.context.set(context);
        Ok(self
            .context
            .get()
            .expect("sections context just initialized"))
    }
}

#[pymethods]
impl PyLocality {
    /// The API (`Corpus`) this member belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// Embedders of node `n` (nodes whose slots contain those of `n`), optionally
    /// restricted to node type(s) `otype`.
    #[pyo3(signature = (n, otype=None))]
    fn u(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node = n;
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let context = self.context()?;
        if refs.is_none()
            && let Some(embedders) = context.embedders(node)?
        {
            return Ok(PyTuple::new(py, embedders)?.into());
        }
        let sections = context.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.u(node, Some(single))?,
            Some(values) => sections.up_types(node, Some(values))?,
            None => sections.u(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    /// Embeddees of node `n` (nodes whose slots are contained in those of `n`),
    /// optionally restricted to node type(s) `otype`.
    #[pyo3(signature = (n, otype=None))]
    fn d(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node = n;
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.d(node, Some(single))?,
            Some(values) => sections.down_types(node, Some(values))?,
            None => sections.d(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    /// Nodes that start right after node `n` ends, optionally restricted to node
    /// type(s) `otype`.
    #[pyo3(signature = (n, otype=None))]
    fn n(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node = n;
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.n(node, Some(single))?,
            Some(values) => sections.next_types(node, Some(values))?,
            None => sections.n(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    /// Nodes that end right before node `n` starts, optionally restricted to node
    /// type(s) `otype`.
    #[pyo3(signature = (n, otype=None))]
    fn p(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node = n;
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.p(node, Some(single))?,
            Some(values) => sections.previous_types(node, Some(values))?,
            None => sections.p(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    /// TF `L.i`: the nodes that share at least one slot with node `n`, as a
    /// *list* in canonical order, optionally restricted to node type(s)
    /// `otype`; a slot or out-of-range node gives the empty *tuple*
    /// (`tf/core/locality.py`).
    #[pyo3(signature = (n, otype=None))]
    fn i(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let node = n;
        if node <= self.corpus.max_slot()? || node > self.corpus.max_node() {
            return Ok(PyTuple::empty(py).into());
        }
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.i(node, Some(single))?,
            Some(values) => sections.intersecting_types(node, Some(values))?,
            None => sections.i(node, None)?,
        };
        Ok(PyList::new(py, nodes)?.into())
    }

    /// Batch embedder lookup (ADDITIVE; W4): like `u` applied to each node in
    /// `nodes`, but the `MappedSections` engine is built once and the whole loop
    /// runs in Rust, so there is a single Python<->Rust boundary crossing for the
    /// batch. Returns a `list` of tuples, one per input node, in input order.
    #[pyo3(signature = (nodes, otype=None))]
    fn u_many(
        &self,
        py: Python<'_>,
        nodes: &Bound<'_, PyAny>,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let nodes = super::features::collect_nodes(nodes)?;
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = MappedSections::new(&self.corpus)?;
        let rows = nodes
            .iter()
            .map(|&node| {
                let result = match refs.as_deref() {
                    Some([single]) => sections.u(node, Some(single))?,
                    Some(values) => sections.up_types(node, Some(values))?,
                    None => sections.u(node, None)?,
                };
                PyTuple::new(py, result).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyList::new(py, rows)?.into())
    }

    /// Batch embeddee lookup (ADDITIVE; W4): the `d` counterpart of
    /// [`u_many`](Self::u_many). One boundary crossing for the whole batch.
    #[pyo3(signature = (nodes, otype=None))]
    fn d_many(
        &self,
        py: Python<'_>,
        nodes: &Bound<'_, PyAny>,
        otype: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let nodes = super::features::collect_nodes(nodes)?;
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = MappedSections::new(&self.corpus)?;
        let rows = nodes
            .iter()
            .map(|&node| {
                let result = match refs.as_deref() {
                    Some([single]) => sections.d(node, Some(single))?,
                    Some(values) => sections.down_types(node, Some(values))?,
                    None => sections.d(node, None)?,
                };
                PyTuple::new(py, result).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyList::new(py, rows)?.into())
    }
}

/// Node API (TF `N`): canonical node order and walking.
#[pyclass(name = "Nodes", module = "cfabric._core")]
pub(crate) struct PyNodes {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
}

impl PyNodes {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self { corpus, api }
    }
}

#[pymethods]
impl PyNodes {
    /// The API (`Corpus`) this member belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// Rank of each node type: `0` for the biggest type, up to the slot type.
    #[getter]
    #[allow(non_snake_case)]
    fn otypeRank(&self) -> PyResult<std::collections::BTreeMap<String, u32>> {
        Ok(self.corpus.otype_rank()?)
    }

    /// Sort key of node `n` in the canonical order (`sorted(nodes, key=N.sortKey)`).
    #[allow(non_snake_case)]
    fn sortKey(&self, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<Option<u32>> {
        Ok(self.corpus.sort_key(n)?)
    }

    /// Sort key of a tuple of nodes: the tuple of their sort keys.
    #[allow(non_snake_case)]
    fn sortKeyTuple(&self, py: Python<'_>, tup: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let nodes = nodes_from_py(Some(tup))?.unwrap_or_default();
        Ok(PyTuple::new(py, self.corpus.sort_key_tuple(&nodes)?)?.into())
    }

    /// Sort key of a chunk `(node, (first_slot, last_slot))` by position.
    #[allow(non_snake_case)]
    fn sortKeyChunk(
        &self,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<super::features::PyChunkPositionKey> {
        super::features::sort_key_chunk_py(&self.corpus, obj)
    }

    /// Sort key of a chunk `(node, (first_slot, last_slot))` by length, then position.
    #[allow(non_snake_case)]
    fn sortKeyChunkLength(
        &self,
        obj: &Bound<'_, PyAny>,
    ) -> PyResult<super::features::PyChunkLengthKey> {
        super::features::sort_key_chunk_length_py(&self.corpus, obj)
    }

    /// The chunk sort keys as a pair `(sortKeyChunk, sortKeyChunkLength)`.
    #[allow(non_snake_case)]
    fn makeSortKeyChunk(slf: &Bound<'_, Self>) -> PyResult<Py<PyTuple>> {
        Ok(PyTuple::new(
            slf.py(),
            [
                slf.getattr("sortKeyChunk")?,
                slf.getattr("sortKeyChunkLength")?,
            ],
        )?
        .unbind())
    }

    /// The nodes of `nodeSet` in canonical order, as a list.
    #[allow(non_snake_case)]
    fn sortNodes(&self, nodeSet: &Bound<'_, PyAny>) -> PyResult<Vec<u32>> {
        let mut nodes = nodes_from_py(Some(nodeSet))?.unwrap_or_default();
        self.corpus.sort_nodes(&mut nodes)?;
        Ok(nodes)
    }

    /// All nodes (or `nodes`) in canonical order; with `events=True`, as
    /// `(node, None)` for slots, `(node, False)` for container starts and
    /// `(node, True)` for container ends.
    #[pyo3(signature = (nodes=None, events=false))]
    fn walk(
        &self,
        py: Python<'_>,
        nodes: Option<&Bound<'_, PyAny>>,
        events: bool,
    ) -> PyResult<PyObject> {
        // TF `N.walk` is a generator; return an iterator so `next(N.walk())`
        // works and the result is consumed once, as in TF.
        let nodes = nodes_from_py(nodes)?;
        if !events
            && nodes.is_none()
            && let Some(order) = self.corpus.canonical_order()?
        {
            return Ok(PyTuple::new(py, order)?.try_iter()?.into_any().unbind());
        }
        let sections = MappedSections::new(&self.corpus)?;
        if !events {
            let walk = PyTuple::new(py, sections.walk(nodes.as_deref())?)?;
            return Ok(walk.try_iter()?.into_any().unbind());
        }
        // TF `N.walk(events=True)` (tf/core/nodes.py:278-292) yields event pairs:
        // slot -> `(n, None)`, container start -> `(n, False)`, container end ->
        // `(n, True)`. `WalkEvent` already models exactly these three cases.
        let rows: Vec<(u32, Option<bool>)> = sections
            .walk_events(nodes.as_deref())?
            .into_iter()
            .map(|event| match event {
                WalkEvent::Slot(node) => (node, None),
                WalkEvent::Start(node) => (node, Some(false)),
                WalkEvent::End(node) => (node, Some(true)),
            })
            .collect();
        Ok(PyTuple::new(py, rows)?.try_iter()?.into_any().unbind())
    }
}

/// Text API (TF `T`): text formats, sections, and structure.
#[pyclass(name = "Text", module = "cfabric._core")]
pub(crate) struct PyText {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Fully-resolved sections context, built once on first section call. Powers
    /// `T.sectionFromNode`/`T.sectionTuple` with no per-call view-cache lookup or
    /// `otext` re-parse.
    sections: OnceLock<SectionsContext>,
    /// The text renderer (compiled formats and their feature handles), built
    /// on first use.
    text: OnceLock<TextRenderer>,
}

impl PyText {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self {
            corpus,
            api,
            sections: OnceLock::new(),
            text: OnceLock::new(),
        }
    }

    fn sections_context(&self) -> PyResult<&SectionsContext> {
        if let Some(context) = self.sections.get() {
            return Ok(context);
        }
        let context = SectionsContext::new(&self.corpus)?;
        let _ = self.sections.set(context);
        Ok(self
            .sections
            .get()
            .expect("sections context just initialized"))
    }

    fn renderer(&self) -> PyResult<&TextRenderer> {
        if let Some(renderer) = self.text.get() {
            return Ok(renderer);
        }
        let renderer = TextRenderer::new(&self.corpus)?;
        Ok(self.text.get_or_init(|| renderer))
    }
}

#[pymethods]
impl PyText {
    /// The API (`Corpus`) this member belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// `T.<sec0>Name` / `T.<sec0>Node`, e.g. `T.bookName(node)` and
    /// `T.bookNode(name)` when the first section type is `book`; TF attaches
    /// these under the name of the dataset's first section type.
    fn __getattr__<'py>(slf: &Bound<'py, Self>, name: &str) -> PyResult<Bound<'py, PyAny>> {
        if let Some(section_0_type) = slf.borrow().corpus.otext()?.section_types.first().cloned() {
            if name == format!("{section_0_type}Name") {
                return slf.getattr("_sec0Name");
            }
            if name == format!("{section_0_type}Node") {
                return slf.getattr("_sec0Node");
            }
        }
        Err(PyAttributeError::new_err(format!(
            "'Text' object has no attribute '{name}'"
        )))
    }

    fn __dir__(slf: &Bound<'_, Self>) -> PyResult<Vec<String>> {
        let mut names: Vec<String> = slf
            .get_type()
            .dir()?
            .iter()
            .map(|name| name.extract::<String>())
            .collect::<PyResult<_>>()?;
        if let Some(section_0_type) = slf.borrow().corpus.otext()?.section_types.first().cloned() {
            names.push(format!("{section_0_type}Name"));
            names.push(format!("{section_0_type}Node"));
        }
        Ok(names)
    }

    /// Name of the section-0 node containing node `n` (exposed as `T.<sec0>Name`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (n, lang="en"))]
    fn _sec0Name(
        &self,
        #[pyo3(from_py_with = node_number)] n: u32,
        lang: &str,
    ) -> PyResult<Option<String>> {
        Ok(self.sections_context()?.section_0_name(n, lang)?)
    }

    /// The section-0 node with name `name` (exposed as `T.<sec0>Node`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (name, lang="en"))]
    fn _sec0Node(&self, name: &str, lang: &str) -> PyResult<Option<u32>> {
        Ok(self.sections_context()?.section_0_node(name, lang)?)
    }

    /// Whether the T-API is usable (always `True`).
    #[getter]
    fn good(&self) -> bool {
        true
    }

    /// The `otext` configuration (metadata of the `otext` feature).
    #[getter]
    fn config(&self) -> PyResult<BTreeMap<String, Option<String>>> {
        Ok(match self.corpus.config_feature("otext")? {
            Some(otext) => otext.metadata()?,
            None => BTreeMap::new(),
        })
    }

    /// The format used when none is specified and no node-type default applies.
    #[getter]
    #[allow(non_snake_case)]
    fn defaultFormat(&self) -> &'static str {
        DEFAULT_FORMAT
    }

    /// Default format per node type, from formats named `<type>-default`.
    #[getter]
    #[allow(non_snake_case)]
    fn defaultFormats(&self) -> PyResult<BTreeMap<String, String>> {
        let renderer = self.renderer()?;
        Ok(self
            .corpus
            .otext()?
            .formats
            .keys()
            .filter_map(|name| Some((renderer.split_default_format(name)?, name.clone())))
            .collect())
    }

    /// Section types, from big to small (e.g. `["book", "chapter", "verse"]`).
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypes(&self) -> PyResult<Vec<String>> {
        Ok(MappedSections::new(&self.corpus)?.section_types().to_vec())
    }

    /// The section types as a set.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypeSet(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        Ok(PySet::new(py, self.sectionTypes()?)?.unbind())
    }

    /// The section feature names, like `sectionFeats` (TF's `T.sectionFeatures`
    /// holds the features' data dicts instead; use `Fs(name).data` for those).
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeatures(&self) -> PyResult<Vec<String>> {
        self.sectionFeats()
    }

    /// Names of the section features, one per section type, as a list.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeats(&self) -> PyResult<Vec<String>> {
        Ok(MappedSections::new(&self.corpus)?
            .section_features()
            .to_vec())
    }

    /// The first section feature and its language variants (`book`, `book@en`, ...).
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeatsWithLanguage(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let mut names: Vec<String> = self
            .corpus
            .section_0_language_features()?
            .into_values()
            .collect();
        names.sort();
        Ok(PyTuple::new(py, names)?.unbind())
    }

    /// Text formats, mapped to the node type they render.
    #[getter]
    fn formats(&self) -> PyResult<std::collections::BTreeMap<String, String>> {
        Ok(self.renderer()?.formats()?)
    }

    /// Structure types, from big to small.
    #[getter]
    #[allow(non_snake_case)]
    fn structureTypes(&self) -> PyResult<Vec<String>> {
        Ok(self.corpus.otext()?.structure_types.clone())
    }

    /// The structure types as a set.
    #[getter]
    #[allow(non_snake_case)]
    fn structureTypeSet(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        Ok(PySet::new(py, self.structureTypes()?)?.unbind())
    }

    /// Names of the structure features, one per structure type.
    #[getter]
    #[allow(non_snake_case)]
    fn structureFeats(&self) -> PyResult<Vec<String>> {
        Ok(self.corpus.otext()?.structure_features.clone())
    }

    /// Languages of the section-0 feature, `{code: {language, languageEnglish}}`.
    #[getter]
    fn languages(
        &self,
    ) -> PyResult<std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>>
    {
        Ok(self.corpus.languages()?)
    }

    /// Print an overview of the structure configuration.
    #[allow(non_snake_case)]
    fn structureInfo(&self, py: Python<'_>) -> PyResult<()> {
        let info = self.corpus.structure_info()?;
        write_console(py, false, true, &info)
    }

    /// TF `T.headings`: the configured `(structure type, structure feature)`
    /// pairs as a tuple, `()` when no structure is configured.
    #[getter]
    fn headings(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.structure_headings()?)?.into())
    }

    /// Split a format template `type#template` into `(type, template)`; the
    /// type defaults to the slot type.
    #[allow(non_snake_case)]
    fn splitFormat(&self, tpl: &str) -> PyResult<(String, String)> {
        Ok(self.renderer()?.split_format(tpl))
    }

    /// The node type of a format named `<type>-default`, else `None`.
    #[allow(non_snake_case)]
    fn splitDefaultFormat(&self, tpl: &str) -> PyResult<Option<String>> {
        Ok(self.renderer()?.split_default_format(tpl))
    }

    /// The text of a node, or of an iterable of nodes (concatenated).
    ///
    /// Args:
    ///     nodes: A node or an iterable of nodes (rendered in the given order).
    ///     fmt: Text format; default: the node type's `<type>-default` format,
    ///         else `text-orig-full`.
    ///     descend: Whether to render the nodes of the format's target type
    ///         inside each node (default: descend unless the node type has its
    ///         own default format).
    ///     func: Custom renderer called as `func(n, **kwargs)` for every node
    ///         that would be rendered, instead of the format.
    ///     explain: Print how the format and descend rules were applied.
    ///     **kwargs: Passed on to `func`.
    #[pyo3(signature = (nodes, fmt=None, descend=None, func=None, explain=false, **kwargs))]
    fn text(
        &self,
        py: Python<'_>,
        nodes: &Bound<'_, PyAny>,
        fmt: Option<&str>,
        descend: Option<bool>,
        func: Option<&Bound<'_, PyAny>>,
        explain: bool,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        // TF `T.text` (tf/core/text.py:972) accepts a single int OR an arbitrary
        // iterable of node ids; iterables are rendered per node and the pieces are
        // concatenated with no separator (`"".join(material)`, text.py:1182).
        let renderer = self.renderer()?;
        let options = TextOptions::new(fmt.map(str::to_string), descend);
        let nodes = if nodes.is_instance_of::<PyInt>() {
            vec![node_number(nodes)?]
        } else {
            nodes_from_py(Some(nodes))?.unwrap_or_default()
        };
        if explain {
            let func_rep = if func.is_some() { "" } else { "no " };
            let message = format!(
                "EXPLANATION: T.text() called with parameters:\n\tnodes  : {nodes:?}\n\tfmt    : {}\n\tdescend: {}\n\tfunc   : {func_rep}custom format implementation",
                fmt.unwrap_or("implicit"),
                descend.map_or_else(|| "implicit".to_string(), |flag| flag.to_string()),
            );
            write_console(py, true, true, &message)?;
        }
        let mut material = String::new();
        for node in nodes {
            if let Some(func) = func {
                for text_node in renderer.rendered_nodes(node, &options)? {
                    if explain {
                        write_console(
                            py,
                            true,
                            true,
                            &format!("\tNODE {node}: render {text_node} with func"),
                        )?;
                    }
                    material.push_str(&func.call((text_node,), kwargs)?.str()?.to_string());
                }
                continue;
            }
            if !explain {
                renderer.render_into(node, &options, &mut material)?;
                continue;
            }
            let rendered = renderer.text(node, &options)?;
            {
                write_console(
                    py,
                    true,
                    true,
                    &format!(
                        "\tNODE {node}: renders {:?} as {rendered:?}",
                        renderer.rendered_nodes(node, &options)?
                    ),
                )?;
            }
            material.push_str(&rendered);
        }
        Ok(material)
    }

    /// Batch text rendering (ADDITIVE; W4). `items` is an iterable whose elements
    /// are each either a single node id (int) or an iterable of node ids; every
    /// element is rendered exactly as the scalar `text` would render it (an
    /// iterable element is rendered per-node and concatenated with no separator,
    /// matching TF `"".join(material)`). Returns a `list[str]`, one string per
    /// element, in input order.
    #[pyo3(signature = (items, fmt=None, descend=None))]
    fn text_many(
        &self,
        py: Python<'_>,
        items: &Bound<'_, PyAny>,
        fmt: Option<&str>,
        descend: Option<bool>,
    ) -> PyResult<PyObject> {
        let renderer = self.renderer()?;
        let options = TextOptions::new(fmt.map(str::to_string), descend);
        let iter = items.try_iter().map_err(|_| {
            PyTypeError::new_err("text_many expects an iterable of nodes or node-iterables")
        })?;
        let out = PyList::empty(py);
        let mut rendered = String::new();
        for item in iter {
            let item = item?;
            rendered.clear();
            if item.is_instance_of::<PyInt>() {
                renderer.render_into(node_number(&item)?, &options, &mut rendered)?;
            } else {
                for node in super::features::collect_nodes(&item)? {
                    renderer.render_into(node, &options, &mut rendered)?;
                }
            }
            out.append(PyString::new(py, &rendered))?;
        }
        Ok(out.into_any().unbind())
    }

    /// The section heading values (e.g. `("Genesis", 1, 1)`) of the sections
    /// containing node `n`.
    ///
    /// Args:
    ///     n: The node.
    ///     lastSlot: Use the last slot of `n` instead of the first.
    ///     lang: Language code for the section-0 name.
    ///     fillup: For section nodes, fill up the lower levels with their first
    ///         subsection.
    ///     level: Only return the headings up to this level.
    #[allow(non_snake_case)]
    #[pyo3(signature = (n, lastSlot=false, lang="en", fillup=false, level=None))]
    fn sectionFromNode(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        lastSlot: bool,
        lang: &str,
        fillup: bool,
        level: Option<usize>,
    ) -> PyResult<PyObject> {
        let node = n;
        let options = SectionOptions {
            last_slot: lastSlot,
            fillup,
            level,
        };
        let values = self
            .sections_context()?
            .section_heading(node, lang, &options)?;
        let items = values
            .iter()
            .map(|value| option_feature_value_to_py(py, value.as_ref()))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyTuple::new(py, items)?.into())
    }

    /// The section nodes containing node `n`, from big to small.
    ///
    /// Args:
    ///     n: The node.
    ///     lastSlot: Use the last slot of `n` instead of the first.
    ///     fillup: For section nodes, fill up the lower levels with their first
    ///         subsection.
    ///     level: Only return the sections up to this level.
    #[allow(non_snake_case)]
    #[pyo3(signature = (n, lastSlot=false, fillup=false, level=None))]
    fn sectionTuple(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        lastSlot: bool,
        fillup: bool,
        level: Option<usize>,
    ) -> PyResult<PyObject> {
        // TF `T.sectionTuple` (tf/core/text.py:461) returns the *section node ids*
        // (not their feature values) that contain `node`, honoring `lastSlot` and
        // `fillup`.
        let node = n;
        let nodes = self.sections_context()?.sections().section_tuple(
            node,
            &SectionOptions {
                last_slot: lastSlot,
                fillup,
                level,
            },
        )?;
        Ok(PyTuple::new(py, nodes)?.into())
    }

    /// The node of a section heading tuple (e.g. `("Genesis", 1, 1)`), or `None`.
    ///
    /// Args:
    ///     section: Heading values from big to small; may be shorter than the
    ///         number of section levels.
    ///     lang: Language code of the section-0 name.
    #[allow(non_snake_case)]
    #[pyo3(signature = (section, lang="en"))]
    fn nodeFromSection(&self, section: &Bound<'_, PyAny>, lang: &str) -> PyResult<Option<u32>> {
        let values = section_from_py(section)?;
        // Resolve the language-aware section-0 feature (e.g. `book@en`) and the
        // section-0 node through the cached sections context (lazily-built value
        // index, O(1)); chapter/verse come from the CFRSECT1 `sec1`/`sec2` maps.
        // A miss returns `None` immediately.
        let context = self.sections_context()?;
        let sec0_feature = context.sec0_feature_for_lang(lang);
        Ok(context.node_from_section_langed(&values, sec0_feature)?)
    }

    /// The structure heading of node `n`: `((type, heading), ...)`.
    #[allow(non_snake_case)]
    fn headingFromNode(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .heading_from_node(n)?
            .map(|heading| heading_to_py(py, &heading))
            .transpose()
    }

    /// The node of a structure heading `((type, heading), ...)`, or `None`.
    #[allow(non_snake_case)]
    fn nodeFromHeading(&self, head: &Bound<'_, PyAny>) -> PyResult<Option<u32>> {
        let heading = heading_from_py(head)?;
        Ok(self.corpus.node_from_heading(&heading)?)
    }

    /// The structure tree of `node` (or of the whole dataset) as nested tuples.
    #[pyo3(signature = (node=None))]
    fn structure(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = optional_node_number)] node: Option<u32>,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .structure(node)?
            .map(|tree| structure_tree_to_py(py, &tree))
            .transpose()
    }

    /// The top-level structure nodes.
    fn top(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        Ok(self
            .corpus
            .top()?
            .map(|nodes| nodes_to_tuple(py, nodes))
            .transpose()?)
    }

    /// TF `T.up`: the parent structural node of node `n`, or `None` without
    /// structure or when `n` is not of a structure type.
    fn up(&self, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<Option<u32>> {
        Ok(self.corpus.structure_up(n)?)
    }

    /// TF `T.down`: the tuple of structural children of node `n`, or `None`
    /// without structure or when `n` is not of a structure type.
    fn down(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .structure_down(n)?
            .map(|nodes| nodes_to_tuple(py, nodes))
            .transpose()
    }

    /// The structure of `node` (or of the whole dataset) as an indented string.
    #[allow(non_snake_case)]
    #[pyo3(signature = (node=None, fullHeading=false))]
    fn structurePretty(
        &self,
        #[pyo3(from_py_with = optional_node_number)] node: Option<u32>,
        fullHeading: bool,
    ) -> PyResult<Option<String>> {
        Ok(self.corpus.structure_pretty(node, fullHeading)?)
    }
}

/// Search API (TF `S`): template search.
#[pyclass(name = "Search", module = "cfabric._core")]
pub(crate) struct PySearch {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Verbosity of the Fabric when the API was made (TF `S.silent`).
    silent: &'static str,
    exe: Option<PySearchExe>,
    template: Option<String>,
    /// Owned custom sets registered via `study(..., sets=...)`, honored by
    /// later `fetch`/`count` calls (TF stashes `sets` on the SearchExe).
    sets: Option<Vec<(String, Vec<u32>)>>,
    /// Projection width registered via `study(..., shallow=...)` (TF semantics).
    shallow: usize,
}

impl PySearch {
    pub(crate) fn new(
        corpus: Arc<MappedCompiledCorpus>,
        api: ApiRef,
        silent: &'static str,
    ) -> Self {
        Self {
            corpus,
            api,
            silent,
            exe: None,
            template: None,
            sets: None,
            shallow: 0,
        }
    }

    /// Counts results of the studied `template` (GIL-free engine work).
    fn count_studied(&self, template: &str, limit: Option<usize>) -> crate::error::Result<usize> {
        let engine = MappedSearch::new(&self.corpus)?;
        // Full-tuple counting runs the join in count-only mode (no row
        // materialization); shallow counting needs the rows for dedup.
        if self.shallow == 0 {
            return match borrow_search_sets(&self.sets) {
                Some(sets) => engine.count_with_sets(template, &sets, limit),
                None => engine.count(template, limit),
            };
        }
        let study = match borrow_search_sets(&self.sets) {
            Some(sets) => engine.study_with_sets(template, &sets)?,
            None => engine.study(template)?,
        };
        Ok(match self.shallow {
            1 => study.count_first_nodes(limit),
            width => study.count_prefixes(width, limit),
        })
    }

    /// TF's result limit (`tf/search/searchexe.py::fetch`/`count`): a negative
    /// limit counts as 0, and 0 means "no limit" up to the fail limit of
    /// `SEARCH_FAIL_FACTOR * maxNode` results.
    fn tf_limit(&self, limit: Option<i64>) -> Option<usize> {
        limit.map(|limit| match usize::try_from(limit) {
            Ok(limit) if limit > 0 => limit,
            _ => SEARCH_FAIL_FACTOR as usize * self.corpus.max_node() as usize,
        })
    }

    /// Run a search and shape the result per TF `shallow` semantics
    /// (`tf/search/stitch.py:872-879`): width 0 -> tuple of full tuples,
    /// width 1 -> set of first components, width k>1 -> set of k-prefix tuples.
    fn run_search(
        &self,
        py: Python<'_>,
        template: &str,
        limit: Option<usize>,
        owned_sets: &Option<Vec<(String, Vec<u32>)>>,
        shallow: usize,
    ) -> PyResult<PyObject> {
        let engine = MappedSearch::new(&self.corpus)?;
        let sets = borrow_search_sets(owned_sets);
        // The engine work runs without the GIL; only building the Python
        // result objects needs it.
        match shallow {
            0 => {
                let rows = py.allow_threads(|| match sets.as_ref() {
                    Some(sets) => engine.search_with_sets(template, sets, limit),
                    None => engine.search(template, limit),
                })?;
                let tuples = rows
                    .into_iter()
                    .map(|row| PyTuple::new(py, row).map(Into::into))
                    .collect::<PyResult<Vec<PyObject>>>()?;
                Ok(PyTuple::new(py, tuples)?.into())
            }
            1 => {
                let nodes = py.allow_threads(|| match sets.as_ref() {
                    Some(sets) => engine.search_first_nodes_with_sets(template, sets, limit),
                    None => engine.search_first_nodes(template, limit),
                })?;
                Ok(PySet::new(py, &nodes)?.into())
            }
            width => {
                let rows = py.allow_threads(|| match sets.as_ref() {
                    Some(sets) => engine.search_prefixes_with_sets(template, sets, width, limit),
                    None => engine.search_prefixes(template, width, limit),
                })?;
                let tuples = rows
                    .into_iter()
                    .map(|row| PyTuple::new(py, row).map(Into::into))
                    .collect::<PyResult<Vec<PyObject>>>()?;
                Ok(PySet::new(py, &tuples)?.into())
            }
        }
    }
}

#[pymethods]
impl PySearch {
    /// The API (`Corpus`) this member belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// Verbosity of the Fabric when the API was made.
    #[getter]
    fn silent(&self) -> &'static str {
        self.silent
    }

    /// Outcome of the last `study`: `exe.good`, `exe.badSyntax`, `exe.badSemantics`.
    #[getter]
    fn exe(&self) -> Option<PySearchExe> {
        self.exe.clone()
    }

    /// Check a search template and prepare it for `fetch` / `count` / `showPlan`.
    ///
    /// Args:
    ///     searchTemplate: The search template.
    ///     strategy: Accepted for TF compatibility; the planner chooses the strategy.
    ///     sets: Custom sets, `{name: iterable of nodes}`, usable as node types.
    ///     shallow: `False`: full result tuples; `True`: the set of first nodes;
    ///         `k > 1`: the set of `k`-prefixes of the result tuples.
    ///     here: Accepted for TF compatibility (no display side channel).
    ///     silent: Verbosity for this call.
    #[allow(non_snake_case)]
    #[pyo3(
        signature = (searchTemplate, strategy=None, sets=None, shallow=Shallow::FULL, here=true, silent=Silent::DEFAULT),
        text_signature = "($self, searchTemplate, strategy=None, sets=None, shallow=False, here=True, silent='auto')"
    )]
    fn study(
        &mut self,
        py: Python<'_>,
        searchTemplate: &str,
        strategy: Option<&Bound<'_, PyAny>>,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: Shallow,
        here: bool,
        silent: Silent,
    ) -> PyResult<()> {
        let _ = (strategy, here);
        let _ = silent;
        let owned_sets = search_sets_from_py(sets)?;
        let engine = MappedSearch::new(&self.corpus)?;
        let result = py.allow_threads(|| match borrow_search_sets(&owned_sets) {
            Some(sets) => engine.search_with_sets(searchTemplate, &sets, Some(1)),
            None => engine.search(searchTemplate, Some(1)),
        });
        self.template = Some(searchTemplate.to_string());
        self.sets = owned_sets;
        self.shallow = shallow.0;
        self.exe = Some(match result {
            Ok(_) => PySearchExe::good(),
            Err(error) => PySearchExe::bad(error.to_string()),
        });
        Ok(())
    }

    /// Search for all matches of a template.
    ///
    /// Args:
    ///     searchTemplate: The search template.
    ///     limit: Return at most this many results.
    ///     sets: Custom sets, `{name: iterable of nodes}`, usable as node types.
    ///     shallow: `False`: full result tuples; `True`: the set of first nodes;
    ///         `k > 1`: the set of `k`-prefixes of the result tuples.
    ///     silent: Verbosity for this call.
    ///     here: Accepted for TF compatibility (no display side channel).
    ///
    /// Returns:
    ///     A tuple of result tuples, or a set when `shallow` is set.
    #[allow(non_snake_case)]
    #[pyo3(
        signature = (searchTemplate, limit=None, sets=None, shallow=Shallow::FULL, silent=Silent::DEFAULT, here=true),
        text_signature = "($self, searchTemplate, limit=None, sets=None, shallow=False, silent='auto', here=True)"
    )]
    fn search(
        &self,
        py: Python<'_>,
        searchTemplate: &str,
        limit: Option<i64>,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: Shallow,
        silent: Silent,
        here: bool,
    ) -> PyResult<PyObject> {
        let _ = here;
        let _ = silent;
        let owned_sets = search_sets_from_py(sets)?;
        let limit = self.tf_limit(limit);
        self.run_search(py, searchTemplate, limit, &owned_sets, shallow.0)
    }

    /// Fetch the results of the template prepared by `study`.
    ///
    /// Args:
    ///     limit: Return at most this many results.
    #[pyo3(signature = (limit=None))]
    fn fetch(&self, py: Python<'_>, limit: Option<i64>) -> PyResult<PyObject> {
        let template = self.template.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        self.run_search(
            py,
            &template,
            self.tf_limit(limit),
            &self.sets,
            self.shallow,
        )
    }

    /// Count the results of the template prepared by `study`.
    ///
    /// Args:
    ///     progress: Accepted for TF compatibility (no progress reporting).
    ///     limit: Stop counting at this many results.
    #[pyo3(signature = (progress=None, limit=None))]
    fn count(
        &self,
        py: Python<'_>,
        progress: Option<usize>,
        limit: Option<i64>,
    ) -> PyResult<usize> {
        let _ = progress;
        let limit = self.tf_limit(limit);
        let template = self.template.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        Ok(py.allow_threads(|| self.count_studied(template, limit))?)
    }

    /// Print the search plan of the template prepared by `study`.
    ///
    /// Args:
    ///     details: Include per-step details.
    #[allow(non_snake_case)]
    #[pyo3(signature = (details=false))]
    fn showPlan(&self, py: Python<'_>, details: bool) -> PyResult<()> {
        // Emit the engine's real study/plan summary rather than echoing the
        // template (TF `S.showPlan`, tf/search/search.py).
        let Some(template) = self.template.as_deref() else {
            return write_console(
                py,
                true,
                true,
                "Cannot show plan if there is no previous \"study()\"",
            );
        };
        let engine = MappedSearch::new(&self.corpus)?;
        let plan = match borrow_search_sets(&self.sets) {
            Some(sets) => engine.show_plan_with_sets(template, &sets, details)?,
            None => engine.show_plan(template, details)?,
        };
        write_console(py, false, true, &plan)
    }

    /// Render a single result tuple into a human-readable string per TF
    /// `S.glean` (tf/search/search.py:475-542): for each node, a level-2 (verse)
    /// section node becomes `"book ch:vs"`, a slot becomes its text, a level-0/1
    /// section node becomes empty, and any other node becomes
    /// `"otype[<text of first 5 slots>...]"`. Fields are joined with a space.
    fn glean(&self, tup: &Bound<'_, PyAny>) -> PyResult<String> {
        let nodes = nodes_from_py(Some(tup))?.unwrap_or_default();
        if nodes.is_empty() {
            return Ok(String::new());
        }
        let context = SectionsContext::new(&self.corpus)?;
        let sections = context.sections();
        let renderer = TextRenderer::new(&self.corpus)?;
        let default_text = TextOptions::default();
        let section_types = sections.section_types().to_vec();
        let level2_type = section_types.get(2).cloned();
        let upper_section_types: std::collections::HashSet<&str> =
            section_types.iter().take(2).map(String::as_str).collect();

        let value_to_string = |value: &FeatureValue| -> String {
            match value {
                FeatureValue::Str(value) => value.to_string(),
                FeatureValue::Int(value) => value.to_string(),
            }
        };

        let mut fields = Vec::with_capacity(nodes.len());
        for node in nodes {
            let otype = sections
                .node_type(node)?
                .map(str::to_string)
                .unwrap_or_default();
            let field = if level2_type.as_deref() == Some(otype.as_str()) {
                let heading = context.section_heading(node, "en", &SectionOptions::default())?;
                let part = |index: usize| {
                    heading
                        .get(index)
                        .and_then(Option::as_ref)
                        .map(&value_to_string)
                        .unwrap_or_default()
                };
                format!("{} {}:{}", part(0), part(1), part(2))
            } else if sections.is_slot(node)? {
                renderer.text(node, &default_text)?
            } else if upper_section_types.contains(otype.as_str()) {
                String::new()
            } else {
                let words = sections.slots(node)?;
                let head: Vec<u32> = words.iter().take(5).copied().collect();
                let ellipsis = if words.len() > 5 { "..." } else { "" };
                let mut text = String::new();
                for word in head {
                    renderer.render_into(word, &default_text, &mut text)?;
                }
                format!("{otype}[{text}{ellipsis}]")
            };
            fields.push(field);
        }
        Ok(fields.join(" "))
    }

    /// Accepted for TF compatibility: the planner has no tunable parameters.
    ///
    /// Args:
    ///     silent: Verbosity for this call.
    ///     **kwargs: TF performance parameters (ignored).
    #[allow(non_snake_case)]
    #[pyo3(signature = (silent=Silent::DEFAULT, **kwargs), text_signature = "($self, silent='auto', **kwargs)")]
    fn tweakPerformance(&self, silent: Silent, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let _ = kwargs;
        let _ = silent;
        Ok(())
    }

    /// Print the legend of the relational operators usable in search templates.
    #[allow(non_snake_case)]
    fn relationsLegend(&self, py: Python<'_>) -> PyResult<()> {
        write_console(py, false, true, crate::search::RELATIONS_LEGEND)
    }

    /// Performance parameters (empty: the planner has no tunable parameters).
    #[getter]
    #[allow(non_snake_case)]
    fn perfParams(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyDict::new(py).into())
    }
}

/// Outcome of `S.study`: whether the template is good, with syntax/semantic errors.
#[pyclass(name = "SearchExe", module = "cfabric._core")]
#[derive(Clone)]
pub(crate) struct PySearchExe {
    /// Whether the template is valid.
    #[pyo3(get)]
    good: bool,
    /// Syntax errors as `(line, message)` pairs.
    #[pyo3(get, name = "badSyntax")]
    bad_syntax: Vec<(Option<usize>, String)>,
    /// Semantic errors as `(line, message)` pairs.
    #[pyo3(get, name = "badSemantics")]
    bad_semantics: Vec<(Option<usize>, String)>,
}

impl PySearchExe {
    fn good() -> Self {
        Self {
            good: true,
            bad_syntax: Vec::new(),
            bad_semantics: Vec::new(),
        }
    }

    fn bad(message: String) -> Self {
        Self {
            good: false,
            bad_syntax: vec![(None, message)],
            bad_semantics: Vec::new(),
        }
    }
}
