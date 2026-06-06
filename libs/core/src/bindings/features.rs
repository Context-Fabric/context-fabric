use std::sync::Arc;

use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyString, PyTuple};

use crate::corpus::{Boundary, Corpus};
use crate::feature::{EdgeFeature, FeatureValue, NodeFeature};

pub(crate) fn feature_value_to_py(py: Python<'_>, value: &FeatureValue) -> PyResult<PyObject> {
    match value {
        FeatureValue::Str(value) => Ok(PyString::new(py, value).into()),
        FeatureValue::Int(value) => Ok(PyInt::new(py, *value).into()),
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

#[pyclass(name = "NodeFeature")]
pub(crate) struct PyNodeFeature {
    feature: NodeFeature,
    corpus: Option<Arc<Corpus>>,
}

impl PyNodeFeature {
    pub(crate) fn new_with_corpus(feature: NodeFeature, corpus: Arc<Corpus>) -> Self {
        Self {
            feature,
            corpus: Some(corpus),
        }
    }
}

#[pymethods]
impl PyNodeFeature {
    #[getter]
    fn name(&self) -> &str {
        &self.feature.name
    }

    #[getter]
    #[allow(non_snake_case)]
    fn metaData(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.feature.meta().clone()
    }

    #[getter]
    fn meta(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.feature.meta().clone()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = PyDict::new(py);
        for (node, value) in self.feature.data() {
            data.set_item(node, feature_value_to_py(py, value)?)?;
        }
        Ok(data.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn valueType(&self) -> Option<&str> {
        self.feature.value_type()
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.feature.description()
    }

    #[getter]
    #[allow(non_snake_case)]
    fn slotType(&self) -> Option<&str> {
        (self.feature.name == "otype")
            .then(|| self.corpus.as_ref().map(|corpus| corpus.slot_type()))
            .flatten()
    }

    #[getter]
    #[allow(non_snake_case)]
    fn maxSlot(&self) -> Option<u32> {
        (self.feature.name == "otype")
            .then(|| self.corpus.as_ref().map(|corpus| corpus.max_slot))
            .flatten()
    }

    #[getter]
    #[allow(non_snake_case)]
    fn maxNode(&self) -> Option<u32> {
        (self.feature.name == "otype")
            .then(|| self.corpus.as_ref().map(|corpus| corpus.max_node))
            .flatten()
    }

    fn v(&self, py: Python<'_>, node: u32) -> PyResult<Option<PyObject>> {
        self.feature
            .v(node)
            .map(|value| feature_value_to_py(py, value))
            .transpose()
    }

    fn s(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let value = feature_value_from_py(value)?;
        Ok(PyTuple::new(py, self.feature.s(&value).iter().copied())?.into())
    }

    fn items(&self, py: Python<'_>) -> PyResult<PyObject> {
        let rows = self
            .feature
            .items()
            .iter()
            .map(|(node, value)| {
                let row: Vec<PyObject> = vec![
                    node.into_pyobject(py)?.into(),
                    feature_value_to_py(py, value)?,
                ];
                PyTuple::new(py, row).map(Into::into)
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }

    #[pyo3(signature = (node_types=None))]
    fn freq_list(
        &self,
        py: Python<'_>,
        node_types: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let _ = node_types;
        let rows = self
            .feature
            .freq_list()
            .iter()
            .map(|(value, count)| {
                Ok(PyTuple::new(
                    py,
                    [
                        feature_value_to_py(py, value)?,
                        count.into_pyobject(py)?.into(),
                    ],
                )?
                .into())
            })
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node_types=None))]
    fn freqList(
        &self,
        py: Python<'_>,
        node_types: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.freq_list(py, node_types)
    }
}

#[pyclass(name = "NodeFeatures")]
pub(crate) struct PyNodeFeatures {
    corpus: Arc<Corpus>,
}

impl PyNodeFeatures {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyNodeFeatures {
    fn __getattr__(&self, name: &str) -> PyResult<PyNodeFeature> {
        self.corpus
            .node_feature(name)
            .cloned()
            .map(|feature| PyNodeFeature::new_with_corpus(feature, Arc::clone(&self.corpus)))
            .ok_or_else(|| PyAttributeError::new_err(format!("no node feature named {name}")))
    }

    fn __dir__(&self) -> Vec<String> {
        self.corpus
            .node_feature_names()
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}

#[pyclass(name = "EdgeFeature")]
pub(crate) struct PyEdgeFeature {
    feature: EdgeFeature,
}

impl PyEdgeFeature {
    pub(crate) fn new(feature: EdgeFeature) -> Self {
        Self { feature }
    }

    fn nodes_to_tuple(&self, py: Python<'_>, nodes: Vec<u32>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, nodes)?.into())
    }

    fn valued_nodes_to_tuple(
        &self,
        py: Python<'_>,
        nodes: Vec<(u32, Option<FeatureValue>)>,
    ) -> PyResult<PyObject> {
        let rows = nodes
            .iter()
            .map(|(node, value)| {
                let value = value
                    .as_ref()
                    .map(|value| feature_value_to_py(py, value))
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
        &self.feature.name
    }

    #[getter]
    #[allow(non_snake_case)]
    fn metaData(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.feature.meta().clone()
    }

    #[getter]
    fn meta(&self) -> std::collections::BTreeMap<String, Option<String>> {
        self.feature.meta().clone()
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = PyDict::new(py);
        if self.feature.has_edge_values() {
            for (source, targets) in self.feature.items_with_values() {
                let target_map = PyDict::new(py);
                for (target, value) in targets {
                    let value = value
                        .as_ref()
                        .map(|value| feature_value_to_py(py, value))
                        .transpose()?
                        .unwrap_or_else(|| py.None());
                    target_map.set_item(target, value)?;
                }
                data.set_item(source, target_map)?;
            }
        } else {
            for (source, targets) in self.feature.items() {
                data.set_item(source, PyTuple::new(py, targets)?)?;
            }
        }
        Ok(data.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn dataInv(&self, py: Python<'_>) -> PyResult<PyObject> {
        let data = PyDict::new(py);
        if self.feature.has_edge_values() {
            for (target, sources) in self.feature.data_inv_with_values() {
                let source_map = PyDict::new(py);
                for (source, value) in sources {
                    let value = value
                        .as_ref()
                        .map(|value| feature_value_to_py(py, value))
                        .transpose()?
                        .unwrap_or_else(|| py.None());
                    source_map.set_item(source, value)?;
                }
                data.set_item(target, source_map)?;
            }
        } else {
            for (target, sources) in self.feature.data_inv() {
                data.set_item(target, PyTuple::new(py, sources)?)?;
            }
        }
        Ok(data.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn valueType(&self) -> Option<&str> {
        self.feature.value_type()
    }

    #[getter]
    fn description(&self) -> Option<&str> {
        self.feature.description()
    }

    fn f(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        if self.feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, self.feature.f_with_values(node))
        } else {
            self.nodes_to_tuple(py, self.feature.f(node))
        }
    }

    fn t(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        if self.feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, self.feature.t_with_values(node))
        } else {
            self.nodes_to_tuple(py, self.feature.t(node))
        }
    }

    fn s(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        self.nodes_to_tuple(py, self.feature.s(node))
    }

    fn b(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        if self.feature.has_edge_values() {
            self.valued_nodes_to_tuple(py, self.feature.b_with_values(node))
        } else {
            self.nodes_to_tuple(py, self.feature.b(node))
        }
    }

    fn items(&self, py: Python<'_>) -> PyResult<PyObject> {
        let rows = self
            .feature
            .items()
            .iter()
            .map(|(source, targets)| {
                let row: Vec<PyObject> = vec![
                    source.into_pyobject(py)?.into(),
                    PyTuple::new(py, targets.iter().copied())?.into(),
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
        let _ = (node_types_from, node_types_to);
        match self.feature.freq_list() {
            crate::feature::EdgeFrequency::Count(count) => Ok(count.into_pyobject(py)?.into()),
            crate::feature::EdgeFrequency::Values(rows) => {
                let rows = rows
                    .iter()
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
    #[pyo3(signature = (node_types_from=None, node_types_to=None))]
    fn freqList(
        &self,
        py: Python<'_>,
        node_types_from: Option<&Bound<'_, PyAny>>,
        node_types_to: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        self.freq_list(py, node_types_from, node_types_to)
    }

    fn has_edge_values(&self) -> bool {
        self.feature.has_edge_values()
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
    corpus: Arc<Corpus>,
}

impl PyEdgeFeatures {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyEdgeFeatures {
    fn __getattr__(&self, name: &str) -> PyResult<PyEdgeFeature> {
        self.corpus
            .edge_feature(name)
            .cloned()
            .map(PyEdgeFeature::new)
            .ok_or_else(|| PyAttributeError::new_err(format!("no edge feature named {name}")))
    }

    fn __dir__(&self) -> Vec<String> {
        self.corpus
            .edge_feature_names()
            .into_iter()
            .map(str::to_string)
            .collect()
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
    corpus: Arc<Corpus>,
}

impl PyComputeds {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
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
            _ => Err(PyAttributeError::new_err(format!(
                "no computed feature named {name}"
            ))),
        }
    }
}

#[pymethods]
impl PyComputeds {
    #[getter]
    fn levels(&self, py: Python<'_>) -> PyResult<PyComputed> {
        let rows = self
            .corpus
            .levels()
            .iter()
            .map(|(node_type, level, first, last)| {
                let row: Vec<PyObject> = vec![
                    PyString::new(py, node_type).into(),
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
            PyTuple::new(py, self.corpus.order())?.into(),
        ))
    }

    #[getter]
    fn rank(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(PyComputed::new(
            PyTuple::new(py, self.corpus.rank())?.into(),
        ))
    }

    #[getter]
    fn boundary(&self, py: Python<'_>) -> PyResult<PyComputed> {
        Ok(PyComputed::new(Self::boundary_to_py(
            py,
            self.corpus.boundary(),
        )?))
    }

    fn __dir__(&self) -> Vec<&'static str> {
        vec!["levels", "order", "rank", "boundary"]
    }

    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        self.get(py, name)
    }
}
