//! Quantifiers (`/where/`, `/have/`, `/without/`, `/with/`, `/or/`): their
//! template blocks and their evaluation as sub-searches.

use std::collections::HashSet;

use crate::error::{CfError, Result};
use crate::search::plan::{SimpleAtom, parse_mapped_plan};
use crate::search::syntax::is_quantifier_init;
use crate::search::template::split_query_whitespace;
use crate::search::{MappedSearch, SearchSets};

impl<'a> MappedSearch<'a> {
    pub(super) fn apply_quantifier_block(
        &self,
        clean_atom: &str,
        block: &TextQuantifierBlock,
        yarn: HashSet<u32>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<HashSet<u32>> {
        match block.kind {
            MappedQuantifierKind::Without => {
                let Some(alternative) = block.alternatives.first() else {
                    return Ok(yarn);
                };
                let template = combine_quantifier_subtemplate(clean_atom, alternative);
                let exclude = self.quantifier_subsearch_roots(&template, sets)?;
                Ok(yarn.difference(&exclude).copied().collect())
            }
            MappedQuantifierKind::With => {
                let mut result = HashSet::new();
                for alternative in &block.alternatives {
                    let template = combine_quantifier_subtemplate(clean_atom, alternative);
                    let roots = self.quantifier_subsearch_roots(&template, sets)?;
                    for node in roots {
                        if yarn.contains(&node) {
                            result.insert(node);
                        }
                    }
                }
                Ok(result)
            }
            MappedQuantifierKind::Where => {
                let Some(antecedent) = block.alternatives.first() else {
                    return Ok(yarn);
                };
                let antecedent_template = combine_quantifier_subtemplate(clean_atom, antecedent);
                let antecedent_rows = self.run_template(&antecedent_template, sets)?;
                if antecedent_rows.is_empty() {
                    return Ok(yarn);
                }
                let size_a = antecedent_rows[0].len();
                let mut combined = antecedent.clone();
                for consequent in &block.consequents {
                    combined = combine_quantifier_templates(&combined, consequent);
                }
                let combined_template = combine_quantifier_subtemplate(clean_atom, &combined);
                let combined_rows = self.run_template(&combined_template, sets)?;
                let satisfied: HashSet<Vec<u32>> = combined_rows
                    .into_iter()
                    .map(|row| row.into_iter().take(size_a).collect())
                    .collect();
                // Roots whose antecedent match lacks a matching consequent are
                // disqualified (the quantifier is universal, not existential).
                let mut disqualified = HashSet::new();
                for row in antecedent_rows {
                    let prefix: Vec<u32> = row.iter().take(size_a).copied().collect();
                    if !satisfied.contains(&prefix)
                        && let Some(root) = row.first()
                    {
                        disqualified.insert(*root);
                    }
                }
                Ok(yarn.difference(&disqualified).copied().collect())
            }
        }
    }

    /// Every match of a quantifier sub-template (checked with the outer
    /// template).
    pub(super) fn run_template(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<Vec<u32>>> {
        self.join_rows(parse_mapped_plan(template)?, sets)
    }

    pub(super) fn quantifier_subsearch_roots(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<HashSet<u32>> {
        Ok(self
            .run_template(template, sets)?
            .into_iter()
            .filter_map(|row| row.first().copied())
            .collect())
    }
}

#[derive(Clone)]
pub(super) struct TextQuantifierBlock {
    pub(super) kind: MappedQuantifierKind,
    /// For `/with/`/`/without/`: the alternative templates. For `/where/`: the
    /// single antecedent template (index 0).
    pub(super) alternatives: Vec<String>,
    /// For `/where/`: the `/have/` consequent template(s).
    pub(super) consequents: Vec<String>,
}

#[derive(Clone, Copy)]
pub(super) enum MappedQuantifierKind {
    With,
    Without,
    Where,
}

pub(super) const PARENT_NAME: &str = "__cf_parent__";

/// A clean, named atom line for `atom` (TF cleanParent), plus the name that `..`
/// parent references inside its quantifier blocks resolve to.
pub(super) fn clean_host_atom(atom: &SimpleAtom) -> (String, String) {
    let parent_name = atom.name.clone().unwrap_or_else(|| PARENT_NAME.to_string());
    let mut line = format!("{parent_name}:{}", atom.node_type);
    for constraint in &atom.constraints {
        line.push(' ');
        line.push_str(&constraint.to_token());
    }
    (line, parent_name)
}

/// Left operand of an operator-prefixed line: the previous sibling at this indent,
/// else the nearest enclosing parent (TF semantics.py).
pub(super) fn sibling_or_parent(
    indent: usize,
    atom_stack: &std::collections::BTreeMap<usize, usize>,
) -> Result<usize> {
    atom_stack
        .get(&indent)
        .copied()
        .or_else(|| atom_stack.range(..indent).next_back().map(|(_, idx)| *idx))
        .ok_or_else(|| {
            CfError::template("Lonely relation: not allowed at outermost level".to_string())
        })
}

/// Read a quantifier block starting at `lines[start]` (the keyword) until its
/// matching `/-/`, stripping `kw_indent` leading spaces from body lines (TF strips
/// by the outermost quantifier keyword's indent). Returns the parsed block and the
/// index just past the terminator.
pub(super) fn collect_quantifier_block(
    lines: &[&str],
    start: usize,
    kw_indent: usize,
) -> Result<(ParsedQuantifierBlock, usize)> {
    let kind = match lines[start].trim() {
        "/where/" => MappedQuantifierKind::Where,
        "/without/" => MappedQuantifierKind::Without,
        _ => MappedQuantifierKind::With,
    };
    let mut alternatives: Vec<String> = Vec::new();
    let mut consequents: Vec<String> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut collecting_consequents = false;
    let mut depth = 0usize;
    let mut i = start + 1;
    while i < lines.len() {
        let token = lines[i].trim();
        if depth == 0 {
            if token == "/-/" {
                push_block_alternative(
                    if collecting_consequents {
                        &mut consequents
                    } else {
                        &mut alternatives
                    },
                    &cur,
                );
                if matches!(kind, MappedQuantifierKind::Where)
                    && (alternatives.is_empty() || consequents.is_empty())
                {
                    return Err(CfError::template(
                        "/where/ requires antecedent and /have/ consequent templates".to_string(),
                    ));
                }
                return Ok((
                    ParsedQuantifierBlock {
                        kind,
                        alternatives,
                        consequents,
                    },
                    i + 1,
                ));
            }
            if token == "/have/" {
                push_block_alternative(
                    if collecting_consequents {
                        &mut consequents
                    } else {
                        &mut alternatives
                    },
                    &cur,
                );
                cur.clear();
                collecting_consequents = true;
                i += 1;
                continue;
            }
            if token == "/or/" {
                push_block_alternative(
                    if collecting_consequents {
                        &mut consequents
                    } else {
                        &mut alternatives
                    },
                    &cur,
                );
                cur.clear();
                i += 1;
                continue;
            }
            if is_quantifier_init(token) {
                depth += 1;
            }
            cur.push(strip_indent(lines[i], kw_indent));
            i += 1;
            continue;
        }
        // Inside a nested quantifier: pass lines through verbatim (stripped by the
        // outer keyword indent) and track nesting.
        if is_quantifier_init(token) {
            depth += 1;
        } else if token == "/-/" {
            depth -= 1;
        }
        cur.push(strip_indent(lines[i], kw_indent));
        i += 1;
    }
    Err(CfError::template(
        "unterminated mapped quantified block".to_string(),
    ))
}

pub(super) fn push_block_alternative(target: &mut Vec<String>, lines: &[String]) {
    if lines.iter().any(|line| !line.trim().is_empty()) {
        target.push(lines.join("\n"));
    }
}

pub(super) fn strip_indent(line: &str, n: usize) -> String {
    let lead = line.chars().take_while(|ch| ch.is_whitespace()).count();
    line.chars().skip(lead.min(n)).collect()
}

/// Replace standalone `..` parent-reference tokens with `parent_name`, but only at
/// the current quantifier depth (tokens inside nested quantifier blocks are left
/// untouched so the recursive parse can resolve them against the inner parent).
pub(super) fn substitute_parent_ref(template: &str, parent_name: &str) -> String {
    let mut depth = 0usize;
    let mut out = Vec::new();
    for line in template.lines() {
        let token = line.trim();
        let is_init = matches!(token, "/where/" | "/with/" | "/without/");
        let is_term = token == "/-/";
        if depth > 0 {
            out.push(line.to_string());
            if is_init {
                depth += 1;
            } else if is_term {
                depth = depth.saturating_sub(1);
            }
            continue;
        }
        if is_init {
            depth += 1;
            out.push(line.to_string());
            continue;
        }
        let indent_len = line.chars().take_while(|ch| ch.is_whitespace()).count();
        let (indent, rest) = line.split_at(indent_len);
        let replaced = split_query_whitespace(rest)
            .into_iter()
            .map(|token| {
                if token == ".." {
                    parent_name.to_string()
                } else {
                    token
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        out.push(format!("{indent}{replaced}"));
    }
    out.join("\n")
}

/// Combine the clean host-atom line (at indent 0) with a quantifier sub-template,
/// preserving the sub-template's keyword-relative indentation (TF joins cleanAtom +
/// quTemplates without re-indenting).
pub(super) fn combine_quantifier_subtemplate(clean_atom: &str, sub: &str) -> String {
    let mut lines = vec![clean_atom.to_string()];
    for line in sub.lines() {
        if line.trim().is_empty() {
            continue;
        }
        lines.push(line.to_string());
    }
    lines.join("\n")
}

pub(super) struct ParsedQuantifierBlock {
    pub(super) kind: MappedQuantifierKind,
    pub(super) alternatives: Vec<String>,
    pub(super) consequents: Vec<String>,
}

pub(super) fn combine_quantifier_templates(first: &str, second: &str) -> String {
    match (first.trim().is_empty(), second.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => second.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first}\n{second}"),
    }
}
