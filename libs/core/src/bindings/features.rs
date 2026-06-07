use std::sync::Arc;

use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyString, PyTuple};

use crate::compiled::{MappedCompiledCorpus, MappedNodeValue};
use crate::corpus::{Boundary, ComputedFeatureData};
use crate::feature::{EdgeFrequency, FeatureValue};

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

#[pyclass(name = "NodeFeature")]
pub(crate) struct PyNodeFeature {
    name: String,
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyNodeFeature {
    pub(crate) fn new(name: String, corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self { name, corpus }
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

    #[pyo3(signature = (node_types=None))]
    fn freq_list(
        &self,
        py: Python<'_>,
        node_types: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyObject> {
        let _ = node_types;
        let rows = self
            .corpus
            .node_frequency_list(&self.name, None)?
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
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyNodeFeatures {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyNodeFeatures {
    fn __getattr__(&self, name: &str) -> PyResult<PyNodeFeature> {
        self.corpus
            .metadata()
            .node_feature(name)
            .map(|_| PyNodeFeature::new(name.to_string(), Arc::clone(&self.corpus)))
            .ok_or_else(|| PyAttributeError::new_err(format!("no node feature named {name}")))
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
        let _ = (node_types_from, node_types_to);
        match self.corpus.edge_frequency_list(&self.name, None, None)? {
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

    fn __dir__(&self) -> Vec<&'static str> {
        vec!["levels", "order", "rank", "boundary"]
    }

    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        self.get(py, name)
    }
}
