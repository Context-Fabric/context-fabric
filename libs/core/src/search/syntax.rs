//! Search template syntax: quantifier tokens and the relations legend.

use std::collections::HashMap;

pub type SearchSets<'a> = HashMap<&'a str, Vec<u32>>;

pub const QWHERE: &str = "/where/";

pub const QWITHOUT: &str = "/without/";

pub const QWITH: &str = "/with/";

pub const QINIT: &[&str] = &[QWHERE, QWITHOUT, QWITH];

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
