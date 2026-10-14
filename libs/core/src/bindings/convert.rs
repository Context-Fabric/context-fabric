//! Conversions between Python objects and core values: node numbers, node
//! lists, feature values, headings and node type filters.

use pyo3::exceptions::{PyAttributeError, PyOverflowError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyInt, PyIterator, PyString, PyTuple};

use crate::index::{StructureHeading, StructureTree};
use crate::mapped::MappedNodeValue;
use crate::value::FeatureValue;

pub(crate) fn nodes_to_tuple(py: Python<'_>, nodes: Vec<u32>) -> PyResult<PyObject> {
    Ok(PyTuple::new(py, nodes)?.into())
}

pub(crate) fn nodes_from_py(items: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Vec<u32>>> {
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

pub(crate) fn option_feature_value_to_py(
    py: Python<'_>,
    value: Option<&FeatureValue>,
) -> PyResult<PyObject> {
    match value {
        Some(value) => feature_value_to_py(py, value),
        None => Ok(py.None()),
    }
}

/// TF `T.structure` shape: a node is `(node, (child, ...))` and the whole
/// corpus a tuple of those, all tuples.
pub(crate) fn structure_tree_to_py(py: Python<'_>, tree: &StructureTree) -> PyResult<PyObject> {
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
pub(crate) fn heading_from_py(items: &Bound<'_, PyAny>) -> PyResult<Vec<StructureHeading>> {
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

pub(crate) fn section_from_py(items: &Bound<'_, PyAny>) -> PyResult<Vec<FeatureValue>> {
    let values = items.extract::<Vec<Bound<'_, PyAny>>>()?;
    values
        .iter()
        .map(feature_value_from_py)
        .collect::<PyResult<Vec<_>>>()
}

/// `AttributeError` for a warp-only member accessed on another feature.
pub(crate) fn warp_only(feature: &str, member: &str) -> PyErr {
    PyAttributeError::new_err(format!("feature {feature} has no attribute '{member}'"))
}

/// Parses a `nodeTypes` style argument (`None` / `str` / iterable of `str`)
/// into an owned `Vec<String>`. Mirrors TF's `freqList(nodeTypes=...)` which
/// accepts any iterable of node-type names (commonly a `set`).
pub(crate) fn node_types_from_py(arg: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Vec<String>>> {
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

pub(crate) fn as_str_refs(values: &Option<Vec<String>>) -> Option<Vec<&str>> {
    values
        .as_ref()
        .map(|values| values.iter().map(String::as_str).collect())
}

pub(crate) fn feature_value_to_py(py: Python<'_>, value: &FeatureValue) -> PyResult<PyObject> {
    match value {
        FeatureValue::Str(value) => Ok(PyString::new(py, value).into()),
        FeatureValue::Int(value) => Ok(PyInt::new(py, *value).into()),
    }
}

pub(crate) fn mapped_value_to_py(py: Python<'_>, value: MappedNodeValue<'_>) -> PyResult<PyObject> {
    match value {
        MappedNodeValue::Str(value) => Ok(PyString::new(py, value).into()),
        MappedNodeValue::Int(value) => Ok(PyInt::new(py, value).into()),
    }
}

pub(crate) fn feature_value_from_py(value: &Bound<'_, PyAny>) -> PyResult<FeatureValue> {
    // Test for `str` first: a failed `extract::<i64>()` on a string builds
    // (and discards) a formatted TypeError, which dominated string lookups.
    if let Ok(text) = value.downcast::<PyString>() {
        return Ok(FeatureValue::string(text.to_str()?));
    }
    if let Ok(number) = value.extract::<i64>() {
        return Ok(FeatureValue::Int(number));
    }
    Err(PyTypeError::new_err("feature values must be str or int"))
}

pub(crate) fn mapped_value_from_feature(value: &FeatureValue) -> MappedNodeValue<'_> {
    match value {
        FeatureValue::Str(value) => MappedNodeValue::Str(value),
        FeatureValue::Int(value) => MappedNodeValue::Int(*value),
    }
}

/// Collects any Python iterable of node ids (`list` / `tuple` / `range` / `set`
/// / numpy integer array) into an owned `Vec<u32>`. Shared by the additive batch
/// APIs (`F.<feat>.vs`, `L.u_many`, `T.text_many`, ...) so the per-element
/// Python<->Rust boundary crossing happens exactly once for the whole batch.
pub(crate) fn collect_nodes(items: &Bound<'_, PyAny>) -> PyResult<Vec<u32>> {
    let iter = items
        .try_iter()
        .map_err(|_| PyTypeError::new_err("expected an iterable of node ids (int)"))?;
    let mut nodes = Vec::with_capacity(items.len().unwrap_or(0));
    for item in iter {
        nodes.push(item?.extract::<u32>()?);
    }
    Ok(nodes)
}

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
