//! `Corpus`, the API object a load returns (Text-Fabric's `api`): its
//! members and feature introspection.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PySet, PyTuple};
use pyo3::{PyTraverseError, PyVisit};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::computed::{COMPUTED_NAMES, PyComputeds};
use crate::bindings::edge_features::{PyEdgeFeature, PyEdgeFeatures};
use crate::bindings::fabric::{PyFabric, feature_names};
use crate::bindings::locality::PyLocality;
use crate::bindings::node_features::{PyNodeFeature, PyNodeFeatures};
use crate::bindings::nodes::PyNodes;
use crate::bindings::search::PySearch;
use crate::bindings::text::PyText;
use crate::bindings::timestamp::{SILENT_D, write_console};
use crate::bindings::visibility::FeatureVisibility;
use crate::config::{OSLOTS, OTYPE};
use crate::mapped::MappedCompiledCorpus;
use crate::tf::FeatureInventory;

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
    pub(crate) corpus: Arc<MappedCompiledCorpus>,
    pub(crate) visibility: Arc<FeatureVisibility>,
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
    pub(crate) fn exposes_data(&self, name: &str) -> bool {
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
        let computeds = self.members().c.bind(py).borrow();
        if !computeds.names()?.contains(&fName) {
            if warn {
                self.report(
                    py,
                    &format!("Computed feature \"{fName}\" not loaded"),
                    true,
                )?;
            }
            return Ok(None);
        }
        computeds.get(py, fName).map(Some)
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
    fn Call(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        let names = self.members().c.bind(py).borrow().names()?;
        Ok(Self::sorted_names(names.into_iter().map(str::to_string)))
    }

    /// Alias of `Call`.
    #[allow(non_snake_case)]
    fn AllComputeds(&self, py: Python<'_>) -> PyResult<Vec<String>> {
        self.Call(py)
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
    pub(crate) fn description() -> Self {
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

pub(crate) struct LoadedReport {
    pub(super) pretty: bool,
    pub(super) value_type: bool,
    pub(super) path: bool,
    pub(super) meta: MetaSelection,
}

/// One row of the `isLoaded` report.
struct LoadedInfo {
    kind: &'static str,
    value_type: String,
    meta: BTreeMap<String, Option<String>>,
    source: Option<String>,
    edge_values: Option<bool>,
}

/// Render `path` with the home directory abbreviated to `~`, like TF's `ux`.
fn home_relative(path: &std::path::Path) -> String {
    let rendered = path.display().to_string();
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() && rendered.starts_with(&home) => {
            format!("~{}", &rendered[home.len()..])
        }
        _ => rendered,
    }
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
        .map(home_relative);
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
            .map(home_relative);
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
    // TF parses `@edgeValues` into a flag instead of keeping it as metadata.
    meta.remove("edgeValues");
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
pub(crate) fn loaded_report(
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
