use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crate::compiled::{
    EdgeFeatureView, MappedCompiledCorpus, MappedNodeValue, OwnedEdgeFeature, OwnedMixedFeature,
    OwnedStringPoolFeature, StringPoolNodeFeatureView,
};
use crate::corpus::{TextFormatInfo, TextFormatSample, TextOptions, TextRepresentationInfo};
use crate::error::{CfError, Result};

pub struct MappedText<'a> {
    corpus: &'a MappedCompiledCorpus,
    otype: StringPoolNodeFeatureView<'a>,
    oslots: EdgeFeatureView<'a>,
    slot_type: String,
}

struct ResolvedMappedTextFormat {
    target_type: String,
    spec: String,
    implicit_node_default: bool,
}

impl<'a> MappedText<'a> {
    pub fn new(corpus: &'a MappedCompiledCorpus) -> Result<Self> {
        let otype = corpus
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype
            .str_value(1)?
            .ok_or_else(|| CfError::MissingFeature("otype slot type".to_string()))?
            .to_string();
        Ok(Self {
            corpus,
            otype,
            oslots,
            slot_type,
        })
    }

    pub fn text(&self, node: u32, format: Option<&str>) -> Result<String> {
        self.text_with_options(node, &TextOptions::new(format.map(str::to_string), None))
    }

    pub fn text_with_options(&self, node: u32, options: &TextOptions) -> Result<String> {
        let Some(node_type) = self.otype.str_value(node)? else {
            return Ok(String::new());
        };
        let Some(resolved) = self.resolve_text_format(node_type, options.format.as_deref())? else {
            return Ok(String::new());
        };
        let down_type = match options.descend {
            Some(true) => Some(resolved.target_type.as_str()),
            Some(false) => None,
            None => {
                if resolved.implicit_node_default {
                    None
                } else {
                    Some(resolved.target_type.as_str())
                }
            }
        }
        .filter(|down_type| *down_type != node_type);

        let nodes = match down_type {
            Some(down_type) if down_type == self.slot_type => self.slots(node)?,
            Some(down_type) => self.descendants_of_type(node, down_type)?,
            None => vec![node],
        };
        nodes
            .into_iter()
            .map(|text_node| self.text_for_node(text_node, &resolved.spec))
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn textWithOptions(&self, node: u32, options: &TextOptions) -> Result<String> {
        self.text_with_options(node, options)
    }

    pub fn text_nodes(&self, nodes: &[u32], format: Option<&str>) -> Result<String> {
        nodes
            .iter()
            .map(|node| self.text(*node, format))
            .collect::<Result<Vec<_>>>()
            .map(|parts| parts.concat())
    }

    #[allow(non_snake_case)]
    pub fn textNodes(&self, nodes: &[u32], format: Option<&str>) -> Result<String> {
        self.text_nodes(nodes, format)
    }

    pub fn split_format(&self, template: &str) -> Result<(String, String)> {
        let mut parts = template.splitn(2, '#');
        let first = parts.next().unwrap_or_default();
        let Some(rest) = parts.next() else {
            return Ok((self.slot_type.clone(), template.to_string()));
        };
        if self.node_types()?.contains(first) {
            Ok((first.to_string(), rest.to_string()))
        } else {
            Ok((self.slot_type.clone(), template.to_string()))
        }
    }

    #[allow(non_snake_case)]
    pub fn splitFormat(&self, template: &str) -> Result<(String, String)> {
        self.split_format(template)
    }

    pub fn split_default_format(&self, template: &str) -> Result<Option<String>> {
        let Some((node_type, suffix)) = template.rsplit_once('-') else {
            return Ok(None);
        };
        Ok(
            (suffix == "default" && self.node_types()?.contains(node_type))
                .then(|| node_type.to_string()),
        )
    }

    #[allow(non_snake_case)]
    pub fn splitDefaultFormat(&self, template: &str) -> Result<Option<String>> {
        self.split_default_format(template)
    }

    pub fn text_representations(&self) -> Result<TextRepresentationInfo> {
        let pairs = self.text_format_pairs()?;
        if pairs.is_empty() {
            return Ok(TextRepresentationInfo {
                description: "No text format metadata available or no orig/trans pairs defined"
                    .to_string(),
                formats: Vec::new(),
            });
        }

        let mut formats = Vec::new();
        for (base_name, original_name, transliterated_name, original_spec, transliterated_spec) in
            pairs
        {
            let (samples, unique_characters) =
                self.text_format_samples(&original_name, &transliterated_name)?;
            if samples.is_empty() {
                continue;
            }
            formats.push(TextFormatInfo {
                name: base_name,
                original_spec,
                transliteration_spec: transliterated_spec,
                total_samples: samples.len(),
                samples,
                unique_characters,
            });
        }

        Ok(TextRepresentationInfo {
            description: "Shows how text values are encoded in this corpus. Samples provide exhaustive character coverage for understanding the relationship between original script and transliterated forms.".to_string(),
            formats,
        })
    }

    fn text_format_pairs(&self) -> Result<Vec<(String, String, String, String, String)>> {
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(Vec::new());
        };
        let mut originals = BTreeMap::<String, (String, String)>::new();
        let mut transliterations = BTreeMap::<String, (String, String)>::new();
        for row in otext.items() {
            let (key, value) = row?;
            let Some(format_name) = key.strip_prefix("fmt:") else {
                continue;
            };
            let Some(spec) = value else {
                continue;
            };
            if format_name.contains("-orig-") {
                originals.insert(
                    format_name.replace("-orig-", "-"),
                    (format_name.to_string(), spec.to_string()),
                );
            } else if format_name.contains("-trans-") {
                transliterations.insert(
                    format_name.replace("-trans-", "-"),
                    (format_name.to_string(), spec.to_string()),
                );
            }
        }

        Ok(originals
            .into_iter()
            .filter_map(|(base_name, (original_name, original_spec))| {
                let (transliterated_name, transliterated_spec) =
                    transliterations.get(&base_name)?.clone();
                Some((
                    base_name,
                    original_name,
                    transliterated_name,
                    original_spec,
                    transliterated_spec,
                ))
            })
            .collect())
    }

    fn text_format_samples(
        &self,
        original_name: &str,
        transliterated_name: &str,
    ) -> Result<(Vec<TextFormatSample>, usize)> {
        let mut samples = Vec::new();
        let mut covered_chars = HashSet::new();
        let mut seen_originals = HashSet::new();
        let max_slot = self
            .otype
            .value_interval(&self.slot_type)?
            .map(|(_, last)| last)
            .unwrap_or_default();
        for slot in 1..=max_slot {
            let original = self.text(slot, Some(original_name))?.trim().to_string();
            if original.is_empty() || seen_originals.contains(&original) {
                continue;
            }
            let new_chars = original
                .chars()
                .filter(|ch| !covered_chars.contains(ch))
                .collect::<Vec<_>>();
            if new_chars.is_empty() {
                continue;
            }
            let transliterated = self
                .text(slot, Some(transliterated_name))?
                .trim()
                .to_string();
            if transliterated.is_empty() {
                continue;
            }
            covered_chars.extend(new_chars);
            seen_originals.insert(original.clone());
            samples.push(TextFormatSample {
                original,
                transliterated,
            });
        }
        Ok((samples, covered_chars.len()))
    }

    fn resolve_text_format(
        &self,
        node_type: &str,
        format: Option<&str>,
    ) -> Result<Option<ResolvedMappedTextFormat>> {
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(None);
        };
        let (format_name, implicit_node_default) = match format {
            Some(format) => (
                format.strip_prefix("fmt:").unwrap_or(format).to_string(),
                false,
            ),
            None => {
                let node_default = format!("{node_type}-default");
                if otext.get(&format!("fmt:{node_default}"))?.is_some() {
                    (node_default, true)
                } else {
                    ("text-orig-full".to_string(), false)
                }
            }
        };
        let Some(spec) = otext.get(&format!("fmt:{format_name}"))?.flatten() else {
            return Ok(None);
        };
        let (target_type, spec) = self.split_format(spec)?;
        Ok(Some(ResolvedMappedTextFormat {
            target_type,
            spec,
            implicit_node_default,
        }))
    }

    fn node_types(&self) -> Result<HashSet<String>> {
        Ok(self
            .otype
            .items()?
            .into_iter()
            .filter_map(|(_, value)| match value {
                MappedNodeValue::Str(value) => Some(value.to_string()),
                MappedNodeValue::Int(_) => None,
            })
            .collect())
    }

    fn is_slot(&self, node: u32) -> Result<bool> {
        Ok(self.otype.str_value(node)? == Some(self.slot_type.as_str()))
    }

    fn slots(&self, node: u32) -> Result<Vec<u32>> {
        if self.is_slot(node)? {
            return Ok(vec![node]);
        }
        self.oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    fn descendants_of_type(&self, node: u32, node_type: &str) -> Result<Vec<u32>> {
        let root_slots = self.slots(node)?;
        let Some(root_first) = root_slots.first().copied() else {
            return Ok(Vec::new());
        };
        let Some(root_last) = root_slots.last().copied() else {
            return Ok(Vec::new());
        };
        let mut descendants = Vec::new();
        for row in self.otype.rows() {
            let (candidate, candidate_type) = row?;
            if candidate_type != node_type {
                continue;
            }
            let slots = self.slots(candidate)?;
            if slots
                .first()
                .zip(slots.last())
                .is_some_and(|(first, last)| root_first <= *first && *last <= root_last)
            {
                descendants.push(candidate);
            }
        }
        Ok(descendants)
    }

    fn text_for_node(&self, slot: u32, spec: &str) -> Result<String> {
        let mut rendered = String::new();
        let mut rest = spec;
        while let Some(start) = rest.find('{') {
            rendered.push_str(&render_format_literal(&rest[..start]));
            let after_start = &rest[(start + 1)..];
            let Some(end) = after_start.find('}') else {
                rendered.push_str(&render_format_literal(&rest[start..]));
                return Ok(rendered);
            };
            let placeholder = &after_start[..end];
            rendered.push_str(&self.placeholder_value(slot, placeholder)?);
            rest = &after_start[(end + 1)..];
        }
        rendered.push_str(&render_format_literal(rest));
        Ok(rendered)
    }

    fn placeholder_value(&self, slot: u32, placeholder: &str) -> Result<String> {
        let (features, default) = placeholder
            .split_once(':')
            .map(|(features, default)| (features, Some(default)))
            .unwrap_or((placeholder, None));
        let default_rendered = default.map(render_format_literal).unwrap_or_default();
        let feature_names = features.split('/').collect::<Vec<_>>();
        // Mirrors TF's format substitution (`tf/core/text.py:_makeFunc`): a feature
        // value falls back to the next feature ONLY when it is *absent* for the
        // node (`f.get(n)` is `None`). An empty string is a valid present value and
        // must be emitted as-is — so we distinguish `None` (absent) from `Some("")`
        // (present-empty) rather than treating both as missing.
        match feature_names.as_slice() {
            // Single feature: `f.get(n, default)`.
            [single] => Ok(self
                .node_value_opt(single, slot)?
                .unwrap_or(default_rendered)),
            // Two features: `f1.get(n, f2.get(n, default))`.
            [first, second] => {
                if let Some(value) = self.node_value_opt(first, slot)? {
                    return Ok(value);
                }
                if let Some(value) = self.node_value_opt(second, slot)? {
                    return Ok(value);
                }
                Ok(default_rendered)
            }
            // Three or more: first present value, then `v or default` — Python
            // truthiness makes an empty present value fall through to the default.
            _ => {
                let mut found = None;
                for feature_name in &feature_names {
                    if let Some(value) = self.node_value_opt(feature_name, slot)? {
                        found = Some(value);
                        break;
                    }
                }
                Ok(match found {
                    Some(value) if !value.is_empty() => value,
                    _ => default_rendered,
                })
            }
        }
    }

    /// Reads a node feature value as text, preserving the absent/present-empty
    /// distinction: `None` means the feature has no value for `node`, `Some("")`
    /// means it has an explicit empty value.
    fn node_value_opt(&self, feature_name: &str, node: u32) -> Result<Option<String>> {
        if let Some(feature) = self.corpus.string_pool_node_feature(feature_name)? {
            return Ok(feature.str_value(node)?.map(str::to_string));
        }
        if let Some(feature) = self.corpus.mixed_node_feature(feature_name)? {
            return Ok(feature
                .value(node)?
                .map(|value| mapped_value_to_string(Some(value))));
        }
        Ok(None)
    }

    /// `T.formats` — the map from each configured text format name to its
    /// descend (target) node type, mirroring text-fabric's `T.formats`
    /// (`tf/core/text.py:_compileFormats`, `formats[fmt] = descendType`).
    ///
    /// Format names come from the `fmt:<name>` keys of the `otext` config; the
    /// descend type is `splitFormat(tpl)[0]` (the node-type prefix before `#`,
    /// defaulting to the slot type).
    pub fn formats(&self) -> Result<BTreeMap<String, String>> {
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(BTreeMap::new());
        };
        let mut formats = BTreeMap::new();
        for row in otext.items() {
            let (key, value) = row?;
            let Some(format_name) = key.strip_prefix("fmt:") else {
                continue;
            };
            let spec = value.unwrap_or("");
            let (descend_type, _tpl) = self.split_format(spec)?;
            formats.insert(format_name.to_string(), descend_type);
        }
        Ok(formats)
    }

    /// `T.languages` — the map from language code to its
    /// `{language, languageEnglish}` metadata, mirroring text-fabric
    /// (`tf/core/text.py:432-444`).
    ///
    /// The relevant features are the first section feature (e.g. `book`) and all
    /// its language variants (`book@<code>`); the code is each feature's
    /// `languageCode` metadata (empty string for the base feature), and the two
    /// inner values default to `"default"` when absent.
    pub fn languages(&self) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
        let mut languages = BTreeMap::new();
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(languages);
        };
        let section_feats = otext.get("sectionFeatures")?.flatten().unwrap_or("");
        let section_types = otext.get("sectionTypes")?.flatten().unwrap_or("");
        let first_section_feat = section_feats
            .split(',')
            .map(str::trim)
            .find(|item| !item.is_empty());
        // TF only populates languages when both sectionFeatures and
        // sectionTypes are configured.
        let (Some(base), false) = (
            first_section_feat,
            section_types.split(',').all(|item| item.trim().is_empty()),
        ) else {
            return Ok(languages);
        };
        let prefix = format!("{base}@");
        for name in self.corpus.all_node_features(true) {
            if name != base && !name.starts_with(&prefix) {
                continue;
            }
            let Some(view) = self.corpus.node_feature(&name)? else {
                continue;
            };
            let code = view.metadata_value("languageCode").unwrap_or("").to_string();
            let mut info = BTreeMap::new();
            for key in ["language", "languageEnglish"] {
                info.insert(
                    key.to_string(),
                    view.metadata_value(key).unwrap_or("default").to_string(),
                );
            }
            languages.insert(code, info);
        }
        Ok(languages)
    }

    /// `T.structureInfo()` — a human-readable summary of how structure is
    /// configured, mirroring text-fabric's `T.structureInfo` (`tf/core/text.py:655`),
    /// backed by `structure_data()`.
    ///
    /// TF prints this summary and returns `None`; here we return the formatted
    /// string so callers can print it (or assert on it). When no structure is
    /// configured we return TF's "No structural elements configured" line.
    pub fn structure_info(&self) -> Result<String> {
        let Some(data) = self.corpus.structure_data()? else {
            return Ok("No structural elements configured".to_string());
        };
        let headings = self.structure_headings()?;
        let n_structure = data.heading_from_node.len();
        let mut out = String::new();
        out.push_str("A heading is a tuple of pairs (node type, feature value)\n");
        out.push_str(
            "\tof node types and features that have been configured as structural elements\n",
        );
        out.push_str(&format!(
            "These {} structural elements have been configured\n",
            headings.len()
        ));
        for (node_type, feature) in &headings {
            out.push_str(&format!(
                "\tnode type {node_type:<10} with heading feature {feature}\n"
            ));
        }
        out.push_str("You can get them as a tuple with T.headings.\n");
        out.push_str(&format!(
            "\nThere are {n_structure} structural elements in the dataset.\n"
        ));
        if !data.multiple.is_empty() {
            let n_multiple = data.multiple.len();
            let t_multiple: usize = data.multiple.values().map(Vec::len).sum();
            out.push_str(&format!(
                "WARNING: {n_multiple} structure headings with hdMult occurrences (total {t_multiple})\n"
            ));
            for (key, nodes) in data.multiple.iter().take(10) {
                let key_rep = key
                    .iter()
                    .map(|part| format!("{}:{}", part.node_type, part.heading))
                    .collect::<Vec<_>>()
                    .join("-");
                let n_nodes = nodes.len();
                out.push_str(&format!("\t{key_rep} has {n_nodes} occurrences\n"));
                let sample = nodes
                    .iter()
                    .take(5)
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("\t\t{sample}\n"));
                if n_nodes > 5 {
                    out.push_str(&format!("\t\tand {} more\n", n_nodes - 5));
                }
            }
        }
        Ok(out)
    }

    fn structure_headings(&self) -> Result<Vec<(String, String)>> {
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(Vec::new());
        };
        let parse = |raw: &str| -> Vec<String> {
            raw.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        };
        let types = parse(otext.get("structureTypes")?.flatten().unwrap_or(""));
        let feats = parse(otext.get("structureFeatures")?.flatten().unwrap_or(""));
        Ok(types.into_iter().zip(feats).collect())
    }
}

/// A feature handle cached inside a [`TextContext`]. Resolved once per distinct
/// feature name referenced by any compiled format, so rendering never re-touches
/// the corpus view cache. Preserves the absent (`None`) vs present-empty
/// (`Some("")`) distinction that the format fallback semantics depend on.
enum TextFeatureHandle {
    StringPool(OwnedStringPoolFeature),
    Mixed(OwnedMixedFeature),
    Absent,
}

impl TextFeatureHandle {
    fn value(&self, node: u32) -> Result<Option<String>> {
        match self {
            Self::StringPool(handle) => Ok(handle.view().str_value(node)?.map(str::to_string)),
            Self::Mixed(handle) => Ok(handle
                .view()
                .value(node)?
                .map(|value| mapped_value_to_string(Some(value)))),
            Self::Absent => Ok(None),
        }
    }
}

/// One step of a compiled `otext` format spec.
enum TextOp {
    /// A constant literal (escapes already expanded at compile time).
    Literal(String),
    /// A `{feat1/feat2/...:default}` placeholder. `features` holds indices into
    /// the context handle table; `default` is the pre-rendered default literal.
    Placeholder { features: Vec<usize>, default: String },
}

/// A single `otext` format compiled to a flat op-list plus its descend (target)
/// node type and the feature handles its placeholders reference. Compiled lazily
/// on the first `T.text` call that uses the format (see [`TextContext::format`]).
struct CompiledFormat {
    target_type: String,
    ops: Vec<TextOp>,
    handles: Vec<TextFeatureHandle>,
}

/// Long-lived text context. Built once by a binding object; holds the
/// `otype`/`oslots` owned handles and the raw `otext` format specs. Each format
/// is compiled to an op-list + resolved feature handles **lazily**, on the first
/// `T.text` call that uses it, and cached. This keeps construction cheap (no
/// eager parts-build for the ~30 features of every BHSA format — a corpus may
/// define a dozen formats but a process typically renders one) while making
/// steady-state `T.text` a loop over cached ops with no per-call `otext` parse
/// or view-cache lookup.
pub struct TextContext {
    corpus: Arc<MappedCompiledCorpus>,
    otype: OwnedStringPoolFeature,
    oslots: OwnedEdgeFeature,
    slot_type: String,
    node_types: HashSet<String>,
    /// Format name -> raw `otext` spec (the `fmt:` value verbatim). Cheap to
    /// hold; compilation is deferred to [`TextContext::format`].
    raw_formats: HashMap<String, String>,
    has_text_orig_full: bool,
    /// Lazily-compiled formats, keyed by format name.
    compiled: std::sync::RwLock<HashMap<String, std::sync::Arc<CompiledFormat>>>,
}

impl TextContext {
    pub fn new(corpus: &Arc<MappedCompiledCorpus>) -> Result<Self> {
        let otype = corpus
            .owned_string_pool_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .owned_edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype
            .str_value(1)?
            .ok_or_else(|| CfError::MissingFeature("otype slot type".to_string()))?;

        // The set of node types (for the `type#tpl` split), read straight from
        // the `otype` string pool — O(#types) with no per-node scan. (Both
        // `otype_rank()`, which reads `oslots` for every one of the ~1.4M nodes,
        // and a full `otype` row scan cost tens of ms on BHSA; the distinct
        // values of the `otype` feature *are* the node types.)
        let node_types: HashSet<String> = otype
            .view()
            .distinct_values()?
            .into_iter()
            .map(str::to_string)
            .collect();

        // Record the raw format specs only; defer compilation (and the feature
        // parts-build it triggers) to first use.
        let mut raw_formats: HashMap<String, String> = HashMap::new();
        if let Some(otext) = corpus.config_feature("otext")? {
            for row in otext.items() {
                let (key, value) = row?;
                let Some(format_name) = key.strip_prefix("fmt:") else {
                    continue;
                };
                raw_formats.insert(format_name.to_string(), value.unwrap_or("").to_string());
            }
        }

        let has_text_orig_full = raw_formats.contains_key("text-orig-full");

        Ok(Self {
            corpus: Arc::clone(corpus),
            otype,
            oslots,
            slot_type,
            node_types,
            raw_formats,
            has_text_orig_full,
            compiled: std::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Lazily compile (and cache) the named format. Returns `None` when the
    /// format is not defined in `otext`. Compilation resolves only the feature
    /// handles this format references, so the first render of a single format
    /// never touches the other formats' (potentially large) features.
    fn format(&self, name: &str) -> Result<Option<std::sync::Arc<CompiledFormat>>> {
        if let Some(format) = self
            .compiled
            .read()
            .expect("text format cache poisoned")
            .get(name)
        {
            return Ok(Some(std::sync::Arc::clone(format)));
        }
        let Some(raw_spec) = self.raw_formats.get(name) else {
            return Ok(None);
        };
        let (target_type, tpl) =
            split_format_with_types(&self.node_types, &self.slot_type, raw_spec);
        let mut handles: Vec<TextFeatureHandle> = Vec::new();
        let mut handle_index: HashMap<String, usize> = HashMap::new();
        let ops = compile_spec(&self.corpus, &tpl, &mut handles, &mut handle_index)?;
        let format = std::sync::Arc::new(CompiledFormat {
            target_type,
            ops,
            handles,
        });
        self.compiled
            .write()
            .expect("text format cache poisoned")
            .insert(name.to_string(), std::sync::Arc::clone(&format));
        Ok(Some(format))
    }

    pub fn text_with_options(&self, node: u32, options: &TextOptions) -> Result<String> {
        let otype = self.otype.view();
        let Some(node_type) = otype.str_value(node)? else {
            return Ok(String::new());
        };
        // Resolve the format name and whether it is the implicit node-default.
        let (format_name, implicit_node_default) = match options.format.as_deref() {
            Some(format) => (
                format.strip_prefix("fmt:").unwrap_or(format).to_string(),
                false,
            ),
            None => {
                let node_default = format!("{node_type}-default");
                if self.raw_formats.contains_key(&node_default) {
                    (node_default, true)
                } else if self.has_text_orig_full {
                    ("text-orig-full".to_string(), false)
                } else {
                    return Ok(String::new());
                }
            }
        };
        let Some(format) = self.format(&format_name)? else {
            return Ok(String::new());
        };

        let down_type = match options.descend {
            Some(true) => Some(format.target_type.as_str()),
            Some(false) => None,
            None => {
                if implicit_node_default {
                    None
                } else {
                    Some(format.target_type.as_str())
                }
            }
        }
        .filter(|down_type| *down_type != node_type);

        let nodes = match down_type {
            Some(down_type) if down_type == self.slot_type => self.slots(node)?,
            Some(down_type) => self.descendants_of_type(node, down_type)?,
            None => vec![node],
        };

        let mut rendered = String::new();
        for text_node in nodes {
            self.render_node(text_node, &format, &mut rendered)?;
        }
        Ok(rendered)
    }

    fn render_node(&self, node: u32, format: &CompiledFormat, out: &mut String) -> Result<()> {
        for op in &format.ops {
            match op {
                TextOp::Literal(literal) => out.push_str(literal),
                TextOp::Placeholder { features, default } => {
                    out.push_str(&self.render_placeholder(&format.handles, node, features, default)?);
                }
            }
        }
        Ok(())
    }

    /// Mirrors `MappedText::placeholder_value` exactly, including the
    /// absent/present-empty fallback semantics, but over cached handles.
    fn render_placeholder(
        &self,
        handles: &[TextFeatureHandle],
        node: u32,
        features: &[usize],
        default: &str,
    ) -> Result<String> {
        match features {
            [single] => {
                Ok(handles[*single].value(node)?.unwrap_or_else(|| default.to_string()))
            }
            [first, second] => {
                if let Some(value) = handles[*first].value(node)? {
                    return Ok(value);
                }
                if let Some(value) = handles[*second].value(node)? {
                    return Ok(value);
                }
                Ok(default.to_string())
            }
            _ => {
                let mut found = None;
                for index in features {
                    if let Some(value) = handles[*index].value(node)? {
                        found = Some(value);
                        break;
                    }
                }
                Ok(match found {
                    Some(value) if !value.is_empty() => value,
                    _ => default.to_string(),
                })
            }
        }
    }

    fn is_slot(&self, node: u32) -> Result<bool> {
        Ok(self.otype.view().str_value(node)? == Some(self.slot_type.as_str()))
    }

    fn slots(&self, node: u32) -> Result<Vec<u32>> {
        if self.is_slot(node)? {
            return Ok(vec![node]);
        }
        self.oslots
            .view()
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    fn descendants_of_type(&self, node: u32, node_type: &str) -> Result<Vec<u32>> {
        let root_slots = self.slots(node)?;
        let Some(root_first) = root_slots.first().copied() else {
            return Ok(Vec::new());
        };
        let Some(root_last) = root_slots.last().copied() else {
            return Ok(Vec::new());
        };
        let otype = self.otype.view();
        let oslots = self.oslots.view();
        let mut descendants = Vec::new();
        for row in otype.rows() {
            let (candidate, candidate_type) = row?;
            if candidate_type != node_type {
                continue;
            }
            let slots: Vec<u32> = oslots
                .targets(candidate)?
                .map(|targets| targets.collect())
                .unwrap_or_else(|| Ok(Vec::new()))?;
            if slots
                .first()
                .zip(slots.last())
                .is_some_and(|(first, last)| root_first <= *first && *last <= root_last)
            {
                descendants.push(candidate);
            }
        }
        Ok(descendants)
    }
}

/// Compile a format template (the part after the `type#` split) into ops,
/// registering each referenced feature handle in `handles`/`handle_index`.
/// Mirrors `MappedText::text_for_node` + `placeholder_value` parsing exactly,
/// but performed once: literal escapes and placeholder defaults are pre-rendered.
fn compile_spec(
    corpus: &Arc<MappedCompiledCorpus>,
    spec: &str,
    handles: &mut Vec<TextFeatureHandle>,
    handle_index: &mut HashMap<String, usize>,
) -> Result<Vec<TextOp>> {
    let mut ops = Vec::new();
    let mut rest = spec;
    while let Some(start) = rest.find('{') {
        if start > 0 {
            ops.push(TextOp::Literal(render_format_literal(&rest[..start])));
        }
        let after_start = &rest[(start + 1)..];
        let Some(end) = after_start.find('}') else {
            // Unterminated placeholder: the rest is literal (matches the runtime
            // fallback in `text_for_node`).
            ops.push(TextOp::Literal(render_format_literal(&rest[start..])));
            return Ok(ops);
        };
        let placeholder = &after_start[..end];
        let (features, default) = placeholder
            .split_once(':')
            .map(|(features, default)| (features, Some(default)))
            .unwrap_or((placeholder, None));
        let default = default.map(render_format_literal).unwrap_or_default();
        let feature_indices = features
            .split('/')
            .map(|name| resolve_handle(corpus, name, handles, handle_index))
            .collect::<Result<Vec<usize>>>()?;
        ops.push(TextOp::Placeholder {
            features: feature_indices,
            default,
        });
        rest = &after_start[(end + 1)..];
    }
    if !rest.is_empty() {
        ops.push(TextOp::Literal(render_format_literal(rest)));
    }
    Ok(ops)
}

fn resolve_handle(
    corpus: &Arc<MappedCompiledCorpus>,
    name: &str,
    handles: &mut Vec<TextFeatureHandle>,
    handle_index: &mut HashMap<String, usize>,
) -> Result<usize> {
    if let Some(index) = handle_index.get(name) {
        return Ok(*index);
    }
    let handle = if let Some(feature) = corpus.owned_string_pool_feature(name)? {
        TextFeatureHandle::StringPool(feature)
    } else if let Some(feature) = corpus.owned_mixed_feature(name)? {
        TextFeatureHandle::Mixed(feature)
    } else {
        TextFeatureHandle::Absent
    };
    let index = handles.len();
    handles.push(handle);
    handle_index.insert(name.to_string(), index);
    Ok(index)
}

/// Same split as `MappedText::split_format`, but against a pre-built node-type
/// set so no per-call `otype` scan is needed.
fn split_format_with_types(
    node_types: &HashSet<String>,
    slot_type: &str,
    template: &str,
) -> (String, String) {
    let mut parts = template.splitn(2, '#');
    let first = parts.next().unwrap_or_default();
    let Some(rest) = parts.next() else {
        return (slot_type.to_string(), template.to_string());
    };
    if node_types.contains(first) {
        (first.to_string(), rest.to_string())
    } else {
        (slot_type.to_string(), template.to_string())
    }
}

fn mapped_value_to_string(value: Option<MappedNodeValue<'_>>) -> String {
    match value {
        Some(MappedNodeValue::Str(value)) => value.to_string(),
        Some(MappedNodeValue::Int(value)) => value.to_string(),
        None => String::new(),
    }
}

fn render_format_literal(raw: &str) -> String {
    let mut rendered = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            rendered.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => rendered.push('\t'),
            Some('n') => rendered.push('\n'),
            Some(other) => {
                rendered.push('\\');
                rendered.push(other);
            }
            None => rendered.push('\\'),
        }
    }
    rendered
}
