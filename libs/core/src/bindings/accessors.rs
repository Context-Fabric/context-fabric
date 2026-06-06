use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyIterator, PyList, PyString, PyTuple};

use crate::corpus::{Corpus, SectionOptions, StructureTree, TextOptions};
use crate::feature::FeatureValue;
use crate::precompute::StructureHeading;

use super::features::feature_value_from_py;

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

fn structure_tree_to_py(py: Python<'_>, tree: &StructureTree) -> PyResult<PyObject> {
    match tree {
        StructureTree::Forest(children) => {
            let items = children
                .iter()
                .map(|child| structure_tree_to_py(py, child))
                .collect::<PyResult<Vec<_>>>()?;
            Ok(PyList::new(py, items)?.into())
        }
        StructureTree::Node { node, children } => {
            let items = children
                .iter()
                .map(|child| structure_tree_to_py(py, child))
                .collect::<PyResult<Vec<_>>>()?;
            let row: Vec<PyObject> = vec![
                node.into_pyobject(py)?.into(),
                PyList::new(py, items)?.into(),
            ];
            Ok(PyTuple::new(py, row)?.into())
        }
    }
}

fn heading_value_to_py(py: Python<'_>, heading: &str) -> PyResult<PyObject> {
    if let Ok(value) = heading.parse::<i64>() {
        return Ok(value.into_pyobject(py)?.into());
    }
    Ok(PyString::new(py, heading).into())
}

fn heading_to_py(py: Python<'_>, heading: &[StructureHeading]) -> PyResult<PyObject> {
    let rows = heading
        .iter()
        .map(|item| {
            let row: Vec<PyObject> = vec![
                PyString::new(py, &item.node_type).into(),
                heading_value_to_py(py, &item.heading)?,
            ];
            PyTuple::new(py, row).map(Into::into)
        })
        .collect::<PyResult<Vec<PyObject>>>()?;
    Ok(PyTuple::new(py, rows)?.into())
}

fn heading_from_py(items: &Bound<'_, PyAny>) -> PyResult<Vec<StructureHeading>> {
    let rows = items.extract::<Vec<(String, Bound<'_, PyAny>)>>()?;
    Ok(rows
        .into_iter()
        .map(|(node_type, heading)| {
            let heading = heading
                .extract::<String>()
                .or_else(|_| heading.extract::<i64>().map(|value| value.to_string()))?;
            Ok(StructureHeading { node_type, heading })
        })
        .collect::<PyResult<Vec<_>>>()?)
}

fn section_from_py(items: &Bound<'_, PyAny>) -> PyResult<Vec<FeatureValue>> {
    let values = items.extract::<Vec<Bound<'_, PyAny>>>()?;
    values
        .iter()
        .map(feature_value_from_py)
        .collect::<PyResult<Vec<_>>>()
}

#[pyclass(name = "Locality")]
pub(crate) struct PyLocality {
    corpus: Arc<Corpus>,
}

impl PyLocality {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyLocality {
    #[pyo3(signature = (node, otype=None))]
    fn u(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let nodes = match refs.as_deref() {
            Some([single]) => self.corpus.u(node, Some(single)),
            Some(values) => self.corpus.up_types(node, Some(values)),
            None => self.corpus.u(node, None),
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn d(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let nodes = match refs.as_deref() {
            Some([single]) => self.corpus.d(node, Some(single)),
            Some(values) => self.corpus.down_types(node, Some(values)),
            None => self.corpus.d(node, None),
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn n(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let nodes = match refs.as_deref() {
            Some([single]) => self.corpus.n(node, Some(single)),
            Some(values) => self.corpus.next_types(node, Some(values)),
            None => self.corpus.n(node, None),
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn p(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let nodes = match refs.as_deref() {
            Some([single]) => self.corpus.p(node, Some(single)),
            Some(values) => self.corpus.previous_types(node, Some(values)),
            None => self.corpus.p(node, None),
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn i(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let nodes = match refs.as_deref() {
            Some([single]) => self.corpus.i(node, Some(single)),
            Some(values) => self.corpus.intersecting_types(node, Some(values)),
            None => self.corpus.i(node, None),
        };
        nodes_to_tuple(py, nodes)
    }
}

#[pyclass(name = "Nodes")]
pub(crate) struct PyNodes {
    corpus: Arc<Corpus>,
}

impl PyNodes {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyNodes {
    #[getter]
    #[allow(non_snake_case)]
    fn otypeRank(&self) -> std::collections::BTreeMap<String, u32> {
        self.corpus.otype_rank()
    }

    #[allow(non_snake_case)]
    fn sortKey(&self, node: u32) -> usize {
        self.corpus.sort_key(node)
    }

    #[allow(non_snake_case)]
    fn sortKeyTuple(&self, py: Python<'_>, nodes: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let nodes = nodes_from_py(Some(nodes))?.unwrap_or_default();
        Ok(PyTuple::new(py, self.corpus.sort_key_tuple(&nodes))?.into())
    }

    #[allow(non_snake_case)]
    fn sortNodes(&self, py: Python<'_>, nodes: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let nodes = nodes_from_py(Some(nodes))?.unwrap_or_default();
        nodes_to_tuple(py, self.corpus.sorted_nodes(nodes))
    }

    #[pyo3(signature = (nodes=None, events=false))]
    fn walk(
        &self,
        py: Python<'_>,
        nodes: Option<&Bound<'_, PyAny>>,
        events: bool,
    ) -> PyResult<PyObject> {
        let _ = events;
        let nodes = nodes_from_py(nodes)?;
        nodes_to_tuple(py, self.corpus.walk(nodes.as_deref()))
    }
}

#[pyclass(name = "Text")]
pub(crate) struct PyText {
    corpus: Arc<Corpus>,
}

impl PyText {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyText {
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypes(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.section_types())?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeatures(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.section_features())?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeats(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.sectionFeatures(py)
    }

    #[getter]
    fn formats(&self, py: Python<'_>) -> PyResult<PyObject> {
        let formats = self
            .corpus
            .config_features
            .get("otext")
            .map(|feature| {
                let mut names = feature
                    .keys()
                    .filter_map(|key| key.strip_prefix("fmt:").map(str::to_string))
                    .collect::<Vec<_>>();
                names.sort();
                names
            })
            .unwrap_or_default();
        Ok(PyTuple::new(py, formats)?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn structureTypes(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.structure_types())?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn structureFeats(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.structure_features())?.into())
    }

    #[getter]
    fn languages(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, ["en"])?.into())
    }

    #[getter]
    fn headings(&self, py: Python<'_>) -> PyResult<PyObject> {
        let headings = PyDict::new(py);
        Ok(headings.into())
    }

    #[pyo3(signature = (node, fmt=None, descend=None))]
    fn text(&self, node: u32, fmt: Option<&str>, descend: Option<bool>) -> String {
        if descend.is_some() {
            self.corpus
                .text_with_options(node, &TextOptions::new(fmt, descend))
        } else {
            self.corpus.text(node, fmt)
        }
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node, lastSlot=false, fillup=false, level=None, lang="en"))]
    fn sectionFromNode(
        &self,
        py: Python<'_>,
        node: u32,
        lastSlot: bool,
        fillup: bool,
        level: Option<usize>,
        lang: &str,
    ) -> PyResult<PyObject> {
        let _ = (lastSlot, fillup, level);
        let values = self
            .corpus
            .section_from_node_lang(node, &SectionOptions::default(), lang);
        let items = values
            .iter()
            .map(|value| option_feature_value_to_py(py, value.as_ref()))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyTuple::new(py, items)?.into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node, lastSlot=false, fillup=false, level=None, lang="en"))]
    fn sectionTuple(
        &self,
        py: Python<'_>,
        node: u32,
        lastSlot: bool,
        fillup: bool,
        level: Option<usize>,
        lang: &str,
    ) -> PyResult<PyObject> {
        self.sectionFromNode(py, node, lastSlot, fillup, level, lang)
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (section, lang="en"))]
    fn nodeFromSection(&self, section: &Bound<'_, PyAny>, lang: &str) -> PyResult<Option<u32>> {
        Ok(self
            .corpus
            .node_from_section_lang(&section_from_py(section)?, lang))
    }

    #[allow(non_snake_case)]
    fn headingFromNode(&self, py: Python<'_>, node: u32) -> PyResult<Option<PyObject>> {
        self.corpus
            .heading_from_node(node)
            .map(|heading| heading_to_py(py, &heading))
            .transpose()
    }

    #[allow(non_snake_case)]
    fn nodeFromHeading(&self, heading: &Bound<'_, PyAny>) -> PyResult<Option<u32>> {
        Ok(self.corpus.node_from_heading(&heading_from_py(heading)?))
    }

    #[pyo3(signature = (node=None))]
    fn structure(&self, py: Python<'_>, node: Option<u32>) -> PyResult<Option<PyObject>> {
        self.corpus
            .structure(node)
            .map(|tree| structure_tree_to_py(py, &tree))
            .transpose()
    }

    fn top(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        self.corpus
            .top()
            .map(|nodes| nodes_to_tuple(py, nodes))
            .transpose()
    }

    fn up(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        nodes_to_tuple(py, self.corpus.u(node, None))
    }

    fn down(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        nodes_to_tuple(py, self.corpus.d(node, None))
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node=None, fullHeading=false))]
    fn structurePretty(&self, node: Option<u32>, fullHeading: bool) -> Option<String> {
        self.corpus.structure_pretty(node, fullHeading)
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node, lang="en"))]
    fn bookName(&self, node: u32, lang: &str) -> Option<String> {
        self.corpus.bookName(node, lang)
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (name, lang="en"))]
    fn bookNode(&self, name: &str, lang: &str) -> Option<u32> {
        self.corpus.bookNode(name, lang)
    }
}

#[pyclass(name = "Search")]
pub(crate) struct PySearch {
    corpus: Arc<Corpus>,
    exe: Option<PySearchExe>,
    template: Option<String>,
}

impl PySearch {
    pub(crate) fn new(corpus: Arc<Corpus>) -> Self {
        Self {
            corpus,
            exe: None,
            template: None,
        }
    }
}

#[pymethods]
impl PySearch {
    #[getter]
    fn exe(&self) -> Option<PySearchExe> {
        self.exe.clone()
    }

    #[pyo3(signature = (template))]
    fn study(&mut self, template: &str) {
        let result = self.corpus.search().search(template, Some(1));
        self.template = Some(template.to_string());
        self.exe = Some(match result {
            Ok(_) => PySearchExe::good(),
            Err(error) => PySearchExe::bad(error.to_string()),
        });
    }

    #[pyo3(signature = (template, limit=None, sets=None, shallow=false, silent=None, here=false))]
    fn search(
        &self,
        py: Python<'_>,
        template: &str,
        limit: Option<usize>,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: bool,
        silent: Option<&str>,
        here: bool,
    ) -> PyResult<PyObject> {
        let _ = (sets, shallow, silent, here);
        let rows = self
            .corpus
            .search()
            .search(template, limit)?
            .into_iter()
            .map(|row| PyTuple::new(py, row).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }

    #[pyo3(signature = (limit=None))]
    fn fetch(&self, py: Python<'_>, limit: Option<usize>) -> PyResult<PyObject> {
        let template = self.template.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        self.search(py, template, limit, None, false, None, false)
    }

    #[pyo3(signature = (progress=None, limit=None))]
    fn count(&self, progress: Option<usize>, limit: Option<usize>) -> PyResult<usize> {
        let _ = progress;
        let template = self.template.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        Ok(self.corpus.search().search(template, limit)?.len())
    }

    #[allow(non_snake_case)]
    fn showPlan(&self, details: bool) -> Option<String> {
        let _ = details;
        self.template.as_ref().map(|template| template.to_string())
    }

    fn glean(&self, py: Python<'_>, tuples: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let rows = PyIterator::from_object(tuples)?
            .map(|item| item.map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }

    #[allow(non_snake_case)]
    fn tweakPerformance(&self) -> bool {
        true
    }

    #[allow(non_snake_case)]
    fn relationsLegend(&self) -> String {
        crate::search::relations_legend().to_string()
    }

    #[getter]
    #[allow(non_snake_case)]
    fn perfParams(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyDict::new(py).into())
    }

    #[getter]
    fn api(&self) -> bool {
        true
    }
}

#[pyclass(name = "SearchExe")]
#[derive(Clone)]
pub(crate) struct PySearchExe {
    #[pyo3(get)]
    good: bool,
    #[pyo3(get, name = "badSyntax")]
    bad_syntax: Vec<(Option<usize>, String)>,
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
