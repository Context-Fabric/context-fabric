//! The `L` API: `Locality` (embedders, embeddees, neighbours, intersectors).

use std::sync::{Arc, OnceLock};

use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{
    NodeRange, as_str_refs, node_list, node_number, node_types_from_py, nodes_to_tuple,
};
use crate::mapped::{MappedCompiledCorpus, MappedSections, SectionsContext};

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
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
        let nodes = node_list(nodes, NodeRange::Lenient)?;
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
        let nodes = node_list(nodes, NodeRange::Lenient)?;
        let types = node_types_from_py(otype)?;
        let refs = as_str_refs(&types);
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
