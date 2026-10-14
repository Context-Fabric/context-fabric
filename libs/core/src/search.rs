use std::collections::{HashMap, HashSet};

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
pub struct RelationSyntax {
    pub indent: String,
    pub left: String,
    pub operator: String,
    pub right: String,
}

pub fn is_search_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
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

    pub fn count_first_nodes(&self, limit: Option<usize>) -> usize {
        self.fetch_first_nodes(limit).len()
    }

    pub fn count_prefixes(&self, width: usize, limit: Option<usize>) -> usize {
        self.fetch_prefixes(width, limit).len()
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
