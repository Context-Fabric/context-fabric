//! The `T` API: `Text` (text rendering, sections, structure).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use pyo3::exceptions::{PyAttributeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyList, PySet, PyString, PyTuple};

use crate::bindings::api_ref::ApiRef;
use crate::bindings::convert::{
    collect_nodes, heading_from_py, heading_to_py, node_number, nodes_from_py, nodes_to_tuple,
    option_feature_value_to_py, optional_node_number, section_from_py, structure_tree_to_py,
};
use crate::bindings::timestamp::write_console;
use crate::config::DEFAULT_FORMAT;
use crate::mapped::{
    MappedCompiledCorpus, MappedSections, SectionOptions, SectionsContext, TextOptions,
    TextRenderer,
};

/// Text API (TF `T`): text formats, sections, and structure.
#[pyclass(name = "Text", module = "cfabric._core")]
pub(crate) struct PyText {
    corpus: Arc<MappedCompiledCorpus>,
    api: ApiRef,
    /// Fully-resolved sections context, built once on first section call. Powers
    /// `T.sectionFromNode`/`T.sectionTuple` with no per-call view-cache lookup or
    /// `otext` re-parse.
    sections: OnceLock<SectionsContext>,
    /// The text renderer (compiled formats and their feature handles), built
    /// on first use.
    text: OnceLock<TextRenderer>,
}

impl PyText {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>, api: ApiRef) -> Self {
        Self {
            corpus,
            api,
            sections: OnceLock::new(),
            text: OnceLock::new(),
        }
    }

    fn sections_context(&self) -> PyResult<&SectionsContext> {
        if let Some(context) = self.sections.get() {
            return Ok(context);
        }
        let context = SectionsContext::new(&self.corpus)?;
        let _ = self.sections.set(context);
        Ok(self
            .sections
            .get()
            .expect("sections context just initialized"))
    }

    fn renderer(&self) -> PyResult<&TextRenderer> {
        if let Some(renderer) = self.text.get() {
            return Ok(renderer);
        }
        let renderer = TextRenderer::new(&self.corpus)?;
        Ok(self.text.get_or_init(|| renderer))
    }
}

#[pymethods]
impl PyText {
    /// The API (`Corpus`) this member belongs to.
    #[getter]
    fn api(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.api.get(py)
    }

    /// `T.<sec0>Name` / `T.<sec0>Node`, e.g. `T.bookName(node)` and
    /// `T.bookNode(name)` when the first section type is `book`; TF attaches
    /// these under the name of the dataset's first section type.
    fn __getattr__<'py>(slf: &Bound<'py, Self>, name: &str) -> PyResult<Bound<'py, PyAny>> {
        if let Some(section_0_type) = slf.borrow().corpus.otext()?.section_types.first().cloned() {
            if name == format!("{section_0_type}Name") {
                return slf.getattr("_sec0Name");
            }
            if name == format!("{section_0_type}Node") {
                return slf.getattr("_sec0Node");
            }
        }
        Err(PyAttributeError::new_err(format!(
            "'Text' object has no attribute '{name}'"
        )))
    }

    fn __dir__(slf: &Bound<'_, Self>) -> PyResult<Vec<String>> {
        let mut names: Vec<String> = slf
            .get_type()
            .dir()?
            .iter()
            .map(|name| name.extract::<String>())
            .collect::<PyResult<_>>()?;
        if let Some(section_0_type) = slf.borrow().corpus.otext()?.section_types.first().cloned() {
            names.push(format!("{section_0_type}Name"));
            names.push(format!("{section_0_type}Node"));
        }
        Ok(names)
    }

    /// Name of the section-0 node containing node `n` (exposed as `T.<sec0>Name`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (n, lang="en"))]
    fn _sec0Name(
        &self,
        #[pyo3(from_py_with = node_number)] n: u32,
        lang: &str,
    ) -> PyResult<Option<String>> {
        Ok(self.sections_context()?.section_0_name(n, lang)?)
    }

    /// The section-0 node with name `name` (exposed as `T.<sec0>Node`).
    #[allow(non_snake_case)]
    #[pyo3(signature = (name, lang="en"))]
    fn _sec0Node(&self, name: &str, lang: &str) -> PyResult<Option<u32>> {
        Ok(self.sections_context()?.section_0_node(name, lang)?)
    }

    /// Whether the T-API is usable (always `True`).
    #[getter]
    fn good(&self) -> bool {
        true
    }

    /// The `otext` configuration (metadata of the `otext` feature).
    #[getter]
    fn config(&self) -> PyResult<BTreeMap<String, Option<String>>> {
        Ok(match self.corpus.config_feature("otext")? {
            Some(otext) => otext.metadata()?,
            None => BTreeMap::new(),
        })
    }

    /// The format used when none is specified and no node-type default applies.
    #[getter]
    #[allow(non_snake_case)]
    fn defaultFormat(&self) -> &'static str {
        DEFAULT_FORMAT
    }

    /// Default format per node type, from formats named `<type>-default`.
    #[getter]
    #[allow(non_snake_case)]
    fn defaultFormats(&self) -> PyResult<BTreeMap<String, String>> {
        let renderer = self.renderer()?;
        Ok(self
            .corpus
            .otext()?
            .formats
            .keys()
            .filter_map(|name| Some((renderer.split_default_format(name)?, name.clone())))
            .collect())
    }

    /// Section types, from big to small (e.g. `["book", "chapter", "verse"]`).
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypes(&self) -> PyResult<Vec<String>> {
        Ok(MappedSections::new(&self.corpus)?.section_types().to_vec())
    }

    /// The section types as a set.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypeSet(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        Ok(PySet::new(py, self.sectionTypes()?)?.unbind())
    }

    /// The section feature names, like `sectionFeats` (TF's `T.sectionFeatures`
    /// holds the features' data dicts instead; use `Fs(name).data` for those).
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeatures(&self) -> PyResult<Vec<String>> {
        self.sectionFeats()
    }

    /// Names of the section features, one per section type, as a list.
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeats(&self) -> PyResult<Vec<String>> {
        Ok(MappedSections::new(&self.corpus)?
            .section_features()
            .to_vec())
    }

    /// The first section feature and its language variants (`book`, `book@en`, ...).
    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeatsWithLanguage(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let mut names: Vec<String> = self
            .corpus
            .section_0_language_features()?
            .into_values()
            .collect();
        names.sort();
        Ok(PyTuple::new(py, names)?.unbind())
    }

    /// Text formats, mapped to the node type they render.
    #[getter]
    fn formats(&self) -> PyResult<std::collections::BTreeMap<String, String>> {
        Ok(self.renderer()?.formats()?)
    }

    /// Structure types, from big to small.
    #[getter]
    #[allow(non_snake_case)]
    fn structureTypes(&self) -> PyResult<Vec<String>> {
        Ok(self.corpus.otext()?.structure_types.clone())
    }

    /// The structure types as a set.
    #[getter]
    #[allow(non_snake_case)]
    fn structureTypeSet(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        Ok(PySet::new(py, self.structureTypes()?)?.unbind())
    }

    /// Names of the structure features, one per structure type.
    #[getter]
    #[allow(non_snake_case)]
    fn structureFeats(&self) -> PyResult<Vec<String>> {
        Ok(self.corpus.otext()?.structure_features.clone())
    }

    /// Languages of the section-0 feature, `{code: {language, languageEnglish}}`.
    #[getter]
    fn languages(
        &self,
    ) -> PyResult<std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>>
    {
        Ok(self.corpus.languages()?)
    }

    /// Print an overview of the structure configuration.
    #[allow(non_snake_case)]
    fn structureInfo(&self, py: Python<'_>) -> PyResult<()> {
        let info = self.corpus.structure_info()?;
        write_console(py, false, true, &info)
    }

    /// TF `T.headings`: the configured `(structure type, structure feature)`
    /// pairs as a tuple, `()` when no structure is configured.
    #[getter]
    fn headings(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, self.corpus.structure_headings()?)?.into())
    }

    /// Split a format template `type#template` into `(type, template)`; the
    /// type defaults to the slot type.
    #[allow(non_snake_case)]
    fn splitFormat(&self, tpl: &str) -> PyResult<(String, String)> {
        Ok(self.renderer()?.split_format(tpl))
    }

    /// The node type of a format named `<type>-default`, else `None`.
    #[allow(non_snake_case)]
    fn splitDefaultFormat(&self, tpl: &str) -> PyResult<Option<String>> {
        Ok(self.renderer()?.split_default_format(tpl))
    }

    /// The text of a node, or of an iterable of nodes (concatenated).
    ///
    /// Args:
    ///     nodes: A node or an iterable of nodes (rendered in the given order).
    ///     fmt: Text format; default: the node type's `<type>-default` format,
    ///         else `text-orig-full`.
    ///     descend: Whether to render the nodes of the format's target type
    ///         inside each node (default: descend unless the node type has its
    ///         own default format).
    ///     func: Custom renderer called as `func(n, **kwargs)` for every node
    ///         that would be rendered, instead of the format.
    ///     explain: Print how the format and descend rules were applied.
    ///     **kwargs: Passed on to `func`.
    #[pyo3(signature = (nodes, fmt=None, descend=None, func=None, explain=false, **kwargs))]
    fn text(
        &self,
        py: Python<'_>,
        nodes: &Bound<'_, PyAny>,
        fmt: Option<&str>,
        descend: Option<bool>,
        func: Option<&Bound<'_, PyAny>>,
        explain: bool,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        // TF `T.text` (tf/core/text.py:972) accepts a single int OR an arbitrary
        // iterable of node ids; iterables are rendered per node and the pieces are
        // concatenated with no separator (`"".join(material)`, text.py:1182).
        let renderer = self.renderer()?;
        let options = TextOptions::new(fmt.map(str::to_string), descend);
        let nodes = if nodes.is_instance_of::<PyInt>() {
            vec![node_number(nodes)?]
        } else {
            nodes_from_py(Some(nodes))?.unwrap_or_default()
        };
        if explain {
            let func_rep = if func.is_some() { "" } else { "no " };
            let message = format!(
                "EXPLANATION: T.text() called with parameters:\n\tnodes  : {nodes:?}\n\tfmt    : {}\n\tdescend: {}\n\tfunc   : {func_rep}custom format implementation",
                fmt.unwrap_or("implicit"),
                descend.map_or_else(|| "implicit".to_string(), |flag| flag.to_string()),
            );
            write_console(py, true, true, &message)?;
        }
        let mut material = String::new();
        for node in nodes {
            if let Some(func) = func {
                for text_node in renderer.rendered_nodes(node, &options)? {
                    if explain {
                        write_console(
                            py,
                            true,
                            true,
                            &format!("\tNODE {node}: render {text_node} with func"),
                        )?;
                    }
                    material.push_str(&func.call((text_node,), kwargs)?.str()?.to_string());
                }
                continue;
            }
            if !explain {
                renderer.render_into(node, &options, &mut material)?;
                continue;
            }
            let rendered = renderer.text(node, &options)?;
            {
                write_console(
                    py,
                    true,
                    true,
                    &format!(
                        "\tNODE {node}: renders {:?} as {rendered:?}",
                        renderer.rendered_nodes(node, &options)?
                    ),
                )?;
            }
            material.push_str(&rendered);
        }
        Ok(material)
    }

    /// Batch text rendering (ADDITIVE; W4). `items` is an iterable whose elements
    /// are each either a single node id (int) or an iterable of node ids; every
    /// element is rendered exactly as the scalar `text` would render it (an
    /// iterable element is rendered per-node and concatenated with no separator,
    /// matching TF `"".join(material)`). Returns a `list[str]`, one string per
    /// element, in input order.
    #[pyo3(signature = (items, fmt=None, descend=None))]
    fn text_many(
        &self,
        py: Python<'_>,
        items: &Bound<'_, PyAny>,
        fmt: Option<&str>,
        descend: Option<bool>,
    ) -> PyResult<PyObject> {
        let renderer = self.renderer()?;
        let options = TextOptions::new(fmt.map(str::to_string), descend);
        let iter = items.try_iter().map_err(|_| {
            PyTypeError::new_err("text_many expects an iterable of nodes or node-iterables")
        })?;
        let out = PyList::empty(py);
        let mut rendered = String::new();
        for item in iter {
            let item = item?;
            rendered.clear();
            if item.is_instance_of::<PyInt>() {
                renderer.render_into(node_number(&item)?, &options, &mut rendered)?;
            } else {
                for node in collect_nodes(&item)? {
                    renderer.render_into(node, &options, &mut rendered)?;
                }
            }
            out.append(PyString::new(py, &rendered))?;
        }
        Ok(out.into_any().unbind())
    }

    /// The section heading values (e.g. `("Genesis", 1, 1)`) of the sections
    /// containing node `n`.
    ///
    /// Args:
    ///     n: The node.
    ///     lastSlot: Use the last slot of `n` instead of the first.
    ///     lang: Language code for the section-0 name.
    ///     fillup: For section nodes, fill up the lower levels with their first
    ///         subsection.
    ///     level: Only return the headings up to this level.
    #[allow(non_snake_case)]
    #[pyo3(signature = (n, lastSlot=false, lang="en", fillup=false, level=None))]
    fn sectionFromNode(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        lastSlot: bool,
        lang: &str,
        fillup: bool,
        level: Option<usize>,
    ) -> PyResult<PyObject> {
        let node = n;
        let options = SectionOptions {
            last_slot: lastSlot,
            fillup,
            level,
        };
        let values = self
            .sections_context()?
            .section_heading(node, lang, &options)?;
        let items = values
            .iter()
            .map(|value| option_feature_value_to_py(py, value.as_ref()))
            .collect::<PyResult<Vec<_>>>()?;
        Ok(PyTuple::new(py, items)?.into())
    }

    /// The section nodes containing node `n`, from big to small.
    ///
    /// Args:
    ///     n: The node.
    ///     lastSlot: Use the last slot of `n` instead of the first.
    ///     fillup: For section nodes, fill up the lower levels with their first
    ///         subsection.
    ///     level: Only return the sections up to this level.
    #[allow(non_snake_case)]
    #[pyo3(signature = (n, lastSlot=false, fillup=false, level=None))]
    fn sectionTuple(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
        lastSlot: bool,
        fillup: bool,
        level: Option<usize>,
    ) -> PyResult<PyObject> {
        // TF `T.sectionTuple` (tf/core/text.py:461) returns the *section node ids*
        // (not their feature values) that contain `node`, honoring `lastSlot` and
        // `fillup`.
        let node = n;
        let nodes = self.sections_context()?.sections().section_tuple(
            node,
            &SectionOptions {
                last_slot: lastSlot,
                fillup,
                level,
            },
        )?;
        Ok(PyTuple::new(py, nodes)?.into())
    }

    /// The node of a section heading tuple (e.g. `("Genesis", 1, 1)`), or `None`.
    ///
    /// Args:
    ///     section: Heading values from big to small; may be shorter than the
    ///         number of section levels.
    ///     lang: Language code of the section-0 name.
    #[allow(non_snake_case)]
    #[pyo3(signature = (section, lang="en"))]
    fn nodeFromSection(&self, section: &Bound<'_, PyAny>, lang: &str) -> PyResult<Option<u32>> {
        let values = section_from_py(section)?;
        // Resolve the language-aware section-0 feature (e.g. `book@en`) and the
        // section-0 node through the cached sections context (lazily-built value
        // index, O(1)); chapter/verse come from the CFRSECT1 `sec1`/`sec2` maps.
        // A miss returns `None` immediately.
        let context = self.sections_context()?;
        let sec0_feature = context.sec0_feature_for_lang(lang);
        Ok(context.node_from_section_langed(&values, sec0_feature)?)
    }

    /// The structure heading of node `n`: `((type, heading), ...)`.
    #[allow(non_snake_case)]
    fn headingFromNode(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .heading_from_node(n)?
            .map(|heading| heading_to_py(py, &heading))
            .transpose()
    }

    /// The node of a structure heading `((type, heading), ...)`, or `None`.
    #[allow(non_snake_case)]
    fn nodeFromHeading(&self, head: &Bound<'_, PyAny>) -> PyResult<Option<u32>> {
        let heading = heading_from_py(head)?;
        Ok(self.corpus.node_from_heading(&heading)?)
    }

    /// The structure tree of `node` (or of the whole dataset) as nested tuples.
    #[pyo3(signature = (node=None))]
    fn structure(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = optional_node_number)] node: Option<u32>,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .structure(node)?
            .map(|tree| structure_tree_to_py(py, &tree))
            .transpose()
    }

    /// The top-level structure nodes.
    fn top(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        Ok(self
            .corpus
            .top()?
            .map(|nodes| nodes_to_tuple(py, nodes))
            .transpose()?)
    }

    /// TF `T.up`: the parent structural node of node `n`, or `None` without
    /// structure or when `n` is not of a structure type.
    fn up(&self, #[pyo3(from_py_with = node_number)] n: u32) -> PyResult<Option<u32>> {
        Ok(self.corpus.structure_up(n)?)
    }

    /// TF `T.down`: the tuple of structural children of node `n`, or `None`
    /// without structure or when `n` is not of a structure type.
    fn down(
        &self,
        py: Python<'_>,
        #[pyo3(from_py_with = node_number)] n: u32,
    ) -> PyResult<Option<PyObject>> {
        self.corpus
            .structure_down(n)?
            .map(|nodes| nodes_to_tuple(py, nodes))
            .transpose()
    }

    /// The structure of `node` (or of the whole dataset) as an indented string.
    #[allow(non_snake_case)]
    #[pyo3(signature = (node=None, fullHeading=false))]
    fn structurePretty(
        &self,
        #[pyo3(from_py_with = optional_node_number)] node: Option<u32>,
        fullHeading: bool,
    ) -> PyResult<Option<String>> {
        Ok(self.corpus.structure_pretty(node, fullHeading)?)
    }
}
