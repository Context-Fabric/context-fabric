//! The `N` API: `Nodes` (canonical order, walking) and the chunk sort keys.

use std::sync::Arc;

use pyo3::basic::CompareOp;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{NodeRange, node_list, node_number};
use crate::mapped::{
    Chunk, ChunkLengthKey, ChunkPositionKey, MappedCompiledCorpus, MappedSections, WalkEvent,
};

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

    /// Ordering calls take only the corpus's nodes: there is no place in the
    /// canonical order for a missing one (TF raises `IndexError` too).
    fn node_range(&self) -> NodeRange {
        NodeRange::Strict {
            max_node: self.corpus.max_node(),
        }
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
        let nodes = node_list(tup, NodeRange::Lenient)?;
        Ok(PyTuple::new(py, self.corpus.sort_key_tuple(&nodes)?)?.into())
    }

    /// Sort key of a chunk `(node, (first_slot, last_slot))` by position.
    #[allow(non_snake_case)]
    fn sortKeyChunk(&self, obj: &Bound<'_, PyAny>) -> PyResult<PyChunkPositionKey> {
        sort_key_chunk_py(&self.corpus, obj)
    }

    /// Sort key of a chunk `(node, (first_slot, last_slot))` by length, then position.
    #[allow(non_snake_case)]
    fn sortKeyChunkLength(&self, obj: &Bound<'_, PyAny>) -> PyResult<PyChunkLengthKey> {
        sort_key_chunk_length_py(&self.corpus, obj)
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
    ///
    /// Raises:
    ///     IndexError: For a number that is not a node of the corpus.
    #[allow(non_snake_case)]
    fn sortNodes(&self, nodeSet: &Bound<'_, PyAny>) -> PyResult<Vec<u32>> {
        let mut nodes = node_list(nodeSet, self.node_range())?;
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
        let nodes = nodes
            .filter(|nodes| !nodes.is_none())
            .map(|nodes| node_list(nodes, self.node_range()))
            .transpose()?;
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
#[pyclass(name = "ChunkPositionKey", module = "cfabric._core")]
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
#[pyclass(name = "ChunkLengthKey", module = "cfabric._core")]
pub(crate) struct PyChunkLengthKey {
    key: ChunkLengthKey,
}

#[pymethods]
impl PyChunkLengthKey {
    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        op.matches(self.key.cmp(&other.key))
    }
}
