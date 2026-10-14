//! Evaluation of the relations between bound atoms.

use regex::Regex;

use crate::error::Result;
use crate::mapped::{EdgeFeatureView, MappedNodeValue};
use crate::search::MappedSearch;
use crate::search::constraint::{MappedMatcher, mapped_value_compare, mapped_values_equal};
use crate::search::plan::{
    MappedFeatureRelationComparison, MappedRelation, MappedRelationOperator,
};

impl<'a> MappedSearch<'a> {
    pub(super) fn relation_holds(&self, relation: &MappedRelation, left: u32, right: u32) -> bool {
        self.relation_operator_holds(&relation.operator, left, right)
    }

    pub(super) fn relation_operator_holds(
        &self,
        operator: &MappedRelationOperator,
        left: u32,
        right: u32,
    ) -> bool {
        match operator {
            MappedRelationOperator::Equal => left == right,
            MappedRelationOperator::Before => self
                .sort_key(left)
                .zip(self.sort_key(right))
                .is_some_and(|(left_key, right_key)| left_key < right_key),
            MappedRelationOperator::After => self
                .sort_key(left)
                .zip(self.sort_key(right))
                .is_some_and(|(left_key, right_key)| left_key > right_key),
            MappedRelationOperator::NotEqual => left != right,
            MappedRelationOperator::AdjacentBefore => self
                .last_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left_last, right_first)| {
                    left_last.checked_add(1) == Some(right_first)
                }),
            MappedRelationOperator::SameSlots => self.slots_equal(left, right),
            MappedRelationOperator::DifferentSlots => !self.slots_equal(left, right),
            MappedRelationOperator::Overlaps => self.slots_overlap(left, right),
            MappedRelationOperator::Disjoint => !self.slots_overlap(left, right),
            MappedRelationOperator::SlotBefore => self
                .last_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left_last, right_first)| left_last < right_first),
            MappedRelationOperator::SlotAfter => self
                .first_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left_first, right_last)| left_first > right_last),
            MappedRelationOperator::AdjacentAfter => self
                .first_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left_first, right_last)| {
                    right_last.checked_add(1) == Some(left_first)
                }),
            MappedRelationOperator::SameFirstSlot => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left_first, right_first)| left_first == right_first),
            MappedRelationOperator::SameLastSlot => self
                .last_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left_last, right_last)| left_last == right_last),
            MappedRelationOperator::SameBoundary => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .zip(self.last_slot(left).zip(self.last_slot(right)))
                .is_some_and(|((left_first, right_first), (left_last, right_last))| {
                    left_first == right_first && left_last == right_last
                }),
            MappedRelationOperator::NearFirstSlot(distance) => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left_first, right_first)| {
                    slots_within(left_first, right_first, *distance)
                }),
            MappedRelationOperator::NearLastSlot(distance) => {
                self.last_slot(left).zip(self.last_slot(right)).is_some_and(
                    |(left_last, right_last)| slots_within(left_last, right_last, *distance),
                )
            }
            MappedRelationOperator::NearBoundary(distance) => self
                .first_slot(left)
                .zip(self.first_slot(right))
                .zip(self.last_slot(left).zip(self.last_slot(right)))
                .is_some_and(|((left_first, right_first), (left_last, right_last))| {
                    slots_within(left_first, right_first, *distance)
                        && slots_within(left_last, right_last, *distance)
                }),
            // TF `<k:` (nearBefore): the right's first slot lies within k of the
            // slot immediately after the left (last_slot(left) + 1) — a symmetric
            // neighborhood, not a strict ordering.
            MappedRelationOperator::NearBefore(distance) => self
                .last_slot(left)
                .zip(self.first_slot(right))
                .is_some_and(|(left_last, right_first)| {
                    let target = i64::from(left_last) + 1;
                    (i64::from(right_first) - target).abs() <= i64::from(*distance)
                }),
            // TF `:k>` (nearAfter): the right's last slot lies within k of the slot
            // immediately before the left (first_slot(left) - 1).
            MappedRelationOperator::NearAfter(distance) => self
                .first_slot(left)
                .zip(self.last_slot(right))
                .is_some_and(|(left_first, right_last)| {
                    let target = i64::from(left_first) - 1;
                    (i64::from(right_last) - target).abs() <= i64::from(*distance)
                }),
            MappedRelationOperator::Embeds => self.contains_by_slots(left, right),
            MappedRelationOperator::EmbeddedIn => self.contains_by_slots(right, left),
            MappedRelationOperator::EdgeForward(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| has_edge(&edge, left, right)),
            MappedRelationOperator::EdgeBackward(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| has_edge(&edge, right, left)),
            MappedRelationOperator::EdgeEither(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| has_edge(&edge, left, right) || has_edge(&edge, right, left)),
            MappedRelationOperator::EdgeForwardValue(edge_name, matcher) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| self.edge_with_value_holds(&edge, left, right, matcher)),
            MappedRelationOperator::EdgeBackwardValue(edge_name, matcher) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| self.edge_with_value_holds(&edge, right, left, matcher)),
            MappedRelationOperator::EdgeEitherValue(edge_name, matcher) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| {
                    self.edge_with_value_holds(&edge, left, right, matcher)
                        || self.edge_with_value_holds(&edge, right, left, matcher)
                }),
            MappedRelationOperator::FeatureCompare {
                left_feature,
                comparison,
                right_feature,
            } => self
                .mapped_feature_relation_holds(
                    left,
                    right,
                    left_feature,
                    *comparison,
                    right_feature,
                )
                .unwrap_or(false),
            MappedRelationOperator::FeatureRegexCompare {
                left_feature,
                pattern,
                right_feature,
            } => self
                .mapped_feature_regex_relation_holds(
                    left,
                    right,
                    left_feature,
                    pattern,
                    right_feature,
                )
                .unwrap_or(false),
        }
    }

    pub(super) fn mapped_feature_relation_holds(
        &self,
        left: u32,
        right: u32,
        left_feature: &str,
        comparison: MappedFeatureRelationComparison,
        right_feature: &str,
    ) -> Result<bool> {
        let left_feature = self.load_node_feature(left_feature)?;
        let right_feature = self.load_node_feature(right_feature)?;
        let Some(left_value) = left_feature.value(left)? else {
            return Ok(false);
        };
        let Some(right_value) = right_feature.value(right)? else {
            return Ok(false);
        };
        Ok(match comparison {
            MappedFeatureRelationComparison::Eq => mapped_values_equal(left_value, right_value),
            MappedFeatureRelationComparison::Ne => !mapped_values_equal(left_value, right_value),
            MappedFeatureRelationComparison::Lt => mapped_value_compare(left_value, right_value)
                .is_some_and(|ordering| ordering.is_lt()),
            MappedFeatureRelationComparison::Gt => mapped_value_compare(left_value, right_value)
                .is_some_and(|ordering| ordering.is_gt()),
        })
    }

    pub(super) fn mapped_feature_regex_relation_holds(
        &self,
        left: u32,
        right: u32,
        left_feature: &str,
        pattern: &Regex,
        right_feature: &str,
    ) -> Result<bool> {
        let left_feature = self.load_node_feature(left_feature)?;
        let right_feature = self.load_node_feature(right_feature)?;
        let Some(MappedNodeValue::Str(left_value)) = left_feature.value(left)? else {
            return Ok(false);
        };
        let Some(MappedNodeValue::Str(right_value)) = right_feature.value(right)? else {
            return Ok(false);
        };
        Ok(pattern.replace_all(left_value, "") == pattern.replace_all(right_value, ""))
    }

    pub(super) fn sort_key(&self, node: u32) -> Option<u32> {
        self.corpus.sort_key(node).ok().flatten()
    }

    pub(super) fn first_slot(&self, node: u32) -> Option<u32> {
        self.node_slots(node)
            .ok()
            .and_then(|slots| slots.first().copied())
    }

    pub(super) fn last_slot(&self, node: u32) -> Option<u32> {
        self.node_slots(node)
            .ok()
            .and_then(|slots| slots.last().copied())
    }

    pub(super) fn slots_equal(&self, left: u32, right: u32) -> bool {
        let Ok(left_slots) = self.node_slots(left) else {
            return false;
        };
        let Ok(right_slots) = self.node_slots(right) else {
            return false;
        };
        left_slots == right_slots
    }

    pub(super) fn slots_overlap(&self, left: u32, right: u32) -> bool {
        let Ok(left_slots) = self.node_slots(left) else {
            return false;
        };
        let Ok(right_slots) = self.node_slots(right) else {
            return false;
        };
        let mut left_index = 0;
        let mut right_index = 0;
        while left_index < left_slots.len() && right_index < right_slots.len() {
            match left_slots[left_index].cmp(&right_slots[right_index]) {
                std::cmp::Ordering::Less => left_index += 1,
                std::cmp::Ordering::Greater => right_index += 1,
                std::cmp::Ordering::Equal => return true,
            }
        }
        false
    }

    pub(super) fn node_slots(&self, node: u32) -> Result<Vec<u32>> {
        if node == 0 {
            return Ok(Vec::new());
        }
        if node <= self.corpus.max_slot()? {
            return Ok(vec![node]);
        }
        let Some(targets) = self.oslots.targets(node)? else {
            return Ok(Vec::new());
        };
        targets.collect()
    }

    /// Edge `source -> target` exists and its value (None when it has none)
    /// satisfies the relation's value condition.
    pub(super) fn edge_with_value_holds(
        &self,
        edge: &EdgeFeatureView<'_>,
        source: u32,
        target: u32,
        matcher: &MappedMatcher,
    ) -> bool {
        has_edge(edge, source, target)
            && edge
                .edge_value(source, target)
                .is_ok_and(|value| matcher.matches(value))
    }

    pub(super) fn contains_by_slots(&self, parent: u32, child: u32) -> bool {
        // Embedding (TF `[[`): a slot embeds nothing (TF `_l_em`), so only a
        // non-slot `parent` can embed `child`, and then iff every slot of `child`
        // is a slot of `parent`.
        let Ok(Some(parent_targets)) = self.oslots.targets(parent) else {
            return false;
        };
        let Ok(parent_slots) = parent_targets.collect::<Result<Vec<_>>>() else {
            return false;
        };
        let Ok(child_slots) = self.node_slots(child) else {
            return false;
        };
        if child_slots.is_empty() {
            return false;
        }
        child_slots
            .iter()
            .all(|slot| parent_slots.binary_search(slot).is_ok())
    }
}

pub(super) fn slots_within(left: u32, right: u32, distance: u32) -> bool {
    left.abs_diff(right) <= distance
}

/// Whether `edge` has an edge `source -> target`.
pub(super) fn has_edge(edge: &EdgeFeatureView<'_>, source: u32, target: u32) -> bool {
    let Ok(Some(targets)) = edge.targets(source) else {
        return false;
    };
    targets
        .into_iter()
        .any(|candidate| candidate.ok() == Some(target))
}
