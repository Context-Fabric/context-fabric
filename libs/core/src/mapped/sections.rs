//! Locality (`L.u/d/n/p/i`), sections (`T.sectionFromNode`,
//! `T.nodeFromSection`) and walking (`N.walk`) over the mapped corpus.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use crate::error::{CfError, Result};
use crate::format::MappedU32s;
use crate::index::sections::section_key;
use crate::mapped::{
    EdgeFeatureView, MappedCompiledCorpus, OwnedEdgeFeature, OwnedNodeFeature,
    OwnedStringPoolFeature, StringPoolNodeFeatureView,
};
use crate::value::FeatureValue;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionOptions {
    pub last_slot: bool,
    pub fillup: bool,
    pub level: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkEvent {
    Start(u32),
    End(u32),
    Slot(u32),
}

impl MappedCompiledCorpus {
    /// The section-0 feature and its language variants by language code (TF
    /// `sectionFeatsWithLanguage`): exactly the first section feature and its
    /// `<feature>@<code>` variants, keyed by their `languageCode` metadata
    /// (empty for the base feature). Empty without sections.
    pub fn section_0_language_features(&self) -> Result<BTreeMap<String, String>> {
        let otext = self.otext()?;
        let mut features = BTreeMap::new();
        if otext.section_types.is_empty() {
            return Ok(features);
        }
        let Some(base) = otext.section_features.first() else {
            return Ok(features);
        };
        let prefix = format!("{base}@");
        for feature in &self.metadata().node_features {
            if feature.name == *base || feature.name.starts_with(&prefix) {
                let code = feature.metadata_value("languageCode").unwrap_or("");
                features.insert(code.to_string(), feature.name.clone());
            }
        }
        Ok(features)
    }

    /// `T.languages`: each section-0 language code with its `language` and
    /// `languageEnglish` metadata (`"default"` when absent).
    pub fn languages(&self) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
        let mut languages = BTreeMap::new();
        for (code, name) in self.section_0_language_features()? {
            let Some(feature) = self.metadata().node_feature(&name) else {
                continue;
            };
            let info = ["language", "languageEnglish"]
                .into_iter()
                .map(|key| {
                    let value = feature.metadata_value(key).unwrap_or("default");
                    (key.to_string(), value.to_string())
                })
                .collect();
            languages.insert(code, info);
        }
        Ok(languages)
    }
}

/// The slot type plus the `otext` section and structure configuration, parsed
/// once and shared by `Arc` between a [`SectionsContext`] and every
/// [`MappedSections`] view it hands out.
struct SectionConfig {
    slot_type: String,
    section_types: Vec<String>,
    section_features: Vec<String>,
}

impl SectionConfig {
    fn load(corpus: &MappedCompiledCorpus, otype: &StringPoolNodeFeatureView<'_>) -> Result<Self> {
        let slot_type = otype
            .str_value(1)?
            .ok_or_else(|| CfError::MissingFeature("otype slot type".to_string()))?
            .to_string();
        let otext = corpus.otext()?;
        Ok(Self {
            section_types: otext.section_types.clone(),
            section_features: otext.section_features.clone(),
            slot_type,
        })
    }
}

/// Long-lived, fully-resolved sections context. A binding object builds this
/// once (resolving the `otype`/`oslots` owned handles and parsing the `otext`
/// section/structure config) and then calls [`sections`](Self::sections) per
/// request to get a thin [`MappedSections`] view with no per-call `RwLock`,
/// `HashMap`, `otext` re-parse, or allocation.
pub struct SectionsContext {
    corpus: Arc<MappedCompiledCorpus>,
    otype: OwnedStringPoolFeature,
    oslots: OwnedEdgeFeature,
    config: Arc<SectionConfig>,
    /// The section-feature handles used by `section_heading` (T.sectionFromNode),
    /// resolved lazily on the first heading call. Building them eagerly would
    /// resolve every `book@<lang>` variant (~20 on BHSA), wasted work for the
    /// locality (`L.u/d/n/p/i`) callers that share this context but never need
    /// feature *values*.
    heading: std::sync::OnceLock<HeadingHandles>,
    /// Section-0 nodes in canonical (`otype.s`) order, scanned once on the first
    /// `nodeFromSection` call. The scan order matters: when two section-0 nodes
    /// share a heading value, the first in canonical order must win, matching the
    /// per-call linear scan this cache replaces.
    sec0_nodes: std::sync::OnceLock<Vec<u32>>,
    /// Per-feature section-0 lookup (`book@<lang>` value -> node), built lazily
    /// from `sec0_nodes` per language feature actually used. A few dozen entries
    /// per map.
    sec0_index: RwLock<HashMap<String, Arc<HashMap<FeatureValue, u32>>>>,
    /// Language code -> section-0 feature name; see
    /// [`MappedCompiledCorpus::section_0_language_features`].
    sec0_lang_features: BTreeMap<String, String>,
}

/// The section-feature handles backing `T.sectionFromNode`. Built once, lazily.
struct HeadingHandles {
    /// Owned handle for each level's section feature, parallel to
    /// `section_types`. Index 0 is the *base* (non-language) section-0 feature;
    /// language-aware variants live in `section0_lang_handles`.
    section_feature_handles: Vec<Option<OwnedNodeFeature>>,
    /// Language-code -> owned handle for the section-0 (book-level) feature, e.g.
    /// `book@en`. Built from the same rule as TF `sectionFeatsWithLanguage`. The
    /// empty-string code is the base feature's own code.
    section0_lang_handles: HashMap<String, Option<OwnedNodeFeature>>,
}

impl SectionsContext {
    pub fn new(corpus: &Arc<MappedCompiledCorpus>) -> Result<Self> {
        let otype = corpus
            .owned_string_pool_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .owned_edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let config = Arc::new(SectionConfig::load(corpus, &otype.view())?);

        Ok(Self {
            corpus: Arc::clone(corpus),
            otype,
            oslots,
            config,
            heading: std::sync::OnceLock::new(),
            sec0_nodes: std::sync::OnceLock::new(),
            sec0_index: RwLock::new(HashMap::new()),
            sec0_lang_features: corpus.section_0_language_features()?,
        })
    }

    /// Resolve (once, lazily) the section-feature handles used by section
    /// headings. See [`HeadingHandles`].
    fn heading_handles(&self) -> Result<&HeadingHandles> {
        if let Some(handles) = self.heading.get() {
            return Ok(handles);
        }
        let corpus = &self.corpus;

        // Resolve one owned handle per configured level. Levels beyond the
        // available `sectionFeatures` (or features absent from the corpus)
        // resolve to `Absent`, matching the per-call `None` result.
        let mut section_feature_handles = Vec::with_capacity(self.config.section_types.len());
        for index in 0..self.config.section_types.len() {
            section_feature_handles.push(match self.config.section_features.get(index) {
                Some(name) => corpus.owned_node_feature(name)?,
                None => None,
            });
        }

        let mut section0_lang_handles: HashMap<String, Option<OwnedNodeFeature>> = HashMap::new();
        for (code, name) in &self.sec0_lang_features {
            section0_lang_handles.insert(code.clone(), corpus.owned_node_feature(name)?);
        }

        let _ = self.heading.set(HeadingHandles {
            section_feature_handles,
            section0_lang_handles,
        });
        Ok(self
            .heading
            .get()
            .expect("heading handles just initialized"))
    }

    /// Section heading values for `node`, mirroring the free-standing
    /// `section_heading` in the bindings (TF `sectionFromNode`,
    /// tf/core/text.py:555) but over cached feature handles: the section tuple
    /// resolved to feature values, with the language-aware feature for level 0
    /// and the configured `sectionFeatures` for deeper levels.
    pub fn section_heading(
        &self,
        node: u32,
        lang: &str,
        options: &SectionOptions,
    ) -> Result<Vec<Option<FeatureValue>>> {
        let tuple = self.sections().section_tuple(node, options)?;
        let handles = self.heading_handles()?;
        tuple
            .into_iter()
            .enumerate()
            .map(|(index, section_node)| {
                let Some(section_node) = section_node else {
                    return Ok(None);
                };
                let handle = if index == 0 {
                    handles
                        .section0_lang_handles
                        .get(lang)
                        .or_else(|| handles.section0_lang_handles.get(""))
                        .or_else(|| handles.section_feature_handles.first())
                } else {
                    handles.section_feature_handles.get(index)
                };
                match handle {
                    Some(Some(handle)) => Ok(handle.value(section_node)?.map(FeatureValue::from)),
                    _ => Ok(None),
                }
            })
            .collect()
    }

    /// The section-0 feature for a language code: its `@<lang>` variant when
    /// present, else the base-code feature, else the first section feature.
    pub fn sec0_feature_for_lang(&self, lang: &str) -> Option<&str> {
        let features = &self.sec0_lang_features;
        features
            .get(lang)
            .or_else(|| features.get(""))
            .map(String::as_str)
            .or_else(|| self.config.section_features.first().map(String::as_str))
    }

    /// `T.<section0>Name` (e.g. `T.bookName`): the name, in language `lang`,
    /// of the section-0 node containing `node` (or of `node` itself).
    pub fn section_0_name(&self, node: u32, lang: &str) -> Result<Option<String>> {
        let Some(section_0_type) = self.config.section_types.first() else {
            return Ok(None);
        };
        let sections = self.sections();
        let section_0_node = if sections.node_type(node)? == Some(section_0_type.as_str()) {
            node
        } else {
            match sections.u(node, Some(section_0_type))?.first() {
                Some(embedder) => *embedder,
                None => return Ok(None),
            }
        };
        let Some(feature) = self.sec0_feature_for_lang(lang) else {
            return Ok(None);
        };
        Ok(
            match sections.node_feature_value(feature, section_0_node)? {
                Some(FeatureValue::Str(name)) => Some(name.to_string()),
                _ => None,
            },
        )
    }

    /// `T.<section0>Node` (e.g. `T.bookNode`): the section-0 node named `name`
    /// in language `lang`; of several with that name, the last in canonical
    /// order.
    pub fn section_0_node(&self, name: &str, lang: &str) -> Result<Option<u32>> {
        let Some(feature) = self.sec0_feature_for_lang(lang) else {
            return Ok(None);
        };
        Ok(self
            .sec0_value_index(feature)?
            .get(&FeatureValue::string(name))
            .copied())
    }

    /// Section-0 nodes in canonical order, scanned from `otype` once.
    fn sec0_nodes(&self) -> Result<&[u32]> {
        if let Some(nodes) = self.sec0_nodes.get() {
            return Ok(nodes);
        }
        let nodes = match self.config.section_types.first() {
            Some(section_0_type) => self.otype.view().s(section_0_type)?,
            None => Vec::new(),
        };
        let _ = self.sec0_nodes.set(nodes);
        Ok(self
            .sec0_nodes
            .get()
            .expect("section-0 nodes just initialized"))
    }

    /// The `value -> node` lookup for one section-0 feature (the base feature or
    /// a `book@<lang>` variant), built once per feature. When several nodes share
    /// a value the last one wins, as in TF's `nodeFromName` dict.
    fn sec0_value_index(&self, feature_name: &str) -> Result<Arc<HashMap<FeatureValue, u32>>> {
        if let Some(index) = self
            .sec0_index
            .read()
            .expect("sec0 index poisoned")
            .get(feature_name)
        {
            return Ok(Arc::clone(index));
        }
        let sections = self.sections();
        let mut index = HashMap::new();
        for node in self.sec0_nodes()? {
            if let Some(value) = sections.node_feature_value(feature_name, *node)? {
                index.insert(value, *node);
            }
        }
        let index = Arc::new(index);
        self.sec0_index
            .write()
            .expect("sec0 index poisoned")
            .insert(feature_name.to_string(), Arc::clone(&index));
        Ok(index)
    }

    /// Cached-handle version of [`MappedSections::node_from_section_langed`]:
    /// the section-0 node resolves through the lazily-built value index (O(1))
    /// instead of a per-call scan over every node of the section-0 type. The
    /// `sec1`/`sec2` lookups and all fallback semantics are identical.
    pub fn node_from_section_langed(
        &self,
        section: &[FeatureValue],
        sec0_feature: Option<&str>,
    ) -> Result<Option<u32>> {
        // TF reads at most the first three parts of a section (text.py).
        let section = &section[..section.len().min(3)];
        if section.is_empty()
            || section.len() > self.config.section_types.len()
            || section.len() > self.config.section_features.len()
        {
            return Ok(None);
        }

        if section.len() <= 3 {
            if let Some(sections_data) = self.corpus.sections_data()? {
                let feature_name = match sec0_feature {
                    Some(feature) => feature,
                    None => match self.config.section_features.first() {
                        Some(feature) => feature.as_str(),
                        None => return Ok(None),
                    },
                };
                let Some(sec0_node) = self
                    .sec0_value_index(feature_name)?
                    .get(&section[0])
                    .copied()
                else {
                    return Ok(None);
                };
                return Ok(match section.len() {
                    1 => Some(sec0_node),
                    2 => sections_data
                        .sec1
                        .get(&sec0_node)
                        .and_then(|headings| headings.get(section_key(&section[1]).as_ref()))
                        .copied(),
                    _ => sections_data
                        .sec2
                        .get(&sec0_node)
                        .and_then(|level1| level1.get(section_key(&section[1]).as_ref()))
                        .and_then(|level2| level2.get(section_key(&section[2]).as_ref()))
                        .copied(),
                });
            }
        }

        // One-level sections or hierarchies deeper than `CFRSECT1` stores:
        // defer to the linear lookup.
        self.sections()
            .node_from_section_langed(section, sec0_feature)
    }

    /// Every embedder of `node` (`L.u` without a type filter) in `L.u` order,
    /// borrowed from the `levUp` CSR without building a `Vec` or a
    /// [`MappedSections`] view. `None` for a node outside the corpus.
    pub fn embedders(&self, node: u32) -> Result<Option<MappedU32s<'_>>> {
        self.corpus.lev_up_values(node)
    }

    /// Build a thin [`MappedSections`] borrowing the cached owned handles. No
    /// corpus view-cache access and no allocation: the config is shared by `Arc`.
    pub fn sections(&self) -> MappedSections<'_> {
        MappedSections {
            corpus: self.otype.corpus(),
            otype: self.otype.view(),
            oslots: self.oslots.view(),
            config: Arc::clone(&self.config),
        }
    }
}

pub struct MappedSections<'a> {
    corpus: &'a MappedCompiledCorpus,
    otype: StringPoolNodeFeatureView<'a>,
    oslots: EdgeFeatureView<'a>,
    config: Arc<SectionConfig>,
}

impl<'a> MappedSections<'a> {
    pub fn new(corpus: &'a MappedCompiledCorpus) -> Result<Self> {
        let otype = corpus
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let config = Arc::new(SectionConfig::load(corpus, &otype)?);
        Ok(Self {
            corpus,
            otype,
            oslots,
            config,
        })
    }

    pub fn section_types(&self) -> &[String] {
        &self.config.section_types
    }

    pub fn section_features(&self) -> &[String] {
        &self.config.section_features
    }

    pub fn section_tuple(&self, node: u32, options: &SectionOptions) -> Result<Vec<Option<u32>>> {
        if self.config.section_types.is_empty() || node == 0 || self.node_type(node)?.is_none() {
            return Ok(Vec::new());
        }

        let node_type = self.node_type(node)?.unwrap_or_default();
        let reference_slot = if node_type == self.config.slot_type {
            node
        } else {
            let slots = self.slots(node)?;
            let Some(slot) = (if options.last_slot {
                slots.last()
            } else {
                slots.first()
            }) else {
                return Ok(Vec::new());
            };
            *slot
        };

        // Every section-type embedder of `reference_slot`, resolved from its
        // `levUp` CSR row in a single pass: the row holds exactly the nodes
        // containing the slot, and a slot is contained by at most one node of
        // each section type.
        let section_embedders = self.section_embedders(reference_slot)?;

        let mut sections = Vec::new();
        for (index, section_type) in self.config.section_types.iter().enumerate() {
            let section_node = if node_type == section_type {
                Some(node)
            } else {
                section_embedders[index]
            };
            if section_node.is_some() || options.fillup || index == 0 {
                sections.push(section_node);
                if node_type == section_type && !options.fillup {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(level) = options.level {
            sections.truncate(level);
        }
        Ok(sections)
    }

    pub fn section_from_node(
        &self,
        node: u32,
        options: &SectionOptions,
    ) -> Result<Vec<Option<FeatureValue>>> {
        self.section_tuple(node, options)?
            .into_iter()
            .enumerate()
            .map(|(index, section_node)| {
                let Some(section_node) = section_node else {
                    return Ok(None);
                };
                let Some(feature_name) = self.config.section_features.get(index) else {
                    return Ok(None);
                };
                self.node_feature_value(feature_name, section_node)
            })
            .collect()
    }

    /// Like [`node_from_section`](Self::node_from_section), but resolves the
    /// section-0 component with `sec0_feature` (a language-specific variant such
    /// as `book@en`) when supplied, mirroring TF `nodeFromSection(lang=...)`.
    /// Deeper levels always use the configured `sectionFeatures`.
    pub fn node_from_section_langed(
        &self,
        section: &[FeatureValue],
        sec0_feature: Option<&str>,
    ) -> Result<Option<u32>> {
        // TF reads at most the first three parts of a section (text.py).
        let section = &section[..section.len().min(3)];
        if section.is_empty()
            || section.len() > self.config.section_types.len()
            || section.len() > self.config.section_features.len()
        {
            return Ok(None);
        }

        // Resolve the section-0 node by value, then index the
        // CFRSECT1 `sec1`/`sec2` lookup tables by heading key (mirrors TF
        // `text.py` `nodeFromSection`). O(section-0 nodes) + O(log n) map lookups
        // instead of a linear scan over every node of the target type. When the
        // index is present, a miss returns `None` immediately — no linear rescan.
        if section.len() <= 3 {
            if let Some(sections) = self.corpus.sections_data()? {
                let Some(sec0_node) = self.sec0_node_with_feature(&section[0], sec0_feature)?
                else {
                    return Ok(None);
                };
                return Ok(match section.len() {
                    1 => Some(sec0_node),
                    2 => sections
                        .sec1
                        .get(&sec0_node)
                        .and_then(|headings| headings.get(section_key(&section[1]).as_ref()))
                        .copied(),
                    _ => sections
                        .sec2
                        .get(&sec0_node)
                        .and_then(|level1| level1.get(section_key(&section[1]).as_ref()))
                        .and_then(|level2| level2.get(section_key(&section[2]).as_ref()))
                        .copied(),
                });
            }
        }

        // Linear lookup for one-level sections and hierarchies deeper than the
        // three levels stored in CFRSECT1.
        // The last matching node wins, as in TF's section dicts.
        let target_type = &self.config.section_types[section.len() - 1];
        for node in self.otype.s(target_type)?.into_iter().rev() {
            let matches = section
                .iter()
                .enumerate()
                .map(|(index, expected)| {
                    let options = SectionOptions {
                        fillup: true,
                        level: Some(index + 1),
                        ..SectionOptions::default()
                    };
                    Ok(self
                        .section_from_node(node, &options)?
                        .get(index)
                        .and_then(Option::as_ref)
                        == Some(expected))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .all(|matches| matches);
            if matches {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    pub fn node_type(&self, node: u32) -> Result<Option<&'a str>> {
        self.otype.str_value(node)
    }

    pub fn is_slot(&self, node: u32) -> Result<bool> {
        Ok(self.node_type(node)? == Some(self.config.slot_type.as_str()))
    }

    pub fn slots(&self, node: u32) -> Result<Vec<u32>> {
        if self.is_slot(node)? {
            return Ok(vec![node]);
        }
        self.oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    pub fn up(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.up_types(node, Some(&[node_type])),
            None => self.up_types(node, None),
        }
    }

    pub fn u(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.up(node, node_type)
    }

    pub fn intersecting(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.intersecting_types(node, Some(&[node_type])),
            None => self.intersecting_types(node, None),
        }
    }

    pub fn i(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.intersecting(node, node_type)
    }

    pub fn intersecting_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let max_slot = self.max_slot()?;
        if node <= max_slot || node > self.max_node() {
            return Ok(Vec::new());
        }
        let slots = self.slots(node)?;
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        let slot_type_allowed =
            node_types.is_none_or(|expected| expected.contains(&self.config.slot_type.as_str()));

        // Intersectors are the union of `levUp` embedders over the node's slots
        // (per TF `locality.py` `i`), plus the slots themselves when the slot
        // type is requested. `levUp` rows never contain slot nodes.
        let mut intersectors: HashSet<u32> = HashSet::new();
        for slot in &slots {
            for embedder in self.corpus.lev_up_row(*slot)?.unwrap_or_default() {
                if self.node_type_allowed(embedder, node_types)? {
                    intersectors.insert(embedder);
                }
            }
            if slot_type_allowed {
                intersectors.insert(*slot);
            }
        }
        intersectors.remove(&node);
        let mut result: Vec<u32> = intersectors.into_iter().collect();
        // TF returns `sortNodes(result - {n})`: canonical (ascending rank) order.
        self.sort_nodes(&mut result)?;
        Ok(result)
    }

    pub fn up_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        // `levUp` rows are precomputed in descending-rank order (right/small
        // embedders before left/big, per TF `L.u`) and already exclude `node`
        // itself and slot nodes; just apply the otype filter, preserving order.
        let row = self.corpus.lev_up_row(node)?.unwrap_or_default();
        self.filter_locality_nodes(row, node_types)
    }

    pub fn down(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.down_types(node, Some(&[node_type])),
            None => self.down_types(node, None),
        }
    }

    pub fn d(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.down(node, node_type)
    }

    pub fn down_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let slots: Vec<u32> = self
            .oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))?;
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        let slot_type_allowed =
            node_types.is_none_or(|expected| expected.contains(&self.config.slot_type.as_str()));

        // `levDown` rows hold the non-slot embeddees; combine them with the
        // node's slots (when the slot type is requested) and sort into canonical
        // (ascending rank) order.
        let mut result = Vec::new();
        if slot_type_allowed {
            result.extend_from_slice(&slots);
        }
        for candidate in self.corpus.lev_down_row(node)?.unwrap_or_default() {
            if self.node_type_allowed(candidate, node_types)? {
                result.push(candidate);
            }
        }
        self.sort_nodes(&mut result)?;
        Ok(result)
    }

    pub fn next(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.next_types(node, Some(&[node_type])),
            None => self.next_types(node, None),
        }
    }

    pub fn n(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.next(node, node_type)
    }

    pub fn next_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let max_slot = self.max_slot()?;
        let max_node = self.max_node();
        if node == 0 || node == max_slot || node > max_node {
            return Ok(Vec::new());
        }
        let Some(next_slot) = self.adjacent_slot(node, true)? else {
            return Ok(Vec::new());
        };
        if next_slot > max_slot {
            return Ok(Vec::new());
        }

        // Nodes whose first slot is `next_slot`, from the boundary CSR
        // (descending-rank order).
        let starting = self.corpus.boundary_first(next_slot)?.unwrap_or_default();
        let mut result = vec![next_slot];
        result.extend(starting);
        self.filter_locality_nodes(result, node_types)
    }

    pub fn previous(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.previous_types(node, Some(&[node_type])),
            None => self.previous_types(node, None),
        }
    }

    pub fn p(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.previous(node, node_type)
    }

    pub fn previous_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let max_node = self.max_node();
        if node <= 1 || node > max_node {
            return Ok(Vec::new());
        }
        let Some(previous_slot) = self.adjacent_slot(node, false)? else {
            return Ok(Vec::new());
        };
        if previous_slot == 0 {
            return Ok(Vec::new());
        }

        // Nodes whose last slot is `previous_slot`, from the boundary CSR
        // (ascending-rank order).
        let mut result = self
            .corpus
            .boundary_last(previous_slot)?
            .unwrap_or_default();
        result.push(previous_slot);
        self.filter_locality_nodes(result, node_types)
    }

    /// Every node in canonical order: the precomputed `order` array when the
    /// cache has one for every node, else a rank sort.
    pub fn all_nodes(&self) -> Result<Vec<u32>> {
        if let Some(order) = self.corpus.canonical_order()? {
            return Ok(order.collect());
        }
        let mut nodes: Vec<_> = (1..=self.max_node()).collect();
        self.sort_nodes(&mut nodes)?;
        Ok(nodes)
    }

    pub fn walk(&self, nodes: Option<&[u32]>) -> Result<Vec<u32>> {
        match nodes {
            Some(nodes) => {
                let mut nodes = nodes.to_vec();
                self.sort_nodes(&mut nodes)?;
                Ok(nodes)
            }
            None => self.all_nodes(),
        }
    }

    pub fn walk_events(&self, nodes: Option<&[u32]>) -> Result<Vec<WalkEvent>> {
        let walk_nodes = self.walk(nodes)?;
        let walk_node_set: Option<HashSet<u32>> =
            nodes.map(|nodes| nodes.iter().copied().collect());
        let mut events = Vec::new();

        for node in walk_nodes {
            if self.node_type(node)? == Some(self.config.slot_type.as_str()) {
                events.push(WalkEvent::Slot(node));
                let mut ending_nodes = self.corpus.boundary_last(node)?.unwrap_or_default();
                ending_nodes.reverse();
                for ending_node in ending_nodes {
                    if walk_node_set
                        .as_ref()
                        .is_none_or(|node_set| node_set.contains(&ending_node))
                    {
                        events.push(WalkEvent::End(ending_node));
                    }
                }
            } else {
                events.push(WalkEvent::Start(node));
            }
        }

        Ok(events)
    }

    /// Resolves the section-0 (e.g. book) node whose section-0 feature value
    /// equals `expected`. Section-0 nodes are few, so a direct scan is cheap and
    /// matches the linear `node_from_section` reference exactly.
    /// Resolves the section-0 node whose section-0 feature value equals
    /// `expected`, matching against `sec0_feature` when given (a language variant)
    /// otherwise the configured `sectionFeatures[0]`.
    fn sec0_node_with_feature(
        &self,
        expected: &FeatureValue,
        sec0_feature: Option<&str>,
    ) -> Result<Option<u32>> {
        let Some(section_0_type) = self.config.section_types.first() else {
            return Ok(None);
        };
        let feature_name = match sec0_feature {
            Some(feature) => feature,
            None => match self.config.section_features.first() {
                Some(feature) => feature.as_str(),
                None => return Ok(None),
            },
        };
        // The last node with the value, as in TF's `nodeFromName` dict.
        for node in self.otype.s(section_0_type)?.into_iter().rev() {
            if self.node_feature_value(feature_name, node)?.as_ref() == Some(expected) {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    /// For each configured section type, the embedder of `reference_slot` of that
    /// type (or `None` if none), read from the slot's `levUp` CSR row. The
    /// returned vector is parallel to `self.config.section_types`.
    fn section_embedders(&self, reference_slot: u32) -> Result<Vec<Option<u32>>> {
        let mut found: Vec<Option<u32>> = vec![None; self.config.section_types.len()];
        for embedder in self.corpus.lev_up_row(reference_slot)?.unwrap_or_default() {
            let Some(node_type) = self.node_type(embedder)? else {
                continue;
            };
            if let Some(index) = self
                .config
                .section_types
                .iter()
                .position(|section_type| section_type == node_type)
            {
                if found[index].is_none() {
                    found[index] = Some(embedder);
                }
            }
        }
        Ok(found)
    }

    fn node_type_allowed(&self, node: u32, node_types: Option<&[&str]>) -> Result<bool> {
        let Some(node_types) = node_types else {
            return Ok(true);
        };
        Ok(self
            .node_type(node)?
            .is_some_and(|node_type| node_types.contains(&node_type)))
    }

    fn filter_locality_nodes(
        &self,
        nodes: Vec<u32>,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<u32>> {
        let Some(node_types) = node_types else {
            return Ok(nodes);
        };
        nodes
            .into_iter()
            .filter_map(
                |node| match self.node_type_allowed(node, Some(node_types)) {
                    Ok(true) => Some(Ok(node)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .collect()
    }

    fn adjacent_slot(&self, node: u32, forward: bool) -> Result<Option<u32>> {
        if node <= self.max_slot()? {
            return Ok(if forward {
                node.checked_add(1)
            } else {
                node.checked_sub(1)
            });
        }
        let slots = self
            .oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))?;
        Ok(if forward {
            slots.last().and_then(|slot| slot.checked_add(1))
        } else {
            slots.first().and_then(|slot| slot.checked_sub(1))
        })
    }

    fn max_slot(&self) -> Result<u32> {
        self.corpus.max_slot()
    }

    fn max_node(&self) -> u32 {
        self.corpus.metadata().rank_len as u32
    }

    fn sort_nodes(&self, nodes: &mut [u32]) -> Result<()> {
        let mut keyed = nodes
            .iter()
            .map(|node| Ok((self.corpus.sort_key(*node)?.unwrap_or(u32::MAX), *node)))
            .collect::<Result<Vec<_>>>()?;
        keyed.sort_unstable_by_key(|(rank, _)| *rank);
        for (slot, (_, node)) in nodes.iter_mut().zip(keyed) {
            *slot = node;
        }
        Ok(())
    }

    pub fn node_feature_value(
        &self,
        feature_name: &str,
        node: u32,
    ) -> Result<Option<FeatureValue>> {
        if let Some(feature) = self.corpus.string_pool_node_feature(feature_name)? {
            return Ok(feature.str_value(node)?.map(FeatureValue::string));
        }
        if let Some(feature) = self.corpus.mixed_node_feature(feature_name)? {
            return Ok(feature.value(node)?.map(FeatureValue::from));
        }
        Ok(None)
    }
}
