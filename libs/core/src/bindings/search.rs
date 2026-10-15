//! The `S` API: `Search` (search, study, fetch, count) and `SearchExe`.

use std::collections::HashMap;
use std::sync::Arc;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PySet, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{NodeRange, node_list};
use crate::bindings::search_results::{PySearchResultIterator, warn_cut_off};
use crate::bindings::signals::interruptible;
use crate::bindings::timestamp::{Silent, write_console};
use crate::error::CfError;
use crate::mapped::{
    MappedCompiledCorpus, SectionOptions, SectionsContext, TextOptions, TextRenderer,
};
use crate::search::{
    Interrupt, MappedSearch, PreparedSearch, SEARCH_FAIL_FACTOR, SearchSets, TemplateErrorKind,
    TemplateErrors,
};
use crate::value::FeatureValue;

/// Custom search sets owned on the Rust side: `(name, nodes)` pairs.
type OwnedSearchSets = Vec<(String, Vec<u32>)>;

/// Borrow owned set storage as the engine's `SearchSets` view.
fn borrow_search_sets(owned: &Option<OwnedSearchSets>) -> Option<SearchSets<'_>> {
    owned.as_ref().map(|pairs| {
        pairs
            .iter()
            .map(|(name, nodes)| (name.as_str(), nodes.clone()))
            .collect::<HashMap<_, _>>()
    })
}

/// The Python `shallow=` argument, normalized to TF semantics
/// (`tf/search/searchexe.py:55`): `0 if not shallow else 1 if shallow is True
/// else shallow`. Holds the projection width: 0 = full tuples, 1 = first
/// component only, k>1 = k-prefix tuples.
#[derive(Clone, Copy)]
struct Shallow(usize);

impl Shallow {
    /// `shallow=False`: full result tuples.
    const FULL: Self = Self(0);
}

impl<'py> FromPyObject<'py> for Shallow {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.is_none() {
            return Ok(Self::FULL);
        }
        // `bool` is a subclass of `int`, so it must be checked before integers.
        if let Ok(flag) = obj.downcast::<PyBool>() {
            return Ok(Self(usize::from(flag.is_true())));
        }
        if let Ok(value) = obj.extract::<i64>() {
            return Ok(Self(if value <= 0 { 0 } else { value as usize }));
        }
        Err(PyTypeError::new_err(
            "shallow must be a bool, an int, or None",
        ))
    }
}

/// How many results a search delivers (TF `tf/search/searchexe.py::fetch` /
/// `count`): a positive `limit` caps them quietly; `None`, 0 or a negative
/// limit fall back to the fail limit, `SEARCH_FAIL_FACTOR * maxNode`, with a
/// cut-off warning when there are more.
#[derive(Clone, Copy)]
struct ResultCap {
    max: usize,
    warns: bool,
}

impl ResultCap {
    fn new(limit: Option<i64>, fail_limit: usize) -> Self {
        match limit.and_then(|limit| usize::try_from(limit).ok()) {
            Some(limit) if limit > 0 => Self {
                max: limit,
                warns: false,
            },
            _ => Self {
                max: fail_limit,
                warns: true,
            },
        }
    }

    /// How many matches to produce: one past a warning cap, to tell whether
    /// there are more.
    fn wanted(self) -> usize {
        self.max.saturating_add(usize::from(self.warns))
    }

    /// Cut `found` matches to the cap, warning when the cap cut some off.
    fn apply(self, py: Python<'_>, found: usize) -> PyResult<usize> {
        if found <= self.max {
            return Ok(found);
        }
        if self.warns {
            warn_cut_off(py, self.max)?;
        }
        Ok(self.max)
    }
}

/// Search API (TF `S`): template search.
#[pyclass(name = "Search", module = "cfabric._core")]
pub(crate) struct PySearch {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Verbosity of the Fabric when the API was made (TF `S.silent`).
    silent: &'static str,
    exe: Option<PySearchExe>,
    template: Option<String>,
    /// Owned custom sets registered via `study(..., sets=...)`, honored by
    /// later `fetch`/`count` calls (TF stashes `sets` on the SearchExe).
    sets: Option<OwnedSearchSets>,
    /// Projection width registered via `study(..., shallow=...)` (TF semantics).
    shallow: usize,
}

impl PySearch {
    pub(crate) fn new(
        corpus: Arc<MappedCompiledCorpus>,
        api: ApiRef,
        silent: &'static str,
    ) -> Self {
        Self {
            corpus,
            api,
            silent,
            exe: None,
            template: None,
            sets: None,
            shallow: 0,
        }
    }

    /// TF's fail limit: `SEARCH_FAIL_FACTOR * F.otype.maxNode` results.
    fn fail_limit(&self) -> usize {
        SEARCH_FAIL_FACTOR as usize * self.corpus.max_node() as usize
    }

    /// The search engine, stopping on `interrupt`.
    fn engine<'s>(
        &'s self,
        interrupt: &'s dyn Interrupt,
    ) -> crate::error::Result<MappedSearch<'s>> {
        Ok(MappedSearch::new(&self.corpus)?.with_interrupt(interrupt))
    }

    /// Convert the Python `sets=` argument (a dict mapping a set name to an
    /// iterable of nodes) into owned `(name, nodes)` pairs. Every node must
    /// be a node of the corpus (`IndexError` otherwise).
    fn search_sets(&self, sets: Option<&Bound<'_, PyAny>>) -> PyResult<Option<OwnedSearchSets>> {
        let Some(sets) = sets.filter(|sets| !sets.is_none()) else {
            return Ok(None);
        };
        let dict = sets.downcast::<PyDict>().map_err(|_| {
            PyTypeError::new_err("sets must be a dict mapping set names to iterables of node ids")
        })?;
        let range = NodeRange::Strict {
            max_node: self.corpus.max_node(),
        };
        let mut owned = Vec::with_capacity(dict.len());
        for (key, value) in dict.iter() {
            let name = key.extract::<String>().map_err(|_| {
                PyTypeError::new_err("sets keys must be strings (custom set names)")
            })?;
            owned.push((name, node_list(&value, range)?));
        }
        Ok(Some(owned))
    }

    /// The template of the last `study`.
    fn studied(&self) -> PyResult<&str> {
        self.template.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })
    }

    /// Check and plan `template`.
    fn prepare(
        &self,
        py: Python<'_>,
        template: &str,
        sets: &Option<OwnedSearchSets>,
    ) -> PyResult<PreparedSearch> {
        interruptible(py, |interrupt| {
            self.engine(interrupt)?
                .prepare(template, borrow_search_sets(sets).as_ref())
        })
    }

    /// The results of `template`, shaped per TF `fetch`: with `shallow`, the
    /// set of first nodes or `shallow`-prefixes (whatever the limit); else
    /// without a limit a lazy iterator, with one a tuple.
    fn results(
        &self,
        py: Python<'_>,
        template: &str,
        sets: &Option<OwnedSearchSets>,
        shallow: usize,
        limit: Option<i64>,
    ) -> PyResult<PyObject> {
        let mut prepared = self.prepare(py, template, sets)?;
        if shallow == 1 {
            let nodes = interruptible(py, |interrupt| {
                prepared.first_nodes(&self.engine(interrupt)?, None)
            })?;
            return Ok(PySet::new(py, &nodes)?.into());
        }
        if shallow > 1 {
            let prefixes = interruptible(py, |interrupt| {
                prepared.prefixes(&self.engine(interrupt)?, shallow, None)
            })?;
            let tuples = prefixes
                .into_iter()
                .map(|prefix| PyTuple::new(py, prefix).map(Into::into))
                .collect::<PyResult<Vec<PyObject>>>()?;
            return Ok(PySet::new(py, &tuples)?.into());
        }
        if limit.is_none() {
            let iterator =
                PySearchResultIterator::new(Arc::clone(&self.corpus), prepared, self.fail_limit());
            return Ok(Py::new(py, iterator)?.into_any());
        }
        let cap = ResultCap::new(limit, self.fail_limit());
        let mut rows = interruptible(py, |interrupt| {
            prepared.next_rows(&self.engine(interrupt)?, cap.wanted())
        })?;
        rows.truncate(cap.apply(py, rows.len())?);
        let tuples = rows
            .into_iter()
            .map(|row| PyTuple::new(py, row).map(Into::into))
            .collect::<PyResult<Vec<PyObject>>>()?;
        Ok(PyTuple::new(py, tuples)?.into())
    }
}

#[pymethods]
impl PySearch {
    /// The API (`Corpus`) this member belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// Verbosity of the Fabric when the API was made.
    #[getter]
    fn silent(&self) -> &'static str {
        self.silent
    }

    /// Outcome of the last `study`: `exe.good`, `exe.badSyntax`, `exe.badSemantics`.
    #[getter]
    fn exe(&self) -> Option<PySearchExe> {
        self.exe.clone()
    }

    /// Check a search template and prepare it for `fetch` / `count` / `showPlan`.
    ///
    /// A bad template does not raise: as in Text-Fabric, its problems are
    /// recorded on `S.exe` (`good`, `badSyntax`, `badSemantics`).
    ///
    /// Args:
    ///     searchTemplate: The search template.
    ///     strategy: Accepted for TF compatibility; the planner chooses the strategy.
    ///     sets: Custom sets, `{name: iterable of nodes}`, usable as node types.
    ///     shallow: `False`: full result tuples; `True`: the set of first nodes;
    ///         `k > 1`: the set of `k`-prefixes of the result tuples.
    ///     here: Accepted for TF compatibility (no display side channel).
    ///     silent: Verbosity for this call.
    ///
    /// Raises:
    ///     IndexError: For a node in `sets` that is not a node of the corpus.
    ///     KeyboardInterrupt: On Ctrl-C while quantifiers are evaluated.
    #[allow(non_snake_case, clippy::too_many_arguments)] // Text-Fabric's signature
    #[pyo3(
        signature = (searchTemplate, strategy=None, sets=None, shallow=Shallow::FULL, here=true, silent=Silent::DEFAULT),
        text_signature = "($self, searchTemplate, strategy=None, sets=None, shallow=False, here=True, silent='auto')"
    )]
    fn study(
        &mut self,
        py: Python<'_>,
        searchTemplate: &str,
        strategy: Option<&Bound<'_, PyAny>>,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: Shallow,
        here: bool,
        silent: Silent,
    ) -> PyResult<()> {
        let _ = (strategy, here);
        let _ = silent;
        let owned_sets = self.search_sets(sets)?;
        let outcome = interruptible(py, |interrupt| {
            match self
                .engine(interrupt)?
                .prepare(searchTemplate, borrow_search_sets(&owned_sets).as_ref())
            {
                Ok(_) => Ok(None),
                Err(CfError::Template(errors)) => Ok(Some(errors)),
                Err(error) => Err(error),
            }
        })?;
        self.template = Some(searchTemplate.to_string());
        self.sets = owned_sets;
        self.shallow = shallow.0;
        self.exe = Some(match outcome {
            None => PySearchExe::good(),
            Some(errors) => PySearchExe::bad(&errors),
        });
        Ok(())
    }

    /// Search for all matches of a template.
    ///
    /// Args:
    ///     searchTemplate: The search template.
    ///     limit: Return at most this many results. A tuple is returned when
    ///         `limit` is given, a lazy iterator when it is `None`. Without a
    ///         positive limit, results stop at `4 * maxNode` with a
    ///         `UserWarning` when there are more.
    ///     sets: Custom sets, `{name: iterable of nodes}`, usable as node types.
    ///     shallow: `False`: full result tuples; `True`: the set of first nodes;
    ///         `k > 1`: the set of `k`-prefixes of the result tuples. A set is
    ///         complete, whatever `limit`.
    ///     silent: Verbosity for this call.
    ///     here: Accepted for TF compatibility (no display side channel).
    ///
    /// Returns:
    ///     An iterator or tuple of result tuples, or a set when `shallow` is set.
    ///
    /// Raises:
    ///     SearchTemplateError: For a template Text-Fabric reports as bad.
    ///     IndexError: For a node in `sets` that is not a node of the corpus.
    ///     KeyboardInterrupt: On Ctrl-C while the search runs.
    #[allow(non_snake_case, clippy::too_many_arguments)] // Text-Fabric's signature
    #[pyo3(
        signature = (searchTemplate, limit=None, sets=None, shallow=Shallow::FULL, silent=Silent::DEFAULT, here=true),
        text_signature = "($self, searchTemplate, limit=None, sets=None, shallow=False, silent='auto', here=True)"
    )]
    fn search(
        &self,
        py: Python<'_>,
        searchTemplate: &str,
        limit: Option<i64>,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: Shallow,
        silent: Silent,
        here: bool,
    ) -> PyResult<PyObject> {
        let _ = here;
        let _ = silent;
        let owned_sets = self.search_sets(sets)?;
        self.results(py, searchTemplate, &owned_sets, shallow.0, limit)
    }

    /// Fetch the results of the template prepared by `study`.
    ///
    /// Args:
    ///     limit: Return at most this many results, as for `search`.
    ///
    /// Raises:
    ///     SearchTemplateError: When the studied template is bad.
    ///     KeyboardInterrupt: On Ctrl-C while the search runs.
    #[pyo3(signature = (limit=None))]
    fn fetch(&self, py: Python<'_>, limit: Option<i64>) -> PyResult<PyObject> {
        let template = self.studied()?;
        self.results(py, template, &self.sets, self.shallow, limit)
    }

    /// Count the results of the template prepared by `study`.
    ///
    /// Args:
    ///     progress: Accepted for TF compatibility (no progress reporting).
    ///     limit: Stop counting at this many results. Without a positive
    ///         limit, counting stops at `4 * maxNode` with a `UserWarning`
    ///         when there are more.
    ///
    /// Returns:
    ///     The number of results (Text-Fabric prints it and returns `None`).
    ///
    /// Raises:
    ///     SearchTemplateError: When the studied template is bad.
    ///     KeyboardInterrupt: On Ctrl-C while counting.
    #[pyo3(signature = (progress=None, limit=None))]
    fn count(
        &self,
        py: Python<'_>,
        progress: Option<usize>,
        limit: Option<i64>,
    ) -> PyResult<usize> {
        let _ = progress;
        let template = self.studied()?;
        let mut prepared = self.prepare(py, template, &self.sets)?;
        let cap = ResultCap::new(limit, self.fail_limit());
        let found = interruptible(py, |interrupt| {
            let engine = self.engine(interrupt)?;
            let wanted = Some(cap.wanted());
            Ok(match self.shallow {
                0 => prepared.count(&engine, wanted)?,
                1 => prepared.first_nodes(&engine, wanted)?.len(),
                width => prepared.prefixes(&engine, width, wanted)?.len(),
            })
        })?;
        cap.apply(py, found)
    }

    /// Print the search plan of the template prepared by `study`.
    ///
    /// Args:
    ///     details: Include the binding order with candidate counts.
    #[allow(non_snake_case)]
    #[pyo3(signature = (details=false))]
    fn showPlan(&self, py: Python<'_>, details: bool) -> PyResult<()> {
        // TF `S.showPlan` (tf/search/search.py) needs a previous study and
        // shows nothing for a bad template (tf/search/graph.py::displayPlan).
        let Some(template) = self.template.as_deref() else {
            return write_console(
                py,
                true,
                true,
                "Cannot show plan if there is no previous \"study()\"",
            );
        };
        if self.exe.as_ref().is_some_and(|exe| !exe.good) {
            return Ok(());
        }
        let plan = self.prepare(py, template, &self.sets)?.describe(details);
        write_console(py, false, true, &plan)
    }

    /// Render a single result tuple into a human-readable string per TF
    /// `S.glean` (tf/search/search.py:475-542): for each node, a level-2 (verse)
    /// section node becomes `"book ch:vs"`, a slot becomes its text, a level-0/1
    /// section node becomes empty, and any other node becomes
    /// `"otype[<text of first 5 slots>...]"`. Fields are joined with a space.
    fn glean(&self, tup: &Bound<'_, PyAny>) -> PyResult<String> {
        let nodes = node_list(tup, NodeRange::Lenient)?;
        if nodes.is_empty() {
            return Ok(String::new());
        }
        let context = SectionsContext::new(&self.corpus)?;
        let sections = context.sections();
        let renderer = TextRenderer::new(&self.corpus)?;
        let default_text = TextOptions::default();
        let section_types = sections.section_types().to_vec();
        let level2_type = section_types.get(2).cloned();
        let upper_section_types: std::collections::HashSet<&str> =
            section_types.iter().take(2).map(String::as_str).collect();

        let value_to_string = |value: &FeatureValue| -> String {
            match value {
                FeatureValue::Str(value) => value.to_string(),
                FeatureValue::Int(value) => value.to_string(),
            }
        };

        let mut fields = Vec::with_capacity(nodes.len());
        for node in nodes {
            let otype = sections
                .node_type(node)?
                .map(str::to_string)
                .unwrap_or_default();
            let field = if level2_type.as_deref() == Some(otype.as_str()) {
                let heading = context.section_heading(node, "en", &SectionOptions::default())?;
                let part = |index: usize| {
                    heading
                        .get(index)
                        .and_then(Option::as_ref)
                        .map(&value_to_string)
                        .unwrap_or_default()
                };
                format!("{} {}:{}", part(0), part(1), part(2))
            } else if sections.is_slot(node)? {
                renderer.text(node, &default_text)?
            } else if upper_section_types.contains(otype.as_str()) {
                String::new()
            } else {
                let words = sections.slots(node)?;
                let head: Vec<u32> = words.iter().take(5).copied().collect();
                let ellipsis = if words.len() > 5 { "..." } else { "" };
                let mut text = String::new();
                for word in head {
                    renderer.render_into(word, &default_text, &mut text)?;
                }
                format!("{otype}[{text}{ellipsis}]")
            };
            fields.push(field);
        }
        Ok(fields.join(" "))
    }

    /// Accepted for TF compatibility: the planner has no tunable parameters.
    ///
    /// Args:
    ///     silent: Verbosity for this call.
    ///     **kwargs: TF performance parameters (ignored).
    #[allow(non_snake_case)]
    #[pyo3(signature = (silent=Silent::DEFAULT, **kwargs), text_signature = "($self, silent='auto', **kwargs)")]
    fn tweakPerformance(&self, silent: Silent, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        let _ = kwargs;
        let _ = silent;
        Ok(())
    }

    /// Print the legend of the relational operators usable in search templates.
    #[allow(non_snake_case)]
    fn relationsLegend(&self, py: Python<'_>) -> PyResult<()> {
        write_console(py, false, true, crate::search::RELATIONS_LEGEND)
    }

    /// Performance parameters (empty: the planner has no tunable parameters).
    #[getter]
    #[allow(non_snake_case)]
    fn perfParams(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyDict::new(py).into())
    }
}

/// Outcome of `S.study`: whether the template is good, with syntax/semantic errors.
#[pyclass(name = "SearchExe", module = "cfabric._core")]
#[derive(Clone)]
pub(crate) struct PySearchExe {
    /// Whether the template is valid.
    #[pyo3(get)]
    good: bool,
    /// Syntax errors as `(line, message)` pairs.
    #[pyo3(get, name = "badSyntax")]
    bad_syntax: Vec<(Option<usize>, String)>,
    /// Semantic errors as `(line, message)` pairs.
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

    /// A bad template's errors, recorded where Text-Fabric records them;
    /// planning errors (such as a disconnected template) on neither list.
    fn bad(errors: &TemplateErrors) -> Self {
        Self {
            good: false,
            bad_syntax: errors.of_kind(TemplateErrorKind::Syntax),
            bad_semantics: errors.of_kind(TemplateErrorKind::Semantics),
        }
    }
}
