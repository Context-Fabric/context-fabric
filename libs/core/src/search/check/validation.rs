//! A template's query graph checked against the corpus
//! (`tf/search/semantics.py::_validation`), then for connectedness
//! (`tf/search/graph.py::connectedness`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::search::check::feature_spec::{
    FeatureSpec, is_edge_operator, is_python_int, regex_error_text, regex_repr,
};
use crate::search::check::grammar::{Query, QueryEdge};
use crate::search::check::schema::{TemplateSchema, ValueType};
use crate::search::check::tokens::Operator;
use crate::search::check::{TemplateErrorKind, TemplateMessage};
use crate::search::syntax::RELATIONS_LEGEND;

/// Relations without parameters (TF `basicRelations`).
const BASIC_RELATIONS: [&str; 17] = [
    "=", "#", "<", ">", "==", "&&", "##", "||", "[[", "]]", "<<", ">>", "=:", ":=", "::", "<:",
    ":>",
];

/// Relations with a slot distance `k` (TF's `=k:` … `:k>`).
const K_RELATIONS: [&str; 5] = ["=k:", ":k=", ":k:", "<k:", ":k>"];

static K_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([^0-9]*)([0-9]+)([^0-9]+)$").expect("valid regex"));
static F_ONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\.([^=#<>]+)\.$").expect("valid regex"));
static F_BOTH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\.([^=#<>]+)([=#<>])(.*)\.$").expect("valid regex"));
static F_MATCH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\.([^~]+)~(.*?)~([^~]+)\.$").expect("valid regex"));

/// A feature comparison relation (`.f.`, `.f=g.`, `.f~r~g.`, …).
struct FeatureRelation {
    left: String,
    right: String,
    /// The value type both features must have, if the comparison needs one.
    required: Option<ValueType>,
    /// The pattern of a `.f~r~g.` comparison.
    pattern: Option<String>,
}

impl FeatureRelation {
    fn parse(op: &str) -> Option<Self> {
        if let Some(captures) = F_MATCH_RE.captures(op) {
            return Some(Self {
                left: captures[1].to_string(),
                right: captures[3].to_string(),
                required: Some(ValueType::Str),
                pattern: Some(captures[2].to_string()),
            });
        }
        if let Some(captures) = F_BOTH_RE.captures(op) {
            let ordered = matches!(&captures[2], "<" | ">");
            return Some(Self {
                left: captures[1].to_string(),
                right: captures[3].to_string(),
                required: ordered.then_some(ValueType::Int),
                pattern: None,
            });
        }
        F_ONE_RE.captures(op).map(|captures| Self {
            left: captures[1].to_string(),
            right: captures[1].to_string(),
            required: None,
            pattern: None,
        })
    }
}

fn is_feature_operator(op: &str) -> bool {
    op.len() >= 2 && op.starts_with('.') && op.ends_with('.')
}

/// Messages about the features of a template, keyed like TF's
/// `missingFeatures`, `hasValues`, `wrongValues` and `wrongTypes`.
#[derive(Default)]
struct FeatureProblems {
    /// Feature -> lines that use it.
    missing: BTreeMap<String, Vec<usize>>,
    /// Edge feature -> superfluous value condition -> lines.
    has_values: BTreeMap<String, BTreeMap<String, Vec<usize>>>,
    /// Feature -> wrong value -> lines.
    wrong_values: BTreeMap<String, BTreeMap<String, Vec<usize>>>,
    /// Feature -> its value type -> lines.
    wrong_types: BTreeMap<String, BTreeMap<String, BTreeSet<usize>>>,
}

impl FeatureProblems {
    /// Whether no feature is missing and no value is wrong: TF only then
    /// instantiates parametrized relations.
    fn relations_resolvable(&self) -> bool {
        self.missing.is_empty() && self.wrong_values.is_empty()
    }

    /// Check one feature condition (TF `_validateFeature`).
    fn check(
        &mut self,
        schema: &TemplateSchema<'_>,
        name: &str,
        spec: &FeatureSpec,
        line: usize,
        as_edge: bool,
    ) {
        let value_type = if as_edge {
            let Some(edge) = schema.edge_features.get(name) else {
                self.missing.entry(name.to_string()).or_default().push(line);
                return;
            };
            if !edge.has_values {
                if *spec != FeatureSpec::Exists {
                    self.has_values
                        .entry(name.to_string())
                        .or_default()
                        .entry(spec.describe())
                        .or_default()
                        .push(line);
                }
                return;
            }
            edge.value_type
        } else {
            let Some(&value_type) = schema.node_features.get(name) else {
                self.missing.entry(name.to_string()).or_default().push(line);
                return;
            };
            value_type
        };
        let wrong: Vec<String> = match spec {
            FeatureSpec::Exists | FeatureSpec::Missing | FeatureSpec::Any => Vec::new(),
            FeatureSpec::Limit { .. } if value_type == ValueType::Str => vec![spec.describe()],
            FeatureSpec::Regex(pattern) if value_type == ValueType::Int => {
                vec![regex_repr(pattern)]
            }
            FeatureSpec::Values { values, .. } if value_type == ValueType::Int => values
                .iter()
                .filter(|value| !is_python_int(value))
                .cloned()
                .collect(),
            _ => Vec::new(),
        };
        for value in wrong {
            self.wrong_values
                .entry(name.to_string())
                .or_default()
                .entry(value)
                .or_default()
                .push(line);
        }
    }
}

/// The semantic errors of `query` against `schema`, else its connectedness
/// errors. Lines are shifted by `offset`.
pub(super) fn validate(
    query: &Query,
    schema: &TemplateSchema<'_>,
    offset: usize,
) -> Vec<TemplateMessage> {
    let mut report = Report::new(offset);

    // Node types.
    let mut types_good = true;
    for node in &query.nodes {
        if node.otype == "." || schema.is_set(&node.otype) {
            continue;
        }
        if !schema.node_types.contains(&node.otype) {
            report.at(
                node.line,
                format!("Unknown object type: \"{}\"", node.otype),
            );
            types_good = false;
        }
    }
    if !types_good {
        report.general(format!(
            "Valid object types are: {}",
            schema.node_types.in_level_order().join(", ")
        ));
        if let Some(sets) = &schema.set_names {
            report.general(format!("Or choose a custom set from: {}", sets.join(", ")));
        }
    }

    // Node features.
    let mut problems = FeatureProblems::default();
    for node in &query.nodes {
        for (name, spec) in &node.features {
            problems.check(schema, name, spec, node.line + offset, false);
        }
    }
    let node_relations_resolvable = problems.relations_resolvable();

    // Feature comparison relations: value types.
    let mut feature_relations: Vec<Option<FeatureRelation>> = Vec::new();
    for edge in &query.edges {
        let relation = (edge.op.edge_features.is_empty())
            .then(|| FeatureRelation::parse(&edge.op.text))
            .flatten();
        if let Some(relation) = &relation
            && let Some(required) = relation.required
        {
            for name in [&relation.left, &relation.right] {
                if let Some(&value_type) = schema.node_features.get(name.as_str())
                    && value_type != required
                {
                    problems
                        .wrong_types
                        .entry(name.clone())
                        .or_default()
                        .entry(value_type.name().to_string())
                        .or_default()
                        .insert(edge.line + offset);
                }
            }
        }
        feature_relations.push(relation);
    }

    // Edge feature conditions.
    for edge in &query.edges {
        for (name, spec) in &edge.op.edge_features {
            problems.check(schema, name, spec, edge.line + offset, true);
        }
    }
    let edge_relations_resolvable = problems.relations_resolvable();

    // Relation names.
    let mut relations_good = true;
    for (edge, feature_relation) in query.edges.iter().zip(&feature_relations) {
        let known = relation_known(
            edge,
            feature_relation.as_ref(),
            schema,
            node_relations_resolvable,
            edge_relations_resolvable,
        );
        if !known {
            report.at(
                edge.line,
                format!("Unknown relation: \"{}\"", edge.op.display()),
            );
            relations_good = false;
        }
    }
    if !relations_good {
        report.general(format!("Allowed relations:\n{RELATIONS_LEGEND}"));
    }

    // Features named by comparison relations must exist; Text-Fabric fails
    // on a missing one at search time instead of reporting it.
    let mut bad_patterns = Vec::new();
    for (edge, relation) in query.edges.iter().zip(&feature_relations) {
        let Some(relation) = relation else { continue };
        let mut names = vec![&relation.left];
        if relation.right != relation.left {
            names.push(&relation.right);
        }
        for name in names {
            if !schema.node_features.contains_key(name.as_str()) {
                problems
                    .missing
                    .entry(name.clone())
                    .or_default()
                    .push(edge.line + offset);
            }
        }
        if let Some(pattern) = &relation.pattern
            && let Err(error) = Regex::new(pattern)
        {
            bad_patterns.push((
                edge.line,
                format!(
                    "Wrong regular expression \"{pattern}\": \"{}\"",
                    regex_error_text(&error)
                ),
            ));
        }
    }

    report.feature_problems(&problems);
    for (line, message) in bad_patterns {
        report.at(line, message);
    }

    if report.messages.is_empty() {
        return connectedness(query);
    }
    report.messages
}

/// Whether TF's relation table has the edge's operator (TF
/// `relationFromName`, after `add_K_Relations`, `add_F_Relations` and
/// `add_V_Relations`).
fn relation_known(
    edge: &QueryEdge,
    feature_relation: Option<&FeatureRelation>,
    schema: &TemplateSchema<'_>,
    node_relations_resolvable: bool,
    edge_relations_resolvable: bool,
) -> bool {
    let op: &Operator = &edge.op;
    if !op.edge_features.is_empty() {
        return edge_relations_resolvable
            && op.edge_features.keys().all(|name| {
                schema
                    .edge_features
                    .get(name.as_str())
                    .is_some_and(|edge| edge.searchable)
            });
    }
    let text = op.text.as_str();
    if is_edge_operator(text) {
        return false;
    }
    if is_feature_operator(text) {
        return node_relations_resolvable && feature_relation.is_some();
    }
    if BASIC_RELATIONS.contains(&text) {
        return true;
    }
    let Some(captures) = K_RE.captures(text) else {
        return false;
    };
    let k = &captures[2];
    let canonical = k.trim_start_matches('0');
    let canonical = if canonical.is_empty() { "0" } else { canonical };
    let template = format!("{}k{}", &captures[1], &captures[3]);
    node_relations_resolvable && K_RELATIONS.contains(&template.as_str()) && canonical == k
}

/// The connectedness errors of a valid query graph.
fn connectedness(query: &Query) -> Vec<TemplateMessage> {
    let count = query.nodes.len();
    let plan = |text: String| TemplateMessage {
        kind: TemplateErrorKind::Plan,
        line: None,
        text,
    };
    if count == 0 {
        return vec![plan(
            "Search without instructions. Tell me what to look for.".to_string(),
        )];
    }
    let mut parent: Vec<usize> = (0..count).collect();
    fn root(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for edge in &query.edges {
        let (from, to) = (root(&mut parent, edge.from), root(&mut parent, edge.to));
        if from != to {
            parent[from] = to;
        }
    }
    let components = (0..count)
        .filter(|&node| root(&mut parent, node) == node)
        .count();
    if components > 1 {
        return vec![
            plan(format!(
                "More than one connected components ({components}):"
            )),
            plan(
                "Either run the subqueries one by one, or connect the components by a relation"
                    .to_string(),
            ),
        ];
    }
    Vec::new()
}

/// Semantic messages in the making, with template lines shifted by `offset`.
struct Report {
    offset: usize,
    messages: Vec<TemplateMessage>,
}

impl Report {
    fn new(offset: usize) -> Self {
        Self {
            offset,
            messages: Vec::new(),
        }
    }

    fn at(&mut self, line: usize, text: String) {
        self.messages.push(TemplateMessage {
            kind: TemplateErrorKind::Semantics,
            line: Some(line + self.offset),
            text,
        });
    }

    fn general(&mut self, text: String) {
        self.messages.push(TemplateMessage {
            kind: TemplateErrorKind::Semantics,
            line: None,
            text,
        });
    }

    fn feature_problems(&mut self, problems: &FeatureProblems) {
        let lines = |lines: &mut dyn Iterator<Item = &usize>| {
            lines.map(usize::to_string).collect::<Vec<_>>().join(", ")
        };
        for (name, used) in &problems.missing {
            self.general(format!(
                "Missing feature \"{name}\" in line(s) {}",
                lines(&mut used.iter())
            ));
        }
        for (name, wrongs) in &problems.has_values {
            self.general(format!("Feature \"{name}\" cannot have values:"));
            for (value, used) in wrongs {
                self.general(format!(
                    "    \"{value}\" superfluous: line(s) {}",
                    lines(&mut used.iter())
                ));
            }
        }
        for (name, wrongs) in &problems.wrong_values {
            self.general(format!("Feature \"{name}\" has wrong values:"));
            for (value, used) in wrongs {
                self.general(format!(
                    "    \"{value}\" is not a number: line(s) {}",
                    lines(&mut used.iter())
                ));
            }
        }
        for (name, wrongs) in &problems.wrong_types {
            self.general(format!("Feature \"{name}\" has wrong type:"));
            for (value_type, used) in wrongs {
                self.general(format!(
                    "    \"{value_type}\" is the wrong type: line(s) {}",
                    lines(&mut used.iter())
                ));
            }
        }
    }
}
