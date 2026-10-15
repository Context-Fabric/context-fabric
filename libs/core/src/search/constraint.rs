//! Feature constraints of atoms (`feature=value`, `feature~regex`, ...) and
//! their evaluation.

use regex::Regex;

use crate::error::{CfError, Result};
use crate::mapped::MappedNodeValue;

#[derive(Clone)]
pub(super) struct SimpleConstraint {
    pub(super) feature: String,
    pub(super) matcher: MappedMatcher,
}

impl SimpleConstraint {
    pub(super) fn cache_key(&self) -> String {
        format!("{}{}", self.feature, self.matcher.cache_key())
    }

    /// Re-serialize as a template token (inverse of `parse_simple_constraint`), used
    /// to rebuild a host atom's clean line for quantifier sub-searches.
    pub(super) fn to_token(&self) -> String {
        let feature = &self.feature;
        match &self.matcher {
            MappedMatcher::Eq(values) => format!("{feature}={}", values.join("|")),
            MappedMatcher::Ne(values) => format!("{feature}#{}", values.join("|")),
            MappedMatcher::Regex(regex) => format!("{feature}~{}", regex.as_str()),
            MappedMatcher::Any => format!("{feature}*"),
            MappedMatcher::Exists => feature.clone(),
            MappedMatcher::Missing => format!("{feature}#"),
            MappedMatcher::Lt(value) => format!("{feature}<{value}"),
            MappedMatcher::Gt(value) => format!("{feature}>{value}"),
        }
    }

    pub(super) fn can_seed_candidates(&self) -> bool {
        !matches!(
            &self.matcher,
            MappedMatcher::Ne(_) | MappedMatcher::Missing | MappedMatcher::Any
        )
    }

    pub(super) fn matches(&self, value: Option<MappedNodeValue<'_>>) -> bool {
        self.matcher.matches(value)
    }
}

/// A feature condition (TF `tf/search/syntax.py::parseFeatureVals`), for a node
/// feature in an atom or for the value of an edge in an edge relation.
#[derive(Debug, Clone)]
pub(super) enum MappedMatcher {
    Eq(Vec<String>),
    /// None of the values; a missing value qualifies.
    Ne(Vec<String>),
    Regex(Regex),
    /// `f*`: no condition at all.
    Any,
    /// `f`: any value.
    Exists,
    /// `f#`: no value.
    Missing,
    Lt(String),
    Gt(String),
}

impl MappedMatcher {
    /// Whether a feature value (`None` when the node or edge has none) meets
    /// the condition.
    pub(super) fn matches(&self, value: Option<MappedNodeValue<'_>>) -> bool {
        match self {
            MappedMatcher::Eq(expected) => value.is_some_and(|value| {
                expected
                    .iter()
                    .any(|expected| value_matches(value, expected))
            }),
            MappedMatcher::Ne(expected) => value.is_none_or(|value| {
                !expected
                    .iter()
                    .any(|expected| value_matches(value, expected))
            }),
            MappedMatcher::Regex(regex) => value
                .and_then(mapped_value_as_str)
                .is_some_and(|raw| regex.is_match(raw)),
            MappedMatcher::Any => true,
            MappedMatcher::Exists => value.is_some(),
            MappedMatcher::Missing => value.is_none(),
            MappedMatcher::Lt(expected) => value
                .and_then(|value| compare_value(value, expected))
                .is_some_and(|ordering| ordering.is_lt()),
            MappedMatcher::Gt(expected) => value
                .and_then(|value| compare_value(value, expected))
                .is_some_and(|ordering| ordering.is_gt()),
        }
    }

    pub(super) fn cache_key(&self) -> String {
        match self {
            Self::Eq(values) => format!("={}", values.join("|")),
            Self::Ne(values) => format!("#{}", values.join("|")),
            Self::Regex(regex) => format!("~{}", regex.as_str()),
            Self::Any => "*".to_string(),
            Self::Exists => String::new(),
            Self::Missing => "#".to_string(),
            Self::Lt(value) => format!("<{value}"),
            Self::Gt(value) => format!(">{value}"),
        }
    }
}

pub(super) fn parse_simple_constraint(part: &str) -> Result<SimpleConstraint> {
    // Mirror TF (tf/search/syntax.py:parseFeatureVals): the feature name is the
    // leading run of name characters and the FIRST unescaped operator that follows
    // determines the comparison. A naive `split_once('=')` would mis-handle forms
    // such as `vbe#H=` (operator `#`, value `H=`) by splitting on `=` first.
    match find_first_constraint_operator(part) {
        Some((index, operator)) => {
            let feature = part[..index].to_string();
            let value = &part[index + operator.len_utf8()..];
            let matcher = match operator {
                '=' => MappedMatcher::Eq(parse_alternative_strings(value)?),
                '#' => {
                    if value.is_empty() {
                        MappedMatcher::Missing
                    } else {
                        MappedMatcher::Ne(parse_alternative_strings(value)?)
                    }
                }
                '~' => MappedMatcher::Regex(Regex::new(value).map_err(|source| {
                    CfError::template(format!("invalid mapped regex {value:?}: {source}"))
                })?),
                '<' => MappedMatcher::Lt(value.to_string()),
                '>' => MappedMatcher::Gt(value.to_string()),
                _ => unreachable!("unexpected constraint operator {operator:?}"),
            };
            Ok(SimpleConstraint { feature, matcher })
        }
        None => {
            // No operator: `feature*` (no condition) or a bare feature (exists).
            if let Some(feature) = part.strip_suffix('*') {
                if feature.is_empty() {
                    return Err(CfError::template(format!(
                        "unsupported mapped constraint syntax {part:?}"
                    )));
                }
                return Ok(SimpleConstraint {
                    feature: feature.to_string(),
                    matcher: MappedMatcher::Any,
                });
            }
            if part.is_empty() {
                return Err(CfError::template(format!(
                    "unsupported mapped constraint syntax {part:?}"
                )));
            }
            Ok(SimpleConstraint {
                feature: part.to_string(),
                matcher: MappedMatcher::Exists,
            })
        }
    }
}

/// Scan left-to-right for the first unescaped constraint operator (`= # < > ~`),
/// skipping escaped characters so that escaped literals in values are preserved.
pub(super) fn find_first_constraint_operator(part: &str) -> Option<(usize, char)> {
    let mut escaped = false;
    for (index, ch) in part.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '=' | '#' | '<' | '>' | '~') {
            return Some((index, ch));
        }
    }
    None
}

pub(super) fn parse_alternative_strings(value: &str) -> Result<Vec<String>> {
    let values: Vec<_> = split_unescaped(value, '|')
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| unescape_query_value(&candidate))
        .collect();
    if values.is_empty() {
        return Err(CfError::template(
            "mapped equality constraint must include at least one value".to_string(),
        ));
    }
    Ok(values)
}

pub(super) fn split_unescaped(raw: &str, delimiter: char) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in raw.chars() {
        if escaped {
            current.push('\\');
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == delimiter {
            values.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    if escaped {
        current.push('\\');
    }
    values.push(current);
    values
}

pub(super) fn value_matches(actual: MappedNodeValue<'_>, expected: &str) -> bool {
    match actual {
        MappedNodeValue::Str(actual) => actual == expected,
        MappedNodeValue::Int(actual) => expected.parse::<i64>() == Ok(actual),
    }
}

pub(super) fn mapped_value_as_str(value: MappedNodeValue<'_>) -> Option<&str> {
    match value {
        MappedNodeValue::Str(value) => Some(value),
        MappedNodeValue::Int(_) => None,
    }
}

pub(super) fn compare_value(
    actual: MappedNodeValue<'_>,
    expected: &str,
) -> Option<std::cmp::Ordering> {
    match actual {
        MappedNodeValue::Str(actual) => Some(actual.cmp(expected)),
        MappedNodeValue::Int(actual) => expected
            .parse::<i64>()
            .ok()
            .map(|expected| actual.cmp(&expected)),
    }
}

pub(super) fn mapped_values_equal(left: MappedNodeValue<'_>, right: MappedNodeValue<'_>) -> bool {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => left == right,
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => left == right,
        (MappedNodeValue::Str(left), MappedNodeValue::Int(right)) => {
            left.parse::<i64>().ok() == Some(right)
        }
        (MappedNodeValue::Int(left), MappedNodeValue::Str(right)) => {
            right.parse::<i64>().ok() == Some(left)
        }
    }
}

pub(super) fn mapped_value_compare(
    left: MappedNodeValue<'_>,
    right: MappedNodeValue<'_>,
) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => Some(left.cmp(right)),
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => Some(left.cmp(&right)),
        (MappedNodeValue::Str(left), MappedNodeValue::Int(right)) => {
            left.parse::<i64>().ok().map(|left| left.cmp(&right))
        }
        (MappedNodeValue::Int(left), MappedNodeValue::Str(right)) => {
            right.parse::<i64>().ok().map(|right| left.cmp(&right))
        }
    }
}

pub(super) fn unescape_query_value(raw: &str) -> String {
    let mut value = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        match chars.next() {
            Some('t') => value.push('\t'),
            Some('n') => value.push('\n'),
            Some(' ') => value.push(' '),
            Some('\\') => value.push('\\'),
            Some('|') => value.push('|'),
            Some('=') => value.push('='),
            Some('#') => value.push('#'),
            Some('~') => value.push('~'),
            Some('<') => value.push('<'),
            Some('>') => value.push('>'),
            Some(other) => {
                value.push('\\');
                value.push(other);
            }
            None => value.push('\\'),
        }
    }
    value
}
