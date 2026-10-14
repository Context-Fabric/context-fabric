//! `Fabric`: locations, loading, compiling, and `TF.save`.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PySet, PyString, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

use crate::bindings::api::{LoadedReport, MetaSelection, PyCorpus, loaded_report};
use crate::bindings::computed::COMPUTED_NAMES;
use crate::bindings::feature_file::{FeatureKind, PyFeatureFile};
use crate::bindings::timestamp::{AUTO, CacheTarget, PRINT_ONLY, PyTimestamp, Silent, VERBOSE};
use crate::bindings::visibility::FeatureVisibility;
use crate::config::OTYPE;
use crate::fabric::Fabric;
use crate::mapped::MappedCompiledCorpus;
use crate::tf::{EdgeFeature, FeatureInventory, NodeFeature, Otext, TfFeature};
use crate::value::FeatureValue;

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
    if let Some(md) = metadata
        && let Some(spec) = md.get_item(name)?
        && let Ok(dict) = spec.downcast::<PyDict>()
    {
        for (key, value) in dict.iter() {
            map.insert(key.extract::<String>()?, meta_value_to_string(&value)?);
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
    if value_type == "int"
        && let Ok(int) = value.extract::<i64>()
    {
        return Ok(FeatureValue::Int(int));
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
    pub(crate) silent: &'static str,
    /// Message channel behind `info` / `warning` / `error` / `indent` / ...
    tm: Py<PyTimestamp>,
    /// Cumulative requested feature names (`TF.featuresRequested`).
    pub(crate) requested: Vec<String>,
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

    /// The `otext` config of the loaded dataset (empty before a load).
    fn otext(&self, py: Python<'_>) -> PyResult<Otext> {
        let Some(api) = &self.api else {
            return Ok(Otext::default());
        };
        Ok(api.bind(py).borrow().corpus.otext()?.clone())
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
    pub(crate) fn new(
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
        Ok(self.otext(py)?.section_types)
    }

    /// Section features from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeats(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        Ok(self.otext(py)?.section_features)
    }

    /// Structure types from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn structureTypes(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        Ok(self.otext(py)?.structure_types)
    }

    /// Structure features from the `otext` config of the loaded dataset.
    #[getter]
    #[allow(non_snake_case)]
    fn structureFeats(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        Ok(self.otext(py)?.structure_features)
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
        // Loading may compile the cache (tens of seconds on large corpora):
        // release the GIL so other Python threads keep running. The Fabric
        // configuration is cloned so no borrow of `slf` is held meanwhile.
        let inner = slf.borrow().inner.clone();
        let corpus = py.allow_threads(|| inner.load(requested.clone()))?;
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
        let inner = slf.borrow().inner.clone();
        let corpus = py.allow_threads(|| inner.load_all())?;
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
        py: Python<'_>,
        output_path: &str,
        features: Option<&Bound<'_, PyAny>>,
        silent: Silent,
    ) -> PyResult<bool> {
        let _ = silent;
        let features = feature_names(features)?;
        py.allow_threads(|| self.inner.compile(output_path, features))?;
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
    fn ensureLoaded(slf: &Bound<'_, Self>, features: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        // Release the borrow of the Fabric before delegating: the API records
        // the newly loaded features on it.
        let api = slf.borrow().current_api(slf.py(), "ensureLoaded")?;
        Ok(api.call_method1("ensureLoaded", (features,))?.unbind())
    }

    /// Export the API members into `scope`; see `Corpus.makeAvailableIn`.
    #[allow(non_snake_case)]
    fn makeAvailableIn(slf: &Bound<'_, Self>, scope: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let api = slf.borrow().current_api(slf.py(), "makeAvailableIn")?;
        Ok(api.call_method1("makeAvailableIn", (scope,))?.unbind())
    }

    /// Show the size of the loaded features; see `Corpus.footprint`.
    #[pyo3(signature = (recompute=false, bySize=true))]
    #[allow(non_snake_case)]
    fn footprint(slf: &Bound<'_, Self>, recompute: bool, bySize: bool) -> PyResult<()> {
        let api = slf.borrow().current_api(slf.py(), "footprint")?;
        api.call_method1("footprint", (recompute, bySize))?;
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
                TfFeature::Node(NodeFeature::new(name.clone(), meta, values))
                    .write_tf(&write_dir.join(format!("{name}.tf")))?;
            }
        }

        // Edge features.
        if let Some(edge_features) = edgeFeatures {
            for (name_obj, data_obj) in edge_features.iter() {
                let name = name_obj.extract::<String>()?;
                edge_names.push(name.clone());
                let mut meta = merged_meta(&generic, metaData, &name)?;
                // Kept as the bare `@edgeValues` flag when set.
                if is_truthy_meta(meta.get("edgeValues")) {
                    meta.insert("edgeValues".to_string(), None);
                } else {
                    meta.remove("edgeValues");
                }
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
                TfFeature::Edge(EdgeFeature::new_with_values(
                    name.clone(),
                    meta,
                    values,
                    edge_values,
                ))
                .write_tf(&write_dir.join(format!("{name}.tf")))?;
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
                TfFeature::Config {
                    name: key.clone(),
                    metadata: meta,
                }
                .write_tf(&write_dir.join(format!("{key}.tf")))?;
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
