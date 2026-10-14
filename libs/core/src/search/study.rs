//! [`SearchStudy`]: the results of a studied search, fetched and counted.

use std::collections::HashSet;

use crate::search::syntax::{is_quantifier_line, is_search_white_line, parse_relation_syntax};
use crate::search::template::split_query_whitespace;

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
