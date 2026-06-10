use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

pub type SearchSets<'a> = HashMap<&'a str, Vec<u32>>;

pub const QWHERE: &str = "/where/";
pub const QHAVE: &str = "/have/";
pub const QWITHOUT: &str = "/without/";
pub const QWITH: &str = "/with/";
pub const QOR: &str = "/or/";
pub const QEND: &str = "/-/";
pub const QINIT: &[&str] = &[QWHERE, QWITHOUT, QWITH];
pub const QCONT: &[&str] = &[QHAVE, QOR];
pub const QTERM: &[&str] = &[QEND];
pub const PARENT_REF: &str = "..";
pub const ESCAPES: &[&str] = &["\\\\", "\\ ", "\\t", "\\n", "\\|", "\\="];
pub const VAL_ESCAPES: &[&str] = &["\\|", "\\="];

const OP_PAT: &str = r"(?:[.#&|\[\]<>:=-]+\S*)";
const NAME_PAT: &str = r"[A-Za-z0-9_.-]+";

#[allow(non_upper_case_globals)]
pub static atomOpRe: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(\s*)({OP_PAT})\s+([^ \t=#<>~*]+)(?:(?:\s*$)|(?:\s+(.*)))$"
    ))
    .expect("atomOpRe should compile")
});

#[allow(non_upper_case_globals)]
pub static atomRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*)([^ \t=#<>~*]+)(?:(?:\s*$)|(?:\s+(.*)))$").unwrap());

#[allow(non_upper_case_globals)]
pub static compRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9-@_]+)([<>])(.*)$").unwrap());

#[allow(non_upper_case_globals)]
pub static identRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9-@_]+)([=#])(.+)$").unwrap());

#[allow(non_upper_case_globals)]
pub static indentLineRe: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)(.*)").unwrap());

#[allow(non_upper_case_globals)]
pub static kRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([^0-9]*)([0-9]+)([^0-9]+)$").unwrap());

#[allow(non_upper_case_globals)]
pub static nameRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("^{NAME_PAT}$")).unwrap());

#[allow(non_upper_case_globals)]
pub static namesRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^\s*(?:{OP_PAT}\s+)?([^ \t:=#<>~*]+):")).unwrap());

#[allow(non_upper_case_globals)]
pub static numRe: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-?[0-9]+$").unwrap());

#[allow(non_upper_case_globals)]
pub static noneRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9-@_]+)(#?)\s*$").unwrap());

#[allow(non_upper_case_globals)]
pub static trueRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9-@_]+)[*]\s*$").unwrap());

#[allow(non_upper_case_globals)]
pub static opLineRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(\s*)({OP_PAT})\s*$")).unwrap());

#[allow(non_upper_case_globals)]
pub static opStripRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^\s*{OP_PAT}\s+(.*)$")).unwrap());

#[allow(non_upper_case_globals)]
pub static quLineRe: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(\s*)({}|{}|{}|{}|{}|{})\s*$",
        QWHERE, QHAVE, QWITHOUT, QWITH, QOR, QEND
    ))
    .unwrap()
});

#[allow(non_upper_case_globals)]
pub static relRe: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(\s*)({NAME_PAT})\s+({OP_PAT})\s+({NAME_PAT})\s*$"
    ))
    .unwrap()
});

#[allow(non_upper_case_globals)]
pub static reRe: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9-@_]+)~(.*)$").unwrap());

#[allow(non_upper_case_globals)]
pub static whiteRe: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(%|$)").unwrap());

pub const RELATIONS_LEGEND: &str = "\
Supported relations:
                      = same node
                      # different node
                      < canonical order before
                      > canonical order after
                     <: immediately before
                     :> immediately after
                     == same slot set
                     ## different slot set
                     && overlapping slot sets
                     || disjoint slot sets
                     << slot set before
                     >> slot set after
                     =: same first slot
                     := same last slot
                     :: same first and last slots
                    [[ embeds
                    ]] embedded in
                  =k: first slots within k slots
                  :k= last slots within k slots
                  :k: boundaries within k slots
                  <k: before within k slots
                  :k> after within k slots
       .feature=feature. feature equality
       .feature#feature. feature inequality
       .feature<feature. feature less-than
       .feature>feature. feature greater-than
       .feature~regex~feature. regex-normalized feature comparison
              -edge> forward edge
              <edge- backward edge
              <edge> bidirectional edge
        -edge=value> forward valued edge
        <edge=value- backward valued edge
        <edge=value> bidirectional valued edge
The warp feature \"oslots\" cannot be used as a normal edge search relation.";

pub fn relations_legend() -> &'static str {
    RELATIONS_LEGEND
}

pub fn is_quantifier_init(token: &str) -> bool {
    QINIT.contains(&token)
}

pub fn is_quantifier_continuation(token: &str) -> bool {
    QCONT.contains(&token)
}

pub fn is_quantifier_terminator(token: &str) -> bool {
    QTERM.contains(&token)
}

pub fn is_quantifier_line(line: &str) -> bool {
    let token = line.trim();
    is_quantifier_init(token)
        || is_quantifier_continuation(token)
        || is_quantifier_terminator(token)
}

pub fn is_search_white_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.is_empty() || trimmed.starts_with('%')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomSyntax {
    pub indent: String,
    pub node_type: String,
    pub features: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomOperatorSyntax {
    pub indent: String,
    pub operator: String,
    pub node_type: String,
    pub features: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureOperatorSyntax {
    pub feature: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeaturePresenceSyntax {
    pub feature: String,
    pub marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationSyntax {
    pub indent: String,
    pub left: String,
    pub operator: String,
    pub right: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KNearnessSyntax {
    pub before: String,
    pub k: u32,
    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantifierLineSyntax {
    pub indent: String,
    pub quantifier: String,
}

pub fn parse_atom_syntax(line: &str) -> Option<AtomSyntax> {
    let indent_len = line.chars().take_while(|ch| ch.is_whitespace()).count();
    let indent = line[..indent_len].to_string();
    let rest = line[indent_len..].trim_end();
    let mut parts = split_query_whitespace(rest).into_iter();
    let node_type = parts.next()?;
    if !is_atom_name_token(&node_type) {
        return None;
    }
    let features = parts.collect::<Vec<_>>().join(" ");
    Some(AtomSyntax {
        indent,
        node_type,
        features: (!features.is_empty()).then_some(features),
    })
}

pub fn parse_atom_operator_syntax(line: &str) -> Option<AtomOperatorSyntax> {
    let indent_len = line.chars().take_while(|ch| ch.is_whitespace()).count();
    let indent = line[..indent_len].to_string();
    let rest = line[indent_len..].trim_end();
    let mut parts = split_query_whitespace(rest).into_iter();
    let operator = parts.next()?;
    if !is_operator_token(&operator) {
        return None;
    }
    let node_type = parts.next()?;
    if !is_atom_name_token(&node_type) {
        return None;
    }
    let features = parts.collect::<Vec<_>>().join(" ");
    Some(AtomOperatorSyntax {
        indent,
        operator,
        node_type,
        features: (!features.is_empty()).then_some(features),
    })
}

pub fn parse_ident_syntax(value: &str) -> Option<FeatureOperatorSyntax> {
    parse_feature_operator_syntax(value, &['=', '#'])
}

pub fn parse_comparison_syntax(value: &str) -> Option<FeatureOperatorSyntax> {
    parse_feature_operator_syntax(value, &['<', '>'])
}

pub fn parse_regex_feature_syntax(value: &str) -> Option<(String, String)> {
    let (feature, pattern) = value.split_once('~')?;
    is_feature_name(feature).then(|| (feature.to_string(), pattern.to_string()))
}

pub fn parse_none_syntax(value: &str) -> Option<FeaturePresenceSyntax> {
    let trimmed = value.trim_end();
    let (feature, marker) = trimmed
        .strip_suffix('#')
        .map(|feature| (feature, Some("#".to_string())))
        .unwrap_or((trimmed, None));
    is_feature_name(feature).then(|| FeaturePresenceSyntax {
        feature: feature.to_string(),
        marker,
    })
}

pub fn parse_true_syntax(value: &str) -> Option<String> {
    let feature = value.trim_end().strip_suffix('*')?;
    is_feature_name(feature).then(|| feature.to_string())
}

pub fn is_search_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

pub fn is_search_number(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
}

pub fn parse_relation_syntax(line: &str) -> Option<RelationSyntax> {
    let indent_len = line.chars().take_while(|ch| ch.is_whitespace()).count();
    let indent = line[..indent_len].to_string();
    let parts = split_query_whitespace(line.trim());
    if parts.len() != 3
        || !is_search_name(&parts[0])
        || !is_operator_token(&parts[1])
        || !is_search_name(&parts[2])
    {
        return None;
    }
    Some(RelationSyntax {
        indent,
        left: parts[0].clone(),
        operator: parts[1].clone(),
        right: parts[2].clone(),
    })
}

pub fn parse_quantifier_line_syntax(line: &str) -> Option<QuantifierLineSyntax> {
    let indent_len = line.chars().take_while(|ch| ch.is_whitespace()).count();
    let indent = line[..indent_len].to_string();
    let quantifier = line[indent_len..].trim();
    is_quantifier_line(quantifier).then(|| QuantifierLineSyntax {
        indent,
        quantifier: quantifier.to_string(),
    })
}

pub fn parse_k_nearness_syntax(value: &str) -> Option<KNearnessSyntax> {
    let first_digit = value.find(|ch: char| ch.is_ascii_digit())?;
    let last_digit = value.rfind(|ch: char| ch.is_ascii_digit())?;
    let before = &value[..first_digit];
    let k_raw = &value[first_digit..=last_digit];
    let after = &value[last_digit + 1..];
    if before.is_empty() || after.is_empty() || !k_raw.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(KNearnessSyntax {
        before: before.to_string(),
        k: k_raw.parse().ok()?,
        after: after.to_string(),
    })
}

pub fn parse_named_atom_prefix(value: &str) -> Option<String> {
    let trimmed = value.trim_start();
    let before_colon = trimmed.split_once(':')?.0;
    (!before_colon.is_empty()
        && !before_colon
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, ':' | '=' | '#' | '<' | '>' | '~' | '*')))
    .then(|| before_colon.to_string())
}

pub fn parse_operator_line_syntax(line: &str) -> Option<(String, String)> {
    let indent_len = line.chars().take_while(|ch| ch.is_whitespace()).count();
    let indent = line[..indent_len].to_string();
    let operator = line[indent_len..].trim();
    (is_operator_token(operator) && !operator.contains(char::is_whitespace))
        .then(|| (indent, operator.to_string()))
}

pub fn strip_operator_syntax(line: &str) -> Option<String> {
    let mut parts = split_query_whitespace(line.trim()).into_iter();
    let operator = parts.next()?;
    if !is_operator_token(&operator) {
        return None;
    }
    let rest = parts.collect::<Vec<_>>().join(" ");
    (!rest.is_empty()).then_some(rest)
}

pub fn search_line_indent(line: &str) -> String {
    line.chars().take_while(|ch| ch.is_whitespace()).collect()
}

fn parse_feature_operator_syntax(value: &str, operators: &[char]) -> Option<FeatureOperatorSyntax> {
    let (index, operator) = value
        .char_indices()
        .find(|(_, ch)| operators.contains(ch))?;
    let feature = &value[..index];
    let value = &value[index + operator.len_utf8()..];
    if !is_feature_name(feature) || value.is_empty() {
        return None;
    }
    Some(FeatureOperatorSyntax {
        feature: feature.to_string(),
        operator: operator.to_string(),
        value: value.to_string(),
    })
}

fn is_feature_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '@' | '_'))
}

fn is_atom_name_token(value: &str) -> bool {
    !value.is_empty()
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || matches!(ch, '=' | '#' | '<' | '>' | '~' | '*'))
}

fn is_operator_token(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|ch| {
            matches!(
                ch,
                '.' | '#' | '&' | '|' | '[' | ']' | '<' | '>' | ':' | '=' | '-'
            ) || !ch.is_whitespace()
        })
        && value.chars().any(|ch| {
            matches!(
                ch,
                '.' | '#' | '&' | '|' | '[' | ']' | '<' | '>' | ':' | '=' | '-'
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchStudy {
    template: String,
    results: Vec<Vec<u32>>,
    plan: SearchPlanSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlanSummary {
    pub template: String,
    pub atom_count: usize,
    pub relation_count: usize,
    pub result_count: usize,
    pub has_quantifiers: bool,
}

impl SearchStudy {
    pub fn new(template: String, results: Vec<Vec<u32>>) -> Self {
        let plan = summarize_template(&template, results.len());
        Self {
            template,
            results,
            plan,
        }
    }

    pub fn template(&self) -> &str {
        &self.template
    }

    pub fn plan(&self) -> &SearchPlanSummary {
        &self.plan
    }

    pub fn show_plan(&self, details: bool) -> String {
        let mut lines = vec![
            format!("template: {}", self.plan.template.escape_default()),
            format!("atoms: {}", self.plan.atom_count),
            format!("relations: {}", self.plan.relation_count),
            format!("results: {}", self.plan.result_count),
            format!("quantifiers: {}", self.plan.has_quantifiers),
        ];
        if details {
            lines.push("rows:".to_string());
            for (index, row) in self.results.iter().enumerate() {
                lines.push(format!("  {}: {:?}", index + 1, row));
            }
        }
        lines.join("\n")
    }

    #[allow(non_snake_case)]
    pub fn showPlan(&self, details: bool) -> String {
        self.show_plan(details)
    }

    pub fn fetch(&self, limit: Option<usize>) -> Vec<Vec<u32>> {
        self.results
            .iter()
            .take(limit.unwrap_or(self.results.len()))
            .cloned()
            .collect()
    }

    pub fn fetch_first_nodes(&self, limit: Option<usize>) -> Vec<u32> {
        if limit == Some(0) {
            return Vec::new();
        }
        let mut seen = HashSet::new();
        let mut projected = Vec::new();
        let limit = limit.unwrap_or(usize::MAX);
        for row in &self.results {
            let Some(node) = row.first().copied() else {
                continue;
            };
            if seen.insert(node) {
                projected.push(node);
                if projected.len() >= limit {
                    break;
                }
            }
        }
        projected
    }

    pub fn fetch_prefixes(&self, width: usize, limit: Option<usize>) -> Vec<Vec<u32>> {
        if width == 0 || limit == Some(0) {
            return Vec::new();
        }
        let mut seen = HashSet::new();
        let mut projected = Vec::new();
        let limit = limit.unwrap_or(usize::MAX);
        for row in &self.results {
            let prefix: Vec<_> = row.iter().take(width).copied().collect();
            if prefix.is_empty() {
                continue;
            }
            if seen.insert(prefix.clone()) {
                projected.push(prefix);
                if projected.len() >= limit {
                    break;
                }
            }
        }
        projected
    }

    pub fn count(&self, limit: Option<usize>) -> usize {
        self.results.len().min(limit.unwrap_or(self.results.len()))
    }

    pub fn count_first_nodes(&self, limit: Option<usize>) -> usize {
        self.fetch_first_nodes(limit).len()
    }

    pub fn count_prefixes(&self, width: usize, limit: Option<usize>) -> usize {
        self.fetch_prefixes(width, limit).len()
    }

    pub fn total_count(&self) -> usize {
        self.results.len()
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

fn split_query_whitespace(line: &str) -> Vec<String> {
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

/// Summarize a search template into a lightweight plan description for
/// [`SearchStudy::show_plan`]. The counts mirror the canonical mapped planner's
/// line classification: each content line is either a relation (three tokens
/// whose middle token is a relation operator), a feature-continuation line that
/// extends the previous atom, or an atom. Quantifier delimiter lines do not
/// contribute atoms or relations themselves.
fn summarize_template(template: &str, result_count: usize) -> SearchPlanSummary {
    let has_quantifiers = template.lines().any(is_quantifier_line);
    let mut atom_count = 0;
    let mut relation_count = 0;
    let mut seen_atom = false;
    for raw_line in template.lines() {
        if is_search_white_line(raw_line) || is_quantifier_line(raw_line) {
            continue;
        }
        if parse_relation_syntax(raw_line).is_some() {
            relation_count += 1;
            continue;
        }
        if seen_atom && is_feature_continuation_line(raw_line) {
            continue;
        }
        atom_count += 1;
        seen_atom = true;
    }
    SearchPlanSummary {
        template: template.to_string(),
        atom_count,
        relation_count,
        result_count,
        has_quantifiers,
    }
}

fn is_feature_continuation_line(line: &str) -> bool {
    let tokens = split_query_whitespace(line.trim());
    !tokens.is_empty()
        && tokens.iter().all(|token| {
            token.ends_with('*')
                || token.ends_with('#')
                || token.contains('=')
                || token.contains('~')
                || token.contains('<')
                || token.contains('>')
        })
}

