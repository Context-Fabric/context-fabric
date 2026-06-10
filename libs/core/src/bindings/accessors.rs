use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyIterator, PyList, PySet, PyString, PyTuple};

use crate::compiled::MappedCompiledCorpus;
use crate::corpus::{SectionOptions, StructureTree, TextOptions, WalkEvent};
use crate::feature::FeatureValue;
use crate::mapped_search::MappedSearch;
use crate::mapped_sections::{MappedSections, SectionsContext};
use crate::mapped_text::{MappedText, TextContext};
use crate::precompute::StructureHeading;
use crate::search::SearchSets;

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
        let name = key.extract::<String>().map_err(|_| {
            PyTypeError::new_err("sets keys must be strings (custom set names)")
        })?;
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

/// Normalize the Python `shallow=` argument to TF semantics
/// (`tf/search/searchexe.py:55`): `0 if not shallow else 1 if shallow is True
/// else shallow`. Returns the projection width: 0 = full tuples, 1 = first
/// component only, k>1 = k-prefix tuples.
fn normalize_shallow(shallow: Option<&Bound<'_, PyAny>>) -> PyResult<usize> {
    let Some(obj) = shallow else {
        return Ok(0);
    };
    if obj.is_none() {
        return Ok(0);
    }
    // `bool` is a subclass of `int`, so it must be checked before integers.
    if let Ok(flag) = obj.downcast::<PyBool>() {
        return Ok(if flag.is_true() { 1 } else { 0 });
    }
    if let Ok(value) = obj.extract::<i64>() {
        return Ok(if value <= 0 { 0 } else { value as usize });
    }
    Err(PyTypeError::new_err(
        "shallow must be a bool, an int, or None",
    ))
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

fn parse_csv_config(value: Option<&str>) -> Vec<String> {
    value
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn mapped_otext_value(corpus: &MappedCompiledCorpus, key: &str) -> crate::error::Result<Option<String>> {
    Ok(corpus
        .config_feature("otext")?
        .and_then(|feature| feature.metadata_value(key).transpose())
        .transpose()?
        .map(str::to_string))
}

fn mapped_section_0_language_features(corpus: &MappedCompiledCorpus) -> crate::error::Result<BTreeMap<String, String>> {
    // TF `sectionFeatsWithLanguage` (tf/core/fabric.py:364): the language-aware
    // section-0 features are EXACTLY the first section feature and its `@<code>`
    // variants — never arbitrary features that happen to carry a `languageCode`
    // (e.g. quran's `name@en`). Section functions also require `sectionTypes`.
    if parse_csv_config(mapped_otext_value(corpus, "sectionTypes")?.as_deref()).is_empty() {
        return Ok(BTreeMap::new());
    }
    let Some(base) = parse_csv_config(mapped_otext_value(corpus, "sectionFeatures")?.as_deref())
        .into_iter()
        .next()
    else {
        return Ok(BTreeMap::new());
    };
    let prefix = format!("{base}@");
    let mut features = BTreeMap::new();
    for feature in &corpus.metadata().node_features {
        if feature.name != base && !feature.name.starts_with(&prefix) {
            continue;
        }
        // Base feature usually has no `languageCode`; TF defaults its code to "".
        let code = feature.metadata_value("languageCode").unwrap_or("").to_string();
        features.insert(code, feature.name.clone());
    }
    Ok(features)
}

fn mapped_section_0_feature_for_lang(corpus: &MappedCompiledCorpus, lang: &str) -> crate::error::Result<Option<String>> {
    let language_features = mapped_section_0_language_features(corpus)?;
    if let Some(feature) = language_features.get(lang) {
        return Ok(Some(feature.clone()));
    }
    if let Some(feature) = language_features.get("") {
        return Ok(Some(feature.clone()));
    }
    Ok(parse_csv_config(mapped_otext_value(corpus, "sectionFeatures")?.as_deref())
        .into_iter()
        .next())
}

fn mapped_name_from_node(corpus: &MappedCompiledCorpus, lang: &str) -> crate::error::Result<BTreeMap<u32, String>> {
    let Some(feature_name) = mapped_section_0_feature_for_lang(corpus, lang)? else {
        return Ok(BTreeMap::new());
    };
    let Some(feature) = corpus.node_feature(&feature_name)? else {
        return Ok(BTreeMap::new());
    };
    let mut names = BTreeMap::new();
    for (node, value) in feature.items()? {
        if let crate::compiled::MappedNodeValue::Str(value) = value {
            names.insert(node, value.to_string());
        }
    }
    Ok(names)
}

#[pyclass(name = "Locality")]
pub(crate) struct PyLocality {
    corpus: Arc<MappedCompiledCorpus>,
    /// Fully-resolved sections context, built once on first locality call. After
    /// that, `L.u/d/n/p/i` are pure CSR-slice + otype-filter + tuple-build with no
    /// per-call view-cache lookup or `otext` re-parse.
    context: OnceLock<SectionsContext>,
}

impl PyLocality {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self {
            corpus,
            context: OnceLock::new(),
        }
    }

    fn context(&self) -> PyResult<&SectionsContext> {
        if let Some(context) = self.context.get() {
            return Ok(context);
        }
        let context = SectionsContext::new(&self.corpus)?;
        let _ = self.context.set(context);
        Ok(self
            .context
            .get()
            .expect("sections context just initialized"))
    }
}

#[pymethods]
impl PyLocality {
    #[pyo3(signature = (node, otype=None))]
    fn u(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.u(node, Some(single))?,
            Some(values) => sections.up_types(node, Some(values))?,
            None => sections.u(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn d(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.d(node, Some(single))?,
            Some(values) => sections.down_types(node, Some(values))?,
            None => sections.d(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn n(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.n(node, Some(single))?,
            Some(values) => sections.next_types(node, Some(values))?,
            None => sections.n(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn p(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.p(node, Some(single))?,
            Some(values) => sections.previous_types(node, Some(values))?,
            None => sections.p(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (node, otype=None))]
    fn i(&self, py: Python<'_>, node: u32, otype: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        let types = type_filter_from_py(otype)?;
        let refs = type_filter_refs(&types);
        let sections = self.context()?.sections();
        let nodes = match refs.as_deref() {
            Some([single]) => sections.i(node, Some(single))?,
            Some(values) => sections.intersecting_types(node, Some(values))?,
            None => sections.i(node, None)?,
        };
        nodes_to_tuple(py, nodes)
    }
}

#[pyclass(name = "Nodes")]
pub(crate) struct PyNodes {
    corpus: Arc<MappedCompiledCorpus>,
}

impl PyNodes {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self { corpus }
    }
}

#[pymethods]
impl PyNodes {
    #[getter]
    #[allow(non_snake_case)]
    fn otypeRank(&self) -> PyResult<std::collections::BTreeMap<String, u32>> {
        Ok(self.corpus.otype_rank()?)
    }

    #[allow(non_snake_case)]
    fn sortKey(&self, node: u32) -> PyResult<Option<u32>> {
        Ok(self.corpus.sort_key(node)?)
    }

    #[allow(non_snake_case)]
    fn sortKeyTuple(&self, py: Python<'_>, nodes: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let nodes = nodes_from_py(Some(nodes))?.unwrap_or_default();
        Ok(PyTuple::new(py, self.corpus.sort_key_tuple(&nodes)?)?.into())
    }

    #[allow(non_snake_case)]
    fn sortKeyChunk(
        &self,
        chunk: &Bound<'_, PyAny>,
    ) -> PyResult<super::features::PyChunkPositionKey> {
        super::features::sort_key_chunk_py(&self.corpus, chunk)
    }

    #[allow(non_snake_case)]
    fn sortKeyChunkLength(
        &self,
        chunk: &Bound<'_, PyAny>,
    ) -> PyResult<super::features::PyChunkLengthKey> {
        super::features::sort_key_chunk_length_py(&self.corpus, chunk)
    }

    #[allow(non_snake_case)]
    fn sortNodes(&self, py: Python<'_>, nodes: &Bound<'_, PyAny>) -> PyResult<PyObject> {
        let nodes = nodes_from_py(Some(nodes))?.unwrap_or_default();
        let mut nodes = nodes;
        self.corpus.sort_nodes(&mut nodes)?;
        nodes_to_tuple(py, nodes)
    }

    #[pyo3(signature = (nodes=None, events=false))]
    fn walk(
        &self,
        py: Python<'_>,
        nodes: Option<&Bound<'_, PyAny>>,
        events: bool,
    ) -> PyResult<PyObject> {
        let nodes = nodes_from_py(nodes)?;
        let sections = MappedSections::new(&self.corpus)?;
        if !events {
            return nodes_to_tuple(py, sections.walk(nodes.as_deref())?);
        }
        // TF `N.walk(events=True)` (tf/core/nodes.py:278-292) yields event pairs:
        // slot -> `(n, None)`, container start -> `(n, False)`, container end ->
        // `(n, True)`. `WalkEvent` already models exactly these three cases.
        let rows: Vec<(u32, Option<bool>)> = sections
            .walk_events(nodes.as_deref())?
            .into_iter()
            .map(|event| match event {
                WalkEvent::Slot(node) => (node, None),
                WalkEvent::Start(node) => (node, Some(false)),
                WalkEvent::End(node) => (node, Some(true)),
            })
            .collect();
        Ok(PyTuple::new(py, rows)?.into())
    }
}

#[pyclass(name = "Text")]
pub(crate) struct PyText {
    corpus: Arc<MappedCompiledCorpus>,
    /// Fully-resolved sections context, built once on first section call. Powers
    /// `T.sectionFromNode`/`T.sectionTuple` with no per-call view-cache lookup or
    /// `otext` re-parse.
    sections: OnceLock<SectionsContext>,
    /// Fully-resolved text context (compiled `otext` formats + cached feature
    /// handles), built once on first `T.text` call.
    text: OnceLock<TextContext>,
}

impl PyText {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self {
            corpus,
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

    fn text_context(&self) -> PyResult<&TextContext> {
        if let Some(context) = self.text.get() {
            return Ok(context);
        }
        let context = TextContext::new(&self.corpus)?;
        let _ = self.text.set(context);
        Ok(self.text.get().expect("text context just initialized"))
    }
}

/// Free-standing version of [`PyText::section_heading`] so other binding classes
/// (e.g. `Search::glean`) can reuse the language-aware section resolution.
fn section_heading(
    corpus: &MappedCompiledCorpus,
    node: u32,
    lang: &str,
    options: &SectionOptions,
) -> crate::error::Result<Vec<Option<FeatureValue>>> {
    let section_features =
        parse_csv_config(mapped_otext_value(corpus, "sectionFeatures")?.as_deref());
    MappedSections::new(corpus)?
        .section_tuple(node, options)?
        .into_iter()
        .enumerate()
        .map(|(index, section_node)| {
            let Some(section_node) = section_node else {
                return Ok(None);
            };
            let feature_name = if index == 0 {
                mapped_section_0_feature_for_lang(corpus, lang)?
                    .or_else(|| section_features.get(index).cloned())
            } else {
                section_features.get(index).cloned()
            };
            let Some(feature_name) = feature_name else {
                return Ok(None);
            };
            corpus
                .node_feature(&feature_name)?
                .and_then(|feature| feature.v(section_node).transpose())
                .transpose()
                .map(|value| {
                    value.map(|value| match value {
                        crate::compiled::MappedNodeValue::Str(value) => FeatureValue::string(value),
                        crate::compiled::MappedNodeValue::Int(value) => FeatureValue::Int(value),
                    })
                })
        })
        .collect()
}

#[pymethods]
impl PyText {
    #[getter]
    #[allow(non_snake_case)]
    fn sectionTypes(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, MappedSections::new(&self.corpus)?.section_types())?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeatures(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(py, MappedSections::new(&self.corpus)?.section_features())?.into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn sectionFeats(&self, py: Python<'_>) -> PyResult<PyObject> {
        self.sectionFeatures(py)
    }

    #[getter]
    fn formats(&self) -> PyResult<std::collections::BTreeMap<String, String>> {
        Ok(MappedText::new(&self.corpus)?.formats()?)
    }

    #[getter]
    #[allow(non_snake_case)]
    fn structureTypes(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            parse_csv_config(mapped_otext_value(&self.corpus, "structureTypes")?.as_deref()),
        )?
        .into())
    }

    #[getter]
    #[allow(non_snake_case)]
    fn structureFeats(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(PyTuple::new(
            py,
            parse_csv_config(mapped_otext_value(&self.corpus, "structureFeatures")?.as_deref()),
        )?
        .into())
    }

    #[getter]
    fn languages(
        &self,
    ) -> PyResult<std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>>
    {
        Ok(MappedText::new(&self.corpus)?.languages()?)
    }

    #[allow(non_snake_case)]
    fn structureInfo(&self) -> PyResult<String> {
        Ok(MappedText::new(&self.corpus)?.structure_info()?)
    }

    #[getter]
    fn headings(&self, py: Python<'_>) -> PyResult<PyObject> {
        let headings = PyDict::new(py);
        Ok(headings.into())
    }

    #[pyo3(signature = (node, fmt=None, descend=None))]
    fn text(
        &self,
        node: &Bound<'_, PyAny>,
        fmt: Option<&str>,
        descend: Option<bool>,
    ) -> PyResult<String> {
        // TF `T.text` (tf/core/text.py:972) accepts a single int OR an arbitrary
        // iterable of node ids; iterables are rendered per node and the pieces are
        // concatenated with no separator (`"".join(material)`, text.py:1182).
        let engine = self.text_context()?;
        let options = TextOptions::new(fmt.map(str::to_string), descend);
        if let Ok(single) = node.extract::<u32>() {
            return Ok(engine.text_with_options(single, &options)?);
        }
        let nodes = nodes_from_py(Some(node))?.unwrap_or_default();
        let parts = nodes
            .into_iter()
            .map(|node| engine.text_with_options(node, &options))
            .collect::<crate::error::Result<Vec<String>>>()?;
        Ok(parts.concat())
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
        let options = SectionOptions {
            last_slot: lastSlot,
            fillup,
            level,
        };
        let values = self.sections_context()?.section_heading(node, lang, &options)?;
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
        // TF `T.sectionTuple` (tf/core/text.py:461) returns the *section node ids*
        // (not their feature values) that contain `node`, honoring `lastSlot` and
        // `fillup`. `lang` is irrelevant for node ids but kept for API symmetry.
        let _ = lang;
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

    #[allow(non_snake_case)]
    #[pyo3(signature = (section, lang="en"))]
    fn nodeFromSection(&self, section: &Bound<'_, PyAny>, lang: &str) -> PyResult<Option<u32>> {
        let values = section_from_py(section)?;
        // Resolve the language-aware section-0 feature (e.g. `book@en`), then defer
        // to the v3 CFRSECT1 index lookup, which is O(section-0 nodes) + O(log n)
        // and returns `None` immediately on a miss. The previous implementation
        // rescanned every node of the target type and re-derived each section
        // tuple, an O(nodes * depth) walk that took tens of seconds on a miss.
        let sec0_feature = mapped_section_0_feature_for_lang(&self.corpus, lang)?;
        Ok(MappedSections::new(&self.corpus)?
            .node_from_section_langed(&values, sec0_feature.as_deref())?)
    }

    #[allow(non_snake_case)]
    fn headingFromNode(&self, py: Python<'_>, node: u32) -> PyResult<Option<PyObject>> {
        self.corpus
            .heading_from_node(node)?
            .map(|heading| heading_to_py(py, &heading))
            .transpose()
    }

    #[allow(non_snake_case)]
    fn nodeFromHeading(&self, heading: &Bound<'_, PyAny>) -> PyResult<Option<u32>> {
        let heading = heading_from_py(heading)?;
        Ok(self.corpus.node_from_heading(&heading)?)
    }

    #[pyo3(signature = (node=None))]
    fn structure(&self, py: Python<'_>, node: Option<u32>) -> PyResult<Option<PyObject>> {
        self.corpus
            .structure(node)?
            .map(|tree| structure_tree_to_py(py, &tree))
            .transpose()
    }

    fn top(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        Ok(self
            .corpus
            .top()?
            .map(|nodes| nodes_to_tuple(py, nodes))
            .transpose()?)
    }

    fn up(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        nodes_to_tuple(py, MappedSections::new(&self.corpus)?.u(node, None)?)
    }

    fn down(&self, py: Python<'_>, node: u32) -> PyResult<PyObject> {
        nodes_to_tuple(py, MappedSections::new(&self.corpus)?.d(node, None)?)
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node=None, fullHeading=false))]
    fn structurePretty(&self, node: Option<u32>, fullHeading: bool) -> PyResult<Option<String>> {
        Ok(self.corpus.structure_pretty(node, fullHeading)?)
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (node, lang="en"))]
    fn bookName(&self, node: u32, lang: &str) -> Option<String> {
        let section_0_type = parse_csv_config(mapped_otext_value(&self.corpus, "sectionTypes").ok().flatten().as_deref())
            .into_iter()
            .next()?;
        let sections = MappedSections::new(&self.corpus).ok()?;
        let section_0_node = if sections.node_type(node).ok().flatten()? == section_0_type {
            node
        } else {
            sections.u(node, Some(&section_0_type)).ok()?.into_iter().next()?
        };
        mapped_name_from_node(&self.corpus, lang)
            .ok()?
            .get(&section_0_node)
            .cloned()
    }

    #[allow(non_snake_case)]
    #[pyo3(signature = (name, lang="en"))]
    fn bookNode(&self, name: &str, lang: &str) -> Option<u32> {
        let section_0_type = parse_csv_config(mapped_otext_value(&self.corpus, "sectionTypes").ok().flatten().as_deref())
            .into_iter()
            .next()?;
        let names = mapped_name_from_node(&self.corpus, lang).ok()?;
        for node in self.corpus.nodes_of_type(&section_0_type).ok()? {
            if names.get(&node).is_some_and(|candidate| candidate == name) {
                return Some(node);
            }
        }
        None
    }
}

#[pyclass(name = "Search")]
pub(crate) struct PySearch {
    corpus: Arc<MappedCompiledCorpus>,
    exe: Option<PySearchExe>,
    template: Option<String>,
    /// Owned custom sets registered via `study(..., sets=...)`, honored by
    /// later `fetch`/`count` calls (TF stashes `sets` on the SearchExe).
    sets: Option<Vec<(String, Vec<u32>)>>,
    /// Projection width registered via `study(..., shallow=...)` (TF semantics).
    shallow: usize,
}

impl PySearch {
    pub(crate) fn new(corpus: Arc<MappedCompiledCorpus>) -> Self {
        Self {
            corpus,
            exe: None,
            template: None,
            sets: None,
            shallow: 0,
        }
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
        let engine = MappedSearch::new(&self.corpus);
        let sets = borrow_search_sets(owned_sets);
        match shallow {
            0 => {
                let rows = match sets.as_ref() {
                    Some(sets) => engine.search_with_sets(template, sets, limit)?,
                    None => engine.search(template, limit)?,
                };
                let tuples = rows
                    .into_iter()
                    .map(|row| PyTuple::new(py, row).map(Into::into))
                    .collect::<PyResult<Vec<PyObject>>>()?;
                Ok(PyTuple::new(py, tuples)?.into())
            }
            1 => {
                let nodes = match sets.as_ref() {
                    Some(sets) => engine.search_first_nodes_with_sets(template, sets, limit)?,
                    None => engine.search_first_nodes(template, limit)?,
                };
                Ok(PySet::new(py, &nodes)?.into())
            }
            width => {
                let rows = match sets.as_ref() {
                    Some(sets) => {
                        engine.search_prefixes_with_sets(template, sets, width, limit)?
                    }
                    None => engine.search_prefixes(template, width, limit)?,
                };
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
    #[getter]
    fn exe(&self) -> Option<PySearchExe> {
        self.exe.clone()
    }

    #[pyo3(signature = (template, sets=None, shallow=None))]
    fn study(
        &mut self,
        template: &str,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let owned_sets = search_sets_from_py(sets)?;
        let shallow = normalize_shallow(shallow)?;
        let engine = MappedSearch::new(&self.corpus);
        let result = match borrow_search_sets(&owned_sets) {
            Some(sets) => engine.search_with_sets(template, &sets, Some(1)),
            None => engine.search(template, Some(1)),
        };
        self.template = Some(template.to_string());
        self.sets = owned_sets;
        self.shallow = shallow;
        self.exe = Some(match result {
            Ok(_) => PySearchExe::good(),
            Err(error) => PySearchExe::bad(error.to_string()),
        });
        Ok(())
    }

    // `silent` and `here` are accepted for TF API compatibility but have no
    // effect here: progress reporting is handled by the Python layer (`silent`)
    // and CF has no notebook-display side channel (`here`).
    #[pyo3(signature = (template, limit=None, sets=None, shallow=None, silent=None, here=false))]
    fn search(
        &self,
        py: Python<'_>,
        template: &str,
        limit: Option<usize>,
        sets: Option<&Bound<'_, PyAny>>,
        shallow: Option<&Bound<'_, PyAny>>,
        silent: Option<&str>,
        here: bool,
    ) -> PyResult<PyObject> {
        let _ = (silent, here);
        let owned_sets = search_sets_from_py(sets)?;
        let shallow = normalize_shallow(shallow)?;
        self.run_search(py, template, limit, &owned_sets, shallow)
    }

    #[pyo3(signature = (limit=None))]
    fn fetch(&self, py: Python<'_>, limit: Option<usize>) -> PyResult<PyObject> {
        let template = self.template.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        self.run_search(py, &template, limit, &self.sets, self.shallow)
    }

    #[pyo3(signature = (progress=None, limit=None))]
    fn count(&self, progress: Option<usize>, limit: Option<usize>) -> PyResult<usize> {
        let _ = progress;
        let template = self.template.as_deref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("no search template has been studied")
        })?;
        let engine = MappedSearch::new(&self.corpus);
        let study = match borrow_search_sets(&self.sets) {
            Some(sets) => engine.study_with_sets(template, &sets)?,
            None => engine.study(template)?,
        };
        Ok(match self.shallow {
            0 => study.count(limit),
            1 => study.count_first_nodes(limit),
            width => study.count_prefixes(width, limit),
        })
    }

    #[allow(non_snake_case)]
    fn showPlan(&self, details: bool) -> PyResult<Option<String>> {
        // Emit the engine's real study/plan summary rather than echoing the
        // template (TF `S.showPlan`, tf/search/search.py).
        let Some(template) = self.template.as_deref() else {
            return Ok(None);
        };
        let engine = MappedSearch::new(&self.corpus);
        let plan = match borrow_search_sets(&self.sets) {
            Some(sets) => engine.show_plan_with_sets(template, &sets, details)?,
            None => engine.show_plan(template, details)?,
        };
        Ok(Some(plan))
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
        let sections = MappedSections::new(&self.corpus)?;
        let text = MappedText::new(&self.corpus)?;
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
            let otype = sections.node_type(node)?.map(str::to_string).unwrap_or_default();
            let field = if level2_type.as_deref() == Some(otype.as_str()) {
                let heading = section_heading(&self.corpus, node, "en", &SectionOptions::default())?;
                let part = |index: usize| {
                    heading
                        .get(index)
                        .and_then(Option::as_ref)
                        .map(&value_to_string)
                        .unwrap_or_default()
                };
                format!("{} {}:{}", part(0), part(1), part(2))
            } else if sections.is_slot(node)? {
                text.text(node, None)?
            } else if upper_section_types.contains(otype.as_str()) {
                String::new()
            } else {
                let words = sections.slots(node)?;
                let head: Vec<u32> = words.iter().take(5).copied().collect();
                let ellipsis = if words.len() > 5 { "..." } else { "" };
                format!("{}[{}{}]", otype, text.text_nodes(&head, None)?, ellipsis)
            };
            fields.push(field);
        }
        Ok(fields.join(" "))
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
