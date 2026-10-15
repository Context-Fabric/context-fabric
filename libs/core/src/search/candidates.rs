//! Candidate nodes of a search atom: its node type and feature constraints
//! resolved to a set that supports slot-interval queries.

use crate::error::{CfError, Result};
use crate::mapped::{MappedNodeFeatureView, MappedNodeValue};
use crate::search::constraint::SimpleConstraint;
use crate::search::plan::{SimpleAtom, custom_set_nodes};
use crate::search::{MappedSearch, SearchSets};

impl<'a> MappedSearch<'a> {
    pub(super) fn load_constraints<'query>(
        &self,
        atom: &'query SimpleAtom,
    ) -> Result<Vec<ResolvedConstraint<'a, 'query>>> {
        let mut constraints = Vec::new();
        for constraint in &atom.constraints {
            let feature = self.load_node_feature(&constraint.feature)?;
            constraints.push(ResolvedConstraint::new(constraint, feature)?);
        }
        Ok(constraints)
    }

    pub(super) fn load_node_feature(&self, feature: &str) -> Result<MappedNodeFeatureView<'a>> {
        if let Some(view) = self.corpus.string_pool_node_feature(feature)? {
            return Ok(MappedNodeFeatureView::StringPool(view));
        }
        if let Some(view) = self.corpus.mixed_node_feature(feature)? {
            return Ok(MappedNodeFeatureView::Mixed(view));
        }
        Err(CfError::MissingFeature(feature.to_string()))
    }

    pub(super) fn search_atom(
        &self,
        atom: &SimpleAtom,
        constraints: &[ResolvedConstraint<'_, '_>],
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        let mut results = Vec::new();
        if let Some(nodes) = custom_set_nodes(atom, sets) {
            let mut nodes = self.filter_candidate_nodes(nodes, constraints)?;
            if let Some(limit) = limit {
                nodes.truncate(limit);
            }
            results.extend(nodes);
        } else if let Some(seed) = constraints
            .iter()
            .find(|constraint| constraint.can_seed_candidates())
        {
            // Seed from the constraint's feature rows. For string-pool features
            // this walks raw `(node, pool_id)` pairs against the precomputed
            // bitmap — no string decode or matcher run per row.
            match (&seed.pool_match, &seed.feature) {
                (Some(pool_match), MappedNodeFeatureView::StringPool(view)) => {
                    view.for_each_id_row(&mut |node, pool_id| {
                        if pool_match.get(pool_id as usize).copied().unwrap_or(false)
                            && self.node_matches(node, atom, constraints, sets)?
                        {
                            results.push(node);
                        }
                        Ok(limit.is_none_or(|limit| results.len() < limit))
                    })?;
                }
                _ => {
                    seed.feature.for_each_row(&mut |node, value| {
                        if seed.constraint.matches(Some(value))
                            && self.node_matches(node, atom, constraints, sets)?
                        {
                            results.push(node);
                        }
                        Ok(limit.is_none_or(|limit| results.len() < limit))
                    })?;
                }
            }
        } else {
            // No seedable constraint: seed from the (cached) per-type node list,
            // which preserves the row order of a plain `otype` scan.
            let typed_nodes = if atom.is_generic_node_type() {
                None
            } else {
                self.corpus.typed_nodes(&atom.node_type)?
            };
            if let Some(typed_nodes) = typed_nodes {
                // List membership already proves the node type; only the
                // (non-seedable) constraints remain to check.
                'nodes: for node in typed_nodes.iter() {
                    for constraint in constraints {
                        if !constraint.matches_node(*node)? {
                            continue 'nodes;
                        }
                    }
                    results.push(*node);
                    if limit.is_some_and(|limit| results.len() >= limit) {
                        break;
                    }
                }
                return Ok(results);
            }
            if limit.is_some() {
                for row in self.otype.rows() {
                    let (node, node_type) = row?;
                    if (atom.is_generic_node_type() || node_type == atom.node_type)
                        && self.node_matches(node, atom, constraints, sets)?
                    {
                        results.push(node);
                        if limit.is_some_and(|limit| results.len() >= limit) {
                            break;
                        }
                    }
                }
                return Ok(results);
            }
            let mut nodes = Vec::new();
            for row in self.otype.rows() {
                let (node, node_type) = row?;
                if atom.is_generic_node_type() || node_type == atom.node_type {
                    nodes.push(node);
                }
            }
            let mut nodes = self.filter_candidate_nodes(nodes, constraints)?;
            if let Some(limit) = limit {
                nodes.truncate(limit);
            }
            results.extend(nodes);
        }
        Ok(results)
    }

    pub(super) fn filter_candidate_nodes(
        &self,
        nodes: Vec<u32>,
        constraints: &[ResolvedConstraint<'_, '_>],
    ) -> Result<Vec<u32>> {
        let mut candidates = nodes;
        for constraint in constraints {
            let mut kept = Vec::with_capacity(candidates.len());
            for node in candidates {
                if constraint.matches_node(node)? {
                    kept.push(node);
                }
            }
            candidates = kept;
        }
        Ok(candidates)
    }

    pub(super) fn build_candidate_set(&self, nodes: Vec<u32>) -> Result<CandidateSet> {
        let entries = nodes
            .into_iter()
            .map(|node| {
                let interval = self.fast_interval(node)?;
                Ok(CandidateEntry {
                    node,
                    first_slot: interval.map(|(first, _)| first),
                    last_slot: interval.map(|(_, last)| last),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(CandidateSet::new(entries))
    }

    pub(super) fn node_matches(
        &self,
        node: u32,
        atom: &SimpleAtom,
        constraints: &[ResolvedConstraint<'_, '_>],
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        if let Some(nodes) = custom_set_nodes(atom, sets) {
            if nodes.binary_search(&node).is_err() {
                return Ok(false);
            }
        } else if !atom.is_generic_node_type()
            && !atom.is_parent_reference()
            && self.otype.str_value(node)? != Some(atom.node_type.as_str())
        {
            return Ok(false);
        }
        for constraint in constraints {
            if !constraint.matches_node(node)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// A constraint resolved against its feature for one query. For string-pool
/// features the matcher is evaluated once per *distinct* pooled value into
/// `pool_match`; every per-node test is then a binary search for the node's
/// pool id plus a bitmap probe — no string decode, no regex run, no value
/// parse. Semantics are identical by construction: `pool_match[id]` is exactly
/// `constraint.matches(Some(Str(pool[id])))`.
pub(super) struct ResolvedConstraint<'a, 'query> {
    pub(super) constraint: &'query SimpleConstraint,
    pub(super) feature: MappedNodeFeatureView<'a>,
    pub(super) pool_match: Option<Vec<bool>>,
    /// `constraint.matches(None)`: whether a node with no value satisfies.
    pub(super) missing_matches: bool,
}

impl<'a, 'query> ResolvedConstraint<'a, 'query> {
    pub(super) fn new(
        constraint: &'query SimpleConstraint,
        feature: MappedNodeFeatureView<'a>,
    ) -> Result<Self> {
        let pool_match = match &feature {
            MappedNodeFeatureView::StringPool(view) => Some(
                view.distinct_values()?
                    .into_iter()
                    .map(|value| constraint.matches(Some(MappedNodeValue::Str(value))))
                    .collect(),
            ),
            MappedNodeFeatureView::Mixed(_) => None,
        };
        Ok(Self {
            constraint,
            feature,
            pool_match,
            missing_matches: constraint.matches(None),
        })
    }

    pub(super) fn can_seed_candidates(&self) -> bool {
        self.constraint.can_seed_candidates()
    }

    pub(super) fn matches_node(&self, node: u32) -> Result<bool> {
        match (&self.pool_match, &self.feature) {
            (Some(pool_match), MappedNodeFeatureView::StringPool(view)) => {
                Ok(match view.pool_id(node)? {
                    Some(pool_id) => pool_match.get(pool_id as usize).copied().unwrap_or(false),
                    None => self.missing_matches,
                })
            }
            _ => Ok(self.constraint.matches(self.feature.value(node)?)),
        }
    }
}

pub(super) struct CandidateSet {
    pub(super) entries: Vec<CandidateEntry>,
    // u32 indices into `entries`: candidate counts are node counts, far below
    // u32::MAX, and the halved width matters — a corpus-wide `word` atom holds
    // 400k+ entries, and several such sets can be live per query.
    pub(super) by_first_slot: Vec<u32>,
    pub(super) by_last_slot: Vec<u32>,
    /// Node ids sorted ascending; membership is a binary search. ~7x smaller
    /// than the `HashSet<u32>` it replaces at these sizes.
    pub(super) sorted_nodes: Vec<u32>,
    /// The longest candidate interval (`last_slot - first_slot`). A candidate
    /// reaching slot `s` must start at or after `s - max_span`, which bounds the
    /// containment and overlap scans to a window of `by_first_slot`.
    pub(super) max_span: u32,
}

impl CandidateSet {
    pub(super) fn new(entries: Vec<CandidateEntry>) -> Self {
        let mut by_first_slot = (0..entries.len() as u32).collect::<Vec<_>>();
        by_first_slot.sort_by_key(|index| {
            let entry = &entries[*index as usize];
            (
                entry.first_slot.unwrap_or(u32::MAX),
                entry.last_slot.unwrap_or(u32::MAX),
                entry.node,
            )
        });
        let mut by_last_slot = (0..entries.len() as u32).collect::<Vec<_>>();
        by_last_slot.sort_by_key(|index| {
            let entry = &entries[*index as usize];
            (
                entry.last_slot.unwrap_or(u32::MAX),
                entry.first_slot.unwrap_or(u32::MAX),
                entry.node,
            )
        });
        let mut sorted_nodes: Vec<u32> = entries.iter().map(|entry| entry.node).collect();
        sorted_nodes.sort_unstable();
        let max_span = entries
            .iter()
            .filter_map(|entry| Some(entry.last_slot?.saturating_sub(entry.first_slot?)))
            .max()
            .unwrap_or(0);
        Self {
            entries,
            by_first_slot,
            by_last_slot,
            sorted_nodes,
            max_span,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn contains(&self, node: u32) -> bool {
        self.sorted_nodes.binary_search(&node).is_ok()
    }

    pub(super) fn nodes(&self) -> Vec<u32> {
        self.entries.iter().map(|entry| entry.node).collect()
    }

    /// Candidates with `first_slot` in `[lo, hi]` (binary search over the
    /// first-slot index). Entries without a first slot are excluded.
    pub(super) fn nodes_with_first_in(&self, lo: u32, hi: u32) -> Vec<u32> {
        if lo > hi {
            return Vec::new();
        }
        let start = self.by_first_slot.partition_point(|index| {
            self.entries[*index as usize]
                .first_slot
                .is_some_and(|first| first < lo)
        });
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index as usize]
                .first_slot
                .is_some_and(|first| first <= hi)
        });
        self.by_first_slot[start..end]
            .iter()
            .map(|index| self.entries[*index as usize].node)
            .collect()
    }

    /// Candidates with `last_slot` in `[lo, hi]` (binary search over the last-slot
    /// index). Entries without a last slot are excluded.
    pub(super) fn nodes_with_last_in(&self, lo: u32, hi: u32) -> Vec<u32> {
        if lo > hi {
            return Vec::new();
        }
        let start = self.by_last_slot.partition_point(|index| {
            self.entries[*index as usize]
                .last_slot
                .is_some_and(|last| last < lo)
        });
        let end = self.by_last_slot.partition_point(|index| {
            self.entries[*index as usize]
                .last_slot
                .is_some_and(|last| last <= hi)
        });
        self.by_last_slot[start..end]
            .iter()
            .map(|index| self.entries[*index as usize].node)
            .collect()
    }

    /// Candidates contained in `[first_slot, last_slot]` (first >= first_slot and
    /// last <= last_slot).
    pub(super) fn nodes_within_slot_interval(&self, first_slot: u32, last_slot: u32) -> Vec<u32> {
        let start = self.by_first_slot.partition_point(|index| {
            self.entries[*index as usize]
                .first_slot
                .is_some_and(|first| first < first_slot)
        });
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index as usize]
                .first_slot
                .is_some_and(|first| first <= last_slot)
        });
        self.by_first_slot[start..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index as usize];
                (entry.last_slot? <= last_slot).then_some(entry.node)
            })
            .collect()
    }

    /// Candidates whose interval contains `[first, last]` (first_slot <= first and
    /// last_slot >= last).
    pub(super) fn nodes_containing(&self, first: u32, last: u32) -> Vec<u32> {
        let (start, end) = self.first_slot_window(last.saturating_sub(self.max_span), first);
        self.by_first_slot[start..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index as usize];
                (entry.last_slot? >= last).then_some(entry.node)
            })
            .collect()
    }

    /// Candidates whose interval overlaps `[first, last]` (first_slot <= last and
    /// last_slot >= first).
    pub(super) fn nodes_overlapping(&self, first: u32, last: u32) -> Vec<u32> {
        let (start, end) = self.first_slot_window(first.saturating_sub(self.max_span), last);
        self.by_first_slot[start..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index as usize];
                (entry.last_slot? >= first).then_some(entry.node)
            })
            .collect()
    }

    /// The `by_first_slot` index range of candidates with `first_slot` in
    /// `[lo, hi]` (entries without a first slot sort last and are excluded).
    pub(super) fn first_slot_window(&self, lo: u32, hi: u32) -> (usize, usize) {
        let start = self.by_first_slot.partition_point(|index| {
            self.entries[*index as usize]
                .first_slot
                .is_some_and(|candidate_first| candidate_first < lo)
        });
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index as usize]
                .first_slot
                .is_some_and(|candidate_first| candidate_first <= hi)
        });
        (start, end.max(start))
    }
}

pub(super) struct CandidateEntry {
    pub(super) node: u32,
    pub(super) first_slot: Option<u32>,
    pub(super) last_slot: Option<u32>,
}
