use std::collections::{BTreeMap, HashSet};

use crate::compiled::{
    EdgeFeatureView, MappedCompiledCorpus, MappedNodeValue, StringPoolNodeFeatureView,
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
        for feature_name in features.split('/') {
            let value = self.node_value_as_text(feature_name, slot)?;
            if !value.is_empty() {
                return Ok(value);
            }
        }
        Ok(default.map(render_format_literal).unwrap_or_default())
    }

    fn node_value_as_text(&self, feature_name: &str, node: u32) -> Result<String> {
        if let Some(feature) = self.corpus.string_pool_node_feature(feature_name)? {
            return Ok(feature.str_value(node)?.unwrap_or_default().to_string());
        }
        if let Some(feature) = self.corpus.mixed_node_feature(feature_name)? {
            return Ok(mapped_value_to_string(feature.value(node)?));
        }
        Ok(String::new())
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
