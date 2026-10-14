//! The `S` API: `Search` (search, study, fetch, count) and `SearchExe`.

use std::collections::HashMap;
use std::sync::Arc;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyIterator, PySet, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::nodes_from_py;
use crate::bindings::timestamp::{Silent, write_console};
use crate::mapped::{
    MappedCompiledCorpus, SectionOptions, SectionsContext, TextOptions, TextRenderer,
};
use crate::search::{MappedSearch, SEARCH_FAIL_FACTOR, SearchSets};
use crate::value::FeatureValue;

/// Convert the Python `sets=` argument (a dict mapping a set name to an
/// iterable of node ids) into owned `(name, nodes)` pairs. The engine's
/// `SearchSets` borrows the names, so the owned storage must outlive the call.
fn search_sets_from_py(
    sets: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<Vec<(String, Vec<u32>)>>> {
    let Some(sets) = sets else {
        return Ok(None);
    };
    if sets.is_none() {
        return Ok(None);
    }
    let dict = sets.downcast::<PyDict>().map_err(|_| {
        PyTypeError::new_err("sets must be a dict mapping set names to iterables of node ids")
    })?;
    let mut owned = Vec::with_capacity(dict.len());
    for (key, value) in dict.iter() {
        let name = key
            .extract::<String>()
            .map_err(|_| PyTypeError::new_err("sets keys must be strings (custom set names)"))?;
        let nodes = PyIterator::from_object(&value)?
            .map(|item| item?.extract::<u32>())
            .collect::<PyResult<Vec<_>>>()?;
        owned.push((name, nodes));
    }
    Ok(Some(owned))
}

/// Borrow owned set storage as the engine's `SearchSets` view.
fn borrow_search_sets(owned: &Option<Vec<(String, Vec<u32>)>>) -> Option<SearchSets<'_>> {
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
    sets: Option<Vec<(String, Vec<u32>)>>,
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

    /// Counts results of the studied `template` (GIL-free engine work).
    fn count_studied(&self, template: &str, limit: Option<usize>) -> crate::error::Result<usize> {
        let engine = MappedSearch::new(&self.corpus)?;
        // Full-tuple counting runs the join in count-only mode (no row
        // materialization); shallow counting needs the rows for dedup.
        if self.shallow == 0 {
            return match borrow_search_sets(&self.sets) {
                Some(sets) => engine.count_with_sets(template, &sets, limit),
                None => engine.count(template, limit),
            };
        }
        let study = match borrow_search_sets(&self.sets) {
            Some(sets) => engine.study_with_sets(template, &sets)?,
            None => engine.study(template)?,
        };
        Ok(match self.shallow {
            1 => study.count_first_nodes(limit),
            width => study.count_prefixes(width, limit),
        })
    }

    /// TF's result limit (`tf/search/searchexe.py::fetch`/`count`): a negative
    /// limit counts as 0, and 0 means "no limit" up to the fail limit of
    /// `SEARCH_FAIL_FACTOR * maxNode` results.
    fn tf_limit(&self, limit: Option<i64>) -> Option<usize> {
        limit.map(|limit| match usize::try_from(limit) {
            Ok(limit) if limit > 0 => limit,
            _ => SEARCH_FAIL_FACTOR as usize * self.corpus.max_node() as usize,
        })
    }

    /// Run a search and shape the result per TF `shallow` semantics
    /// (`tf/search/stitch.py:872-879`): width 0 -> tuple of full tuples,
    /// width 1 -> set of first components, width k>1 -> set of k-prefix tuples.
    fn run_search(
        &self,
        py: Python<'_>,
        template: &str,
        limit: Option<usize>,
        owned_sets: &Option<Vec<(String, Vec<u32>)>>,
        shallow: usize,
    ) -> PyResult<PyObject> {
        let engine = MappedSearch::new(&self.corpus)?;
        let sets = borrow_search_sets(owned_sets);
        // The engine work runs without the GIL; only building the Python
        // result objects needs it.
        match shallow {
            0 => {
                let rows = py.allow_threads(|| match sets.as_ref() {
                    Some(sets) => engine.search_with_sets(template, sets, limit),
                    None => engine.search(template, limit),
                })?;
                let tuples = rows
                    .into_iter()
                    .map(|row| PyTuple::new(py, row).map(Into::into))
                    .collect::<PyResult<Vec<PyObject>>>()?;
                Ok(PyTuple::new(py, tuples)?.into())
            }
            1 => {
                let nodes = py.allow_threads(|| match sets.as_ref() {
                    Some(sets) => engine.search_first_nodes_with_sets(template, sets, limit),
                    None => engine.search_first_nodes(template, limit),
                })?;
                Ok(PySet::new(py, &nodes)?.into())
            }
            width => {
                let rows = py.allow_threads(|| match sets.as_ref() {
                    Some(sets) => engine.search_prefixes_with_sets(template, sets, width, limit),
                    None => engine.search_prefixes(template, width, limit),
                })?;
                let tuples = rows
                    .into_iter()
                    .map(|row| PyTuple::new(py, row).map(Into::into))
                    .collect::<PyResult<Vec<PyObject>>>()?;
                Ok(PySet::new(py, &tuples)?.into())
            }
        }
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
    /// Args:
    ///     searchTemplate: The search template.
    ///     strategy: Accepted for TF compatibility; the planner chooses the strategy.
    ///     sets: Custom sets, `{name: iterable of nodes}`, usable as node types.
    ///     shallow: `False`: full result tuples; `True`: the set of first nodes;
    ///         `k > 1`: the set of `k`-prefixes of the result tuples.
    ///     here: Accepted for TF compatibility (no display side channel).
    ///     silent: Verbosity for this call.
    #[allow(non_snake_case)]
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
        let owned_sets = search_sets_from_py(sets)?;
        let engine = MappedSearch::new(&self.corpus)?;
        let result = py.allow_threads(|| match borrow_search_sets(&owned_sets) {
            Some(sets) => engine.search_with_sets(searchTemplate, &sets, Some(1)),
            None => engine.search(searchTemplate, Some(1)),
        });
        self.template = Some(searchTemplate.to_string());
        self.sets = owned_sets;
        self.shallow = shallow.0;
        self.exe = Some(match result {
            Ok(_) => PySearchExe::good(),
            Err(error) => PySearchExe::bad(error.to_string()),
        });
        Ok(())
    }

    /// Search for all matches of a template.
    ///
    /// Args:
    ///     searchTemplate: The search template.
    ///     limit: Return at most this many results.
    ///     sets: Custom sets, `{name: iterable of nodes}`, usable as node types.
    ///     shallow: `False`: full result tuples; `True`: the set of first nodes;
    ///         `k > 1`: the set of `k`-prefixes of the result tuples.
    ///     silent: Verbosity for this call.
    ///     here: Accepted for TF compatibility (no display side channel).
    ///
    /// Returns:
    ///     A tuple of result tuples, or a set when `shallow` is set.
    #[allow(non_snake_case)]
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
        let owned_sets = search_sets_from_py(sets)?;
        let limit = self.tf_limit(limit);
        self.run_search(py, searchTemplate, limit, &owned_sets, shallow.0)
    }

    /// Fetch the results of the template prepared by `study`.
    ///
    /// Args:
    ///     limit: Return at most this many results.
    #[pyo3(signature = (limit=None))]
    fn fetch(&self, py: Python<'_>, limit: Option<i64>) -> PyResult<PyObject> {
        let template = self.template.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        self.run_search(
            py,
            &template,
            self.tf_limit(limit),
            &self.sets,
            self.shallow,
        )
    }

    /// Count the results of the template prepared by `study`.
    ///
    /// Args:
    ///     progress: Accepted for TF compatibility (no progress reporting).
    ///     limit: Stop counting at this many results.
    #[pyo3(signature = (progress=None, limit=None))]
    fn count(
        &self,
        py: Python<'_>,
        progress: Option<usize>,
        limit: Option<i64>,
    ) -> PyResult<usize> {
        let _ = progress;
        let limit = self.tf_limit(limit);
        let template = self.template.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        Ok(py.allow_threads(|| self.count_studied(template, limit))?)
    }

    /// Print the search plan of the template prepared by `study`.
    ///
    /// Args:
    ///     details: Include per-step details.
    #[allow(non_snake_case)]
    #[pyo3(signature = (details=false))]
    fn showPlan(&self, py: Python<'_>, details: bool) -> PyResult<()> {
        // Emit the engine's real study/plan summary rather than echoing the
        // template (TF `S.showPlan`, tf/search/search.py).
        let Some(template) = self.template.as_deref() else {
            return write_console(
                py,
                true,
                true,
                "Cannot show plan if there is no previous \"study()\"",
            );
        };
        let engine = MappedSearch::new(&self.corpus)?;
        let plan = match borrow_search_sets(&self.sets) {
            Some(sets) => engine.show_plan_with_sets(template, &sets, details)?,
            None => engine.show_plan(template, details)?,
        };
        write_console(py, false, true, &plan)
    }

    /// Render a single result tuple into a human-readable string per TF
    /// `S.glean` (tf/search/search.py:475-542): for each node, a level-2 (verse)
    /// section node becomes `"book ch:vs"`, a slot becomes its text, a level-0/1
    /// section node becomes empty, and any other node becomes
    /// `"otype[<text of first 5 slots>...]"`. Fields are joined with a space.
    fn glean(&self, tup: &Bound<'_, PyAny>) -> PyResult<String> {
        let nodes = nodes_from_py(Some(tup))?.unwrap_or_default();
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

    fn bad(message: String) -> Self {
        Self {
            good: false,
            bad_syntax: vec![(None, message)],
            bad_semantics: Vec::new(),
        }
    }
}
