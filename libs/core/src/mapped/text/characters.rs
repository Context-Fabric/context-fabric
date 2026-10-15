//! `C.characters`: character frequencies of the text formats.

use std::collections::BTreeMap;

use crate::error::Result;
use crate::mapped::text::FormatTemplate;
use crate::mapped::{MappedCompiledCorpus, MappedNodeValue};

impl MappedCompiledCorpus {
    /// `C.characters`: for every text format, how often each character occurs
    /// in the values of the features its template refers to, by character.
    pub fn format_characters(&self) -> Result<BTreeMap<String, BTreeMap<char, usize>>> {
        let mut by_feature: BTreeMap<String, BTreeMap<char, usize>> = BTreeMap::new();
        let mut by_format = BTreeMap::new();
        for (name, template) in &self.otext()?.formats {
            let template = FormatTemplate::parse(template);
            let mut counts: BTreeMap<char, usize> = BTreeMap::new();
            for feature in template.features() {
                if !by_feature.contains_key(feature) {
                    let feature_counts = self.feature_characters(feature)?;
                    by_feature.insert(feature.to_string(), feature_counts);
                }
                for (character, count) in &by_feature[feature] {
                    *counts.entry(*character).or_default() += count;
                }
            }
            by_format.insert(name.clone(), counts);
        }
        Ok(by_format)
    }

    fn feature_characters(&self, feature: &str) -> Result<BTreeMap<char, usize>> {
        let mut counts = BTreeMap::new();
        let Some(view) = self.node_feature(feature)? else {
            return Ok(counts);
        };
        for (_, value) in view.items()? {
            let mut count = |text: &str| {
                for character in text.chars() {
                    *counts.entry(character).or_default() += 1;
                }
            };
            match value {
                MappedNodeValue::Str(text) => count(text),
                MappedNodeValue::Int(number) => count(&number.to_string()),
            }
        }
        Ok(counts)
    }
}
