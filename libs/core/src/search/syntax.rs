//! Search template syntax: quantifier tokens and the relations legend.

use std::collections::HashMap;

use crate::search::study::split_query_whitespace;

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

/// Text-Fabric caps an unlimited search at this many results per node
/// (`tf/search/searchexe.py`).
pub const SEARCH_FAIL_FACTOR: u32 = 4;

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
