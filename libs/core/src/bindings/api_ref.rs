//! Back-reference from the API members (`F`, `E`, `C`, `L`, `N`, `T`, `S`, and
//! the individual feature objects) to the `Corpus` (TF `api`) that owns them.
//!
//! Text-Fabric gives every API member an `api` attribute. The `Corpus` caches
//! its members, so a strong back-reference would form a reference cycle through
//! every member; the members therefore hold a weak reference instead. The
//! `Corpus` owns its members, so the reference is live for as long as anyone can
//! reach the corpus; it only goes dead if a member outlives every reference to
//! its corpus, in which case reading `.api` raises.

use pyo3::exceptions::PyReferenceError;
use pyo3::prelude::*;
use pyo3::types::PyWeakrefReference;

/// Weak reference to the owning `Corpus`.
pub(crate) struct ApiRef(Py<PyWeakrefReference>);

impl ApiRef {
    /// Create a weak reference to `corpus` (which must support weak references).
    pub(crate) fn new(corpus: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self(PyWeakrefReference::new(corpus)?.unbind()))
    }

    /// Another handle on the same weak reference.
    pub(crate) fn clone_ref(&self, py: Python<'_>) -> Self {
        Self(self.0.clone_ref(py))
    }

    /// The owning `Corpus`, or `ReferenceError` if it has been garbage collected.
    pub(crate) fn get(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.0.bind(py).upgrade().map(Bound::unbind).ok_or_else(|| {
            PyReferenceError::new_err(
                "the api (Corpus) this member belongs to has been garbage collected",
            )
        })
    }
}
