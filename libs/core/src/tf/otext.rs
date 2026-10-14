//! The `otext` config feature: how a corpus is sectioned, structured and
//! rendered as text.

use std::collections::BTreeMap;

/// The parsed `otext` config feature. Comma-separated settings become lists
/// (items trimmed, empty items dropped); every setting is optional.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Otext {
    /// Section levels from big to small (`@sectionTypes`, e.g. book, chapter, verse).
    pub section_types: Vec<String>,
    /// The feature naming each section level (`@sectionFeatures`).
    pub section_features: Vec<String>,
    /// Structure levels (`@structureTypes`).
    pub structure_types: Vec<String>,
    /// The heading feature of each structure level (`@structureFeatures`).
    pub structure_features: Vec<String>,
    /// Text formats by name, without the `fmt:` prefix: the format template.
    pub formats: BTreeMap<String, String>,
    /// `@levelConstraints`: explicit order constraints between node types.
    pub level_constraints: Option<String>,
}

impl Otext {
    pub fn from_metadata(metadata: &BTreeMap<String, Option<String>>) -> Self {
        let value = |key: &str| metadata.get(key).and_then(Option::as_deref);
        let list = |key: &str| csv_list(value(key));
        Self {
            section_types: list("sectionTypes"),
            section_features: list("sectionFeatures"),
            structure_types: list("structureTypes"),
            structure_features: list("structureFeatures"),
            formats: metadata
                .iter()
                .filter_map(|(key, template)| {
                    let name = key.strip_prefix("fmt:")?;
                    Some((name.to_string(), template.clone().unwrap_or_default()))
                })
                .collect(),
            level_constraints: value("levelConstraints").map(str::to_string),
        }
    }

    /// Whether sections are configured the way Text-Fabric requires: one to
    /// three levels, each with a section feature.
    pub fn has_sections(&self) -> bool {
        (1..=3).contains(&self.section_types.len())
            && (1..=3).contains(&self.section_features.len())
    }

    /// Whether structure types and their heading features are configured.
    pub fn has_structure(&self) -> bool {
        !self.structure_types.is_empty() && !self.structure_features.is_empty()
    }

    /// The configured `(structure type, heading feature)` pairs.
    pub fn structure_headings(&self) -> Vec<(String, String)> {
        self.structure_types
            .iter()
            .cloned()
            .zip(self.structure_features.iter().cloned())
            .collect()
    }
}

/// Splits a comma-separated `otext` setting into its trimmed, non-empty items.
fn csv_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lists_formats_and_constraints() {
        let metadata = BTreeMap::from([
            (
                "sectionTypes".to_string(),
                Some("book, chapter,,verse".to_string()),
            ),
            (
                "sectionFeatures".to_string(),
                Some("book,chapter,verse".to_string()),
            ),
            ("structureTypes".to_string(), Some(String::new())),
            (
                "fmt:text-orig-full".to_string(),
                Some("{word} ".to_string()),
            ),
            ("fmt:empty".to_string(), None),
            ("levelConstraints".to_string(), Some("a < b".to_string())),
        ]);
        let otext = Otext::from_metadata(&metadata);
        assert_eq!(otext.section_types, ["book", "chapter", "verse"]);
        assert!(otext.has_sections());
        assert!(otext.structure_types.is_empty());
        assert!(!otext.has_structure());
        assert_eq!(otext.formats["text-orig-full"], "{word} ");
        assert_eq!(otext.formats["empty"], "");
        assert_eq!(otext.level_constraints.as_deref(), Some("a < b"));
    }
}
