use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PySet, PyString, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

use crate::compiled::MappedCompiledCorpus;
use crate::config::{OSLOTS, OTYPE};
use crate::explore::FeatureInventory;
use crate::fabric::Fabric;
use crate::feature::{EdgeFeature, FeatureValue, NodeFeature};
use crate::io::{TfData, TfDataContent};

use super::accessors::{PyLocality, PyNodes, PySearch, PyText};
use super::api_ref::ApiRef;
use super::feature_file::{FeatureKind, PyFeatureFile};
use super::features::{
    COMPUTED_NAMES, PyComputeds, PyEdgeFeature, PyEdgeFeatures, PyNodeFeature, PyNodeFeatures,
};
use super::timestamp::{
    AUTO, CacheTarget, PRINT_ONLY, PyTimestamp, SILENT_D, Silent, VERBOSE, write_console,
};
use super::visibility::FeatureVisibility;

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
/// `fitemize` (`tf/core/helpers.py`): a single string is split on whitespace
/// and commas, while an iterable is taken element-wise.
pub(crate) fn feature_names(value: Option<&Bound<'_, PyAny>>) -> PyResult<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_none() {
        return Ok(Vec::new());
    }
    if let Ok(text) = value.downcast::<PyString>() {
        return Ok(text
            .to_str()?
            .split(|ch: char| ch.is_whitespace() || ch == ',')
            .filter(|name| !name.is_empty())
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

/// Entry point for loading a dataset, compatible with `tf.fabric.Fabric`.
///
/// `Fabric(locations, modules)` points at one or more `.tf` directories.
/// `load(features)` / `loadAll()` return the core API (`Corpus`: `F`, `E`, `L`,
/// `N`, `T`, `S`, `C`, ...), which is also kept as `TF.api`. The corpus is
/// compiled once into a memory-mapped `.cfr` cache and opened from there.
#[pyclass(name = "Fabric", module = "cfabric._core")]
pub(crate) struct PyFabric {
    inner: Fabric,
    silent: &'static str,
    /// Message channel behind `info` / `warning` / `error` / `indent` / ...
    tm: Py<PyTimestamp>,
    /// Cumulative requested feature names (`TF.featuresRequested`).
    requested: Vec<String>,
    /// The API made by the most recent `load` / `loadAll` / `openMapped`.
    api: Option<Py<PyCorpus>>,
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

    pub(crate) fn inventory(&self) -> PyResult<FeatureInventory> {
        Ok(self.inner.explore()?)
    }

    /// Sorted names of features shadowed by a later location.
    pub(crate) fn ignored_names(&self) -> PyResult<Vec<String>> {
        Ok(self.inner.ignored_feature_paths()?.into_keys().collect())
    }

    pub(crate) fn timestamp(&self, py: Python<'_>) -> Py<PyTimestamp> {
        self.tm.clone_ref(py)
    }

    fn report_error(&self, py: Python<'_>, msg: &str) -> PyResult<()> {
        self.tm.bind(py).borrow_mut().error_str(py, msg)
    }

    /// The current API, or `AttributeError` before anything has been loaded
    /// (Text-Fabric only attaches API shortcuts to the Fabric after loading).
    fn current_api<'py>(&self, py: Python<'py>, member: &str) -> PyResult<Bound<'py, PyCorpus>> {
        self.api
            .as_ref()
            .map(|api| api.bind(py).clone())
            .ok_or_else(|| {
                PyAttributeError::new_err(format!(
                    "'Fabric' has no attribute '{member}' before features are loaded"
                ))
            })
    }

    /// Names that do not exist in `corpus`, reported like TF's `_loadFeature`.
    fn unknown_features(&self, corpus: &MappedCompiledCorpus, names: &[String]) -> Vec<String> {
        let metadata = corpus.metadata();
        names
            .iter()
            .filter(|name| {
                metadata.node_feature(name).is_none()
                    && metadata.edge_feature(name).is_none()
                    && metadata.config_feature(name).is_none()
            })
            .cloned()
            .collect()
    }

    fn report_unknown(&self, py: Python<'_>, unknown: &[String]) -> PyResult<()> {
        for name in unknown {
            self.report_error(
                py,
                &format!(
                    "Feature \"{name}\" not available in\n{}",
                    self.inner.location_rep()
                ),
            )?;
        }
        self.report_error(py, "Not all features could be loaded / computed")
    }

    fn otext_list(&self, py: Python<'_>, key: &str) -> PyResult<Vec<String>> {
        let Some(api) = &self.api else {
            return Ok(Vec::new());
        };
        let api = api.bind(py).borrow();
        let value = api
            .corpus
            .config_feature("otext")?
            .and_then(|otext| otext.metadata_value(key).ok().flatten().map(str::to_string));
        Ok(value
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[pymethods]
impl PyFabric {
    /// Point a Fabric at one or more dataset locations.
    ///
    /// Args:
    ///     locations: Directory (or directories) holding `.tf` files.
    ///     modules: Subdirectories of each location to load from.
    ///     silent: Verbosity: `verbose`, `auto` (default), `terse`, or `deep`.
    ///     _withGc: Accepted for Text-Fabric compatibility; has no effect.
    #[new]
    #[allow(non_snake_case)]
    #[pyo3(signature = (locations=None, modules=None, silent=Silent::DEFAULT, _withGc=false), text_signature = "(locations=None, modules=None, silent='auto', _withGc=False)")]
    fn new(
        py: Python<'_>,
        locations: Option<&Bound<'_, PyAny>>,
        modules: Option<&Bound<'_, PyAny>>,
        silent: Silent,
        _withGc: bool,
    ) -> PyResult<Self> {
        let silent = silent.0;
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
            tm: Py::new(py, PyTimestamp::with_silent(silent))?,
            requested: Vec::new(),
            api: None,
        })
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.tm)?;
        if let Some(api) = &self.api {
            visit.call(api)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.api = None;
    }

    /// Startup banner (`This is Context-Fabric <version>`).
    #[getter]
    fn banner(&self) -> &str {
        self.inner.banner()
    }

    /// The Context-Fabric version.
    #[getter]
    fn version(&self) -> &str {
        self.inner.version()
    }

    /// Whether the Fabric is in a usable state.
    #[getter]
    fn good(&self) -> bool {
        self.inner.good()
    }

    /// The verbosity given at construction.
    #[getter]
    fn silent(&self) -> &'static str {
        self.silent
    }

    /// The message channel (`tf.core.timestamp.Timestamp` equivalent).
    #[getter]
    #[allow(non_snake_case)]
    fn tmObj(&self, py: Python<'_>) -> Py<PyTimestamp> {
        self.tm.clone_ref(py)
    }

    /// The configured locations.
    #[getter]
    fn locations(&self) -> Vec<String> {
        self.inner
            .locations()
            .iter()
            .map(|path| path.display().to_string())
            .collect()
    }

    /// The configured modules.
    #[getter]
    fn modules(&self) -> Vec<String> {
        self.inner.modules().to_vec()
    }

    /// The configured locations, one per line.
    #[getter]
    #[allow(non_snake_case)]
    fn locationRep(&self) -> String {
        self.inner.location_rep()
    }

    /// Every feature of the dataset, keyed by name: the `.tf` features of all
    /// locations (later locations win) plus the precomputed `__<name>__` blocks.
    #[getter]
    fn features(&self, py: Python<'_>) -> PyResult<PyObject> {
        let inventory = self.inventory()?;
        let api = self.api.as_ref().map(|api| api.bind(py).borrow());
        let loaded = |name: &str| api.as_ref().is_some_and(|api| api.exposes_data(name));
        let result = PyDict::new(py);
        let kinds = inventory
            .nodes
            .iter()
            .map(|name| (name, FeatureKind::Node))
            .chain(inventory.edges.iter().map(|name| (name, FeatureKind::Edge)))
            .chain(
                inventory
                    .configs
                    .iter()
                    .map(|name| (name, FeatureKind::Config)),
            );
        for (name, kind) in kinds {
            let Some(path) = inventory.paths.get(name).and_then(|paths| paths.last()) else {
                continue;
            };
            let meta = inventory.metadata.get(name).cloned().unwrap_or_default();
            let entry = PyFeatureFile::new(path, kind, meta, loaded(name));
            result.set_item(name, Py::new(py, entry)?)?;
        }
        if let Some(warp) = inventory.paths.get(OTYPE).and_then(|paths| paths.last()) {
            let dir = warp.parent().map(PathBuf::from).unwrap_or_default();
            for computed in COMPUTED_NAMES {
                let key = format!("__{computed}__");
                let path = dir.join(format!("{key}.tf"));
                let entry = PyFeatureFile::new(
                    &path,
                    FeatureKind::Computed(computed.to_string()),
                    BTreeMap::new(),
                    api.is_some(),
                );
                result.set_item(key, Py::new(py, entry)?)?;
            }
        }
        Ok(result.into())
    }

    /// Feature names requested by `load` calls so far, in request order.
    #[getter]
    #[allow(non_snake_case)]
    fn featuresRequested(&self) -> Vec<String> {
        self.requested.clone()
    }

    /// Features found in more than one location, with the paths of the copies
    /// that are ignored because a later location also has them.
    #[getter]
    #[allow(non_snake_case)]
    fn featuresIgnored(&self) -> PyResult<BTreeMap<String, Vec<String>>> {
        Ok(self
            .inner
            .ignored_feature_paths()?
            .into_iter()
            .map(|(name, paths)| {
                (
                    name,
                    paths
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect(),
                )
            })
            .collect())
    }

    /// Names of the features that are ignored (see `featuresIgnored`).
    #[getter]
    fn ignored(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(PyTuple::new(py, self.ignored_names()?)?.unbind())
    }

    /// Always `False`: datasets are loaded with their warp features.
    #[getter]
    #[allow(non_snake_case)]
    fn featuresOnly(&self) -> bool {
        false
    }

    /// Feature categories of the dataset: a dict with keys `computeds`,
    /// `configs`, `edges`, `nodes`, each a set of feature names.
    #[getter]
    #[allow(non_snake_case)]
    fn featureSets(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        let sets = PyDict::new(py);
        for (key, names) in feature_categories(&self.inventory()?) {
            sets.set_item(key, PySet::new(py, names)?)?;
        }
        Ok(sets.unbind())
    }

    /// Section types from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypes(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        self.otext_list(py, "sectionTypes")
    }

    /// Section features from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeats(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        self.otext_list(py, "sectionFeatures")
    }

    /// Structure types from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn structureTypes(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        self.otext_list(py, "structureTypes")
    }

    /// Structure features from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn structureFeats(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        self.otext_list(py, "structureFeatures")
    }

    /// The API made by the most recent load.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<Py<PyCorpus>> {
        Ok(self.current_api(py, "api")?.unbind())
    }

    /// Load features and return the API.
    ///
    /// Args:
    ///     features: Feature names, as a whitespace/comma separated string or
    ///         an iterable. The warp features and the features needed by the
    ///         T-API are always loaded.
    ///     add: Add the features to the current API instead of making a new one.
    ///     silent: Verbosity for this call.
    ///
    /// Returns:
    ///     The new API, or `True` when `add=True`; `False` if a feature does not
    ///     exist in the dataset.
    #[pyo3(signature = (features, add=false, silent=Silent::DEFAULT), text_signature = "($self, features, add=False, silent='auto')")]
    fn load(
        slf: &Bound<'_, Self>,
        features: &Bound<'_, PyAny>,
        add: bool,
        silent: Silent,
    ) -> PyResult<PyObject> {
        let py = slf.py();
        let _ = silent;
        let requested = feature_names(Some(features))?;
        let existing = if add {
            slf.borrow().api.as_ref().map(|api| api.clone_ref(py))
        } else {
            None
        };
        if let Some(api) = existing {
            let unknown = slf
                .borrow()
                .unknown_features(&api.bind(py).borrow().corpus, &requested);
            if !unknown.is_empty() {
                slf.borrow().report_unknown(py, &unknown)?;
                return Ok(PyBool::new(py, false).to_owned().into_any().unbind());
            }
            api.bind(py)
                .borrow()
                .visibility
                .extend(requested.iter().cloned());
            slf.borrow_mut().requested.extend(requested);
            return Ok(PyBool::new(py, true).to_owned().into_any().unbind());
        }
        let corpus = slf.borrow().inner.load(requested.clone())?;
        let unknown = slf.borrow().unknown_features(&corpus, &requested);
        if !unknown.is_empty() {
            slf.borrow().report_unknown(py, &unknown)?;
            return Ok(PyBool::new(py, false).to_owned().into_any().unbind());
        }
        let visibility = FeatureVisibility::requested(&corpus, &requested)?;
        let api = PyCorpus::create(py, corpus, visibility, Some(slf.clone().unbind()))?;
        {
            let mut fabric = slf.borrow_mut();
            if add {
                fabric.requested.extend(requested);
            } else {
                fabric.requested = requested;
            }
            fabric.api = Some(api.clone_ref(py));
        }
        if add {
            return Ok(PyBool::new(py, true).to_owned().into_any().unbind());
        }
        Ok(api.into_any())
    }

    /// Load all node and edge features and return the API.
    ///
    /// Args:
    ///     silent: Verbosity for this call.
    #[allow(non_snake_case)]
    #[pyo3(signature = (silent=Silent::DEFAULT), text_signature = "($self, silent='auto')")]
    fn loadAll(slf: &Bound<'_, Self>, silent: Silent) -> PyResult<Py<PyCorpus>> {
        let py = slf.py();
        let _ = silent;
        let corpus = slf.borrow().inner.load_all()?;
        let api = PyCorpus::create(
            py,
            corpus,
            FeatureVisibility::all(),
            Some(slf.clone().unbind()),
        )?;
        slf.borrow_mut().api = Some(api.clone_ref(py));
        Ok(api)
    }

    /// Compile the dataset into a memory-mapped `.cfr` cache file.
    ///
    /// Args:
    ///     output_path: Where to write the cache.
    ///     features: Accepted for symmetry with `load`; the cache holds all features.
    ///     silent: Verbosity for this call.
    #[pyo3(signature = (output_path, features=None, silent=Silent::DEFAULT), text_signature = "($self, output_path, features=None, silent='auto')")]
    fn compile(
        &self,
        output_path: &str,
        features: Option<&Bound<'_, PyAny>>,
        silent: Silent,
    ) -> PyResult<bool> {
        let _ = silent;
        self.inner.compile(output_path, feature_names(features)?)?;
        Ok(true)
    }

    /// Categorize the features of the dataset (also available as `featureSets`).
    ///
    /// Args:
    ///     silent: Verbosity for this call.
    ///     show: Return the categories (otherwise return `None`).
    ///
    /// Returns:
    ///     A dict with keys `computeds`, `configs`, `edges`, `nodes`, each a sorted
    ///     tuple of feature names, or `None` when `show` is false.
    #[pyo3(signature = (silent=Silent::DEFAULT, show=true), text_signature = "($self, silent='auto', show=True)")]
    fn explore(&self, py: Python<'_>, silent: Silent, show: bool) -> PyResult<Option<PyObject>> {
        let categories = feature_categories(&self.inventory()?);
        if matches!(silent.0, VERBOSE | AUTO) {
            let count = |key: &str| categories.get(key).map_or(0, Vec::len);
            self.tm.bind(py).borrow_mut().emit_text(
                py,
                &format!(
                    "Feature overview: {} for nodes; {} for edges; {} configs; {} computed",
                    count("nodes"),
                    count("edges"),
                    count("configs"),
                    count("computeds"),
                ),
                false,
            )?;
        }
        if !show {
            return Ok(None);
        }
        let shown = PyDict::new(py);
        for (key, names) in categories {
            shown.set_item(key, PyTuple::new(py, names)?)?;
        }
        Ok(Some(shown.into_any().unbind()))
    }

    /// Delete the compiled `.cfr` cache, forcing a recompile on the next load.
    #[allow(non_snake_case)]
    fn clearCache(&self) -> PyResult<()> {
        let path = self.inner.cfr_cache_path();
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// Show (or return) information about loaded features; see `Corpus.isLoaded`.
    #[allow(non_snake_case)]
    #[pyo3(
        signature = (features=None, pretty=true, valueType=true, path=false, meta=MetaSelection::description()),
        text_signature = "($self, features=None, pretty=True, valueType=True, path=False, meta='description')"
    )]
    fn isLoaded(
        &self,
        py: Python<'_>,
        features: Option<&Bound<'_, PyAny>>,
        pretty: bool,
        valueType: bool,
        path: bool,
        meta: MetaSelection,
    ) -> PyResult<Option<PyObject>> {
        let inventory = self.inventory()?;
        let api = self.api.as_ref().map(|api| api.bind(py).borrow());
        let options = LoadedReport {
            pretty,
            value_type: valueType,
            path,
            meta,
        };
        loaded_report(py, api.as_deref(), &inventory, features, &options)
    }

    /// Load features into the current API if they are not loaded yet; see
    /// `Corpus.ensureLoaded`.
    #[allow(non_snake_case)]
    fn ensureLoaded(&self, py: Python<'_>, features: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Ok(self
            .current_api(py, "ensureLoaded")?
            .call_method1("ensureLoaded", (features,))?
            .unbind())
    }

    /// Export the API members into `scope`; see `Corpus.makeAvailableIn`.
    #[allow(non_snake_case)]
    fn makeAvailableIn(&self, py: Python<'_>, scope: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        Ok(self
            .current_api(py, "makeAvailableIn")?
            .call_method1("makeAvailableIn", (scope,))?
            .unbind())
    }

    /// Show the size of the loaded features; see `Corpus.footprint`.
    #[pyo3(signature = (recompute=false, bySize=true))]
    #[allow(non_snake_case)]
    fn footprint(&self, py: Python<'_>, recompute: bool, bySize: bool) -> PyResult<()> {
        self.current_api(py, "footprint")?
            .call_method1("footprint", (recompute, bySize))?;
        Ok(())
    }

    /// Write new features to `.tf` files, like `tf.core.fabric.Fabric.save`.
    ///
    /// Args:
    ///     nodeFeatures: `{feature: {node: value}}`.
    ///     edgeFeatures: `{feature: {node: {targets}}}` or
    ///         `{feature: {node: {target: value}}}`.
    ///     metaData: `{feature: {key: value}}`; the `""` key holds metadata for
    ///         every feature; `edgeValues=True` marks a valued edge feature; keys
    ///         that are neither node nor edge features become config features.
    ///     location: Output directory (default: the last location).
    ///     module: Subdirectory of `location` (default: the last module).
    ///     silent: Verbosity for this call.
    ///
    /// Returns:
    ///     `True` when the files were written.
    #[allow(non_snake_case)]
    #[pyo3(signature = (nodeFeatures=None, edgeFeatures=None, metaData=None, location=None, module=None, silent=Silent::DEFAULT), text_signature = "($self, nodeFeatures=None, edgeFeatures=None, metaData=None, location=None, module=None, silent='auto')")]
    fn save(
        &self,
        nodeFeatures: Option<&Bound<'_, PyDict>>,
        edgeFeatures: Option<&Bound<'_, PyDict>>,
        metaData: Option<&Bound<'_, PyDict>>,
        location: Option<&Bound<'_, PyAny>>,
        module: Option<&Bound<'_, PyAny>>,
        silent: Silent,
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
                let feature = EdgeFeature::new_with_values(name.clone(), meta, values, edge_values);
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

    /// Open a compiled `.cfr` cache (see `compile`) and return its API.
    ///
    /// Args:
    ///     cache_path: Path of the `.cfr` file.
    #[allow(non_snake_case)]
    fn openMapped(slf: &Bound<'_, Self>, cache_path: &str) -> PyResult<Py<PyCorpus>> {
        let py = slf.py();
        let corpus = slf.borrow().inner.open_mapped(cache_path)?;
        let api = PyCorpus::create(
            py,
            corpus,
            FeatureVisibility::all(),
            Some(slf.clone().unbind()),
        )?;
        slf.borrow_mut().api = Some(api.clone_ref(py));
        Ok(api)
    }

    /// Show an informational message (TF `info`).
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=false), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=False)")]
    fn info(
        &self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        self.tm
            .bind(py)
            .borrow_mut()
            .info(py, msg, tm, nl, cache, force)
    }

    /// Show a warning (TF `warning`).
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=false), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=False)")]
    fn warning(
        &self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        self.tm
            .bind(py)
            .borrow_mut()
            .warning(py, msg, tm, nl, cache, force)
    }

    /// Show an error on standard error (TF `error`).
    #[pyo3(signature = (msg, tm=true, nl=true, cache=PRINT_ONLY, force=true), text_signature = "($self, msg, tm=True, nl=True, cache=0, force=True)")]
    fn error(
        &self,
        py: Python<'_>,
        msg: &Bound<'_, PyAny>,
        tm: bool,
        nl: bool,
        cache: CacheTarget<'_>,
        force: bool,
    ) -> PyResult<()> {
        self.tm
            .bind(py)
            .borrow_mut()
            .error(py, msg, tm, nl, cache, force)
    }

    /// Set the indentation level of messages (TF `indent`).
    #[pyo3(signature = (level=None, reset=false))]
    fn indent(
        &self,
        py: Python<'_>,
        level: Option<&Bound<'_, PyAny>>,
        reset: bool,
    ) -> PyResult<()> {
        self.tm.bind(py).borrow_mut().indent(level, reset)
    }

    /// The current verbosity mode (TF `isSilent`).
    #[allow(non_snake_case)]
    fn isSilent(&self, py: Python<'_>) -> &'static str {
        self.tm.bind(py).borrow().isSilent()
    }

    /// Set the verbosity mode (TF `setSilent`).
    #[allow(non_snake_case)]
    fn setSilent(&self, py: Python<'_>, silent: Option<&Bound<'_, PyAny>>) -> PyResult<()> {
        self.tm.bind(py).borrow_mut().setSilent(silent)
    }

    /// Suppress informational messages, and warnings too if `deep` (TF `silentOn`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (deep=false))]
    fn silentOn(&self, py: Python<'_>, deep: bool) {
        self.tm.bind(py).borrow_mut().silentOn(deep);
    }

    /// Restore the verbosity in effect before `silentOn` (TF `silentOff`).
    #[allow(non_snake_case)]
    fn silentOff(&self, py: Python<'_>) {
        self.tm.bind(py).borrow_mut().silentOff();
    }

    /// Clear the message cache (TF `reset`).
    fn reset(&self, py: Python<'_>) {
        self.tm.bind(py).borrow_mut().reset();
    }

    /// Flush the message cache, or return it as a string (TF `cache`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (_asString=false))]
    fn cache(&self, py: Python<'_>, _asString: bool) -> PyResult<Option<String>> {
        self.tm.bind(py).borrow_mut().cache(py, _asString)
    }
}

/// Sorted feature names per category, as TF's `explore` reports them; the
/// computed blocks are named `__<name>__`.
fn feature_categories(inventory: &FeatureInventory) -> BTreeMap<&'static str, Vec<String>> {
    let sorted = |names: &[String]| {
        let mut names = names.to_vec();
        names.sort();
        names
    };
    BTreeMap::from([
        (
            "computeds",
            COMPUTED_NAMES
                .iter()
                .map(|name| format!("__{name}__"))
                .collect(),
        ),
        ("configs", sorted(&inventory.configs)),
        ("edges", sorted(&inventory.edges)),
        ("nodes", sorted(&inventory.nodes)),
    ])
}

/// The API members a `Corpus` owns, created once so that repeated access
/// (`api.S`, `api.F`, ...) returns the same, stateful object, as in TF.
struct Members {
    f: Py<PyNodeFeatures>,
    e: Py<PyEdgeFeatures>,
    c: Py<PyComputeds>,
    l: Py<PyLocality>,
    n: Py<PyNodes>,
    t: Py<PyText>,
    s: Py<PySearch>,
}

/// TF `API_REFS` (`tf/core/api.py`): documentation group of each exported
/// member, as `(member, (head, sub, ref))`.
const API_REFS: [(&str, (&str, &str, &str)); 27] = [
    ("AllComputeds", ("Computed", "computedall", "computed-data")),
    ("AllEdges", ("Features", "edgeall", "edge-features")),
    ("AllFeatures", ("Features", "nodeall", "node-features")),
    ("C", ("Computed", "computed", "computed-data")),
    ("Call", ("Computed", "computedall", "computed-data")),
    ("Computed", ("Computed", "computed", "computed-data")),
    (
        "ComputedString",
        ("Computed", "computedstr", "computed-data"),
    ),
    ("Cs", ("Computed", "computedstr", "computed-data")),
    ("E", ("Features", "edge", "edge-features")),
    ("Eall", ("Features", "edgeall", "edge-features")),
    ("Edge", ("Features", "edge", "edge-features")),
    ("EdgeString", ("Features", "edgestr", "edge-features")),
    ("Es", ("Features", "edgestr", "edge-features")),
    ("F", ("Features", "node", "node-features")),
    ("Fall", ("Features", "nodeall", "node-features")),
    ("Feature", ("Features", "node", "node-features")),
    ("FeatureString", ("Features", "nodestr", "node-features")),
    ("Fs", ("Features", "nodestr", "node-features")),
    ("L", ("Locality", "locality", "locality")),
    ("Locality", ("Locality", "locality", "locality")),
    ("N", ("Nodes", "nodes", "navigating-nodes")),
    ("Nodes", ("Nodes", "nodes", "navigating-nodes")),
    ("S", ("Search", "search", "search")),
    ("Search", ("Search", "search", "search")),
    ("T", ("Text", "text", "text")),
    ("TF", ("Fabric", "fabric", "loading")),
    ("Text", ("Text", "text", "text")),
];

/// The core API of a loaded dataset (TF `api`): `F`, `E`, `C`, `L`, `N`, `T`,
/// `S`, the `TF` Fabric, and feature-inventory helpers.
#[pyclass(name = "Corpus", module = "cfabric._core", weakref)]
pub(crate) struct PyCorpus {
    corpus: Arc<MappedCompiledCorpus>,
    visibility: Arc<FeatureVisibility>,
    fabric: Option<Py<PyFabric>>,
    members: OnceLock<Members>,
}

impl PyCorpus {
    /// Wrap a mapped corpus as an API object owned by `fabric`.
    pub(crate) fn create(
        py: Python<'_>,
        corpus: MappedCompiledCorpus,
        visibility: FeatureVisibility,
        fabric: Option<Py<PyFabric>>,
    ) -> PyResult<Py<Self>> {
        let corpus = Arc::new(corpus);
        let visibility = Arc::new(visibility);
        let silent = fabric
            .as_ref()
            .map_or(SILENT_D, |fabric| fabric.bind(py).borrow().silent);
        let api = Py::new(
            py,
            Self {
                corpus: Arc::clone(&corpus),
                visibility: Arc::clone(&visibility),
                fabric,
                members: OnceLock::new(),
            },
        )?;
        let api_ref = ApiRef::new(api.bind(py).as_any())?;
        let members = Members {
            f: Py::new(
                py,
                PyNodeFeatures::new(
                    Arc::clone(&corpus),
                    Arc::clone(&visibility),
                    api_ref.clone_ref(py),
                ),
            )?,
            e: Py::new(
                py,
                PyEdgeFeatures::new(
                    Arc::clone(&corpus),
                    Arc::clone(&visibility),
                    api_ref.clone_ref(py),
                ),
            )?,
            c: Py::new(
                py,
                PyComputeds::new(Arc::clone(&corpus), api_ref.clone_ref(py)),
            )?,
            l: Py::new(
                py,
                PyLocality::new(Arc::clone(&corpus), api_ref.clone_ref(py)),
            )?,
            n: Py::new(py, PyNodes::new(Arc::clone(&corpus), api_ref.clone_ref(py)))?,
            t: Py::new(py, PyText::new(Arc::clone(&corpus), api_ref.clone_ref(py)))?,
            s: Py::new(py, PySearch::new(Arc::clone(&corpus), api_ref, silent))?,
        };
        let _ = api.bind(py).borrow().members.set(members);
        Ok(api)
    }

    fn members(&self) -> &Members {
        self.members
            .get()
            .expect("Corpus members are set at construction")
    }

    /// Whether node or edge feature `name` is loaded and exposed.
    fn exposes_data(&self, name: &str) -> bool {
        let metadata = self.corpus.metadata();
        (metadata.node_feature(name).is_some() || metadata.edge_feature(name).is_some())
            && self.visibility.exposes(name)
    }

    fn exposed_node_feature(&self, name: &str) -> bool {
        self.corpus.metadata().node_feature(name).is_some() && self.visibility.exposes(name)
    }

    fn exposed_edge_feature(&self, name: &str) -> bool {
        self.corpus.metadata().edge_feature(name).is_some() && self.visibility.exposes(name)
    }

    fn report(&self, py: Python<'_>, msg: &str, error: bool) -> PyResult<()> {
        let Some(fabric) = &self.fabric else {
            return Ok(());
        };
        let tm = fabric.bind(py).borrow().timestamp(py);
        let mut tm = tm.bind(py).borrow_mut();
        if error {
            tm.error_str(py, msg)
        } else {
            tm.warn_str(py, msg)
        }
    }

    fn inventory(&self, py: Python<'_>) -> PyResult<FeatureInventory> {
        match &self.fabric {
            Some(fabric) => fabric.bind(py).borrow().inventory(),
            None => Ok(FeatureInventory::default()),
        }
    }

    fn sorted_names(names: impl IntoIterator<Item = String>) -> Vec<String> {
        let mut names: Vec<String> = names.into_iter().collect();
        names.sort();
        names
    }
}

#[pymethods]
impl PyCorpus {
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(fabric) = &self.fabric {
            visit.call(fabric)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.fabric = None;
    }

    /// Node features: `F.<feature>.v(node)`, `F.<feature>.s(value)`, ...
    #[getter]
    #[allow(non_snake_case)]
    fn F(&self, py: Python<'_>) -> Py<PyNodeFeatures> {
        self.members().f.clone_ref(py)
    }

    /// Alias of `F`.
    #[getter]
    #[allow(non_snake_case)]
    fn Feature(&self, py: Python<'_>) -> Py<PyNodeFeatures> {
        self.F(py)
    }

    /// Edge features: `E.<feature>.f(node)`, `E.<feature>.t(node)`, ...
    #[getter]
    #[allow(non_snake_case)]
    fn E(&self, py: Python<'_>) -> Py<PyEdgeFeatures> {
        self.members().e.clone_ref(py)
    }

    /// Alias of `E`.
    #[getter]
    #[allow(non_snake_case)]
    fn Edge(&self, py: Python<'_>) -> Py<PyEdgeFeatures> {
        self.E(py)
    }

    /// Precomputed data: `C.levels.data`, `C.order.data`, ...
    #[getter]
    #[allow(non_snake_case)]
    fn C(&self, py: Python<'_>) -> Py<PyComputeds> {
        self.members().c.clone_ref(py)
    }

    /// Alias of `C`.
    #[getter]
    #[allow(non_snake_case)]
    fn Computed(&self, py: Python<'_>) -> Py<PyComputeds> {
        self.C(py)
    }

    /// Locality: embedders, embeddees, neighbours (`L.u`, `L.d`, `L.n`, ...).
    #[getter]
    #[allow(non_snake_case)]
    fn L(&self, py: Python<'_>) -> Py<PyLocality> {
        self.members().l.clone_ref(py)
    }

    /// Alias of `L`.
    #[getter]
    #[allow(non_snake_case)]
    fn Locality(&self, py: Python<'_>) -> Py<PyLocality> {
        self.L(py)
    }

    /// Node navigation and ordering (`N.walk`, `N.sortNodes`, ...).
    #[getter]
    #[allow(non_snake_case)]
    fn N(&self, py: Python<'_>) -> Py<PyNodes> {
        self.members().n.clone_ref(py)
    }

    /// Alias of `N`.
    #[getter]
    #[allow(non_snake_case)]
    fn Nodes(&self, py: Python<'_>) -> Py<PyNodes> {
        self.N(py)
    }

    /// Text and sections (`T.text`, `T.sectionFromNode`, ...).
    #[getter]
    #[allow(non_snake_case)]
    fn T(&self, py: Python<'_>) -> Py<PyText> {
        self.members().t.clone_ref(py)
    }

    /// Alias of `T`.
    #[getter]
    #[allow(non_snake_case)]
    fn Text(&self, py: Python<'_>) -> Py<PyText> {
        self.T(py)
    }

    /// Template search (`S.search`, `S.study`, `S.fetch`, ...).
    #[getter]
    #[allow(non_snake_case)]
    fn S(&self, py: Python<'_>) -> Py<PySearch> {
        self.members().s.clone_ref(py)
    }

    /// Alias of `S`.
    #[getter]
    #[allow(non_snake_case)]
    fn Search(&self, py: Python<'_>) -> Py<PySearch> {
        self.S(py)
    }

    /// The Fabric that loaded this API.
    #[getter]
    #[allow(non_snake_case)]
    fn TF(&self, py: Python<'_>) -> Option<Py<PyFabric>> {
        self.fabric.as_ref().map(|fabric| fabric.clone_ref(py))
    }

    /// Names of features that were found but ignored because a later
    /// location also has them.
    #[getter]
    fn ignored(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let names = match &self.fabric {
            Some(fabric) => fabric.bind(py).borrow().ignored_names()?,
            None => Vec::new(),
        };
        Ok(PyTuple::new(py, names)?.unbind())
    }

    /// Always `False`: the warp features and T-API are always available.
    #[getter]
    #[allow(non_snake_case)]
    fn featuresOnly(&self) -> bool {
        false
    }

    /// The node feature object for `fName`, for names that are not valid
    /// Python identifiers.
    ///
    /// Args:
    ///     fName: Feature name.
    ///     warn: Report an error if the feature is not loaded.
    ///
    /// Returns:
    ///     The feature, or `None` if it is not loaded.
    #[allow(non_snake_case)]
    #[pyo3(signature = (fName, warn=true))]
    fn Fs(&self, py: Python<'_>, fName: &str, warn: bool) -> PyResult<Option<Py<PyNodeFeature>>> {
        if !self.exposed_node_feature(fName) {
            if warn {
                self.report(py, &format!("Node feature \"{fName}\" not loaded"), true)?;
            }
            return Ok(None);
        }
        self.members()
            .f
            .bind(py)
            .borrow()
            .handle(py, fName)
            .map(Some)
    }

    /// Alias of `Fs`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (fName, warn=true))]
    fn FeatureString(
        &self,
        py: Python<'_>,
        fName: &str,
        warn: bool,
    ) -> PyResult<Option<Py<PyNodeFeature>>> {
        self.Fs(py, fName, warn)
    }

    /// The edge feature object for `fName`.
    ///
    /// Args:
    ///     fName: Feature name.
    ///     warn: Report an error if the feature is not loaded.
    ///
    /// Returns:
    ///     The feature, or `None` if it is not loaded.
    #[allow(non_snake_case)]
    #[pyo3(signature = (fName, warn=true))]
    fn Es(&self, py: Python<'_>, fName: &str, warn: bool) -> PyResult<Option<Py<PyEdgeFeature>>> {
        if !self.exposed_edge_feature(fName) {
            if warn {
                self.report(py, &format!("Edge feature \"{fName}\" not loaded"), true)?;
            }
            return Ok(None);
        }
        self.members()
            .e
            .bind(py)
            .borrow()
            .handle(py, fName)
            .map(Some)
    }

    /// Alias of `Es`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (fName, warn=true))]
    fn EdgeString(
        &self,
        py: Python<'_>,
        fName: &str,
        warn: bool,
    ) -> PyResult<Option<Py<PyEdgeFeature>>> {
        self.Es(py, fName, warn)
    }

    /// The computed data object for `fName` (`levels`, `order`, ...).
    ///
    /// Args:
    ///     fName: Name of the computed data.
    ///     warn: Report an error if it does not exist.
    ///
    /// Returns:
    ///     The computed data, or `None` if it does not exist.
    #[allow(non_snake_case)]
    #[pyo3(signature = (fName, warn=true))]
    fn Cs(&self, py: Python<'_>, fName: &str, warn: bool) -> PyResult<Option<PyObject>> {
        if !COMPUTED_NAMES.contains(&fName) {
            if warn {
                self.report(
                    py,
                    &format!("Computed feature \"{fName}\" not loaded"),
                    true,
                )?;
            }
            return Ok(None);
        }
        self.members().c.bind(py).borrow().get(py, fName).map(Some)
    }

    /// Alias of `Cs`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (fName, warn=true))]
    fn ComputedString(
        &self,
        py: Python<'_>,
        fName: &str,
        warn: bool,
    ) -> PyResult<Option<PyObject>> {
        self.Cs(py, fName, warn)
    }

    /// Sorted names of the loaded node features.
    ///
    /// Args:
    ///     warp: Include the warp feature `otype`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Fall(&self, warp: bool) -> Vec<String> {
        Self::sorted_names(
            self.corpus
                .all_node_features(true)
                .into_iter()
                .filter(|name| (warp || name != OTYPE) && self.visibility.exposes(name)),
        )
    }

    /// Alias of `Fall`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn AllFeatures(&self, warp: bool) -> Vec<String> {
        self.Fall(warp)
    }

    /// Sorted names of the loaded edge features.
    ///
    /// Args:
    ///     warp: Include the warp feature `oslots`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn Eall(&self, warp: bool) -> Vec<String> {
        Self::sorted_names(
            self.corpus
                .all_edge_features(true)
                .into_iter()
                .filter(|name| (warp || name != OSLOTS) && self.visibility.exposes(name)),
        )
    }

    /// Alias of `Eall`.
    #[allow(non_snake_case)]
    #[pyo3(signature = (warp=true))]
    fn AllEdges(&self, warp: bool) -> Vec<String> {
        self.Eall(warp)
    }

    /// Sorted names of the computed data blocks.
    #[allow(non_snake_case)]
    fn Call(&self) -> Vec<String> {
        Self::sorted_names(COMPUTED_NAMES.iter().map(|name| name.to_string()))
    }

    /// Alias of `Call`.
    #[allow(non_snake_case)]
    fn AllComputeds(&self) -> Vec<String> {
        self.Call()
    }

    /// Show information about loaded features, like TF's `api.isLoaded`.
    ///
    /// Args:
    ///     features: Feature names (string or iterable); default: all features.
    ///     pretty: Print an overview instead of returning it.
    ///     valueType: When pretty, show the value type.
    ///     path: When pretty, show the feature's directory.
    ///     meta: When pretty, which metadata keys to show (`True` for all).
    ///
    /// Returns:
    ///     `None` when `pretty`; otherwise a dict mapping each feature name to
    ///     `None` (not loaded) or a dict with `kind`, `type`, `meta`, `source`,
    ///     and `edgeValues`.
    #[allow(non_snake_case)]
    #[pyo3(
        signature = (features=None, pretty=true, valueType=true, path=false, meta=MetaSelection::description()),
        text_signature = "($self, features=None, pretty=True, valueType=True, path=False, meta='description')"
    )]
    fn isLoaded(
        &self,
        py: Python<'_>,
        features: Option<&Bound<'_, PyAny>>,
        pretty: bool,
        valueType: bool,
        path: bool,
        meta: MetaSelection,
    ) -> PyResult<Option<PyObject>> {
        let inventory = self.inventory(py)?;
        let options = LoadedReport {
            pretty,
            value_type: valueType,
            path,
            meta,
        };
        loaded_report(py, Some(self), &inventory, features, &options)
    }

    /// Alias of `isLoaded`.
    #[allow(non_snake_case)]
    #[pyo3(
        signature = (features=None, pretty=true, valueType=true, path=false, meta=MetaSelection::description()),
        text_signature = "($self, features=None, pretty=True, valueType=True, path=False, meta='description')"
    )]
    fn loadLog(
        &self,
        py: Python<'_>,
        features: Option<&Bound<'_, PyAny>>,
        pretty: bool,
        valueType: bool,
        path: bool,
        meta: MetaSelection,
    ) -> PyResult<Option<PyObject>> {
        self.isLoaded(py, features, pretty, valueType, path, meta)
    }

    /// Make features available in this API if they are not yet.
    ///
    /// Args:
    ///     features: Feature names (string or iterable).
    ///
    /// Returns:
    ///     The set of requested features that are now loaded.
    #[allow(non_snake_case)]
    fn ensureLoaded(&self, py: Python<'_>, features: &Bound<'_, PyAny>) -> PyResult<Py<PySet>> {
        let mut loaded = BTreeSet::new();
        let requested: BTreeSet<String> = feature_names(Some(features))?.into_iter().collect();
        let metadata = self.corpus.metadata();
        for name in requested {
            if metadata.node_feature(&name).is_some() || metadata.edge_feature(&name).is_some() {
                loaded.insert(name);
            } else {
                self.report(
                    py,
                    &format!("Cannot load feature \"{name}\": not in dataset"),
                    false,
                )?;
            }
        }
        self.visibility.extend(loaded.iter().cloned());
        if let Some(fabric) = &self.fabric {
            let mut fabric = fabric.bind(py).borrow_mut();
            for name in &loaded {
                if !fabric.requested.contains(name) {
                    fabric.requested.push(name.clone());
                }
            }
        }
        Ok(PySet::new(py, &loaded)?.unbind())
    }

    /// Export the capitalized API members (`F`, `E`, `L`, `T`, `Fs`, `TF`,
    /// `Feature`, ...) into `scope`, typically `globals()`.
    ///
    /// Returns:
    ///     The exported members grouped by documentation topic, as a list of
    ///     `(heading, reference, (member groups...))`.
    #[allow(non_snake_case)]
    fn makeAvailableIn(slf: &Bound<'_, Self>, scope: &Bound<'_, PyAny>) -> PyResult<Py<PyList>> {
        let py = slf.py();
        for (member, _) in API_REFS {
            scope.set_item(member, slf.getattr(member)?)?;
        }
        let mut grouped: BTreeMap<&str, BTreeMap<(&str, &str), Vec<&str>>> = BTreeMap::new();
        for (member, (head, sub, reference)) in API_REFS {
            grouped
                .entry(reference)
                .or_default()
                .entry((head, sub))
                .or_default()
                .push(member);
        }
        let docs = PyList::empty(py);
        for (reference, groups) in grouped {
            let mut chunks = Vec::new();
            let mut head_name = "";
            for ((head, _), mut members) in groups {
                head_name = head;
                members.sort_by_key(|member| (member.len(), *member));
                chunks.push(members.join(" "));
            }
            docs.append((head_name, reference, PyTuple::new(py, chunks)?))?;
        }
        Ok(docs.unbind())
    }

    /// Print the number of entries and the mapped size of each loaded feature.
    ///
    /// Args:
    ///     recompute: Accepted for TF compatibility; sizes are read from the
    ///         compiled cache header and never need recomputing.
    ///     bySize: Sort by size (largest first) instead of by name.
    #[allow(non_snake_case)]
    #[pyo3(signature = (recompute=false, bySize=true))]
    fn footprint(&self, py: Python<'_>, recompute: bool, bySize: bool) -> PyResult<()> {
        let _ = recompute;
        let metadata = self.corpus.metadata();
        let mut rows: Vec<(String, usize, usize)> = metadata
            .node_features
            .iter()
            .filter(|feature| self.visibility.exposes(&feature.name))
            .map(|feature| {
                (
                    feature.name.clone(),
                    feature.row_count,
                    feature.payload_end - feature.payload_start,
                )
            })
            .chain(
                metadata
                    .edge_features
                    .iter()
                    .filter(|feature| self.visibility.exposes(&feature.name))
                    .map(|feature| {
                        (
                            feature.name.clone(),
                            feature.row_count,
                            feature.payload_end - feature.payload_start,
                        )
                    }),
            )
            .collect();
        if bySize {
            rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        } else {
            rows.sort_by(|a, b| a.0.cmp(&b.0));
        }
        let (total_rows, total_bytes) = rows
            .iter()
            .fold((0, 0), |(n, s), row| (n + row.1, s + row.2));
        let mut table = format!(
            "# {} features\n\nfeature | members | size in bytes\n--- | --- | ---\n",
            rows.len()
        );
        for (name, members, size) in &rows {
            table.push_str(&format!("{name} | {members} | {size}\n"));
        }
        table.push_str(&format!("TOTAL | {total_rows} | {total_bytes}"));
        write_console(py, false, true, &table)
    }
}

/// Which metadata keys `isLoaded(pretty=True)` prints.
pub(crate) enum MetaSelection {
    None,
    All,
    Keys(Vec<String>),
}

impl MetaSelection {
    /// The TF default: show the `description` key.
    fn description() -> Self {
        Self::Keys(vec!["description".to_string()])
    }
}

/// TF semantics: a falsy value (`None`, `False`, `""`, `[]`) shows no
/// metadata, `True` shows all keys, and a string or list selects keys.
impl<'py> FromPyObject<'py> for MetaSelection {
    fn extract_bound(value: &Bound<'py, PyAny>) -> PyResult<Self> {
        if value.is_none() {
            return Ok(Self::None);
        }
        if let Ok(flag) = value.downcast::<PyBool>() {
            return Ok(if flag.is_true() {
                Self::All
            } else {
                Self::None
            });
        }
        let keys = feature_names(Some(value))?;
        Ok(if keys.is_empty() {
            Self::None
        } else {
            Self::Keys(keys)
        })
    }
}

struct LoadedReport {
    pretty: bool,
    value_type: bool,
    path: bool,
    meta: MetaSelection,
}

/// One row of the `isLoaded` report.
struct LoadedInfo {
    kind: &'static str,
    value_type: String,
    meta: BTreeMap<String, Option<String>>,
    source: Option<String>,
    edge_values: Option<bool>,
}

fn loaded_info(
    corpus: Option<&PyCorpus>,
    inventory: &FeatureInventory,
    name: &str,
) -> Option<LoadedInfo> {
    let corpus = corpus?;
    let file_meta = inventory.metadata.get(name);
    let source = inventory
        .paths
        .get(name)
        .and_then(|paths| paths.last())
        .and_then(|path| path.parent())
        .map(|dir| dir.display().to_string());
    let metadata = corpus.corpus.metadata();
    let (kind, meta, edge_values) = if corpus.exposed_node_feature(name) {
        let meta = file_meta
            .cloned()
            .or_else(|| metadata.node_feature(name).map(|f| f.metadata.clone()));
        ("node", meta, None)
    } else if corpus.exposed_edge_feature(name) {
        let feature = metadata.edge_feature(name)?;
        let meta = file_meta
            .cloned()
            .or_else(|| Some(feature.metadata.clone()));
        let values = name != OSLOTS && feature.edge_value_count > 0;
        ("edge", meta, Some(values))
    } else if name
        .strip_prefix("__")
        .and_then(|rest| rest.strip_suffix("__"))
        .is_some_and(|computed| COMPUTED_NAMES.contains(&computed))
    {
        let warp_dir = inventory
            .paths
            .get(OTYPE)
            .and_then(|paths| paths.last())
            .and_then(|path| path.parent())
            .map(|dir| dir.display().to_string());
        return Some(LoadedInfo {
            kind: "computed",
            value_type: String::new(),
            meta: BTreeMap::new(),
            source: warp_dir,
            edge_values: None,
        });
    } else if inventory.configs.iter().any(|config| config == name)
        || metadata.config_feature(name).is_some()
    {
        ("config", file_meta.cloned(), None)
    } else {
        return None;
    };
    let mut meta = meta.unwrap_or_default();
    let value_type = meta.remove("valueType").flatten().unwrap_or_default();
    Some(LoadedInfo {
        kind,
        value_type,
        meta,
        source,
        edge_values,
    })
}

/// Shared implementation of `Fabric.isLoaded` and `Corpus.isLoaded`.
fn loaded_report(
    py: Python<'_>,
    corpus: Option<&PyCorpus>,
    inventory: &FeatureInventory,
    features: Option<&Bound<'_, PyAny>>,
    options: &LoadedReport,
) -> PyResult<Option<PyObject>> {
    let names = match features {
        Some(features) if !features.is_none() => feature_names(Some(features))?,
        _ => {
            let mut names: BTreeSet<String> = inventory.paths.keys().cloned().collect();
            names.extend(COMPUTED_NAMES.iter().map(|name| format!("__{name}__")));
            if let Some(corpus) = corpus {
                let metadata = corpus.corpus.metadata();
                names.extend(metadata.node_features.iter().map(|f| f.name.clone()));
                names.extend(metadata.edge_features.iter().map(|f| f.name.clone()));
                names.extend(metadata.config_features.iter().map(|f| f.name.clone()));
            }
            names.into_iter().collect()
        }
    };
    let infos: BTreeMap<String, Option<LoadedInfo>> = names
        .into_iter()
        .map(|name| {
            let info = loaded_info(corpus, inventory, &name);
            (name, info)
        })
        .collect();
    if options.pretty {
        for (name, info) in &infos {
            let line = match info {
                None => format!("{name:<20} {:<10}", "NOT LOADED"),
                Some(info) => pretty_loaded_line(name, info, options),
            };
            write_console(py, false, true, &line)?;
        }
        return Ok(None);
    }
    let result = PyDict::new(py);
    for (name, info) in infos {
        match info {
            None => result.set_item(name, py.None())?,
            Some(info) => {
                let row = PyDict::new(py);
                row.set_item("kind", info.kind)?;
                row.set_item("type", info.value_type)?;
                row.set_item("meta", info.meta)?;
                row.set_item("source", info.source)?;
                row.set_item("edgeValues", info.edge_values)?;
                result.set_item(name, row)?;
            }
        }
    }
    Ok(Some(result.into_any().unbind()))
}

fn pretty_loaded_line(name: &str, info: &LoadedInfo, options: &LoadedReport) -> String {
    let kind = if options.value_type {
        let kind = match (info.kind, info.edge_values) {
            ("node", _) => format!("node ({})", info.value_type),
            ("edge", Some(true)) => format!("edge ({})", info.value_type),
            (kind, _) => kind.to_string(),
        };
        format!(" {kind:<10}")
    } else {
        String::new()
    };
    let source = match (&info.source, options.path) {
        (Some(source), true) => format!(" {source}"),
        _ => String::new(),
    };
    let heading = format!("{name:<20}{kind}{source}");
    let value = |key: &str| info.meta.get(key).cloned().flatten().unwrap_or_default();
    let keys: Vec<String> = match &options.meta {
        MetaSelection::None => Vec::new(),
        MetaSelection::All => info.meta.keys().cloned().collect(),
        MetaSelection::Keys(keys) => keys
            .iter()
            .filter(|key| info.meta.contains_key(*key))
            .cloned()
            .collect(),
    };
    match keys.as_slice() {
        [] => heading,
        [single] => {
            let value = value(single);
            if value.is_empty() {
                heading
            } else {
                format!("{heading} {value}")
            }
        }
        many => {
            let mut line = heading;
            for key in many {
                line.push_str(&format!("\n\t{key:<20} = {}", value(key)));
            }
            line
        }
    }
}
