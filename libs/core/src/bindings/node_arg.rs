//! Conversion of Python node arguments.

use pyo3::exceptions::PyOverflowError;
use pyo3::prelude::*;

/// Converts a Python node argument to a node number.
///
/// Integers outside the node-number range (negative, or beyond `u32`) name no
/// node, so they map to node 0, which no corpus has. Every accessor then
/// answers the way Text-Fabric does for a nonexistent node (`None` / `()`)
/// instead of raising `OverflowError`. Non-integers still raise `TypeError`.
/// Used through `#[pyo3(from_py_with = node_number)]`; the in-range path is
/// the plain `u32` extraction, so per-call cost is unchanged.
pub(crate) fn node_number(value: &Bound<'_, PyAny>) -> PyResult<u32> {
    match value.extract::<u32>() {
        Ok(node) => Ok(node),
        Err(error) if error.is_instance_of::<PyOverflowError>(value.py()) => Ok(0),
        Err(error) => Err(error),
    }
}

/// Converts an optional Python node argument (`None` or a node) like
/// [`node_number`]. Used through `#[pyo3(from_py_with = optional_node_number)]`
/// on parameters that default to `None`.
pub(crate) fn optional_node_number(value: &Bound<'_, PyAny>) -> PyResult<Option<u32>> {
    if value.is_none() {
        return Ok(None);
    }
    node_number(value).map(Some)
}
