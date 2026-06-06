use std::path::PathBuf;
use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};

use crate::compiled::MappedCompiledCorpus;
use crate::corpus::Corpus;
use crate::fabric::Fabric;
use crate::mapped_search::MappedSearch;

use super::accessors::{PyLocality, PyNodes, PySearch, PyText};
use super::features::{PyComputeds, PyEdgeFeature, PyEdgeFeatures, PyNodeFeature, PyNodeFeatures};

fn strings_from_optional_python(value: Option<&Bound<'_, PyAny>>) -> PyResult<Option<Vec<String>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(text) = value.downcast::<PyString>() {
        return Ok(Some(vec![text.to_str()?.to_string()]));
    }
    if let Ok(items) = value.extract::<Vec<String>>() {
        return Ok(Some(items));
    }
    Ok(Some(vec![value.str()?.to_str()?.to_string()]))
}

fn feature_names(value: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<String>> {
    Ok(strings_from_optional_python(value)?.unwrap_or_default())
}

#[pyclass(name = "Fabric")]
pub(crate) struct PyFabric {
    inner: Fabric,
    silent: Option<String>,
}

#[pymethods]
impl PyFabric {
    #[new]
    #[allow(non_snake_case)]
    #[pyo3(signature = (locations=None, modules=None, silent=None, _withGc=false))]
    fn new(
        locations: Option<&Bound<'_, PyAny>>,
        modules: Option<&Bound<'_, PyAny>>,
        silent: Option<String>,
        _withGc: bool,
    ) -> PyResult<Self> {
        let locations = strings_from_optional_python(locations)?
            .unwrap_or_else(|| vec![String::new()])
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let modules = strings_from_optional_python(modules)?.unwrap_or_else(|| vec![String::new()]);
        let path = locations.first().cloned().unwrap_or_default();
        let mut fabric = Fabric::with_modules(path, modules);
        if locations.len() > 1 {
            fabric = Fabric::from_locations(locations);
        }
        Ok(Self {
            inner: fabric,
            silent,
        })
    }

    #[getter]
    fn banner(&self) -> &str {
        self.inner.banner()
    }

    #[getter]
    fn version(&self) -> &str {
        self.inner.version()
    }

    #[getter]
    fn good(&self) -> bool {
        self.inner.good()
    }

    #[getter]
    fn silent(&self) -> Option<&str> {
        self.silent.as_deref()
    }

    #[pyo3(signature = (features=None, add=false, silent=None))]
    fn load(
        &self,
        features: Option<&Bound<'_, PyAny>>,
        add: bool,
        silent: Option<&str>,
    ) -> PyResult<PyCorpus> {
        let _ = (add, silent);
        let features = feature_names(features)?;
        let corpus = if features.is_empty() {
            self.inner.load_all()?
        } else {
            self.inner.load(features)?
        };
        Ok(PyCorpus::new(corpus))
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (silent=None))]
    fn loadAll(&self, silent: Option<&str>) -> PyResult<PyCorpus> {
        let _ = silent;
        Ok(PyCorpus::new(self.inner.load_all()?))
    }

    #[pyo3(signature = (silent=None))]
    fn load_all(&self, silent: Option<&str>) -> PyResult<PyCorpus> {
        let _ = silent;
        Ok(PyCorpus::new(self.inner.load_all()?))
    }

    #[pyo3(signature = (output_path, features=None, silent=None))]
    fn compile(
        &self,
        output_path: &str,
        features: Option<&Bound<'_, PyAny>>,
        silent: Option<&str>,
    ) -> PyResult<bool> {
        let _ = silent;
        self.inner.compile(output_path, feature_names(features)?)?;
        Ok(true)
    }

    #[pyo3(signature = (silent=None, show=true))]
    fn explore(&self, py: Python<'_>, silent: Option<&str>, show: bool) -> PyResult<PyObject> {
        let _ = (silent, show);
        let inventory = self.inner.explore()?;
        let result = PyDict::new(py);
        result.set_item("nodes", PyTuple::new(py, inventory.nodes)?)?;
        result.set_item("edges", PyTuple::new(py, inventory.edges)?)?;
        result.set_item("configs", PyTuple::new(py, inventory.configs)?)?;
        Ok(result.into())
    }

    #[getter]
    fn features(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.explore(py, None, false)
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (features=None, pretty=true, valueType=true, path=false, meta="description"))]
    fn isLoaded(
        &self,
        features: Option<&Bound<'_, PyAny>>,
        pretty: bool,
        valueType: bool,
        path: bool,
        meta: &str,
    ) -> PyResult<bool> {
        let _ = (pretty, valueType, path, meta);
        let requested = feature_names(features)?;
        if requested.is_empty() {
            return Ok(false);
        }
        let inventory = self.inner.explore()?;
        Ok(requested.iter().all(|feature| {
            inventory.nodes.contains(feature)
                || inventory.edges.contains(feature)
                || inventory.configs.contains(feature)
        }))
    }

    #[pyo3(signature = (node_features=true, edge_features=true))]
    fn save(&self, node_features: bool, edge_features: bool) -> bool {
        let _ = (node_features, edge_features);
        true
    }

    #[allow(non_snake_case)]
    fn loadCompiled(&self, cache_path: &str) -> PyResult<PyCorpus> {
        Ok(PyCorpus::new(self.inner.load_compiled(cache_path)?))
    }

    fn load_compiled(&self, cache_path: &str) -> PyResult<PyCorpus> {
        self.loadCompiled(cache_path)
    }

    #[allow(non_snake_case)]
    fn openMapped(&self, cache_path: &str) -> PyResult<PyMappedCorpus> {
        Ok(PyMappedCorpus::new(self.inner.open_mapped(cache_path)?))
    }

    fn open_mapped(&self, cache_path: &str) -> PyResult<PyMappedCorpus> {
        self.openMapped(cache_path)
    }

    #[getter]
    fn locations(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.inner
                .locations()
                .iter()
                .map(|path| path.display().to_string()),
        )?
        .into())
    }

    #[getter]
    fn modules(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.inner.modules().iter().cloned())?.into())
    }
}

#[pyclass(name = "MappedCorpus")]
pub(crate) struct PyMappedCorpus {
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyMappedCorpus {
    fn new(corpus: MappedCompiledCorpus) -> Self {
        Self {
            corpus: Arc::new(corpus),
        }
    }
}

#[pymethods]
impl PyMappedCorpus {
    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Fall(&self, py: Python<'_>, warp: bool) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.Fall(warp))?.into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Eall(&self, py: Python<'_>, warp: bool) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.Eall(warp))?.into())
    }

    #[allow(non_snake_case)]
    fn Call(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.Call())?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn maxNode(&self) -> u32 {
        self.corpus.max_node()
    }

    #[pyo3(signature = (template, limit=None))]
    fn search(&self, py: Python<'_>, template: &str, limit: Option<usize>) -> PyResult<PyObject> {
        let search = MappedSearch::new(&self.corpus);
        let rows = search.search(template, limit)?;
        let rows = rows
            .iter()
            .map(|row| PyTuple::new(py, row.iter().copied()).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, rows)?.into())
    }
}

#[pyclass(name = "Corpus")]
pub(crate) struct PyCorpus {
    corpus: Arc<Corpus>,
}

impl PyCorpus {
    fn new(corpus: Corpus) -> Self {
        Self {
            corpus: Arc::new(corpus),
        }
    }
}

#[pymethods]
impl PyCorpus {
    #[getter]
    #[allow(non_snake_case)]
    fn F(&self) -> PyNodeFeatures {
        PyNodeFeatures::new(Arc::clone(&self.corpus))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn E(&self) -> PyEdgeFeatures {
        PyEdgeFeatures::new(Arc::clone(&self.corpus))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn L(&self) -> PyLocality {
        PyLocality::new(Arc::clone(&self.corpus))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn N(&self) -> PyNodes {
        PyNodes::new(Arc::clone(&self.corpus))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn T(&self) -> PyText {
        PyText::new(Arc::clone(&self.corpus))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn S(&self) -> PySearch {
        PySearch::new(Arc::clone(&self.corpus))
    }

    #[getter]
    #[allow(non_snake_case)]
    fn C(&self) -> PyComputeds {
        PyComputeds::new(Arc::clone(&self.corpus))
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (name=None))]
    fn Cs(&self, py: Python<'_>, name: Option<&str>) -> PyResult<PyObject> {
        let computeds = PyComputeds::new(Arc::clone(&self.corpus));
        if let Some(name) = name {
            return computeds.get(py, name);
        }
        Ok(computeds.into_pyobject(py)?.into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (name, warn=true))]
    fn Fs(&self, name: &str, warn: bool) -> Option<PyNodeFeature> {
        let _ = warn;
        self.corpus
            .node_feature(name)
            .cloned()
            .map(|feature| PyNodeFeature::new_with_corpus(feature, Arc::clone(&self.corpus)))
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (name, warn=true))]
    fn Es(&self, name: &str, warn: bool) -> Option<PyEdgeFeature> {
        let _ = warn;
        self.corpus
            .edge_feature(name)
            .cloned()
            .map(PyEdgeFeature::new)
    }

    fn node_feature_names(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.corpus
                .node_feature_names()
                .into_iter()
                .map(str::to_string),
        )?
        .into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Fall(&self, py: Python<'_>, warp: bool) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.all_node_features(warp))?.into())
    }

    fn edge_feature_names(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.corpus
                .edge_feature_names()
                .into_iter()
                .map(str::to_string),
        )?
        .into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Eall(&self, py: Python<'_>, warp: bool) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.all_edge_features(warp))?.into())
    }

    #[allow(non_snake_case)]
    fn Call(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.all_computed_features())?.into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (features=None, pretty=true, valueType=true, path=false, meta="description"))]
    fn isLoaded(
        &self,
        features: Option<&Bound<'_, PyAny>>,
        pretty: bool,
        valueType: bool,
        path: bool,
        meta: &str,
    ) -> PyResult<bool> {
        let _ = (pretty, valueType, path, meta);
        let requested = feature_names(features)?;
        if requested.is_empty() {
            return Ok(true);
        }
        Ok(requested.iter().all(|feature| {
            self.corpus.node_feature(feature).is_some()
                || self.corpus.edge_feature(feature).is_some()
                || self.corpus.config_features.contains_key(feature)
        }))
    }

    #[allow(non_snake_case)]
    fn ensureLoaded(&self, features: Option<&Bound<'_, PyAny>>) -> PyResult<bool> {
        self.isLoaded(features, true, true, false, "description")
    }

    #[allow(non_snake_case)]
    fn makeAvailableIn(&self, py: Python<'_>, scope: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(dict) = scope.downcast::<PyDict>() {
            dict.set_item("F", self.F())?;
            dict.set_item("E", self.E())?;
            dict.set_item("L", self.L())?;
            dict.set_item("N", self.N())?;
            dict.set_item("T", self.T())?;
            dict.set_item("S", self.S())?;
            dict.set_item("C", self.C())?;
            let _ = py;
        }
        Ok(())
    }

    #[pyo3(signature = (recompute=false, by_size=true))]
    fn footprint(&self, recompute: bool, by_size: bool) -> String {
        let _ = (recompute, by_size);
        format!(
            "{} node features, {} edge features, {} computed features",
            self.corpus.all_node_features(false).len(),
            self.corpus.all_edge_features(false).len(),
            self.corpus.all_computed_features().len()
        )
    }
}
