//! Line-level syntax of search templates: atoms, operators and relations.

use regex::Regex;

use crate::error::{CfError, Result};
use crate::search::constraint::{MappedMatcher, SimpleConstraint, parse_simple_constraint};
use crate::search::plan::{MappedFeatureRelationComparison, MappedRelationOperator, SimpleAtom};

pub(super) fn parse_simple_atom(
    line: &str,
    indent: usize,
    skip_operator: bool,
) -> Result<SimpleAtom> {
    let parts = split_query_whitespace(line);
    let mut parts = parts.into_iter();
    if skip_operator {
        // discard the leading relation-operator token
        let Some(operator) = parts.next() else {
            return Err(CfError::InvalidQuery("empty mapped query".to_string()));
        };
        if parts.len() == 0 {
            return Err(CfError::InvalidQuery(format!(
                "missing mapped atom after operator {operator:?}"
            )));
        }
    }
    let Some(first) = parts.next() else {
        return Err(CfError::InvalidQuery("empty mapped query".to_string()));
    };
    let node_type = first
        .split_once(':')
        .map(|(_, node_type)| node_type)
        .unwrap_or(&first)
        .to_string();
    let name = first.split_once(':').map(|(name, _)| name.to_string());
    let mut constraints = Vec::new();
    for part in parts {
        constraints.push(parse_simple_constraint(&part)?);
    }
    Ok(SimpleAtom {
        indent,
        name,
        node_type,
        constraints,
        quantifiers: Vec::new(),
        clean: None,
    })
}

pub(super) fn parse_mapped_atom_operator(raw: &str) -> Option<MappedRelationOperator> {
    parse_mapped_relation_operator(raw)
}

pub(super) fn parse_mapped_feature_continuation_line(
    line: &str,
) -> Result<Option<Vec<SimpleConstraint>>> {
    let mut constraints = Vec::new();
    for part in split_query_whitespace(line) {
        if !looks_like_explicit_mapped_feature_constraint(&part) {
            return Ok(None);
        }
        let constraint = parse_simple_constraint(&part)?;
        constraints.push(constraint);
    }
    if constraints.is_empty() {
        Ok(None)
    } else {
        Ok(Some(constraints))
    }
}

pub(super) fn looks_like_explicit_mapped_feature_constraint(part: &str) -> bool {
    part.ends_with('*')
        || part.ends_with('#')
        || part.contains('=')
        || part.contains('~')
        || part.contains('<')
        || part.contains('>')
}

pub(super) fn split_query_whitespace(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
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
        if ch.is_whitespace() {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if escaped {
        current.push('\\');
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

pub(super) fn parse_mapped_relation_operator(operator: &str) -> Option<MappedRelationOperator> {
    match operator {
        "=" => Some(MappedRelationOperator::Equal),
        "<" => Some(MappedRelationOperator::Before),
        ">" => Some(MappedRelationOperator::After),
        "#" => Some(MappedRelationOperator::NotEqual),
        "<:" => Some(MappedRelationOperator::AdjacentBefore),
        "==" => Some(MappedRelationOperator::SameSlots),
        "##" => Some(MappedRelationOperator::DifferentSlots),
        "&&" => Some(MappedRelationOperator::Overlaps),
        "||" => Some(MappedRelationOperator::Disjoint),
        "<<" => Some(MappedRelationOperator::SlotBefore),
        ">>" => Some(MappedRelationOperator::SlotAfter),
        ":>" => Some(MappedRelationOperator::AdjacentAfter),
        "=:" => Some(MappedRelationOperator::SameFirstSlot),
        ":=" => Some(MappedRelationOperator::SameLastSlot),
        "::" => Some(MappedRelationOperator::SameBoundary),
        "[[" => Some(MappedRelationOperator::Embeds),
        "]]" => Some(MappedRelationOperator::EmbeddedIn),
        _ => None,
    }
    .or_else(|| parse_mapped_near_relation_operator(operator))
    .or_else(|| {
        operator
            .strip_prefix('-')
            .and_then(|rest| rest.strip_suffix('>'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_mapped_edge_forward_operator)
    })
    .or_else(|| {
        operator
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('-'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_mapped_edge_backward_operator)
    })
    .or_else(|| {
        operator
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_mapped_edge_either_operator)
    })
    .or_else(|| parse_mapped_feature_relation_operator(operator))
}

pub(super) fn parse_mapped_near_relation_operator(
    operator: &str,
) -> Option<MappedRelationOperator> {
    parse_surrounded_u32(operator, '=', ':')
        .map(MappedRelationOperator::NearFirstSlot)
        .or_else(|| {
            parse_surrounded_u32(operator, ':', '=').map(MappedRelationOperator::NearLastSlot)
        })
        .or_else(|| {
            parse_surrounded_u32(operator, ':', ':').map(MappedRelationOperator::NearBoundary)
        })
        .or_else(|| {
            parse_surrounded_u32(operator, '<', ':').map(MappedRelationOperator::NearBefore)
        })
        .or_else(|| parse_surrounded_u32(operator, ':', '>').map(MappedRelationOperator::NearAfter))
}

pub(super) fn parse_surrounded_u32(operator: &str, prefix: char, suffix: char) -> Option<u32> {
    operator
        .strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse::<u32>()
        .ok()
}

pub(super) fn parse_mapped_edge_forward_operator(raw: &str) -> MappedRelationOperator {
    if let Some((edge_name, matcher)) = parse_mapped_edge_value_matcher(raw) {
        return MappedRelationOperator::EdgeForwardValue(edge_name, matcher);
    }
    MappedRelationOperator::EdgeForward(raw.to_string())
}

pub(super) fn parse_mapped_edge_backward_operator(raw: &str) -> MappedRelationOperator {
    if let Some((edge_name, matcher)) = parse_mapped_edge_value_matcher(raw) {
        return MappedRelationOperator::EdgeBackwardValue(edge_name, matcher);
    }
    MappedRelationOperator::EdgeBackward(raw.to_string())
}

pub(super) fn parse_mapped_edge_either_operator(raw: &str) -> MappedRelationOperator {
    if let Some((edge_name, matcher)) = parse_mapped_edge_value_matcher(raw) {
        return MappedRelationOperator::EdgeEitherValue(edge_name, matcher);
    }
    MappedRelationOperator::EdgeEither(raw.to_string())
}

/// An edge relation with a value condition (`-f=v>`, `-f#v>`, `-f>n>`, `-f*>`,
/// ...), parsed like a node feature condition (TF reuses `parseFeatureVals`).
/// `None` for a plain edge relation (`-f>`).
pub(super) fn parse_mapped_edge_value_matcher(raw: &str) -> Option<(String, MappedMatcher)> {
    let constraint = parse_simple_constraint(raw).ok()?;
    match constraint.matcher {
        MappedMatcher::Exists => None,
        matcher => Some((constraint.feature, matcher)),
    }
}

pub(super) fn parse_mapped_feature_relation_operator(
    operator: &str,
) -> Option<MappedRelationOperator> {
    let inner = operator.strip_prefix('.')?.strip_suffix('.')?;
    if inner.is_empty() {
        return None;
    }
    if let Some(operator) = parse_mapped_feature_regex_relation_operator(inner) {
        return Some(operator);
    }
    for (raw_operator, comparison) in [
        ('=', MappedFeatureRelationComparison::Eq),
        ('#', MappedFeatureRelationComparison::Ne),
        ('<', MappedFeatureRelationComparison::Lt),
        ('>', MappedFeatureRelationComparison::Gt),
    ] {
        if let Some((left_feature, right_feature)) = inner.split_once(raw_operator) {
            if left_feature.is_empty() || right_feature.is_empty() {
                return None;
            }
            return Some(MappedRelationOperator::FeatureCompare {
                left_feature: left_feature.to_string(),
                comparison,
                right_feature: right_feature.to_string(),
            });
        }
    }
    Some(MappedRelationOperator::FeatureCompare {
        left_feature: inner.to_string(),
        comparison: MappedFeatureRelationComparison::Eq,
        right_feature: inner.to_string(),
    })
}

pub(super) fn parse_mapped_feature_regex_relation_operator(
    inner: &str,
) -> Option<MappedRelationOperator> {
    let (left_feature, rest) = inner.split_once('~')?;
    let (pattern, right_feature) = rest.rsplit_once('~')?;
    if left_feature.is_empty() || pattern.is_empty() || right_feature.is_empty() {
        return None;
    }
    Some(MappedRelationOperator::FeatureRegexCompare {
        left_feature: left_feature.to_string(),
        pattern: Regex::new(pattern).ok()?,
        right_feature: right_feature.to_string(),
    })
}
