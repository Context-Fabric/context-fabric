//! `SearchResultIterator`: the lazy results of `S.search` / `S.fetch`
//! without a limit (Text-Fabric returns a generator).

use std::collections::VecDeque;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use crate::bindings::signals::interruptible;
use crate::mapped::MappedCompiledCorpus;
use crate::search::{MappedSearch, PreparedSearch};

/// Matches produced per engine call: bounds the memory of a read in progress
/// and amortizes releasing the GIL.
const CHUNK_ROWS: usize = 4096;

/// The cut-off warning of an unlimited search
/// (`tf/search/searchexe.py::fetch` / `count`).
pub(crate) fn warn_cut_off(py: Python<'_>, fail_limit: usize) -> PyResult<()> {
    let message = format!("cut off at {fail_limit} results. There are more ...");
    let message = std::ffi::CString::new(message).expect("no NUL in the message");
    let category = py.get_type::<pyo3::exceptions::PyUserWarning>();
    PyErr::warn(py, &category, &message, 1)
}

/// The results of a search, read lazily: matches are produced in chunks as
/// the iterator is read, so memory does not grow with the number of matches.
/// As in Text-Fabric, at most `fail_limit` results are delivered; a search
/// with more ends with a `UserWarning` `cut off at N results. There are more
/// ...`. Ctrl-C stops a running chunk with `KeyboardInterrupt`.
#[pyclass(name = "SearchResultIterator", module = "cfabric._core")]
pub(crate) struct PySearchResultIterator {
    corpus: Arc<MappedCompiledCorpus>,
    /// `None` once the matches have run out.
    prepared: Option<PreparedSearch>,
    /// Matches produced but not yet delivered.
    buffer: VecDeque<Vec<u32>>,
    delivered: usize,
    fail_limit: usize,
}

impl PySearchResultIterator {
    pub(crate) fn new(
        corpus: Arc<MappedCompiledCorpus>,
        prepared: PreparedSearch,
        fail_limit: usize,
    ) -> Self {
        Self {
            corpus,
            prepared: Some(prepared),
            buffer: VecDeque::new(),
            delivered: 0,
            fail_limit,
        }
    }

    /// Produce the next chunk of matches: up to one past the fail limit, to
    /// tell whether there are more.
    fn refill(&mut self, py: Python<'_>) -> PyResult<()> {
        let Some(prepared) = self.prepared.as_mut() else {
            return Ok(());
        };
        let wanted = CHUNK_ROWS.min(self.fail_limit - self.delivered + 1);
        let corpus = &self.corpus;
        let rows = interruptible(py, |interrupt| {
            let engine = MappedSearch::new(corpus)?.with_interrupt(interrupt);
            prepared.next_rows(&engine, wanted)
        })?;
        if rows.len() < wanted {
            self.prepared = None;
        }
        self.buffer.extend(rows);
        Ok(())
    }

    fn finish(&mut self) {
        self.prepared = None;
        self.buffer.clear();
    }
}

#[pymethods]
impl PySearchResultIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyTuple>>> {
        if self.buffer.is_empty() {
            self.refill(py)?;
        }
        let Some(row) = self.buffer.pop_front() else {
            return Ok(None);
        };
        if self.delivered == self.fail_limit {
            // A match beyond the fail limit.
            self.finish();
            warn_cut_off(py, self.fail_limit)?;
            return Ok(None);
        }
        self.delivered += 1;
        Ok(Some(PyTuple::new(py, row)?.unbind()))
    }
}
