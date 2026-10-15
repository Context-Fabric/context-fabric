//! The structure of a tokenized template (`tf/search/semantics.py::_grammar`):
//! atoms become query nodes, indentation and operators become edges, and
//! indentation, lonely relations and undefined names become errors.

use std::collections::{BTreeMap, HashMap};

use crate::search::check::feature_spec::FeatureMap;
use crate::search::check::tokens::{Operator, RawQuantifier, Token, TokenKind};

/// A query node: one atom of the template.
#[derive(Debug, Clone)]
pub(super) struct QueryNode {
    pub(super) otype: String,
    pub(super) features: FeatureMap,
    /// The atom's name (empty when unnamed).
    pub(super) name: String,
    /// The atom line without its indent.
    pub(super) src: String,
    pub(super) quantifiers: Vec<RawQuantifier>,
    /// The template line of the atom.
    pub(super) line: usize,
}

/// A query edge: a relation between two query nodes.
#[derive(Debug, Clone)]
pub(super) struct QueryEdge {
    pub(super) from: usize,
    pub(super) op: Operator,
    pub(super) to: usize,
    /// The template line that made the edge.
    pub(super) line: usize,
}

/// A template's query graph (TF `qnodes` / `qedgesRaw`).
#[derive(Debug, Clone, Default)]
pub(super) struct Query {
    pub(super) nodes: Vec<QueryNode>,
    pub(super) edges: Vec<QueryEdge>,
}

impl Query {
    fn edge(&mut self, from: usize, op: Operator, to: usize, line: usize) {
        self.edges.push(QueryEdge { from, op, to, line });
    }
}

fn structural(text: &str) -> Operator {
    Operator {
        text: text.to_string(),
        edge_features: FeatureMap::new(),
    }
}

/// Build the query graph of `tokens`, or return TF's `badSemantics` messages.
pub(super) fn analyze(tokens: Vec<Token>) -> Result<Query, Vec<(Option<usize>, String)>> {
    let token_count = tokens.len();
    let mut tokens = tokens;
    // Relations are resolved after every atom is known.
    tokens.sort_by_key(|token| match token.kind {
        TokenKind::Relation { .. } => token_count + token.line,
        _ => token.line,
    });

    let mut errors: Vec<(Option<usize>, String)> = Vec::new();
    let mut query = Query::default();
    let mut names: HashMap<String, usize> = HashMap::new();
    // The last query node seen at each indent of the current nesting (TF
    // `atomStack`).
    let mut atom_stack: BTreeMap<usize, usize> = BTreeMap::new();
    // TF keeps the last query node in a variable that outlives its atom line;
    // a lonely operator records it on the stack.
    let mut last_node: Option<usize> = None;

    for token in tokens {
        let i = token.line;
        match token.kind {
            TokenKind::Atom(atom) => {
                let q = query.nodes.len();
                query.nodes.push(QueryNode {
                    otype: atom.otype,
                    features: atom.features,
                    name: atom.name.clone(),
                    src: atom.src,
                    quantifiers: atom.quantifiers,
                    line: i,
                });
                let name = if atom.name.is_empty() {
                    format!(":{i}")
                } else {
                    atom.name
                };
                names.insert(name, q);
                last_node = Some(q);
                place(
                    i,
                    atom.indent,
                    atom.op,
                    Some(q),
                    last_node,
                    &mut atom_stack,
                    &mut query,
                    &mut errors,
                );
            }
            TokenKind::Operator { indent, op } => {
                place(
                    i,
                    indent,
                    Some(op),
                    None,
                    last_node,
                    &mut atom_stack,
                    &mut query,
                    &mut errors,
                );
            }
            TokenKind::Features(features) => {
                if let Some(node) = query.nodes.last_mut() {
                    node.features.extend(features);
                }
            }
            TokenKind::Relation { from, op, to } => {
                let ends = [(&from, names.get(&from)), (&to, names.get(&to))];
                let mut defined = true;
                for (name, node) in ends {
                    if node.is_none() {
                        errors.push((Some(i), format!("Relation with undefined name: \"{name}\"")));
                        defined = false;
                    }
                }
                if defined {
                    query.edge(names[&from], op, names[&to], i);
                }
            }
        }
    }

    // An atom whose type is the name of another atom refers to that atom.
    for q in 0..query.nodes.len() {
        if let Some(&refer) = names.get(&query.nodes[q].otype) {
            query.nodes[q].otype = query.nodes[refer].otype.clone();
            let line = query.nodes[q].line;
            query.edge(q, structural("="), refer, line);
        }
    }

    if errors.is_empty() {
        Ok(query)
    } else {
        Err(errors)
    }
}

/// Place an atom (`node` is its query node) or a lonely operator (`node` is
/// `None`) in the indentation structure, adding the edges it implies.
#[allow(clippy::too_many_arguments)] // mirrors TF's shared loop state
fn place(
    i: usize,
    indent: usize,
    op: Option<Operator>,
    node: Option<usize>,
    last_node: Option<usize>,
    atom_stack: &mut BTreeMap<usize, usize>,
    query: &mut Query,
    errors: &mut Vec<(Option<usize>, String)>,
) {
    let Some((&top_indent, &top)) = atom_stack.iter().next_back() else {
        if indent > 0 {
            errors.push((Some(i), format!("Unexpected indent: {indent}, expected 0")));
        }
        if op.is_some() {
            errors.push((
                Some(i),
                "Lonely relation: not allowed at outermost level".to_string(),
            ));
        }
        if let Some(q) = node {
            atom_stack.insert(0, q);
        }
        return;
    };
    let parent_of_top = atom_stack.range(..top_indent).next_back().map(|(_, &q)| q);
    if indent == top_indent {
        // A sibling of the previous atom.
        match (parent_of_top, node) {
            (Some(parent), Some(q)) => {
                query.edge(q, structural("]]"), parent, i);
                if let Some(op) = op {
                    query.edge(top, op, q, i);
                }
            }
            (Some(parent), None) => {
                // A lonely operator relates the previous atom to its parent.
                if let Some(op) = op {
                    query.edge(top, op, parent, i);
                }
            }
            (None, node) => {
                // At the outermost level a lonely operator relates the
                // previous atom to the last atom (itself), as in TF.
                if let (Some(op), Some(q)) = (op, node.or(last_node)) {
                    query.edge(top, op, q, i);
                }
            }
        }
    } else if indent > top_indent {
        match node {
            Some(q) => {
                // A child of the previous atom.
                query.edge(q, structural("]]"), top, i);
                if let Some(op) = op {
                    query.edge(top, op, q, i);
                }
            }
            None => errors.push((
                Some(i),
                "Lonely relation: not allowed as first child".to_string(),
            )),
        }
    } else if let Some(&sibling) = atom_stack.get(&indent) {
        // An outdent to an earlier level.
        let parent = atom_stack.range(..indent).next_back().map(|(_, &q)| q);
        match node {
            Some(q) => {
                if let Some(op) = op {
                    query.edge(sibling, op, q, i);
                }
                if let Some(parent) = parent {
                    query.edge(q, structural("]]"), parent, i);
                }
            }
            None => {
                if let (Some(parent), Some(op)) = (parent, op) {
                    query.edge(sibling, op, parent, i);
                }
            }
        }
        let deeper: Vec<usize> = atom_stack.range(indent + 1..).map(|(&k, _)| k).collect();
        for key in deeper {
            atom_stack.remove(&key);
        }
    } else {
        let expected: Vec<String> = atom_stack
            .range(..indent)
            .map(|(k, _)| k.to_string())
            .collect();
        errors.push((
            Some(i),
            format!(
                "Unexpected indent: {indent}, expected one of {}",
                expected.join(", ")
            ),
        ));
    }
    if let Some(q) = last_node {
        atom_stack.insert(indent, q);
    }
}
