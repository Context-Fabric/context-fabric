use std::collections::{BTreeMap, HashSet};

use crate::compiled::{
    EdgeFeatureView, MappedCompiledCorpus, MappedNodeValue, StringPoolNodeFeatureView,
};
use crate::corpus::{TextFormatInfo, TextFormatSample, TextRepresentationInfo};
use crate::error::{CfError, Result};

pub struct MappedText<'a> {
    corpus: &'a MappedCompiledCorpus,
    otype: StringPoolNodeFeatureView<'a>,
    oslots: EdgeFeatureView<'a>,
    slot_type: String,
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
        let Some(spec) = self.format_spec(format)? else {
            return Ok(String::new());
        };
        if self.is_slot(node)? {
            return self.text_for_slot(node, spec);
        }
        self.slots(node)?
            .into_iter()
            .map(|slot| self.text_for_slot(slot, spec))
            .collect()
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

    fn format_spec(&self, format: Option<&str>) -> Result<Option<&'a str>> {
        let format = format.unwrap_or("text-orig-full");
        let format_key = if format.starts_with("fmt:") {
            format.to_string()
        } else {
            format!("fmt:{format}")
        };
        let Some(otext) = self.corpus.config_feature("otext")? else {
            return Ok(None);
        };
        Ok(otext.get(&format_key)?.flatten())
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

    fn text_for_slot(&self, slot: u32, spec: &str) -> Result<String> {
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
