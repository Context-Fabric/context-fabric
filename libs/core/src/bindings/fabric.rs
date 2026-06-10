use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};

use crate::compiled::MappedCompiledCorpus;
use crate::fabric::Fabric;
use crate::feature::{EdgeFeature, FeatureValue, NodeFeature};
use crate::io::{TfData, TfDataContent};

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

/// Resolve a feature specification to a list of feature names, matching TF's
/// `fitemize` (`tf/core/helpers.py`): a single string is split on whitespace,
/// while an iterable is taken element-wise (no splitting of individual items).
fn feature_names(value: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    if let Ok(text) = value.downcast::<PyString>() {
        return Ok(text
            .to_str()?
            .split_whitespace()
            .map(str::to_string)
            .collect());
    }
    if let Ok(items) = value.extract::<Vec<String>>() {
        return Ok(items);
    }
    Ok(vec![value.str()?.to_str()?.to_string()])
}

fn meta_value_to_string(value: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
    if value.is_none() {
        return Ok(None);
    }
    Ok(Some(value.str()?.to_str()?.to_string()))
}

/// Read a single metadata sub-dictionary (`metaData[name]`) into a sorted map.
fn metadata_section(
    metadata: Option<&Bound<'_, PyDict>>,
    name: &str,
) -> PyResult<BTreeMap<String, Option<String>>> {
    let mut map = BTreeMap::new();
    if let Some(md) = metadata {
        if let Some(spec) = md.get_item(name)? {
            if let Ok(dict) = spec.downcast::<PyDict>() {
                for (key, value) in dict.iter() {
                    map.insert(key.extract::<String>()?, meta_value_to_string(&value)?);
                }
            }
        }
    }
    Ok(map)
}

/// Merge the generic (`""`) metadata section with the feature-specific section,
/// mirroring TF's `fMeta.update(metaData.get("", {}))` then `metaData.get(fName)`.
fn merged_meta(
    generic: &BTreeMap<String, Option<String>>,
    metadata: Option<&Bound<'_, PyDict>>,
    name: &str,
) -> PyResult<BTreeMap<String, Option<String>>> {
    let mut merged = generic.clone();
    merged.extend(metadata_section(metadata, name)?);
    Ok(merged)
}

fn is_truthy_meta(value: Option<&Option<String>>) -> bool {
    matches!(
        value.and_then(Option::as_deref),
        Some("True" | "true" | "1" | "yes")
    )
}

fn py_to_feature_value(value: &Bound<'_, PyAny>, value_type: &str) -> PyResult<FeatureValue> {
    if value_type == "int" {
        if let Ok(int) = value.extract::<i64>() {
            return Ok(FeatureValue::Int(int));
        }
    }
    if let Ok(text) = value.extract::<String>() {
        return Ok(FeatureValue::string(text));
    }
    Ok(FeatureValue::string(value.str()?.to_str()?))
}

#[pyclass(name = "Fabric")]
pub(crate) struct PyFabric {
    inner: Fabric,
    silent: Option<String>,
    /// Cumulative set of requested feature names, so `load(..., add=True)` can
    /// extend the visible feature set instead of replacing it (TF semantics).
    requested: Mutex<Vec<String>>,
}

impl PyFabric {
    /// Resolve the output directory for `save`, mirroring TF's `_getWriteLoc`
    /// (`tf/core/fabric.py:896`): `location` defaults to the last configured
    /// location, `module` to the last configured module, joined as `loc/mod`.
    fn resolve_write_dir(
        &self,
        location: Option<&Bound<'_, PyAny>>,
        module: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PathBuf> {
        let loc = match location {
            Some(value) if !value.is_none() => value.str()?.to_str()?.to_string(),
            _ => self
                .inner
                .locations()
                .last()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        };
        let module = match module {
            Some(value) if !value.is_none() => value.str()?.to_str()?.to_string(),
            _ => self.inner.modules().last().cloned().unwrap_or_default(),
        };
        let dir = if loc.is_empty() && module.is_empty() {
            PathBuf::from(".")
        } else if loc.is_empty() {
            PathBuf::from(module)
        } else if module.is_empty() {
            PathBuf::from(loc)
        } else {
            PathBuf::from(loc).join(module)
        };
        Ok(dir)
    }
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
            requested: Mutex::new(Vec::new()),
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
        let _ = silent;
        let requested = feature_names(features)?;
        // TF semantics: `add=True` extends the currently requested feature set
        // (dynamic load), otherwise it replaces it.
        let effective = {
            let mut state = self.requested.lock().unwrap_or_else(|e| e.into_inner());
            if add {
                for name in &requested {
                    if !state.contains(name) {
                        state.push(name.clone());
                    }
                }
            } else {
                *state = requested;
            }
            state.clone()
        };
        let corpus = if effective.is_empty() {
            self.inner.load_all()?
        } else {
            self.inner.load(effective.clone())?
        };
        Ok(PyCorpus::new(corpus, effective))
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (silent=None))]
    fn loadAll(&self, silent: Option<&str>) -> PyResult<PyCorpus> {
        let _ = silent;
        Ok(PyCorpus::new(self.inner.load_all()?, Vec::new()))
    }

    #[pyo3(signature = (silent=None))]
    fn load_all(&self, silent: Option<&str>) -> PyResult<PyCorpus> {
        let _ = silent;
        Ok(PyCorpus::new(self.inner.load_all()?, Vec::new()))
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

    /// Author new TF feature files from in-memory data, matching
    /// `tf.core.fabric.Fabric.save` (`tf/core/fabric.py:561`).
    ///
    /// - `nodeFeatures`: `{feature: {node: value}}`
    /// - `edgeFeatures`: `{feature: {node: {targets}}}` or `{feature: {node: {target: value}}}`
    /// - `metaData`: `{feature: {key: value}}`; the `""` key holds generic metadata
    ///   merged into every feature; `edgeValues=True` marks a valued edge feature;
    ///   keys that are neither node nor edge features become config (metadata-only) features.
    /// - `location` / `module`: output directory, joined as `location/module`
    ///   (defaulting to the Fabric's last location/module).
    #[allow(non_snake_case)]
    #[pyo3(signature = (nodeFeatures=None, edgeFeatures=None, metaData=None, location=None, module=None, silent=None))]
    fn save(
        &self,
        nodeFeatures: Option<&Bound<'_, PyDict>>,
        edgeFeatures: Option<&Bound<'_, PyDict>>,
        metaData: Option<&Bound<'_, PyDict>>,
        location: Option<&Bound<'_, PyAny>>,
        module: Option<&Bound<'_, PyAny>>,
        silent: Option<&str>,
    ) -> PyResult<bool> {
        let _ = silent;
        let write_dir = self.resolve_write_dir(location, module)?;
        std::fs::create_dir_all(&write_dir)?;

        let generic = metadata_section(metaData, "")?;

        let mut node_names: Vec<String> = Vec::new();
        let mut edge_names: Vec<String> = Vec::new();

        // Node features.
        if let Some(node_features) = nodeFeatures {
            for (name_obj, data_obj) in node_features.iter() {
                let name = name_obj.extract::<String>()?;
                node_names.push(name.clone());
                let mut meta = merged_meta(&generic, metaData, &name)?;
                meta.remove("edgeValues");
                let value_type = meta
                    .get("valueType")
                    .and_then(Option::as_deref)
                    .unwrap_or("str")
                    .to_string();
                let data = data_obj.downcast::<PyDict>()?;
                let mut values: HashMap<u32, FeatureValue> = HashMap::new();
                for (node_obj, value_obj) in data.iter() {
                    let node = node_obj.extract::<u32>()?;
                    values.insert(node, py_to_feature_value(&value_obj, &value_type)?);
                }
                let feature = NodeFeature::new(name.clone(), meta, values);
                let mut tf = TfData::new(write_dir.join(format!("{name}.tf")));
                tf.is_edge = Some(false);
                tf.is_config = Some(false);
                tf.data = Some(TfDataContent::Node(feature));
                tf.save_result()?;
            }
        }

        // Edge features.
        if let Some(edge_features) = edgeFeatures {
            for (name_obj, data_obj) in edge_features.iter() {
                let name = name_obj.extract::<String>()?;
                edge_names.push(name.clone());
                let mut meta = merged_meta(&generic, metaData, &name)?;
                let edge_values_flag = is_truthy_meta(meta.get("edgeValues"));
                meta.remove("edgeValues");
                let value_type = meta
                    .get("valueType")
                    .and_then(Option::as_deref)
                    .unwrap_or("str")
                    .to_string();
                let data = data_obj.downcast::<PyDict>()?;
                let mut values: HashMap<u32, Vec<u32>> = HashMap::new();
                let mut edge_values: HashMap<(u32, u32), FeatureValue> = HashMap::new();
                for (source_obj, targets_obj) in data.iter() {
                    let source = source_obj.extract::<u32>()?;
                    if let Ok(target_map) = targets_obj.downcast::<PyDict>() {
                        // {target: value}
                        let mut targets = Vec::new();
                        for (target_obj, value_obj) in target_map.iter() {
                            let target = target_obj.extract::<u32>()?;
                            targets.push(target);
                            edge_values.insert(
                                (source, target),
                                py_to_feature_value(&value_obj, &value_type)?,
                            );
                        }
                        values.insert(source, targets);
                    } else {
                        // set / list / tuple of target nodes
                        let mut targets = Vec::new();
                        for target_obj in targets_obj.try_iter()? {
                            targets.push(target_obj?.extract::<u32>()?);
                        }
                        values.insert(source, targets);
                    }
                }
                let has_values = edge_values_flag || !edge_values.is_empty();
                let feature =
                    EdgeFeature::new_with_values(name.clone(), meta, values, edge_values);
                let mut tf = TfData::new(write_dir.join(format!("{name}.tf")));
                tf.is_edge = Some(true);
                tf.is_config = Some(false);
                tf.edge_values = has_values;
                tf.data = Some(TfDataContent::Edge(feature));
                tf.save_result()?;
            }
        }

        // Config (metadata-only) features: metaData keys other than "" that are
        // not node or edge features.
        if let Some(md) = metaData {
            for (key_obj, value_obj) in md.iter() {
                let key = key_obj.extract::<String>()?;
                if key.is_empty() || node_names.contains(&key) || edge_names.contains(&key) {
                    continue;
                }
                let mut meta = generic.clone();
                if let Ok(spec) = value_obj.downcast::<PyDict>() {
                    for (mk, mv) in spec.iter() {
                        meta.insert(mk.extract::<String>()?, meta_value_to_string(&mv)?);
                    }
                }
                meta.remove("edgeValues");
                let mut tf = TfData::new(write_dir.join(format!("{key}.tf")));
                tf.is_edge = Some(false);
                tf.is_config = Some(true);
                tf.data = Some(TfDataContent::Config {
                    name: key.clone(),
                    metadata: meta,
                });
                tf.save_result()?;
            }
        }

        Ok(true)
    }

    #[allow(non_snake_case)]
    fn openMapped(&self, cache_path: &str) -> PyResult<PyCorpus> {
        Ok(PyCorpus::new(self.inner.open_mapped(cache_path)?, Vec::new()))
    }

    fn open_mapped(&self, cache_path: &str) -> PyResult<PyCorpus> {
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

#[pyclass(name = "Corpus")]
pub(crate) struct PyCorpus {
    corpus: Arc<MappedCompiledCorpus>,
    visible_features: Vec<String>,
}

impl PyCorpus {
    fn new(corpus: MappedCompiledCorpus, visible_features: Vec<String>) -> Self {
        Self {
            corpus: Arc::new(corpus),
            visible_features,
        }
    }

    fn exposes(&self, name: &str) -> bool {
        self.visible_features.is_empty() || self.visible_features.iter().any(|feature| feature == name)
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
        if !self.exposes(name) {
            return None;
        }
        self.corpus
            .metadata()
            .node_feature(name)
            .map(|_| PyNodeFeature::new(name.to_string(), Arc::clone(&self.corpus)))
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (name, warn=true))]
    fn Es(&self, name: &str, warn: bool) -> Option<PyEdgeFeature> {
        let _ = warn;
        if !self.exposes(name) {
            return None;
        }
        self.corpus
            .metadata()
            .edge_feature(name)
            .map(|_| PyEdgeFeature::new(name.to_string(), Arc::clone(&self.corpus)))
    }

    fn node_feature_names(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.corpus
                .all_node_features(true)
                .into_iter()
                .filter(|name| self.exposes(name))
                .collect::<Vec<_>>(),
        )?
        .into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Fall(&self, py: Python<'_>, warp: bool) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.corpus
                .all_node_features(warp)
                .into_iter()
                .filter(|name| self.exposes(name))
                .collect::<Vec<_>>(),
        )?
        .into())
    }

    fn edge_feature_names(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.corpus
                .all_edge_features(true)
                .into_iter()
                .filter(|name| self.exposes(name))
                .collect::<Vec<_>>(),
        )?
        .into())
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Eall(&self, py: Python<'_>, warp: bool) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            self.corpus
                .all_edge_features(warp)
                .into_iter()
                .filter(|name| self.exposes(name))
                .collect::<Vec<_>>(),
        )?
        .into())
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
            self.corpus.metadata().node_feature(feature).is_some()
                || self.corpus.metadata().edge_feature(feature).is_some()
                || self.corpus.metadata().config_feature(feature).is_some()
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
