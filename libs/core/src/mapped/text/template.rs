//! Text format templates (`otext` `fmt:` values) parsed into literals and
//! feature placeholders.

use std::collections::HashSet;

/// One piece of a format template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplatePart {
    /// Literal text, with the `\t` and `\n` escapes expanded.
    Literal(String),
    /// A `{feature1/feature2/...:default}` placeholder: the value of the first
    /// feature that has one, else the default (escapes expanded).
    Placeholder {
        features: Vec<String>,
        default: String,
    },
}

/// A parsed format template, e.g. `{qere_utf8/g_word_utf8}{trailer_utf8}`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormatTemplate {
    pub parts: Vec<TemplatePart>,
}

impl FormatTemplate {
    /// Parses a template (without its `type#` prefix). An unterminated `{` and
    /// everything after it is literal text.
    pub fn parse(template: &str) -> Self {
        let mut parts = Vec::new();
        let mut rest = template;
        while let Some(start) = rest.find('{') {
            if start > 0 {
                parts.push(TemplatePart::Literal(expand_escapes(&rest[..start])));
            }
            let after_start = &rest[start + 1..];
            let Some(end) = after_start.find('}') else {
                parts.push(TemplatePart::Literal(expand_escapes(&rest[start..])));
                return Self { parts };
            };
            let placeholder = &after_start[..end];
            let (features, default) = placeholder.split_once(':').unwrap_or((placeholder, ""));
            parts.push(TemplatePart::Placeholder {
                features: features.split('/').map(str::to_string).collect(),
                default: expand_escapes(default),
            });
            rest = &after_start[end + 1..];
        }
        if !rest.is_empty() {
            parts.push(TemplatePart::Literal(expand_escapes(rest)));
        }
        Self { parts }
    }

    /// The features the placeholders refer to, in order of first appearance.
    pub fn features(&self) -> Vec<&str> {
        let mut seen = HashSet::new();
        self.parts
            .iter()
            .filter_map(|part| match part {
                TemplatePart::Placeholder { features, .. } => Some(features),
                TemplatePart::Literal(_) => None,
            })
            .flatten()
            .map(String::as_str)
            .filter(|feature| !feature.is_empty() && seen.insert(*feature))
            .collect()
    }
}

/// Splits `type#template` into the node type the format renders and the
/// template proper. Without a known node type before `#`, the format renders
/// slots and the whole string is the template (Text-Fabric `splitFormat`).
pub fn split_format<'t>(
    node_types: &HashSet<String>,
    slot_type: &str,
    format: &'t str,
) -> (String, &'t str) {
    match format.split_once('#') {
        Some((node_type, template)) if node_types.contains(node_type) => {
            (node_type.to_string(), template)
        }
        _ => (slot_type.to_string(), format),
    }
}

fn expand_escapes(raw: &str) -> String {
    let mut expanded = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            expanded.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => expanded.push('\t'),
            Some('n') => expanded.push('\n'),
            Some(other) => {
                expanded.push('\\');
                expanded.push(other);
            }
            None => expanded.push('\\'),
        }
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholder(features: &[&str], default: &str) -> TemplatePart {
        TemplatePart::Placeholder {
            features: features.iter().map(|feature| feature.to_string()).collect(),
            default: default.to_string(),
        }
    }

    #[test]
    fn parses_literals_placeholders_and_defaults() {
        let template = FormatTemplate::parse("{qere/g_word}\\t{trailer:-}{x}end{open");
        assert_eq!(
            template.parts,
            vec![
                placeholder(&["qere", "g_word"], ""),
                TemplatePart::Literal("\t".to_string()),
                placeholder(&["trailer"], "-"),
                placeholder(&["x"], ""),
                TemplatePart::Literal("end".to_string()),
                TemplatePart::Literal("{open".to_string()),
            ]
        );
        assert_eq!(template.features(), ["qere", "g_word", "trailer", "x"]);
    }

    #[test]
    fn empty_alternatives_are_kept_for_rendering_but_not_listed() {
        let template = FormatTemplate::parse("{a//b}");
        assert_eq!(template.parts, vec![placeholder(&["a", "", "b"], "")]);
        assert_eq!(template.features(), ["a", "b"]);
    }

    #[test]
    fn splits_a_known_node_type_prefix() {
        let types = HashSet::from(["phrase".to_string(), "word".to_string()]);
        assert_eq!(
            split_format(&types, "word", "phrase#{x}"),
            ("phrase".to_string(), "{x}")
        );
        assert_eq!(
            split_format(&types, "word", "other#{x}"),
            ("word".to_string(), "other#{x}")
        );
        assert_eq!(
            split_format(&types, "word", "{x}"),
            ("word".to_string(), "{x}")
        );
    }
}
