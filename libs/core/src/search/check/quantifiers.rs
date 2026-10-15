//! Quantifier sub-templates checked like templates of their own, as
//! Text-Fabric runs them (`tf/search/spin.py::_doQuantifier`): the quantified
//! atom (`cleanParent`) followed by the block's lines with `..` resolved to
//! the atom's name (`deContext`).

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::search::check::grammar::{Query, QueryNode};
use crate::search::check::schema::TemplateSchema;
use crate::search::check::tokens::{ParsedLine, QuKind, RawQuantifier, gen_line, parse_line};
use crate::search::check::{TemplateMessage, check_at_offset};

/// The parent reference inside a quantifier block (TF `PARENT_REF`).
const PARENT_REF: &str = "..";

static NAMES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:[.#&|\[\]<>:=\-]+\S*)\s+)?([^ \t:=#<>~*]+):").expect("valid regex")
});

/// The errors of every quantifier sub-template of `query`, each group headed
/// by the quantifier it belongs to.
pub(super) fn check(
    query: &Query,
    schema: &TemplateSchema<'_>,
    offset: usize,
) -> Vec<TemplateMessage> {
    let mut messages = Vec::new();
    for node in &query.nodes {
        for quantifier in &node.quantifiers {
            for sub in sub_templates(node, quantifier, offset) {
                let errors = check_at_offset(&sub.template, schema, sub.offset);
                if let Some(first) = errors.first() {
                    messages.push(TemplateMessage {
                        kind: first.kind,
                        line: Some(sub.offset),
                        text: format!("Error under {}:", sub.kind.keyword()),
                    });
                    messages.extend(errors);
                }
            }
        }
    }
    messages
}

/// A sub-template a quantifier runs, with the line of the outermost template
/// its first line counts from, and the keyword it runs under.
struct SubTemplate {
    template: String,
    offset: usize,
    kind: QuKind,
}

fn sub_templates(node: &QueryNode, quantifier: &RawQuantifier, offset: usize) -> Vec<SubTemplate> {
    let (templates, parent_name) = de_context(quantifier, &node.name);
    let clean_atom = clean_parent(&node.src, &parent_name);
    let joined = |parts: &[&String]| {
        let mut lines = vec![clean_atom.as_str()];
        lines.extend(parts.iter().map(|part| part.as_str()));
        lines.join("\n")
    };
    let mut offset = offset + quantifier.line;
    match quantifier.kind {
        QuKind::Without => vec![SubTemplate {
            template: joined(&[&templates[0]]),
            offset,
            kind: QuKind::Without,
        }],
        QuKind::Where => {
            let antecedent = SubTemplate {
                template: joined(&[&templates[0]]),
                offset,
                kind: QuKind::Where,
            };
            offset += templates[0].split('\n').count();
            let all: Vec<&String> = templates.iter().collect();
            vec![
                antecedent,
                SubTemplate {
                    template: joined(&all),
                    offset,
                    kind: QuKind::Have,
                },
            ]
        }
        _ => templates
            .iter()
            .enumerate()
            .map(|(index, alternative)| {
                let sub = SubTemplate {
                    template: joined(&[alternative]),
                    offset,
                    kind: if index == 0 { QuKind::With } else { QuKind::Or },
                };
                offset += alternative.split('\n').count() + 1;
                sub
            })
            .collect(),
    }
}

/// The block's sub-templates with `..` replaced by the parent's name, and
/// that name: the atom's own, else `parent` (with `x`s appended until no
/// atom in the block uses it) (TF `deContext`).
fn de_context(quantifier: &RawQuantifier, name: &str) -> (Vec<String>, String) {
    let parent_name = if name.is_empty() {
        let used: HashSet<&str> = quantifier
            .templates
            .iter()
            .flatten()
            .filter_map(|line| NAMES_RE.captures(line))
            .filter_map(|captures| captures.get(1).map(|found| found.as_str()))
            .collect();
        let mut candidate = "parent".to_string();
        while used.contains(candidate.as_str()) {
            candidate.push('x');
        }
        candidate
    } else {
        name.to_string()
    };
    let templates = quantifier
        .templates
        .iter()
        .map(|lines| {
            lines
                .iter()
                .map(|line| resolve_parent(line, &parent_name))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();
    (templates, parent_name)
}

fn resolve_parent(line: &str, parent_name: &str) -> String {
    match parse_line(line) {
        ParsedLine::Relation {
            indent,
            from,
            op,
            to,
        } if from == PARENT_REF || to == PARENT_REF => {
            let swap = |end: String| {
                if end == PARENT_REF {
                    parent_name.to_string()
                } else {
                    end
                }
            };
            gen_line(&ParsedLine::Relation {
                indent,
                from: swap(from),
                op,
                to: swap(to),
            })
        }
        ParsedLine::Atom {
            indent,
            op,
            name,
            otype,
            features,
        } if name.is_empty() && otype == PARENT_REF => gen_line(&ParsedLine::Atom {
            indent,
            op,
            name,
            otype: parent_name.to_string(),
            features,
        }),
        _ => line.to_string(),
    }
}

/// The quantified atom as the first line of a sub-template: named, without
/// its relation operator (TF `cleanParent`).
fn clean_parent(src: &str, parent_name: &str) -> String {
    match parse_line(src) {
        ParsedLine::Atom {
            indent,
            name,
            otype,
            features,
            ..
        } => gen_line(&ParsedLine::Atom {
            indent,
            op: None,
            name: if name.is_empty() {
                parent_name.to_string()
            } else {
                name
            },
            otype,
            features,
        }),
        _ => src.to_string(),
    }
}
