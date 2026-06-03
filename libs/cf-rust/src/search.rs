use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

use crate::corpus::Corpus;
use crate::error::{CfError, Result};
use crate::feature::FeatureValue;

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

#[derive(Debug, Clone, PartialEq)]
pub enum SearchPerfValue {
    Int(i64),
    Float(f64),
}

pub fn search_perf_defaults() -> BTreeMap<String, SearchPerfValue> {
    BTreeMap::from([
        ("tryLimitFrom".to_string(), SearchPerfValue::Int(40)),
        ("tryLimitTo".to_string(), SearchPerfValue::Int(40)),
        ("yarnRatio".to_string(), SearchPerfValue::Float(1.25)),
    ])
}

pub struct Search<'a> {
    corpus: &'a Corpus,
}

pub struct SearchSession<'a> {
    search: Search<'a>,
    exe: Option<SearchStudy>,
    perf_params: BTreeMap<String, SearchPerfValue>,
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

impl<'a> Search<'a> {
    pub fn new(corpus: &'a Corpus) -> Self {
        Self { corpus }
    }

    pub fn search(&self, template: &str, limit: Option<usize>) -> Result<Vec<Vec<u32>>> {
        self.search_inner(template, None, limit)
    }

    pub fn fetch(&self, template: &str, limit: Option<usize>) -> Result<Vec<Vec<u32>>> {
        Ok(self.study(template)?.fetch(limit))
    }

    pub fn count(&self, template: &str, limit: Option<usize>) -> Result<usize> {
        Ok(self.study(template)?.count(limit))
    }

    pub fn glean(&self, nodes: &[u32]) -> String {
        if nodes.is_empty() {
            return String::new();
        }
        self.corpus.text_nodes(nodes, None)
    }

    pub fn relations_legend(&self) -> &'static str {
        relations_legend()
    }

    #[allow(non_snake_case)]
    pub fn relationsLegend(&self) -> &'static str {
        self.relations_legend()
    }

    pub fn search_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.search_inner(template, Some(sets), limit)
    }

    pub fn fetch_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        Ok(self.study_with_sets(template, sets)?.fetch(limit))
    }

    pub fn count_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<usize> {
        Ok(self.study_with_sets(template, sets)?.count(limit))
    }

    pub fn search_first_nodes(&self, template: &str, limit: Option<usize>) -> Result<Vec<u32>> {
        Ok(self.study(template)?.fetch_first_nodes(limit))
    }

    #[allow(non_snake_case)]
    pub fn searchFirstNodes(&self, template: &str, limit: Option<usize>) -> Result<Vec<u32>> {
        self.search_first_nodes(template, limit)
    }

    pub fn search_first_nodes_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        Ok(self
            .study_with_sets(template, sets)?
            .fetch_first_nodes(limit))
    }

    #[allow(non_snake_case)]
    pub fn searchFirstNodesWithSets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        self.search_first_nodes_with_sets(template, sets, limit)
    }

    pub fn search_prefixes(
        &self,
        template: &str,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        Ok(self.study(template)?.fetch_prefixes(width, limit))
    }

    #[allow(non_snake_case)]
    pub fn searchPrefixes(
        &self,
        template: &str,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.search_prefixes(template, width, limit)
    }

    pub fn search_prefixes_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        Ok(self
            .study_with_sets(template, sets)?
            .fetch_prefixes(width, limit))
    }

    #[allow(non_snake_case)]
    pub fn searchPrefixesWithSets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.search_prefixes_with_sets(template, sets, width, limit)
    }

    pub fn show_plan(&self, template: &str, details: bool) -> Result<String> {
        Ok(self.study(template)?.show_plan(details))
    }

    #[allow(non_snake_case)]
    pub fn showPlan(&self, template: &str, details: bool) -> Result<String> {
        self.show_plan(template, details)
    }

    pub fn show_plan_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        details: bool,
    ) -> Result<String> {
        Ok(self.study_with_sets(template, sets)?.show_plan(details))
    }

    #[allow(non_snake_case)]
    pub fn showPlanWithSets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        details: bool,
    ) -> Result<String> {
        self.show_plan_with_sets(template, sets, details)
    }

    pub fn study(&self, template: &str) -> Result<SearchStudy> {
        self.study_inner(template, None)
    }

    pub fn study_with_sets(&self, template: &str, sets: &SearchSets<'_>) -> Result<SearchStudy> {
        self.study_inner(template, Some(sets))
    }

    fn study_inner(&self, template: &str, sets: Option<&SearchSets<'_>>) -> Result<SearchStudy> {
        let results = self.search_inner(template, sets, None)?;
        Ok(SearchStudy {
            template: template.to_string(),
            plan: summarize_template(template, results.len())?,
            results,
        })
    }

    fn search_inner(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        if template.lines().any(|line| {
            let token = line.trim();
            matches!(
                token,
                "/where/" | "/have/" | "/with/" | "/without/" | "/or/"
            )
        }) {
            return self.search_quantified(template, sets, limit);
        }
        let plan = parse_query(template)?;
        if plan.atoms.is_empty() {
            return Ok(Vec::new());
        }
        let mut matches: Vec<Vec<u32>> = Vec::new();
        let mut cache = CandidateCache::default();
        self.extend_matches(
            &plan,
            0,
            &mut Vec::new(),
            &mut matches,
            limit,
            &mut cache,
            sets,
        )?;
        Ok(matches)
    }

    fn search_quantified(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        let quantified = parse_quantified_template(template)?;
        let executable = ExecutableQuantifiedTemplate::parse(&quantified)?;
        let mut cache = CandidateCache::default();
        if let Some(limit) = limit {
            let mut filtered = Vec::new();
            self.extend_quantified_matches(
                &executable,
                0,
                &mut Vec::new(),
                &mut filtered,
                limit,
                &mut cache,
                sets,
            )?;
            return Ok(filtered);
        }
        let mut results = Vec::new();
        self.extend_matches(
            &executable.base_plan,
            0,
            &mut Vec::new(),
            &mut results,
            None,
            &mut cache,
            sets,
        )?;
        let mut filtered = Vec::new();
        for row in results {
            let Some(root) = row.first().copied() else {
                continue;
            };
            if self.quantifier_blocks_hold(root, &executable.blocks, &mut cache, sets)? {
                filtered.push(row);
            }
        }
        Ok(filtered)
    }

    fn extend_matches(
        &self,
        plan: &QueryPlan,
        index: usize,
        current: &mut Vec<u32>,
        matches: &mut Vec<Vec<u32>>,
        limit: Option<usize>,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<()> {
        if limit.is_some_and(|limit| matches.len() >= limit) {
            return Ok(());
        }
        if index == plan.atoms.len() {
            if !self.relations_hold(plan, current) {
                return Ok(());
            }
            matches.push(current.clone());
            return Ok(());
        }
        let candidates =
            self.candidates_for_bound_atom(&plan.atoms, index, current, cache, sets)?;
        for node in candidates {
            if self.parent_constraints_hold(&plan.atoms, index, node, current) {
                current.push(node);
                if self.bound_relations_hold(plan, current) {
                    self.extend_matches(plan, index + 1, current, matches, limit, cache, sets)?;
                }
                current.pop();
            }
            if limit.is_some_and(|limit| matches.len() >= limit) {
                break;
            }
        }
        Ok(())
    }

    fn extend_quantified_matches(
        &self,
        executable: &ExecutableQuantifiedTemplate,
        index: usize,
        current: &mut Vec<u32>,
        matches: &mut Vec<Vec<u32>>,
        limit: usize,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<()> {
        if matches.len() >= limit {
            return Ok(());
        }
        let plan = &executable.base_plan;
        if index == plan.atoms.len() {
            if !self.relations_hold(plan, current) {
                return Ok(());
            }
            let Some(root) = current.first().copied() else {
                return Ok(());
            };
            if self.quantifier_blocks_hold(root, &executable.blocks, cache, sets)? {
                matches.push(current.clone());
            }
            return Ok(());
        }
        let candidates =
            self.candidates_for_bound_atom(&plan.atoms, index, current, cache, sets)?;
        for node in candidates {
            if self.parent_constraints_hold(&plan.atoms, index, node, current) {
                current.push(node);
                if self.bound_relations_hold(plan, current) {
                    self.extend_quantified_matches(
                        executable,
                        index + 1,
                        current,
                        matches,
                        limit,
                        cache,
                        sets,
                    )?;
                }
                current.pop();
            }
            if matches.len() >= limit {
                break;
            }
        }
        Ok(())
    }

    fn candidates(&self, atom: &QueryAtom, sets: Option<&SearchSets<'_>>) -> Result<Vec<u32>> {
        let mut candidates = self.initial_candidates(atom, sets)?;
        for constraint in &atom.constraints {
            let feature = self
                .corpus
                .node_feature(&constraint.feature)
                .ok_or_else(|| {
                    CfError::InvalidQuery(format!("unknown feature {}", constraint.feature))
                })?;
            candidates = match &constraint.matcher {
                Matcher::Eq(expected) => feature.filter_by_values(&candidates, expected),
                Matcher::Ne(expected) => {
                    let excluded = feature.filter_by_values(&candidates, expected);
                    candidates
                        .into_iter()
                        .filter(|node| excluded.binary_search(node).is_err())
                        .collect()
                }
                Matcher::Regex(regex) => candidates
                    .into_iter()
                    .filter(|node| {
                        feature
                            .value(*node)
                            .and_then(FeatureValue::as_str)
                            .is_some_and(|raw| regex.is_match(raw))
                    })
                    .collect(),
                Matcher::Exists => feature.filter_has_value(&candidates),
                Matcher::Missing => feature.filter_missing_value(&candidates),
                Matcher::Lt(expected) => feature.filter_less_than(&candidates, expected),
                Matcher::Gt(expected) => feature.filter_greater_than(&candidates, expected),
            };
        }
        Ok(candidates)
    }

    fn candidates_cached(
        &self,
        atom: &QueryAtom,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<u32>> {
        let key = atom.cache_key();
        if let Some(candidates) = cache.nodes.get(&key) {
            return Ok(candidates.clone());
        }
        let candidates = self.candidates(atom, sets)?;
        cache.nodes.insert(key, candidates.clone());
        Ok(candidates)
    }

    fn contained_candidates_cached(
        &self,
        root: u32,
        atom: &QueryAtom,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<u32>> {
        let Some(root_first) = self.first_slot(root) else {
            return Ok(Vec::new());
        };
        let Some(root_last) = self.last_slot(root) else {
            return Ok(Vec::new());
        };
        let key = atom.cache_key();
        if !cache.intervals.contains_key(&key) {
            let candidates = self.candidates_cached(atom, cache, sets)?;
            let mut intervals = Vec::with_capacity(candidates.len());
            for node in candidates {
                let Some(first) = self.first_slot(node) else {
                    continue;
                };
                let Some(last) = self.last_slot(node) else {
                    continue;
                };
                intervals.push(CandidateInterval { node, first, last });
            }
            intervals.sort_unstable_by_key(|candidate| {
                (candidate.first, candidate.last, candidate.node)
            });
            cache.intervals.insert(key.clone(), intervals);
        }
        let intervals = cache
            .intervals
            .get(&key)
            .expect("candidate interval cache should be initialized");
        let start = intervals.partition_point(|candidate| candidate.first < root_first);
        let end = intervals.partition_point(|candidate| candidate.first <= root_last);
        Ok(intervals[start..end]
            .iter()
            .filter(|candidate| candidate.last <= root_last)
            .filter(|candidate| self.corpus.contains(root, candidate.node))
            .map(|candidate| candidate.node)
            .collect())
    }

    fn candidates_for_bound_atom(
        &self,
        atoms: &[QueryAtom],
        index: usize,
        current: &[u32],
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<u32>> {
        let atom = &atoms[index];
        if let Some(parent_node) = self.containment_parent_node(atoms, index, current) {
            return self.contained_candidates_cached(parent_node, atom, cache, sets);
        }
        self.candidates_cached(atom, cache, sets)
    }

    fn initial_candidates(
        &self,
        atom: &QueryAtom,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<u32>> {
        let type_nodes = self.initial_type_nodes(atom, sets);
        let mut best_eq: Option<(&Constraint, usize)> = None;
        for constraint in &atom.constraints {
            if let Matcher::Eq(expected_values) = &constraint.matcher {
                let feature = self
                    .corpus
                    .node_feature(&constraint.feature)
                    .ok_or_else(|| {
                        CfError::InvalidQuery(format!("unknown feature {}", constraint.feature))
                    })?;
                let count = expected_values
                    .iter()
                    .flat_map(candidate_values_for)
                    .map(|value| feature.nodes_with_value(&value).len())
                    .sum();
                if best_eq.is_none_or(|(_, best_count)| count < best_count) {
                    best_eq = Some((constraint, count));
                }
            }
        }
        let Some((constraint, _)) = best_eq else {
            return Ok(type_nodes);
        };
        let feature = self
            .corpus
            .node_feature(&constraint.feature)
            .ok_or_else(|| {
                CfError::InvalidQuery(format!("unknown feature {}", constraint.feature))
            })?;
        let mut nodes = Vec::new();
        match &constraint.matcher {
            Matcher::Eq(expected_values) => {
                for expected in expected_values {
                    for value in candidate_values_for(expected) {
                        nodes.extend(feature.nodes_with_value(&value));
                    }
                }
            }
            _ => unreachable!(),
        }
        nodes.sort_unstable();
        nodes.dedup();
        Ok(nodes
            .into_iter()
            .filter(|node| type_nodes.binary_search(node).is_ok())
            .collect())
    }

    fn initial_type_nodes(&self, atom: &QueryAtom, sets: Option<&SearchSets<'_>>) -> Vec<u32> {
        if atom.is_generic_node_type() {
            return self.corpus.feature_nodes();
        }
        if let Some(nodes) = sets.and_then(|sets| sets.get(atom.node_type.as_str())) {
            let mut nodes = nodes.clone();
            nodes.sort_unstable();
            nodes.dedup();
            return nodes;
        }
        self.corpus.nodes_of_type(&atom.node_type).to_vec()
    }

    fn parent_constraints_hold(
        &self,
        atoms: &[QueryAtom],
        index: usize,
        node: u32,
        current: &[u32],
    ) -> bool {
        let indent = atoms[index].indent;
        if indent == 0 {
            return true;
        }
        for previous in (0..index).rev() {
            if atoms[previous].indent < indent {
                let default_operator = RelationOperator::Embeds;
                let operator = atoms[index].operator.as_ref().unwrap_or(&default_operator);
                return self.relation_operator_holds(operator, current[previous], node);
            }
        }
        true
    }

    fn containment_parent_node(
        &self,
        atoms: &[QueryAtom],
        index: usize,
        current: &[u32],
    ) -> Option<u32> {
        let indent = atoms[index].indent;
        if indent == 0 {
            return None;
        }
        for previous in (0..index).rev() {
            if atoms[previous].indent < indent {
                let is_containment = atoms[index]
                    .operator
                    .as_ref()
                    .is_none_or(|operator| matches!(operator, RelationOperator::Embeds));
                return is_containment.then(|| current[previous]);
            }
        }
        None
    }

    fn relations_hold(&self, plan: &QueryPlan, current: &[u32]) -> bool {
        self.relations_hold_with_root(plan, current, None)
    }

    fn relations_hold_with_root(
        &self,
        plan: &QueryPlan,
        current: &[u32],
        root: Option<u32>,
    ) -> bool {
        plan.relations.iter().all(|relation| {
            self.bound_relation_nodes(plan, current, relation, root)
                .is_some_and(|(left, right)| self.relation_holds(relation, left, right))
        })
    }

    fn bound_relations_hold(&self, plan: &QueryPlan, current: &[u32]) -> bool {
        self.bound_relations_hold_with_root(plan, current, None)
    }

    fn bound_relations_hold_with_root(
        &self,
        plan: &QueryPlan,
        current: &[u32],
        root: Option<u32>,
    ) -> bool {
        plan.relations.iter().all(|relation| {
            self.bound_relation_nodes(plan, current, relation, root)
                .is_none_or(|(left, right)| self.relation_holds(relation, left, right))
        })
    }

    fn bound_relation_nodes(
        &self,
        plan: &QueryPlan,
        current: &[u32],
        relation: &Relation,
        root: Option<u32>,
    ) -> Option<(u32, u32)> {
        let left = relation_endpoint_node(plan, current, &relation.left, root)?;
        let right = relation_endpoint_node(plan, current, &relation.right, root)?;
        Some((left, right))
    }

    fn relation_holds(&self, relation: &Relation, left: u32, right: u32) -> bool {
        self.relation_operator_holds(&relation.operator, left, right)
    }

    fn relation_operator_holds(&self, operator: &RelationOperator, left: u32, right: u32) -> bool {
        match operator {
            RelationOperator::Equal => left == right,
            RelationOperator::Before => self.corpus.sort_key(left) < self.corpus.sort_key(right),
            RelationOperator::After => self.corpus.sort_key(left) > self.corpus.sort_key(right),
            RelationOperator::NotEqual => left != right,
            RelationOperator::SameSlots => self.slots_equal(left, right),
            RelationOperator::DifferentSlots => !self.slots_equal(left, right),
            RelationOperator::Overlaps => self.slots_overlap(left, right),
            RelationOperator::Disjoint => !self.slots_overlap(left, right),
            RelationOperator::Embeds => self.corpus.contains(left, right),
            RelationOperator::EmbeddedIn => self.corpus.contains(right, left),
            RelationOperator::SlotBefore => self
                .last_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left, right)| left < right),
            RelationOperator::SlotAfter => self
                .first_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left, right)| left > right),
            RelationOperator::AdjacentBefore => self
                .last_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left, right)| left.checked_add(1) == Some(right)),
            RelationOperator::AdjacentAfter => self
                .first_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left, right)| right.checked_add(1) == Some(left)),
            RelationOperator::SameFirstSlot => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left, right)| left == right),
            RelationOperator::SameLastSlot => self
                .last_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left, right)| left == right),
            RelationOperator::SameBoundary => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .zip(self.last_slot(left).zip(self.last_slot(right)))
                .is_some_and(|((left_first, right_first), (left_last, right_last))| {
                    left_first == right_first && left_last == right_last
                }),
            RelationOperator::NearFirstSlot(distance) => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left, right)| slots_within(left, right, *distance)),
            RelationOperator::NearLastSlot(distance) => self
                .last_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left, right)| slots_within(left, right, *distance)),
            RelationOperator::NearBoundary(distance) => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .zip(self.last_slot(left).zip(self.last_slot(right)))
                .is_some_and(|((left_first, right_first), (left_last, right_last))| {
                    slots_within(left_first, right_first, *distance)
                        && slots_within(left_last, right_last, *distance)
                }),
            RelationOperator::NearBefore(distance) => self
                .last_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left_last, right_first)| {
                    left_last < right_first
                        && right_first.saturating_sub(left_last) <= distance.saturating_add(1)
                }),
            RelationOperator::NearAfter(distance) => self
                .first_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left_first, right_last)| {
                    right_last < left_first
                        && left_first.saturating_sub(right_last) <= distance.saturating_add(1)
                }),
            RelationOperator::EdgeForward(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .and_then(|feature| feature.targets(left))
                .is_some_and(|targets| targets.binary_search(&right).is_ok()),
            RelationOperator::EdgeBackward(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .and_then(|feature| feature.targets(right))
                .is_some_and(|targets| targets.binary_search(&left).is_ok()),
            RelationOperator::EdgeEither(edge_name) => {
                self.corpus.edge_feature(edge_name).is_some_and(|feature| {
                    feature
                        .targets(left)
                        .is_some_and(|targets| targets.binary_search(&right).is_ok())
                        || feature
                            .targets(right)
                            .is_some_and(|targets| targets.binary_search(&left).is_ok())
                })
            }
            RelationOperator::EdgeForwardValue(edge_name, matcher) => {
                self.corpus.edge_feature(edge_name).is_some_and(|feature| {
                    feature
                        .targets(left)
                        .is_some_and(|targets| targets.binary_search(&right).is_ok())
                        && feature
                            .edge_value(left, right)
                            .is_some_and(|actual| edge_value_matches(actual, matcher))
                })
            }
            RelationOperator::EdgeBackwardValue(edge_name, matcher) => {
                self.corpus.edge_feature(edge_name).is_some_and(|feature| {
                    feature
                        .targets(right)
                        .is_some_and(|targets| targets.binary_search(&left).is_ok())
                        && feature
                            .edge_value(right, left)
                            .is_some_and(|actual| edge_value_matches(actual, matcher))
                })
            }
            RelationOperator::EdgeEitherValue(edge_name, matcher) => {
                self.corpus.edge_feature(edge_name).is_some_and(|feature| {
                    (feature
                        .targets(left)
                        .is_some_and(|targets| targets.binary_search(&right).is_ok())
                        && feature
                            .edge_value(left, right)
                            .is_some_and(|actual| edge_value_matches(actual, matcher)))
                        || (feature
                            .targets(right)
                            .is_some_and(|targets| targets.binary_search(&left).is_ok())
                            && feature
                                .edge_value(right, left)
                                .is_some_and(|actual| edge_value_matches(actual, matcher)))
                })
            }
            RelationOperator::FeatureCompare {
                left_feature,
                comparison,
                right_feature,
            } => self.feature_relation_holds(left, right, left_feature, *comparison, right_feature),
            RelationOperator::FeatureRegexCompare {
                left_feature,
                pattern,
                right_feature,
            } => {
                self.feature_regex_relation_holds(left, right, left_feature, pattern, right_feature)
            }
        }
    }

    fn feature_relation_holds(
        &self,
        left: u32,
        right: u32,
        left_feature: &str,
        comparison: FeatureRelationComparison,
        right_feature: &str,
    ) -> bool {
        let Some(left_value) = self
            .corpus
            .node_feature(left_feature)
            .and_then(|feature| feature.value(left))
        else {
            return false;
        };
        let Some(right_value) = self
            .corpus
            .node_feature(right_feature)
            .and_then(|feature| feature.value(right))
        else {
            return false;
        };
        match comparison {
            FeatureRelationComparison::Eq => feature_value_matches(left_value, right_value),
            FeatureRelationComparison::Ne => !feature_value_matches(left_value, right_value),
            FeatureRelationComparison::Lt => feature_value_compare(left_value, right_value)
                .is_some_and(|ordering| ordering.is_lt()),
            FeatureRelationComparison::Gt => feature_value_compare(left_value, right_value)
                .is_some_and(|ordering| ordering.is_gt()),
        }
    }

    fn feature_regex_relation_holds(
        &self,
        left: u32,
        right: u32,
        left_feature: &str,
        pattern: &Regex,
        right_feature: &str,
    ) -> bool {
        let Some(left_value) = self
            .corpus
            .node_feature(left_feature)
            .and_then(|feature| feature.value(left))
            .and_then(FeatureValue::as_str)
        else {
            return false;
        };
        let Some(right_value) = self
            .corpus
            .node_feature(right_feature)
            .and_then(|feature| feature.value(right))
            .and_then(FeatureValue::as_str)
        else {
            return false;
        };
        pattern.replace_all(left_value, "") == pattern.replace_all(right_value, "")
    }

    fn first_slot(&self, node: u32) -> Option<u32> {
        if node <= self.corpus.max_slot {
            Some(node)
        } else {
            self.corpus
                .slots_of(node)
                .and_then(|slots| slots.first().copied())
        }
    }

    fn last_slot(&self, node: u32) -> Option<u32> {
        if node <= self.corpus.max_slot {
            Some(node)
        } else {
            self.corpus
                .slots_of(node)
                .and_then(|slots| slots.last().copied())
        }
    }

    fn slots_equal(&self, left: u32, right: u32) -> bool {
        self.node_slots(left) == self.node_slots(right)
    }

    fn slots_overlap(&self, left: u32, right: u32) -> bool {
        let left_slots = self.node_slots(left);
        let right_slots = self.node_slots(right);
        let mut left_index = 0;
        let mut right_index = 0;
        while let (Some(left), Some(right)) =
            (left_slots.get(left_index), right_slots.get(right_index))
        {
            match left.cmp(right) {
                std::cmp::Ordering::Equal => return true,
                std::cmp::Ordering::Less => left_index += 1,
                std::cmp::Ordering::Greater => right_index += 1,
            }
        }
        false
    }

    fn node_slots(&self, node: u32) -> Vec<u32> {
        if node <= self.corpus.max_slot {
            vec![node]
        } else {
            self.corpus
                .slots_of(node)
                .map(<[u32]>::to_vec)
                .unwrap_or_default()
        }
    }

    fn exists_contained_match(
        &self,
        root: u32,
        plan: &QueryPlan,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        if plan.atoms.is_empty() {
            return Ok(false);
        }
        self.exists_contained_match_at(root, plan, 0, &mut Vec::new(), cache, sets)
    }

    fn exists_contained_quantified(
        &self,
        root: u32,
        quantified: &ExecutableQuantifiedTemplate,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        let mut rows = Vec::new();
        self.extend_contained_matches(
            root,
            &quantified.base_plan,
            0,
            &mut Vec::new(),
            &mut rows,
            cache,
            sets,
        )?;
        for row in rows {
            let Some(nested_root) = row.first().copied() else {
                continue;
            };
            if self.quantifier_blocks_hold(nested_root, &quantified.blocks, cache, sets)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn exists_contained_alternative(
        &self,
        root: u32,
        alternative: &ExecutableQuantifierAlternative,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        match alternative {
            ExecutableQuantifierAlternative::Plan(plan) => {
                self.exists_contained_match(root, plan, cache, sets)
            }
            ExecutableQuantifierAlternative::Quantified(quantified) => {
                self.exists_contained_quantified(root, quantified, cache, sets)
            }
        }
    }

    fn quantifier_blocks_hold(
        &self,
        root: u32,
        blocks: &[ExecutableQuantifierBlock],
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        for block in blocks {
            let mut exists = false;
            for alternative in &block.alternatives {
                if self.exists_contained_alternative(root, alternative, cache, sets)? {
                    exists = true;
                    break;
                }
            }
            let keep = match block.kind {
                QuantifierKind::With => exists,
                QuantifierKind::Without => !exists,
            };
            if !keep {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn extend_contained_matches(
        &self,
        root: u32,
        plan: &QueryPlan,
        index: usize,
        current: &mut Vec<u32>,
        matches: &mut Vec<Vec<u32>>,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<()> {
        if index == plan.atoms.len() {
            if self.relations_hold_with_root(plan, current, Some(root)) {
                matches.push(current.clone());
            }
            return Ok(());
        }
        let atom = &plan.atoms[index];
        let candidates = if atom.is_parent_reference() {
            self.parent_reference_candidates(root, atom, cache, sets)?
        } else if atom.indent == 0 {
            self.contained_candidates_cached(root, atom, cache, sets)?
        } else if let Some(parent_node) = self.containment_parent_node(&plan.atoms, index, current)
        {
            self.contained_candidates_cached(parent_node, atom, cache, sets)?
        } else {
            self.candidates_cached(atom, cache, sets)?
        };
        for node in candidates {
            if index > 0 && !self.parent_constraints_hold(&plan.atoms, index, node, current) {
                continue;
            }
            current.push(node);
            if self.bound_relations_hold_with_root(plan, current, Some(root)) {
                self.extend_contained_matches(
                    root,
                    plan,
                    index + 1,
                    current,
                    matches,
                    cache,
                    sets,
                )?;
            }
            current.pop();
        }
        Ok(())
    }

    fn exists_contained_match_at(
        &self,
        root: u32,
        plan: &QueryPlan,
        index: usize,
        current: &mut Vec<u32>,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        if index == plan.atoms.len() {
            return Ok(self.relations_hold_with_root(plan, current, Some(root)));
        }
        let atom = &plan.atoms[index];
        let candidates = if atom.is_parent_reference() {
            self.parent_reference_candidates(root, atom, cache, sets)?
        } else if atom.indent == 0 {
            self.contained_candidates_cached(root, atom, cache, sets)?
        } else if let Some(parent_node) = self.containment_parent_node(&plan.atoms, index, current)
        {
            self.contained_candidates_cached(parent_node, atom, cache, sets)?
        } else {
            self.candidates_cached(atom, cache, sets)?
        };
        for node in candidates {
            if index > 0 && !self.parent_constraints_hold(&plan.atoms, index, node, current) {
                continue;
            }
            current.push(node);
            if self.bound_relations_hold_with_root(plan, current, Some(root))
                && self.exists_contained_match_at(root, plan, index + 1, current, cache, sets)?
            {
                return Ok(true);
            }
            current.pop();
        }
        Ok(false)
    }

    fn parent_reference_candidates(
        &self,
        root: u32,
        atom: &QueryAtom,
        cache: &mut CandidateCache,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<u32>> {
        let Some(node_type) = self.corpus.node_type(root) else {
            return Ok(Vec::new());
        };
        let mut parent_atom = atom.clone();
        parent_atom.node_type = node_type.to_string();
        Ok(self
            .candidates_cached(&parent_atom, cache, sets)?
            .binary_search(&root)
            .is_ok()
            .then_some(root)
            .into_iter()
            .collect())
    }
}

impl<'a> SearchSession<'a> {
    pub fn new(corpus: &'a Corpus) -> Self {
        Self {
            search: Search::new(corpus),
            exe: None,
            perf_params: search_perf_defaults(),
        }
    }

    pub fn exe(&self) -> Option<&SearchStudy> {
        self.exe.as_ref()
    }

    pub fn perf_params(&self) -> &BTreeMap<String, SearchPerfValue> {
        &self.perf_params
    }

    #[allow(non_snake_case)]
    pub fn perfParams(&self) -> &BTreeMap<String, SearchPerfValue> {
        self.perf_params()
    }

    pub fn tweak_performance<I, S>(&mut self, updates: I) -> Vec<String>
    where
        I: IntoIterator<Item = (S, Option<SearchPerfValue>)>,
        S: Into<String>,
    {
        let defaults = search_perf_defaults();
        let mut errors = Vec::new();
        for (key, value) in updates {
            let key = key.into();
            let Some(default_value) = defaults.get(&key) else {
                errors.push(format!("No such performance parameter: \"{key}\""));
                continue;
            };
            let value = value.unwrap_or_else(|| default_value.clone());
            if key != "yarnRatio" && !matches!(value, SearchPerfValue::Int(_)) {
                errors.push(format!(
                    "Performance parameter \"{key}\" must be set to an integer"
                ));
                continue;
            }
            self.perf_params.insert(key, value);
        }
        errors
    }

    #[allow(non_snake_case)]
    pub fn tweakPerformance<I, S>(&mut self, updates: I) -> Vec<String>
    where
        I: IntoIterator<Item = (S, Option<SearchPerfValue>)>,
        S: Into<String>,
    {
        self.tweak_performance(updates)
    }

    pub fn search(
        &mut self,
        template: &str,
        limit: Option<usize>,
        here: bool,
    ) -> Result<Vec<Vec<u32>>> {
        let study = self.search.study(template)?;
        let results = study.fetch(limit);
        if here {
            self.exe = Some(study);
        }
        Ok(results)
    }

    pub fn study(&mut self, template: &str, here: bool) -> Result<()> {
        let study = self.search.study(template)?;
        if here {
            self.exe = Some(study);
        }
        Ok(())
    }

    pub fn fetch(&self, limit: Option<usize>) -> Result<Vec<Vec<u32>>> {
        self.exe
            .as_ref()
            .map(|study| study.fetch(limit))
            .ok_or_else(|| CfError::InvalidQuery("No previous study".to_string()))
    }

    pub fn count(&self, limit: Option<usize>) -> Result<usize> {
        self.exe
            .as_ref()
            .map(|study| study.count(limit))
            .ok_or_else(|| CfError::InvalidQuery("No previous study".to_string()))
    }

    pub fn show_plan(&self, details: bool) -> Result<String> {
        self.exe
            .as_ref()
            .map(|study| study.show_plan(details))
            .ok_or_else(|| CfError::InvalidQuery("No previous study".to_string()))
    }

    #[allow(non_snake_case)]
    pub fn showPlan(&self, details: bool) -> Result<String> {
        self.show_plan(details)
    }

    pub fn relations_legend(&self) -> &'static str {
        self.search.relations_legend()
    }

    #[allow(non_snake_case)]
    pub fn relationsLegend(&self) -> &'static str {
        self.relations_legend()
    }

    pub fn glean(&self, nodes: &[u32]) -> String {
        self.search.glean(nodes)
    }
}

impl SearchStudy {
    pub fn new(template: String, results: Vec<Vec<u32>>) -> Self {
        let plan =
            summarize_template(&template, results.len()).unwrap_or_else(|_| SearchPlanSummary {
                template: template.clone(),
                atom_count: 0,
                relation_count: 0,
                result_count: results.len(),
                has_quantifiers: false,
            });
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

fn summarize_template(template: &str, result_count: usize) -> Result<SearchPlanSummary> {
    let has_quantifiers = template.lines().any(|line| {
        let token = line.trim();
        matches!(
            token,
            "/where/" | "/have/" | "/with/" | "/without/" | "/or/"
        )
    });
    let (atom_count, relation_count) = if has_quantifiers {
        let quantified = parse_quantified_template(template)?;
        let base_plan = parse_query(&quantified.base)?;
        let alternative_counts = quantified
            .blocks
            .iter()
            .flat_map(|block| block.alternatives.iter())
            .map(|alternative| parse_query(alternative))
            .collect::<Result<Vec<_>>>()?;
        (
            base_plan.atoms.len()
                + alternative_counts
                    .iter()
                    .map(|plan| plan.atoms.len())
                    .sum::<usize>(),
            base_plan.relations.len()
                + alternative_counts
                    .iter()
                    .map(|plan| plan.relations.len())
                    .sum::<usize>(),
        )
    } else {
        let plan = parse_query(template)?;
        (plan.atoms.len(), plan.relations.len())
    };
    Ok(SearchPlanSummary {
        template: template.to_string(),
        atom_count,
        relation_count,
        result_count,
        has_quantifiers,
    })
}

#[derive(Debug, Default)]
struct CandidateCache {
    nodes: HashMap<String, Vec<u32>>,
    intervals: HashMap<String, Vec<CandidateInterval>>,
}

#[derive(Debug, Clone, Copy)]
struct CandidateInterval {
    node: u32,
    first: u32,
    last: u32,
}

#[derive(Debug)]
struct QueryPlan {
    atoms: Vec<QueryAtom>,
    names: HashMap<String, usize>,
    relations: Vec<Relation>,
}

#[derive(Debug, Clone)]
struct QueryAtom {
    indent: usize,
    operator: Option<RelationOperator>,
    name: Option<String>,
    node_type: String,
    constraints: Vec<Constraint>,
}

impl QueryAtom {
    fn is_generic_node_type(&self) -> bool {
        self.node_type == "."
    }

    fn is_parent_reference(&self) -> bool {
        self.node_type == ".."
    }

    fn cache_key(&self) -> String {
        let mut key = format!("{}:{}", self.indent, self.node_type);
        if let Some(operator) = &self.operator {
            key.push_str(&format!("{operator:?}"));
        }
        if let Some(name) = &self.name {
            key.push('@');
            key.push_str(name);
        }
        for constraint in &self.constraints {
            key.push('|');
            key.push_str(&constraint.cache_key());
        }
        key
    }
}

#[derive(Debug, Clone)]
struct Constraint {
    feature: String,
    matcher: Matcher,
}

impl Constraint {
    fn cache_key(&self) -> String {
        format!("{}{}", self.feature, self.matcher.cache_key())
    }
}

#[derive(Debug, Clone)]
enum Matcher {
    Eq(Vec<FeatureValue>),
    Ne(Vec<FeatureValue>),
    Regex(Regex),
    Exists,
    Missing,
    Lt(FeatureValue),
    Gt(FeatureValue),
}

impl Matcher {
    fn cache_key(&self) -> String {
        match self {
            Self::Eq(values) => format!(
                "={}",
                values
                    .iter()
                    .map(value_cache_key)
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            Self::Ne(values) => format!(
                "#{}",
                values
                    .iter()
                    .map(value_cache_key)
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            Self::Regex(regex) => format!("~{}", regex.as_str()),
            Self::Exists => "*".to_string(),
            Self::Missing => "#".to_string(),
            Self::Lt(value) => format!("<{}", value_cache_key(value)),
            Self::Gt(value) => format!(">{}", value_cache_key(value)),
        }
    }
}

#[derive(Debug)]
struct Relation {
    left: String,
    operator: RelationOperator,
    right: String,
}

#[derive(Debug, Clone)]
enum RelationOperator {
    Equal,
    Before,
    After,
    NotEqual,
    SameSlots,
    DifferentSlots,
    Overlaps,
    Disjoint,
    Embeds,
    EmbeddedIn,
    SlotBefore,
    SlotAfter,
    AdjacentBefore,
    AdjacentAfter,
    SameFirstSlot,
    SameLastSlot,
    SameBoundary,
    NearFirstSlot(u32),
    NearLastSlot(u32),
    NearBoundary(u32),
    NearBefore(u32),
    NearAfter(u32),
    EdgeForward(String),
    EdgeBackward(String),
    EdgeEither(String),
    EdgeForwardValue(String, EdgeValueMatcher),
    EdgeBackwardValue(String, EdgeValueMatcher),
    EdgeEitherValue(String, EdgeValueMatcher),
    FeatureCompare {
        left_feature: String,
        comparison: FeatureRelationComparison,
        right_feature: String,
    },
    FeatureRegexCompare {
        left_feature: String,
        pattern: Regex,
        right_feature: String,
    },
}

#[derive(Debug, Clone)]
enum EdgeValueMatcher {
    Eq(Vec<FeatureValue>),
    Ne(Vec<FeatureValue>),
    Regex(Regex),
}

#[derive(Debug, Clone, Copy)]
enum FeatureRelationComparison {
    Eq,
    Ne,
    Lt,
    Gt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuantifierKind {
    With,
    Without,
}

#[derive(Debug)]
struct QuantifierBlock {
    kind: QuantifierKind,
    alternatives: Vec<String>,
}

#[derive(Debug)]
struct QuantifiedTemplate {
    base: String,
    blocks: Vec<QuantifierBlock>,
}

struct ExecutableQuantifiedTemplate {
    base_plan: QueryPlan,
    blocks: Vec<ExecutableQuantifierBlock>,
}

struct ExecutableQuantifierBlock {
    kind: QuantifierKind,
    alternatives: Vec<ExecutableQuantifierAlternative>,
}

enum ExecutableQuantifierAlternative {
    Plan(QueryPlan),
    Quantified(Box<ExecutableQuantifiedTemplate>),
}

impl ExecutableQuantifiedTemplate {
    fn parse(template: &QuantifiedTemplate) -> Result<Self> {
        let base_plan = parse_query(&template.base)?;
        let blocks = template
            .blocks
            .iter()
            .map(|block| {
                let alternatives = block
                    .alternatives
                    .iter()
                    .map(|alternative| {
                        if contains_quantifier_token(alternative) {
                            parse_quantified_template(alternative)
                                .and_then(|nested| Self::parse(&nested))
                                .map(|nested| {
                                    ExecutableQuantifierAlternative::Quantified(Box::new(nested))
                                })
                        } else {
                            parse_query(alternative).map(ExecutableQuantifierAlternative::Plan)
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(ExecutableQuantifierBlock {
                    kind: block.kind,
                    alternatives,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { base_plan, blocks })
    }
}

fn parse_quantified_template(template: &str) -> Result<QuantifiedTemplate> {
    let mut base_lines = Vec::new();
    let mut blocks = Vec::new();
    let mut current_kind: Option<QuantifierKind> = None;
    let mut current_lines = Vec::new();
    let mut current_alternatives = Vec::new();
    let mut nested_depth = 0usize;

    for line in template.lines() {
        let token = line.trim();
        match token {
            "/where/" | "/with/" | "/without/" => {
                if current_kind.is_some() {
                    nested_depth += 1;
                    current_lines.push(line.to_string());
                    continue;
                }
                current_kind = Some(match token {
                    "/without/" => QuantifierKind::Without,
                    _ => QuantifierKind::With,
                });
            }
            "/have/" => {
                if nested_depth > 0 {
                    current_lines.push(line.to_string());
                    continue;
                }
                let Some(kind) = current_kind else {
                    return Err(CfError::InvalidQuery(
                        "quantifier continuation without quantifier".to_string(),
                    ));
                };
                if kind != QuantifierKind::With {
                    return Err(CfError::InvalidQuery(
                        "/have/ is only supported after /where/ or /with/".to_string(),
                    ));
                }
                push_quantifier_alternative(&mut current_alternatives, &current_lines);
                blocks.push(QuantifierBlock {
                    kind,
                    alternatives: std::mem::take(&mut current_alternatives),
                });
                current_lines.clear();
                current_kind = Some(QuantifierKind::With);
            }
            "/or/" => {
                if nested_depth > 0 {
                    current_lines.push(line.to_string());
                    continue;
                }
                if current_kind.is_none() {
                    return Err(CfError::InvalidQuery(
                        "quantifier alternative without quantifier".to_string(),
                    ));
                }
                push_quantifier_alternative(&mut current_alternatives, &current_lines);
                current_lines.clear();
            }
            "/-/" => {
                if nested_depth > 0 {
                    nested_depth -= 1;
                    current_lines.push(line.to_string());
                    continue;
                }
                let Some(kind) = current_kind.take() else {
                    return Err(CfError::InvalidQuery(
                        "quantifier terminator without quantifier".to_string(),
                    ));
                };
                push_quantifier_alternative(&mut current_alternatives, &current_lines);
                blocks.push(QuantifierBlock {
                    kind,
                    alternatives: std::mem::take(&mut current_alternatives),
                });
                current_lines.clear();
            }
            _ => {
                if current_kind.is_some() {
                    current_lines.push(line.to_string());
                } else {
                    base_lines.push(line.to_string());
                }
            }
        }
    }

    if current_kind.is_some() || nested_depth > 0 {
        return Err(CfError::InvalidQuery(
            "unterminated quantified block".to_string(),
        ));
    }
    Ok(QuantifiedTemplate {
        base: base_lines.join("\n"),
        blocks,
    })
}

fn contains_quantifier_token(template: &str) -> bool {
    template.lines().any(|line| {
        matches!(
            line.trim(),
            "/where/" | "/with/" | "/without/" | "/have/" | "/or/" | "/-/"
        )
    })
}

fn push_quantifier_alternative(alternatives: &mut Vec<String>, lines: &[String]) {
    if lines.iter().any(|line| !line.trim().is_empty()) {
        alternatives.push(normalize_parent_reference_relations(
            &normalize_indentation(lines),
        ));
    }
}

fn normalize_parent_reference_relations(template: &str) -> String {
    template
        .lines()
        .filter(|line| !is_redundant_parent_containment_relation(line.trim()))
        .map(str::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_redundant_parent_containment_relation(line: &str) -> bool {
    let parts = split_query_whitespace(line);
    (parts.len() == 3 && parts[0] == ".." && parts[1] == "[[")
        || (parts.len() == 3 && parts[1] == "]]" && parts[2] == "..")
}

fn normalize_indentation(lines: &[String]) -> String {
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).count())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| {
            if line.len() >= min_indent {
                line[min_indent..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn candidate_values_for(value: &FeatureValue) -> Vec<FeatureValue> {
    let mut values = vec![value.clone()];
    if let FeatureValue::Str(raw) = value {
        if let Ok(parsed) = raw.parse::<i64>() {
            values.push(FeatureValue::Int(parsed));
        }
    }
    values
}

fn feature_value_matches(actual: &FeatureValue, expected: &FeatureValue) -> bool {
    if actual == expected {
        return true;
    }
    match (actual, expected) {
        (FeatureValue::Int(actual), FeatureValue::Str(expected)) => expected
            .parse::<i64>()
            .is_ok_and(|expected| *actual == expected),
        _ => false,
    }
}

fn edge_value_matches(actual: &FeatureValue, matcher: &EdgeValueMatcher) -> bool {
    match matcher {
        EdgeValueMatcher::Eq(expected_values) => expected_values
            .iter()
            .any(|expected| feature_value_matches(actual, expected)),
        EdgeValueMatcher::Ne(expected_values) => expected_values
            .iter()
            .all(|expected| !feature_value_matches(actual, expected)),
        EdgeValueMatcher::Regex(pattern) => match actual {
            FeatureValue::Str(actual) => pattern.is_match(actual),
            FeatureValue::Int(actual) => pattern.is_match(&actual.to_string()),
        },
    }
}

fn feature_value_compare(
    actual: &FeatureValue,
    expected: &FeatureValue,
) -> Option<std::cmp::Ordering> {
    match (actual, expected) {
        (FeatureValue::Int(actual), FeatureValue::Int(expected)) => Some(actual.cmp(expected)),
        (FeatureValue::Int(actual), FeatureValue::Str(expected)) => expected
            .parse::<i64>()
            .ok()
            .map(|expected| actual.cmp(&expected)),
        (FeatureValue::Str(actual), FeatureValue::Str(expected)) => Some(actual.cmp(expected)),
        (FeatureValue::Str(actual), FeatureValue::Int(expected)) => actual
            .parse::<i64>()
            .ok()
            .map(|actual| actual.cmp(expected)),
    }
}

fn value_cache_key(value: &FeatureValue) -> String {
    match value {
        FeatureValue::Str(value) => format!("s:{value}"),
        FeatureValue::Int(value) => format!("i:{value}"),
    }
}

fn relation_endpoint_node(
    plan: &QueryPlan,
    current: &[u32],
    endpoint: &str,
    root: Option<u32>,
) -> Option<u32> {
    if endpoint == ".." {
        return root;
    }
    plan.names
        .get(endpoint)
        .and_then(|index| current.get(*index))
        .copied()
}

fn parse_query(template: &str) -> Result<QueryPlan> {
    let mut atoms: Vec<QueryAtom> = Vec::new();
    let mut names: HashMap<String, usize> = HashMap::new();
    let mut relations = Vec::new();
    let mut pending_atom_operator: Option<RelationOperator> = None;
    for raw_line in template.lines() {
        if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('%') {
            continue;
        }
        let indent = raw_line.chars().take_while(|ch| ch.is_whitespace()).count();
        let line = raw_line.trim();
        if let Some(operator) = parse_atom_operator(line) {
            pending_atom_operator = Some(operator);
            continue;
        }
        let relation_parts = split_query_whitespace(line);
        if relation_parts.len() == 3 {
            if let Some(operator) = parse_relation_operator(&relation_parts[1]) {
                relations.push(Relation {
                    left: relation_parts[0].clone(),
                    operator,
                    right: relation_parts[2].clone(),
                });
                continue;
            }
        }
        if pending_atom_operator.is_none() && !atoms.is_empty() {
            if let Some(constraints) = parse_feature_continuation_line(line)? {
                let last_index = atoms.len() - 1;
                atoms[last_index].constraints.extend(constraints);
                continue;
            }
        }

        let parts = split_query_whitespace(line);
        let mut parts = parts.into_iter();
        let Some(first) = parts.next() else {
            continue;
        };
        let (operator, first) = match parse_atom_operator(&first) {
            Some(operator) => {
                let Some(atom) = parts.next() else {
                    return Err(CfError::InvalidQuery(format!(
                        "missing atom after operator {first:?}"
                    )));
                };
                (Some(operator), atom)
            }
            None => (None, first),
        };
        let operator = operator.or_else(|| pending_atom_operator.take());
        if let Some(referenced_index) = names.get(&first).copied() {
            let node_type = atoms[referenced_index].node_type.clone();
            let constraints = parts
                .map(|part| parse_constraint(&part))
                .collect::<Result<Vec<_>>>()?;
            let reference_name = format!("\0ref{}", atoms.len());
            names.insert(reference_name.clone(), atoms.len());
            atoms.push(QueryAtom {
                indent,
                operator,
                name: Some(reference_name.clone()),
                node_type,
                constraints,
            });
            relations.push(Relation {
                left: reference_name,
                operator: RelationOperator::Equal,
                right: first,
            });
            continue;
        }
        let (name, node_type) = first
            .split_once(':')
            .map(|(name, node_type)| (Some(name.to_string()), node_type.to_string()))
            .unwrap_or((None, first.to_string()));
        let constraints = parts
            .map(|part| parse_constraint(&part))
            .collect::<Result<Vec<_>>>()?;
        if let Some(name) = &name {
            names.insert(name.clone(), atoms.len());
        }
        atoms.push(QueryAtom {
            indent,
            operator,
            name,
            node_type,
            constraints,
        });
    }
    Ok(QueryPlan {
        atoms,
        names,
        relations,
    })
}

fn parse_atom_operator(raw: &str) -> Option<RelationOperator> {
    parse_relation_operator(raw)
}

fn parse_feature_continuation_line(line: &str) -> Result<Option<Vec<Constraint>>> {
    let mut constraints = Vec::new();
    for part in split_query_whitespace(line) {
        if !looks_like_explicit_feature_constraint(&part) {
            return Ok(None);
        }
        let constraint = parse_constraint(&part)?;
        constraints.push(constraint);
    }
    if constraints.is_empty() {
        Ok(None)
    } else {
        Ok(Some(constraints))
    }
}

fn looks_like_explicit_feature_constraint(part: &str) -> bool {
    part.ends_with('*')
        || part.ends_with('#')
        || part.contains('=')
        || part.contains('~')
        || part.contains('<')
        || part.contains('>')
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

fn parse_constraint(part: &str) -> Result<Constraint> {
    if let Some(feature) = part.strip_suffix('*') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Exists,
        });
    }
    if let Some(feature) = part.strip_suffix('#') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Missing,
        });
    }
    if let Some((feature, value)) = part.split_once('=') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Eq(parse_alternative_values(value)),
        });
    }
    if let Some((feature, value)) = part.split_once('#') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Ne(parse_alternative_values(value)),
        });
    }
    if let Some((feature, pattern)) = part.split_once('~') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Regex(Regex::new(pattern)?),
        });
    }
    if let Some((feature, value)) = part.split_once('<') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Lt(parse_value(value)),
        });
    }
    if let Some((feature, value)) = part.split_once('>') {
        return Ok(Constraint {
            feature: feature.to_string(),
            matcher: Matcher::Gt(parse_value(value)),
        });
    }
    if !part.is_empty() {
        return Ok(Constraint {
            feature: part.to_string(),
            matcher: Matcher::Exists,
        });
    }
    Err(CfError::InvalidQuery(format!(
        "unsupported constraint syntax {part:?}"
    )))
}

fn parse_alternative_values(value: &str) -> Vec<FeatureValue> {
    split_unescaped(value, '|')
        .into_iter()
        .map(|value| parse_value(&unescape_query_value(&value)))
        .collect()
}

fn parse_value(value: &str) -> FeatureValue {
    value
        .parse::<i64>()
        .map(FeatureValue::Int)
        .unwrap_or_else(|_| FeatureValue::string(value))
}

fn split_unescaped(raw: &str, delimiter: char) -> Vec<String> {
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

fn unescape_query_value(raw: &str) -> String {
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

fn parse_relation_operator(operator: &str) -> Option<RelationOperator> {
    match operator {
        "=" => Some(RelationOperator::Equal),
        "<" => Some(RelationOperator::Before),
        ">" => Some(RelationOperator::After),
        "#" => Some(RelationOperator::NotEqual),
        "==" => Some(RelationOperator::SameSlots),
        "##" => Some(RelationOperator::DifferentSlots),
        "&&" => Some(RelationOperator::Overlaps),
        "||" => Some(RelationOperator::Disjoint),
        "[[" => Some(RelationOperator::Embeds),
        "]]" => Some(RelationOperator::EmbeddedIn),
        "<<" => Some(RelationOperator::SlotBefore),
        ">>" => Some(RelationOperator::SlotAfter),
        "<:" => Some(RelationOperator::AdjacentBefore),
        ":>" => Some(RelationOperator::AdjacentAfter),
        "=:" => Some(RelationOperator::SameFirstSlot),
        ":=" => Some(RelationOperator::SameLastSlot),
        "::" => Some(RelationOperator::SameBoundary),
        _ => None,
    }
    .or_else(|| parse_near_relation_operator(operator))
    .or_else(|| {
        operator
            .strip_prefix('-')
            .and_then(|rest| rest.strip_suffix('>'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_edge_forward_operator)
    })
    .or_else(|| {
        operator
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('-'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_edge_backward_operator)
    })
    .or_else(|| {
        operator
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_edge_either_operator)
    })
    .or_else(|| parse_feature_relation_operator(operator))
}

fn parse_near_relation_operator(operator: &str) -> Option<RelationOperator> {
    parse_surrounded_u32(operator, '=', ':')
        .map(RelationOperator::NearFirstSlot)
        .or_else(|| parse_surrounded_u32(operator, ':', '=').map(RelationOperator::NearLastSlot))
        .or_else(|| parse_surrounded_u32(operator, ':', ':').map(RelationOperator::NearBoundary))
        .or_else(|| parse_surrounded_u32(operator, '<', ':').map(RelationOperator::NearBefore))
        .or_else(|| parse_surrounded_u32(operator, ':', '>').map(RelationOperator::NearAfter))
}

fn parse_surrounded_u32(operator: &str, prefix: char, suffix: char) -> Option<u32> {
    operator
        .strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse::<u32>()
        .ok()
}

fn parse_edge_forward_operator(raw: &str) -> RelationOperator {
    if let Some((edge_name, matcher)) = parse_edge_value_matcher(raw) {
        return RelationOperator::EdgeForwardValue(edge_name, matcher);
    }
    RelationOperator::EdgeForward(raw.to_string())
}

fn slots_within(left: u32, right: u32, distance: u32) -> bool {
    left.abs_diff(right) <= distance
}

fn parse_edge_backward_operator(raw: &str) -> RelationOperator {
    if let Some((edge_name, matcher)) = parse_edge_value_matcher(raw) {
        return RelationOperator::EdgeBackwardValue(edge_name, matcher);
    }
    RelationOperator::EdgeBackward(raw.to_string())
}

fn parse_edge_either_operator(raw: &str) -> RelationOperator {
    if let Some((edge_name, matcher)) = parse_edge_value_matcher(raw) {
        return RelationOperator::EdgeEitherValue(edge_name, matcher);
    }
    RelationOperator::EdgeEither(raw.to_string())
}

fn parse_edge_value_matcher(raw: &str) -> Option<(String, EdgeValueMatcher)> {
    if let Some((edge_name, value)) = raw.split_once('=') {
        if !edge_name.is_empty() {
            return Some((
                edge_name.to_string(),
                EdgeValueMatcher::Eq(parse_alternative_values(value)),
            ));
        }
    }
    if let Some((edge_name, value)) = raw.split_once('#') {
        if !edge_name.is_empty() {
            return Some((
                edge_name.to_string(),
                EdgeValueMatcher::Ne(parse_alternative_values(value)),
            ));
        }
    }
    if let Some((edge_name, pattern)) = raw.split_once('~') {
        if !edge_name.is_empty() {
            return Regex::new(&unescape_query_value(pattern))
                .ok()
                .map(|pattern| (edge_name.to_string(), EdgeValueMatcher::Regex(pattern)));
        }
    }
    None
}

fn parse_feature_relation_operator(operator: &str) -> Option<RelationOperator> {
    let inner = operator.strip_prefix('.')?.strip_suffix('.')?;
    if inner.is_empty() {
        return None;
    }
    if let Some(operator) = parse_feature_regex_relation_operator(inner) {
        return Some(operator);
    }
    for (raw_operator, comparison) in [
        ('=', FeatureRelationComparison::Eq),
        ('#', FeatureRelationComparison::Ne),
        ('<', FeatureRelationComparison::Lt),
        ('>', FeatureRelationComparison::Gt),
    ] {
        if let Some((left_feature, right_feature)) = inner.split_once(raw_operator) {
            if left_feature.is_empty() || right_feature.is_empty() {
                return None;
            }
            return Some(RelationOperator::FeatureCompare {
                left_feature: left_feature.to_string(),
                comparison,
                right_feature: right_feature.to_string(),
            });
        }
    }
    Some(RelationOperator::FeatureCompare {
        left_feature: inner.to_string(),
        comparison: FeatureRelationComparison::Eq,
        right_feature: inner.to_string(),
    })
}

fn parse_feature_regex_relation_operator(inner: &str) -> Option<RelationOperator> {
    let (left_feature, rest) = inner.split_once('~')?;
    let (pattern, right_feature) = rest.rsplit_once('~')?;
    if left_feature.is_empty() || pattern.is_empty() || right_feature.is_empty() {
        return None;
    }
    Some(RelationOperator::FeatureRegexCompare {
        left_feature: left_feature.to_string(),
        pattern: Regex::new(pattern).ok()?,
        right_feature: right_feature.to_string(),
    })
}
