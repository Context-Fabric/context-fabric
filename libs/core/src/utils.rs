use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedFormat {
    pub template: String,
    pub rendered_template: String,
    pub features: Vec<(Vec<String>, String)>,
}

pub fn collect_formats(
    config: &BTreeMap<String, String>,
) -> (BTreeMap<String, CollectedFormat>, Vec<String>) {
    let mut feature_set = BTreeSet::new();
    let mut formats = BTreeMap::new();

    for (name, template) in config {
        let Some(format_name) = name.strip_prefix("fmt:") else {
            continue;
        };
        let mut rendered_template = String::new();
        let mut features = Vec::new();
        let mut rest = template.as_str();

        while let Some(start) = rest.find('{') {
            rendered_template.push_str(&rest[..start]);
            let after_start = &rest[start + 1..];
            let Some(end) = after_start.find('}') else {
                rendered_template.push_str(&rest[start..]);
                rest = "";
                break;
            };
            let placeholder = &after_start[..end];
            let (feature_part, default) = placeholder
                .split_once(':')
                .map(|(feature_part, default)| (feature_part, default))
                .unwrap_or((placeholder, ""));
            let alternatives = feature_part
                .split('/')
                .filter(|feature| !feature.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            for feature in &alternatives {
                feature_set.insert(feature.clone());
            }
            features.push((alternatives, default.to_string()));
            rendered_template.push_str("{}");
            rest = &after_start[end + 1..];
        }
        rendered_template.push_str(rest);

        formats.insert(
            format_name.to_string(),
            CollectedFormat {
                template: template.clone(),
                rendered_template,
                features,
            },
        );
    }

    (formats, feature_set.into_iter().collect())
}
