//! The search plan: atoms with their constraints and the relations between
//! them, built from a template.

use std::collections::HashMap;

use regex::Regex;

use crate::error::{CfError, Result};
use crate::search::SearchSets;
use crate::search::constraint::{MappedMatcher, SimpleConstraint, parse_simple_constraint};
use crate::search::quantifier::{
    TextQuantifierBlock, clean_host_atom, collect_quantifier_block, sibling_or_parent,
    substitute_parent_ref,
};
use crate::search::syntax::is_quantifier_init;
use crate::search::template::split_query_whitespace;
use crate::search::template::{
    parse_mapped_atom_operator, parse_mapped_feature_continuation_line,
    parse_mapped_relation_operator, parse_simple_atom,
};

#[derive(Clone)]
pub(super) struct MappedRelationPlan {
    pub(super) atoms: Vec<SimpleAtom>,
    pub(super) relations: Vec<MappedRelation>,
}

#[derive(Clone)]
pub(super) struct MappedRelation {
    pub(super) left: MappedRelationEndpoint,
    pub(super) operator: MappedRelationOperator,
    pub(super) right: MappedRelationEndpoint,
}

#[derive(Clone)]
pub(super) enum MappedRelationEndpoint {
    Atom(usize),
    Parent,
}

pub(super) struct PendingMappedRelation {
    pub(super) left: String,
    pub(super) operator: MappedRelationOperator,
    pub(super) right: String,
}

#[derive(Clone)]
pub(super) enum MappedRelationOperator {
    Equal,
    Before,
    After,
    NotEqual,
    AdjacentBefore,
    SameSlots,
    DifferentSlots,
    Overlaps,
    Disjoint,
    SlotBefore,
    SlotAfter,
    AdjacentAfter,
    SameFirstSlot,
    SameLastSlot,
    SameBoundary,
    NearFirstSlot(u32),
    NearLastSlot(u32),
    NearBoundary(u32),
    NearBefore(u32),
    NearAfter(u32),
    Embeds,
    EmbeddedIn,
    EdgeForward(String),
    EdgeBackward(String),
    EdgeEither(String),
    EdgeForwardValue(String, MappedMatcher),
    EdgeBackwardValue(String, MappedMatcher),
    EdgeEitherValue(String, MappedMatcher),
    FeatureCompare {
        left_feature: String,
        comparison: MappedFeatureRelationComparison,
        right_feature: String,
    },
    FeatureRegexCompare {
        left_feature: String,
        pattern: Regex,
        right_feature: String,
    },
}

#[derive(Clone, Copy)]
pub(super) enum MappedFeatureRelationComparison {
    Eq,
    Ne,
    Lt,
    Gt,
}

#[derive(Clone)]
pub(super) struct SimpleAtom {
    pub(super) indent: usize,
    pub(super) name: Option<String>,
    pub(super) node_type: String,
    pub(super) constraints: Vec<SimpleConstraint>,
    /// Quantifier blocks attached to this atom (TF: an atom's `quantifiers` list).
    /// Each reduces the atom's candidate yarn via global set algebra before the
    /// join (TF spin.py:_doQuantifier).
    pub(super) quantifiers: Vec<TextQuantifierBlock>,
    /// `(clean_atom_line, parent_name)` for this atom, used to build quantifier
    /// sub-search templates. Set only when `quantifiers` is non-empty.
    pub(super) clean: Option<(String, String)>,
}

impl SimpleAtom {
    pub(super) fn is_generic_node_type(&self) -> bool {
        self.node_type == "."
    }

    pub(super) fn is_parent_reference(&self) -> bool {
        self.node_type == ".."
    }

    pub(super) fn candidate_cache_key(&self) -> String {
        let mut constraints = self
            .constraints
            .iter()
            .map(SimpleConstraint::cache_key)
            .collect::<Vec<_>>();
        constraints.sort_unstable();
        format!("{} {}", self.node_type, constraints.join(" "))
    }
}

pub(super) fn custom_set_nodes(
    atom: &SimpleAtom,
    sets: Option<&SearchSets<'_>>,
) -> Option<Vec<u32>> {
    let mut nodes = sets?.get(atom.node_type.as_str())?.clone();
    nodes.sort_unstable();
    nodes.dedup();
    Some(nodes)
}

pub(super) fn parse_mapped_plan(template: &str) -> Result<MappedRelationPlan> {
    let mut atoms: Vec<SimpleAtom> = Vec::new();
    let mut names: HashMap<String, usize> = HashMap::new();
    let mut pending_relations = Vec::new();
    // Op edges discovered structurally (op-prefixed atoms, lonely operators) are
    // emitted directly with resolved atom indices.
    let mut direct_relations: Vec<MappedRelation> = Vec::new();
    // TF `atomStack`: indent -> last atom index seen at that indent in the current
    // nesting. Used to determine the operand of operator edges and to detect
    // illegal "lonely" relations.
    let mut atom_stack: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();

    // Index-based iteration so quantifier blocks can be consumed wholesale and
    // attached to the most recently seen atom (TF: an atom's `quantifiers` list).
    let all_lines: Vec<&str> = template.lines().collect();
    let mut li = 0usize;
    while li < all_lines.len() {
        let line = all_lines[li];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            li += 1;
            continue;
        }
        let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();

        // 0. Quantifier-init line: collect the whole block and attach it to the
        //    most recently created atom (its host).
        if is_quantifier_init(trimmed) {
            let (block, next) = collect_quantifier_block(&all_lines, li, indent)?;
            let host = atoms
                .len()
                .checked_sub(1)
                .ok_or_else(|| CfError::InvalidQuery("quantifier without host atom".to_string()))?;
            let (clean, parent_name) = clean_host_atom(&atoms[host]);
            let block = TextQuantifierBlock {
                kind: block.kind,
                alternatives: block
                    .alternatives
                    .iter()
                    .map(|t| substitute_parent_ref(t, &parent_name))
                    .collect(),
                consequents: block
                    .consequents
                    .iter()
                    .map(|t| substitute_parent_ref(t, &parent_name))
                    .collect(),
            };
            atoms[host].clean = Some((clean, parent_name));
            atoms[host].quantifiers.push(block);
            li = next;
            continue;
        }

        let parts = split_query_whitespace(trimmed);

        'line: {
            // 1. Explicit relation line: `left OP right`.
            if parts.len() == 3 {
                if let Some(operator) = parse_mapped_relation_operator(&parts[1]) {
                    pending_relations.push(PendingMappedRelation {
                        left: parts[0].clone(),
                        operator,
                        right: parts[2].clone(),
                    });
                    break 'line;
                }
            }

            // 2. Lines whose first token is a relation operator: either a lonely
            //    operator (single token) or an operator-prefixed atom or an
            //    operator-prefixed reference to an existing atom (`&& parent`).
            if let Some(first) = parts.first() {
                if let Some(operator) = parse_mapped_atom_operator(first) {
                    if parts.len() == 1 {
                        handle_lonely_operator(
                            indent,
                            operator,
                            &atom_stack,
                            &mut direct_relations,
                        )?;
                        break 'line;
                    }
                    // Operator-prefixed reference: `OP name` where `name` is an
                    // already-defined atom (or a parent ref substituted to one).
                    // TF treats this as an operator edge to the referenced node
                    // without creating a new column.
                    if parts.len() == 2 && !parts[1].contains(':') {
                        if let Some(target) = names.get(&parts[1]).copied() {
                            let other = sibling_or_parent(indent, &atom_stack)?;
                            direct_relations.push(MappedRelation {
                                left: MappedRelationEndpoint::Atom(other),
                                operator,
                                right: MappedRelationEndpoint::Atom(target),
                            });
                            break 'line;
                        }
                    }
                    let atom = parse_simple_atom(line, indent, true)?;
                    let q = atoms.len();
                    if let Some(name) = &atom.name {
                        names.insert(name.clone(), q);
                    }
                    atoms.push(atom);
                    register_atom(
                        indent,
                        Some(operator),
                        q,
                        &mut atom_stack,
                        &mut direct_relations,
                    )?;
                    break 'line;
                }
            }

            // 3. Reference to a previously named atom (adds constraints + an equality).
            if let Some(first) = parts.first() {
                if !first.contains(':') {
                    if let Some(atom_index) = names.get(first).copied() {
                        let constraints = parts[1..]
                            .iter()
                            .map(|part| parse_simple_constraint(part))
                            .collect::<Result<Vec<_>>>()?;
                        let node_type = atoms[atom_index].node_type.clone();
                        let reference_index = atoms.len();
                        atoms.push(SimpleAtom {
                            indent,
                            name: None,
                            node_type,
                            constraints,
                            quantifiers: Vec::new(),
                            clean: None,
                        });
                        pending_relations.push(PendingMappedRelation {
                            left: format!("\0ref{reference_index}"),
                            operator: MappedRelationOperator::Equal,
                            right: first.clone(),
                        });
                        names.insert(format!("\0ref{reference_index}"), reference_index);
                        register_atom(
                            indent,
                            None,
                            reference_index,
                            &mut atom_stack,
                            &mut direct_relations,
                        )?;
                        break 'line;
                    }
                }
            }

            // 4. Feature-continuation line (every token is an explicit constraint).
            if !atoms.is_empty() {
                if let Some(constraints) = parse_mapped_feature_continuation_line(trimmed)? {
                    let last_index = atoms.len() - 1;
                    atoms[last_index].constraints.extend(constraints);
                    break 'line;
                }
            }

            // 5. Plain atom line.
            let atom = parse_simple_atom(line, indent, false)?;
            let q = atoms.len();
            if let Some(name) = &atom.name {
                names.insert(name.clone(), q);
            }
            atoms.push(atom);
            register_atom(indent, None, q, &mut atom_stack, &mut direct_relations)?;
        }
        li += 1;
    }

    if atoms.is_empty() {
        return Err(CfError::InvalidQuery("empty mapped query".to_string()));
    }
    let mut relations = direct_relations;
    for relation in pending_relations {
        let left = mapped_relation_endpoint(&names, &relation.left)?;
        let right = mapped_relation_endpoint(&names, &relation.right)?;
        relations.push(MappedRelation {
            left,
            operator: relation.operator,
            right,
        });
    }
    Ok(MappedRelationPlan { atoms, relations })
}

/// Update the indent/sibling stack with a newly seen atom `q`, emitting the
/// operator edge when one is present. Mirrors TF semantics.py:_grammar: the left
/// operand of an operator edge is the previous sibling at the same indent, else
/// the nearest enclosing parent; the right operand is the operator-prefixed atom.
pub(super) fn register_atom(
    indent: usize,
    operator: Option<MappedRelationOperator>,
    q: usize,
    atom_stack: &mut std::collections::BTreeMap<usize, usize>,
    direct_relations: &mut Vec<MappedRelation>,
) -> Result<()> {
    if let Some(operator) = operator {
        let other = atom_stack
            .get(&indent)
            .copied()
            .or_else(|| atom_stack.range(..indent).next_back().map(|(_, idx)| *idx));
        let Some(other) = other else {
            return Err(CfError::InvalidQuery(
                "Lonely relation: not allowed at outermost level".to_string(),
            ));
        };
        direct_relations.push(MappedRelation {
            left: MappedRelationEndpoint::Atom(other),
            operator,
            right: MappedRelationEndpoint::Atom(q),
        });
    }
    let deeper: Vec<usize> = atom_stack
        .range((indent + 1)..)
        .map(|(indent, _)| *indent)
        .collect();
    for deeper_indent in deeper {
        atom_stack.remove(&deeper_indent);
    }
    atom_stack.insert(indent, q);
    Ok(())
}

/// Handle a line that consists solely of a relation operator (a "lonely"
/// operator). Per TF, it connects the previous sibling at this indent to the
/// enclosing parent; as a first child or at the outermost level it is an error.
pub(super) fn handle_lonely_operator(
    indent: usize,
    operator: MappedRelationOperator,
    atom_stack: &std::collections::BTreeMap<usize, usize>,
    direct_relations: &mut Vec<MappedRelation>,
) -> Result<()> {
    let Some((&top_indent, _)) = atom_stack.iter().next_back() else {
        return Err(CfError::InvalidQuery(
            "Lonely relation: not allowed at outermost level".to_string(),
        ));
    };
    if indent > top_indent {
        return Err(CfError::InvalidQuery(
            "Lonely relation: not allowed as first child".to_string(),
        ));
    }
    let Some(&sibling) = atom_stack.get(&indent) else {
        return Err(CfError::InvalidQuery(format!(
            "Unexpected indent: {indent}"
        )));
    };
    // Connect previous sibling to the enclosing parent, if any.
    if let Some((_, &parent)) = atom_stack.range(..indent).next_back() {
        direct_relations.push(MappedRelation {
            left: MappedRelationEndpoint::Atom(sibling),
            operator,
            right: MappedRelationEndpoint::Atom(parent),
        });
    }
    Ok(())
}

pub(super) fn mapped_relation_endpoint(
    names: &HashMap<String, usize>,
    endpoint: &str,
) -> Result<MappedRelationEndpoint> {
    if endpoint == ".." {
        return Ok(MappedRelationEndpoint::Parent);
    }
    names
        .get(endpoint)
        .copied()
        .map(MappedRelationEndpoint::Atom)
        .ok_or_else(|| CfError::InvalidQuery(format!("unknown mapped relation atom {endpoint:?}")))
}

pub(super) fn nearest_bound_parent_index(atoms: &[SimpleAtom], index: usize) -> Option<usize> {
    let indent = atoms[index].indent;
    if indent == 0 {
        return None;
    }
    (0..index)
        .rev()
        .find(|previous| atoms[*previous].indent < indent)
}
