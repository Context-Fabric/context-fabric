use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::compiled::{
    EdgeFeatureView, MappedCompiledCorpus, MappedNodeValue, MixedNodeFeatureView,
    StringPoolNodeFeatureView,
};
use crate::error::{CfError, Result};
use crate::mapped_text::MappedText;
use crate::search::{SearchSets, SearchStudy, relations_legend};

pub struct MappedSearch<'a> {
    corpus: &'a MappedCompiledCorpus,
}

impl<'a> MappedSearch<'a> {
    pub fn new(corpus: &'a MappedCompiledCorpus) -> Self {
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

    pub fn glean(&self, nodes: &[u32]) -> Result<String> {
        if nodes.is_empty() {
            return Ok(String::new());
        }
        MappedText::new(self.corpus)?.text_nodes(nodes, None)
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
        Ok(SearchStudy::new(
            template.to_string(),
            self.search_inner(template, sets, None)?,
        ))
    }

    fn search_inner(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        let query = parse_mapped_query(template)?;
        let otype = self
            .corpus
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = self
            .corpus
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;

        match query {
            MappedQuery::Plan(plan) => self.search_plan(&plan, &otype, &oslots, sets, limit),
            MappedQuery::Quantified(quantified) => {
                self.search_quantified(&quantified, &otype, &oslots, sets, limit)
            }
        }
    }

    fn load_constraints<'query>(
        &self,
        atom: &'query SimpleAtom,
    ) -> Result<Vec<(&'query SimpleConstraint, MappedNodeFeatureView<'a>)>> {
        let mut constraints = Vec::new();
        for constraint in &atom.constraints {
            let feature = self.load_node_feature(&constraint.feature)?;
            constraints.push((constraint, feature));
        }
        Ok(constraints)
    }

    fn load_node_feature(&self, feature: &str) -> Result<MappedNodeFeatureView<'a>> {
        if let Some(view) = self.corpus.string_pool_node_feature(feature)? {
            return Ok(MappedNodeFeatureView::StringPool(view));
        }
        if let Some(view) = self.corpus.mixed_node_feature(feature)? {
            return Ok(MappedNodeFeatureView::Mixed(view));
        }
        Err(CfError::MissingFeature(feature.to_string()))
    }

    fn search_atom(
        &self,
        atom: &SimpleAtom,
        otype: &StringPoolNodeFeatureView<'_>,
        constraints: &[(&SimpleConstraint, MappedNodeFeatureView<'_>)],
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        let mut results = Vec::new();
        if let Some(nodes) = custom_set_nodes(atom, sets) {
            let mut nodes = self.filter_candidate_nodes(nodes, constraints)?;
            if let Some(limit) = limit {
                nodes.truncate(limit);
            }
            results.extend(nodes.into_iter().map(|node| vec![node]));
        } else if let Some((constraint, feature)) = constraints
            .iter()
            .find(|(constraint, _)| constraint.can_seed_candidates())
        {
            feature.for_each_row(&mut |node, value| {
                if constraint.matches(Some(value))
                    && self.node_matches(node, atom, otype, constraints, sets)?
                {
                    results.push(vec![node]);
                }
                Ok(!limit.is_some_and(|limit| results.len() >= limit))
            })?;
        } else {
            if limit.is_some() {
                for row in otype.rows() {
                    let (node, node_type) = row?;
                    if (atom.is_generic_node_type() || node_type == atom.node_type)
                        && self.node_matches(node, atom, otype, constraints, sets)?
                    {
                        results.push(vec![node]);
                        if limit.is_some_and(|limit| results.len() >= limit) {
                            break;
                        }
                    }
                }
                return Ok(results);
            }
            let mut nodes = Vec::new();
            for row in otype.rows() {
                let (node, node_type) = row?;
                if atom.is_generic_node_type() || node_type == atom.node_type {
                    nodes.push(node);
                }
            }
            let mut nodes = self.filter_candidate_nodes(nodes, constraints)?;
            if let Some(limit) = limit {
                nodes.truncate(limit);
            }
            results.extend(nodes.into_iter().map(|node| vec![node]));
        }
        Ok(results)
    }

    fn filter_candidate_nodes(
        &self,
        nodes: Vec<u32>,
        constraints: &[(&SimpleConstraint, MappedNodeFeatureView<'_>)],
    ) -> Result<Vec<u32>> {
        let mut candidates = nodes;
        for (constraint, feature) in constraints {
            candidates = feature.filter_candidates(candidates, constraint)?;
        }
        Ok(candidates)
    }

    fn search_plan(
        &self,
        plan: &MappedRelationPlan,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.search_plan_with_root(plan, otype, oslots, sets, limit, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_plan_with_root(
        &self,
        plan: &MappedRelationPlan,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
        root: Option<u32>,
        // Optional restriction of one atom's candidate set (the quantified atom),
        // used by the quantifier engine to intersect the global yarn before the
        // join. The tuple is `(atom_index, allowed_nodes)`.
        root_filter: Option<(usize, &HashSet<u32>)>,
    ) -> Result<Vec<Vec<u32>>> {
        if plan.relations.is_empty() && plan.atoms.len() == 1 && plan.atoms[0].indent == 0 {
            let atom = &plan.atoms[0];
            let constraints = self.load_constraints(atom)?;
            let mut rows = self.search_atom(atom, otype, &constraints, sets, None)?;
            if let Some((_, filter)) = root_filter {
                rows.retain(|row| row.first().is_some_and(|node| filter.contains(node)));
            }
            if let Some(limit) = limit {
                rows.truncate(limit);
            }
            return Ok(rows);
        }
        let mut candidate_row_cache: HashMap<String, Vec<Vec<u32>>> = HashMap::new();
        let candidate_rows = plan
            .atoms
            .iter()
            .map(|atom| {
                let cache_key = atom.candidate_cache_key();
                if let Some(rows) = candidate_row_cache.get(&cache_key) {
                    return Ok(rows.clone());
                }
                let constraints = self.load_constraints(atom)?;
                let rows = self.search_atom(atom, otype, &constraints, sets, None)?;
                candidate_row_cache.insert(cache_key, rows.clone());
                Ok(rows)
            })
            .collect::<Result<Vec<_>>>()?;
        let candidate_nodes = candidate_rows
            .iter()
            .enumerate()
            .map(|(index, rows)| {
                rows.iter()
                    .map(|row| row[0])
                    .filter(|node| match root_filter {
                        Some((filter_index, filter)) if filter_index == index => {
                            filter.contains(node)
                        }
                        _ => true,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let needs_intervals = atoms_needing_intervals(plan);
        let candidates = candidate_nodes
            .into_iter()
            .enumerate()
            .map(|(index, nodes)| {
                self.build_candidate_set(nodes, needs_intervals[index], otype, oslots)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut results = Vec::new();
        let mut slot_cache = HashMap::new();
        self.extend_plan_matches(
            plan,
            &candidates,
            0,
            &mut Vec::new(),
            &mut results,
            limit,
            root,
            otype,
            oslots,
            &mut slot_cache,
        )?;
        Ok(results)
    }

    #[allow(clippy::too_many_arguments)]
    fn extend_plan_matches(
        &self,
        plan: &MappedRelationPlan,
        candidates: &[CandidateSet],
        index: usize,
        current: &mut Vec<u32>,
        results: &mut Vec<Vec<u32>>,
        limit: Option<usize>,
        root: Option<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
    ) -> Result<()> {
        if limit.is_some_and(|limit| results.len() >= limit) {
            return Ok(());
        }
        if index == candidates.len() {
            if self.mapped_relations_hold(plan, current, root, otype, oslots) {
                results.push(current.clone());
            }
            return Ok(());
        }
        for node in self.candidate_nodes_for_position(
            plan, candidates, index, current, otype, oslots, slot_cache,
        )? {
            if self.parent_constraints_hold(
                &plan.atoms,
                index,
                node,
                current,
                otype,
                oslots,
                slot_cache,
            )? {
                current.push(node);
                if self.bound_mapped_relations_hold(plan, current, root, otype, oslots) {
                    self.extend_plan_matches(
                        plan,
                        candidates,
                        index + 1,
                        current,
                        results,
                        limit,
                        root,
                        otype,
                        oslots,
                        slot_cache,
                    )?;
                }
                current.pop();
            }
            if limit.is_some_and(|limit| results.len() >= limit) {
                break;
            }
        }
        Ok(())
    }

    fn parent_constraints_hold(
        &self,
        atoms: &[SimpleAtom],
        index: usize,
        node: u32,
        current: &[u32],
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        _slot_cache: &mut HashMap<u32, Vec<u32>>,
    ) -> Result<bool> {
        let indent = atoms[index].indent;
        if indent == 0 {
            return Ok(true);
        }
        for previous in (0..index).rev() {
            if atoms[previous].indent < indent {
                let Some(parent) = current.get(previous) else {
                    return Ok(false);
                };
                // Indentation always denotes embedding (parent contains child);
                // operator edges are carried separately as plan relations.
                return Ok(self.relation_operator_holds(
                    &MappedRelationOperator::Embeds,
                    *parent,
                    node,
                    otype,
                    oslots,
                ));
            }
        }
        Ok(true)
    }

    fn build_candidate_set(
        &self,
        nodes: Vec<u32>,
        include_slot_intervals: bool,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<CandidateSet> {
        let entries = nodes
            .into_iter()
            .map(|node| {
                let (first_slot, last_slot) = if include_slot_intervals {
                    let slots = self.node_slots(oslots, otype, node)?;
                    (slots.first().copied(), slots.last().copied())
                } else {
                    (None, None)
                };
                Ok(CandidateEntry {
                    node,
                    first_slot,
                    last_slot,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(CandidateSet::new(entries))
    }

    fn candidate_nodes_for_position(
        &self,
        plan: &MappedRelationPlan,
        candidates: &[CandidateSet],
        index: usize,
        current: &[u32],
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
    ) -> Result<Vec<u32>> {
        let Some(parent_index) = nearest_bound_parent_index(&plan.atoms, index) else {
            return Ok(candidates[index].nodes());
        };
        let Some(parent) = current.get(parent_index).copied() else {
            return Ok(Vec::new());
        };
        let parent_slots = self.cached_node_slots(slot_cache, oslots, otype, parent)?;
        let Some(parent_first) = parent_slots.first().copied() else {
            return Ok(Vec::new());
        };
        let Some(parent_last) = parent_slots.last().copied() else {
            return Ok(Vec::new());
        };
        // Indentation is always embedding, so slot-interval pruning always applies.
        // When the atom is also the right operand of an already-bound adjacency /
        // before / k-near sibling relation, narrow the candidate set to the slot
        // window implied by that relation via binary search on first_slot.
        let window = self.first_slot_window(plan, index, current, otype, oslots, slot_cache)?;
        Ok(match window {
            Some((min_first, max_first)) => candidates[index]
                .nodes_within_slot_interval_windowed(
                    parent_first,
                    parent_last,
                    min_first,
                    max_first,
                ),
            None => candidates[index].nodes_within_slot_interval(parent_first, parent_last),
        })
    }

    /// If atom `index` is the right operand of a slot-window relation whose left
    /// operand is already bound, compute the inclusive `[min_first, max_first]`
    /// window for the candidate's first slot. Used purely to prune candidates; the
    /// relation itself is still verified during the join, so a conservative window
    /// is always safe.
    fn first_slot_window(
        &self,
        plan: &MappedRelationPlan,
        index: usize,
        current: &[u32],
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
    ) -> Result<Option<(u32, u32)>> {
        for relation in &plan.relations {
            let MappedRelationEndpoint::Atom(right) = relation.right else {
                continue;
            };
            if right != index {
                continue;
            }
            let MappedRelationEndpoint::Atom(left) = relation.left else {
                continue;
            };
            let Some(&left_node) = current.get(left) else {
                continue;
            };
            let left_slots = self.cached_node_slots(slot_cache, oslots, otype, left_node)?;
            let Some(&left_last) = left_slots.last() else {
                continue;
            };
            let window = match &relation.operator {
                MappedRelationOperator::AdjacentBefore => {
                    Some((left_last.saturating_add(1), left_last.saturating_add(1)))
                }
                MappedRelationOperator::SlotBefore => {
                    Some((left_last.saturating_add(1), u32::MAX))
                }
                MappedRelationOperator::NearBefore(distance) => Some((
                    // first(right) within [last(left)+1-k, last(left)+1+k]
                    left_last.saturating_add(1).saturating_sub(*distance),
                    left_last.saturating_add(1).saturating_add(*distance),
                )),
                _ => None,
            };
            if window.is_some() {
                return Ok(window);
            }
        }
        Ok(None)
    }

    fn mapped_relations_hold(
        &self,
        plan: &MappedRelationPlan,
        current: &[u32],
        root: Option<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> bool {
        plan.relations.iter().all(|relation| {
            self.bound_relation_nodes(current, relation, root)
                .is_some_and(|(left, right)| {
                    self.relation_holds(relation, left, right, otype, oslots)
                })
        })
    }

    fn bound_mapped_relations_hold(
        &self,
        plan: &MappedRelationPlan,
        current: &[u32],
        root: Option<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> bool {
        plan.relations.iter().all(|relation| {
            self.bound_relation_nodes(current, relation, root)
                .is_none_or(|(left, right)| {
                    self.relation_holds(relation, left, right, otype, oslots)
                })
        })
    }

    fn bound_relation_nodes(
        &self,
        current: &[u32],
        relation: &MappedRelation,
        root: Option<u32>,
    ) -> Option<(u32, u32)> {
        let left = mapped_relation_endpoint_node(current, &relation.left, root)?;
        let right = mapped_relation_endpoint_node(current, &relation.right, root)?;
        Some((left, right))
    }

    fn search_quantified(
        &self,
        quantified: &MappedQuantifiedQuery,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        // Universe = candidate nodes of the quantified (root) atom, matching its
        // own type and feature constraints. Each quantifier block reduces this
        // set via global set algebra (TF spin.py:_doQuantifier); the reduced
        // "yarn" is then intersected into the root atom's candidates before the
        // base join.
        let root_atom = &quantified.base.atoms[quantified.root_atom_index];
        let mut yarn: HashSet<u32> = {
            let constraints = self.load_constraints(root_atom)?;
            self.search_atom(root_atom, otype, &constraints, sets, None)?
                .into_iter()
                .filter_map(|row| row.first().copied())
                .collect()
        };

        for block in &quantified.blocks {
            yarn = self.apply_quantifier_block(
                &quantified.clean_atom,
                block,
                yarn,
                otype,
                oslots,
                sets,
            )?;
            if yarn.is_empty() {
                break;
            }
        }

        self.search_plan_with_root(
            &quantified.base,
            otype,
            oslots,
            sets,
            limit,
            None,
            Some((quantified.root_atom_index, &yarn)),
        )
    }

    fn apply_quantifier_block(
        &self,
        clean_atom: &str,
        block: &TextQuantifierBlock,
        yarn: HashSet<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<HashSet<u32>> {
        match block.kind {
            MappedQuantifierKind::Without => {
                let Some(alternative) = block.alternatives.first() else {
                    return Ok(yarn);
                };
                let template = combine_quantifier_subtemplate(clean_atom, alternative);
                let exclude = self.quantifier_subsearch_roots(&template, otype, oslots, sets)?;
                Ok(yarn.difference(&exclude).copied().collect())
            }
            MappedQuantifierKind::With => {
                let mut result = HashSet::new();
                for alternative in &block.alternatives {
                    let template = combine_quantifier_subtemplate(clean_atom, alternative);
                    let roots = self.quantifier_subsearch_roots(&template, otype, oslots, sets)?;
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
                let antecedent_rows =
                    self.run_template(&antecedent_template, otype, oslots, sets, None)?;
                if antecedent_rows.is_empty() {
                    return Ok(yarn);
                }
                let size_a = antecedent_rows[0].len();
                let mut combined = antecedent.clone();
                for consequent in &block.consequents {
                    combined = combine_quantifier_templates(&combined, consequent);
                }
                let combined_template = combine_quantifier_subtemplate(clean_atom, &combined);
                let combined_rows =
                    self.run_template(&combined_template, otype, oslots, sets, None)?;
                let satisfied: HashSet<Vec<u32>> = combined_rows
                    .into_iter()
                    .map(|row| row.into_iter().take(size_a).collect())
                    .collect();
                // Roots whose antecedent match lacks a matching consequent are
                // disqualified (the quantifier is universal, not existential).
                let mut disqualified = HashSet::new();
                for row in antecedent_rows {
                    let prefix: Vec<u32> = row.iter().take(size_a).copied().collect();
                    if !satisfied.contains(&prefix) {
                        if let Some(root) = row.first() {
                            disqualified.insert(*root);
                        }
                    }
                }
                Ok(yarn.difference(&disqualified).copied().collect())
            }
        }
    }

    fn run_template(
        &self,
        template: &str,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        match parse_mapped_query(template)? {
            MappedQuery::Plan(plan) => self.search_plan(&plan, otype, oslots, sets, limit),
            MappedQuery::Quantified(quantified) => {
                self.search_quantified(&quantified, otype, oslots, sets, limit)
            }
        }
    }

    fn quantifier_subsearch_roots(
        &self,
        template: &str,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<HashSet<u32>> {
        Ok(self
            .run_template(template, otype, oslots, sets, None)?
            .into_iter()
            .filter_map(|row| row.first().copied())
            .collect())
    }

    fn node_matches(
        &self,
        node: u32,
        atom: &SimpleAtom,
        otype: &StringPoolNodeFeatureView<'_>,
        constraints: &[(&SimpleConstraint, MappedNodeFeatureView<'_>)],
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        if let Some(nodes) = custom_set_nodes(atom, sets) {
            if nodes.binary_search(&node).is_err() {
                return Ok(false);
            }
        } else if !atom.is_generic_node_type()
            && !atom.is_parent_reference()
            && otype.str_value(node)? != Some(atom.node_type.as_str())
        {
            return Ok(false);
        }
        for (constraint, feature) in constraints {
            if !constraint.matches(feature.value(node)?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn relation_holds(
        &self,
        relation: &MappedRelation,
        left: u32,
        right: u32,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> bool {
        self.relation_operator_holds(&relation.operator, left, right, otype, oslots)
    }

    fn relation_operator_holds(
        &self,
        operator: &MappedRelationOperator,
        left: u32,
        right: u32,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
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
                .last_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .is_some_and(|(left_last, right_first)| {
                    left_last.checked_add(1) == Some(right_first)
                }),
            MappedRelationOperator::SameSlots => self.slots_equal(oslots, otype, left, right),
            MappedRelationOperator::DifferentSlots => !self.slots_equal(oslots, otype, left, right),
            MappedRelationOperator::Overlaps => self.slots_overlap(oslots, otype, left, right),
            MappedRelationOperator::Disjoint => !self.slots_overlap(oslots, otype, left, right),
            MappedRelationOperator::SlotBefore => self
                .last_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .is_some_and(|(left_last, right_first)| left_last < right_first),
            MappedRelationOperator::SlotAfter => self
                .first_slot(oslots, otype, left)
                .zip(self.last_slot(oslots, otype, right))
                .is_some_and(|(left_first, right_last)| left_first > right_last),
            MappedRelationOperator::AdjacentAfter => self
                .first_slot(oslots, otype, left)
                .zip(self.last_slot(oslots, otype, right))
                .is_some_and(|(left_first, right_last)| {
                    right_last.checked_add(1) == Some(left_first)
                }),
            MappedRelationOperator::SameFirstSlot => self
                .first_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .is_some_and(|(left_first, right_first)| left_first == right_first),
            MappedRelationOperator::SameLastSlot => self
                .last_slot(oslots, otype, left)
                .zip(self.last_slot(oslots, otype, right))
                .is_some_and(|(left_last, right_last)| left_last == right_last),
            MappedRelationOperator::SameBoundary => self
                .first_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .zip(
                    self.last_slot(oslots, otype, left)
                        .zip(self.last_slot(oslots, otype, right)),
                )
                .is_some_and(|((left_first, right_first), (left_last, right_last))| {
                    left_first == right_first && left_last == right_last
                }),
            MappedRelationOperator::NearFirstSlot(distance) => self
                .first_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .is_some_and(|(left_first, right_first)| {
                    slots_within(left_first, right_first, *distance)
                }),
            MappedRelationOperator::NearLastSlot(distance) => self
                .last_slot(oslots, otype, left)
                .zip(self.last_slot(oslots, otype, right))
                .is_some_and(|(left_last, right_last)| {
                    slots_within(left_last, right_last, *distance)
                }),
            MappedRelationOperator::NearBoundary(distance) => self
                .first_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .zip(
                    self.last_slot(oslots, otype, left)
                        .zip(self.last_slot(oslots, otype, right)),
                )
                .is_some_and(|((left_first, right_first), (left_last, right_last))| {
                    slots_within(left_first, right_first, *distance)
                        && slots_within(left_last, right_last, *distance)
                }),
            // TF `<k:` (nearBefore): the right's first slot lies within k of the
            // slot immediately after the left (last_slot(left) + 1) — a symmetric
            // neighborhood, not a strict ordering.
            MappedRelationOperator::NearBefore(distance) => self
                .last_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .is_some_and(|(left_last, right_first)| {
                    let target = i64::from(left_last) + 1;
                    (i64::from(right_first) - target).abs() <= i64::from(*distance)
                }),
            // TF `:k>` (nearAfter): the right's last slot lies within k of the slot
            // immediately before the left (first_slot(left) - 1).
            MappedRelationOperator::NearAfter(distance) => self
                .first_slot(oslots, otype, left)
                .zip(self.last_slot(oslots, otype, right))
                .is_some_and(|(left_first, right_last)| {
                    let target = i64::from(left_first) - 1;
                    (i64::from(right_last) - target).abs() <= i64::from(*distance)
                }),
            MappedRelationOperator::Embeds => self.contains_by_slots(oslots, otype, left, right),
            MappedRelationOperator::EmbeddedIn => {
                self.contains_by_slots(oslots, otype, right, left)
            }
            MappedRelationOperator::EdgeForward(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| self.contains(&edge, left, right)),
            MappedRelationOperator::EdgeBackward(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| self.contains(&edge, right, left)),
            MappedRelationOperator::EdgeEither(edge_name) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| {
                    self.contains(&edge, left, right) || self.contains(&edge, right, left)
                }),
            MappedRelationOperator::EdgeForwardValue(edge_name, matcher) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| {
                    self.contains(&edge, left, right)
                        && edge
                            .edge_value(left, right)
                            .ok()
                            .flatten()
                            .is_some_and(|actual| mapped_edge_value_matches(actual, matcher))
                }),
            MappedRelationOperator::EdgeBackwardValue(edge_name, matcher) => self
                .corpus
                .edge_feature(edge_name)
                .ok()
                .flatten()
                .is_some_and(|edge| {
                    self.contains(&edge, right, left)
                        && edge
                            .edge_value(right, left)
                            .ok()
                            .flatten()
                            .is_some_and(|actual| mapped_edge_value_matches(actual, matcher))
                }),
            MappedRelationOperator::EdgeEitherValue(edge_name, matcher) => {
                self.corpus
                    .edge_feature(edge_name)
                    .ok()
                    .flatten()
                    .is_some_and(|edge| {
                        (self.contains(&edge, left, right)
                            && edge
                                .edge_value(left, right)
                                .ok()
                                .flatten()
                                .is_some_and(|actual| mapped_edge_value_matches(actual, matcher)))
                            || (self.contains(&edge, right, left)
                                && edge.edge_value(right, left).ok().flatten().is_some_and(
                                    |actual| mapped_edge_value_matches(actual, matcher),
                                ))
                    })
            }
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

    fn mapped_feature_relation_holds(
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

    fn mapped_feature_regex_relation_holds(
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

    fn sort_key(&self, node: u32) -> Option<u32> {
        self.corpus.sort_key(node).ok().flatten()
    }

    fn first_slot(
        &self,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        node: u32,
    ) -> Option<u32> {
        self.node_slots(oslots, otype, node)
            .ok()
            .and_then(|slots| slots.first().copied())
    }

    fn last_slot(
        &self,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        node: u32,
    ) -> Option<u32> {
        self.node_slots(oslots, otype, node)
            .ok()
            .and_then(|slots| slots.last().copied())
    }

    fn slots_equal(
        &self,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        left: u32,
        right: u32,
    ) -> bool {
        let Ok(left_slots) = self.node_slots(oslots, otype, left) else {
            return false;
        };
        let Ok(right_slots) = self.node_slots(oslots, otype, right) else {
            return false;
        };
        left_slots == right_slots
    }

    fn slots_overlap(
        &self,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        left: u32,
        right: u32,
    ) -> bool {
        let Ok(left_slots) = self.node_slots(oslots, otype, left) else {
            return false;
        };
        let Ok(right_slots) = self.node_slots(oslots, otype, right) else {
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

    fn node_slots(
        &self,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        node: u32,
    ) -> Result<Vec<u32>> {
        let slot_type = otype
            .str_value(1)?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        if otype.str_value(node)? == Some(slot_type) {
            return Ok(vec![node]);
        }
        let Some(targets) = oslots.targets(node)? else {
            return Ok(Vec::new());
        };
        targets.collect()
    }

    fn contains(&self, oslots: &EdgeFeatureView<'_>, parent: u32, child: u32) -> bool {
        let Ok(Some(targets)) = oslots.targets(parent) else {
            return false;
        };
        for target in targets {
            if target.ok() == Some(child) {
                return true;
            }
        }
        false
    }

    fn contains_by_slots(
        &self,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        parent: u32,
        child: u32,
    ) -> bool {
        // Embedding (TF `[[`): a slot embeds nothing (TF `_l_em`), so only a
        // non-slot `parent` can embed `child`, and then iff every slot of `child`
        // is a slot of `parent`.
        let Ok(Some(parent_targets)) = oslots.targets(parent) else {
            return false;
        };
        let Ok(parent_slots) = parent_targets.collect::<Result<Vec<_>>>() else {
            return false;
        };
        let Ok(child_slots) = self.node_slots(oslots, otype, child) else {
            return false;
        };
        if child_slots.is_empty() {
            return false;
        }
        child_slots
            .iter()
            .all(|slot| parent_slots.binary_search(slot).is_ok())
    }

    fn cached_node_slots(
        &self,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        node: u32,
    ) -> Result<Vec<u32>> {
        if let Some(slots) = slot_cache.get(&node) {
            return Ok(slots.clone());
        }
        let slots = self.node_slots(oslots, otype, node)?;
        slot_cache.insert(node, slots.clone());
        Ok(slots)
    }
}

enum MappedNodeFeatureView<'a> {
    StringPool(StringPoolNodeFeatureView<'a>),
    Mixed(MixedNodeFeatureView<'a>),
}

impl<'a> MappedNodeFeatureView<'a> {
    fn value(&self, node: u32) -> Result<Option<MappedNodeValue<'a>>> {
        match self {
            Self::StringPool(feature) => feature
                .str_value(node)
                .map(|value| value.map(MappedNodeValue::Str)),
            Self::Mixed(feature) => feature.value(node),
        }
    }

    fn for_each_row(
        &self,
        visit: &mut impl FnMut(u32, MappedNodeValue<'a>) -> Result<bool>,
    ) -> Result<()> {
        match self {
            Self::StringPool(feature) => {
                for row in feature.rows() {
                    let (node, value) = row?;
                    if !visit(node, MappedNodeValue::Str(value))? {
                        break;
                    }
                }
            }
            Self::Mixed(feature) => {
                for row in feature.rows() {
                    let (node, value) = row?;
                    if !visit(node, value)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn filter_candidates(
        &self,
        nodes: Vec<u32>,
        constraint: &SimpleConstraint,
    ) -> Result<Vec<u32>> {
        let mut candidates = Vec::with_capacity(nodes.len());
        for node in nodes {
            if constraint.matches(self.value(node)?) {
                candidates.push(node);
            }
        }
        Ok(candidates)
    }
}

enum MappedQuery {
    Plan(MappedRelationPlan),
    Quantified(MappedQuantifiedQuery),
}

struct CandidateSet {
    entries: Vec<CandidateEntry>,
    by_first_slot: Vec<usize>,
}

impl CandidateSet {
    fn new(entries: Vec<CandidateEntry>) -> Self {
        let mut by_first_slot = (0..entries.len()).collect::<Vec<_>>();
        by_first_slot.sort_by_key(|index| {
            (
                entries[*index].first_slot.unwrap_or(u32::MAX),
                entries[*index].last_slot.unwrap_or(u32::MAX),
                entries[*index].node,
            )
        });
        Self {
            entries,
            by_first_slot,
        }
    }

    fn nodes(&self) -> Vec<u32> {
        self.entries.iter().map(|entry| entry.node).collect()
    }

    fn nodes_within_slot_interval(&self, first_slot: u32, last_slot: u32) -> Vec<u32> {
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|candidate_first| candidate_first <= last_slot)
        });
        self.by_first_slot[..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index];
                let candidate_first = entry.first_slot?;
                let candidate_last = entry.last_slot?;
                (candidate_first >= first_slot && candidate_last <= last_slot).then_some(entry.node)
            })
            .collect()
    }

    /// Like `nodes_within_slot_interval`, but additionally restricts the
    /// candidate's first slot to `[min_first, max_first]` (inclusive) using the
    /// `by_first_slot` index.
    fn nodes_within_slot_interval_windowed(
        &self,
        parent_first: u32,
        parent_last: u32,
        min_first: u32,
        max_first: u32,
    ) -> Vec<u32> {
        let lo_first = parent_first.max(min_first);
        let hi_first = parent_last.min(max_first);
        if lo_first > hi_first {
            return Vec::new();
        }
        let start = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|candidate_first| candidate_first < lo_first)
        });
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|candidate_first| candidate_first <= hi_first)
        });
        self.by_first_slot[start..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index];
                let candidate_last = entry.last_slot?;
                (candidate_last <= parent_last).then_some(entry.node)
            })
            .collect()
    }
}

struct CandidateEntry {
    node: u32,
    first_slot: Option<u32>,
    last_slot: Option<u32>,
}

#[derive(Clone)]
struct MappedQuantifiedQuery {
    base: MappedRelationPlan,
    /// Index into `base.atoms` of the quantified atom (TF: the atom immediately
    /// preceding the quantifier, i.e. the last base atom).
    root_atom_index: usize,
    /// The quantified atom as a self-contained, named atom line; quantifier
    /// sub-templates are prepended with this line so `..` parent references resolve
    /// to a concrete name in the global sub-searches.
    clean_atom: String,
    blocks: Vec<TextQuantifierBlock>,
}

#[derive(Clone)]
struct TextQuantifierBlock {
    kind: MappedQuantifierKind,
    /// For `/with/`/`/without/`: the alternative templates. For `/where/`: the
    /// single antecedent template (index 0).
    alternatives: Vec<String>,
    /// For `/where/`: the `/have/` consequent template(s).
    consequents: Vec<String>,
}

#[derive(Clone, Copy)]
enum MappedQuantifierKind {
    With,
    Without,
    Where,
}

#[derive(Clone)]
struct MappedRelationPlan {
    atoms: Vec<SimpleAtom>,
    relations: Vec<MappedRelation>,
}

#[derive(Clone)]
struct MappedRelation {
    left: MappedRelationEndpoint,
    operator: MappedRelationOperator,
    right: MappedRelationEndpoint,
}

#[derive(Clone)]
enum MappedRelationEndpoint {
    Atom(usize),
    Parent,
}

struct PendingMappedRelation {
    left: String,
    operator: MappedRelationOperator,
    right: String,
}

#[derive(Clone)]
enum MappedRelationOperator {
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
    EdgeForwardValue(String, MappedEdgeValueMatcher),
    EdgeBackwardValue(String, MappedEdgeValueMatcher),
    EdgeEitherValue(String, MappedEdgeValueMatcher),
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

#[derive(Debug, Clone)]
enum MappedEdgeValueMatcher {
    Eq(Vec<String>),
    Ne(Vec<String>),
    Regex(Regex),
}

#[derive(Clone, Copy)]
enum MappedFeatureRelationComparison {
    Eq,
    Ne,
    Lt,
    Gt,
}

#[derive(Clone)]
struct SimpleAtom {
    indent: usize,
    name: Option<String>,
    node_type: String,
    constraints: Vec<SimpleConstraint>,
}

impl SimpleAtom {
    fn is_generic_node_type(&self) -> bool {
        self.node_type == "."
    }

    fn is_parent_reference(&self) -> bool {
        self.node_type == ".."
    }

    fn candidate_cache_key(&self) -> String {
        let mut constraints = self
            .constraints
            .iter()
            .map(SimpleConstraint::cache_key)
            .collect::<Vec<_>>();
        constraints.sort_unstable();
        format!("{} {}", self.node_type, constraints.join(" "))
    }
}

fn custom_set_nodes(atom: &SimpleAtom, sets: Option<&SearchSets<'_>>) -> Option<Vec<u32>> {
    let mut nodes = sets?.get(atom.node_type.as_str())?.clone();
    nodes.sort_unstable();
    nodes.dedup();
    Some(nodes)
}

#[derive(Clone)]
struct SimpleConstraint {
    feature: String,
    matcher: MappedMatcher,
}

impl SimpleConstraint {
    fn cache_key(&self) -> String {
        format!("{}{}", self.feature, self.matcher.cache_key())
    }

    fn can_seed_candidates(&self) -> bool {
        !matches!(&self.matcher, MappedMatcher::Ne(_) | MappedMatcher::Missing)
    }

    fn matches(&self, value: Option<MappedNodeValue<'_>>) -> bool {
        match &self.matcher {
            MappedMatcher::Eq(expected) => value.is_some_and(|value| {
                expected
                    .iter()
                    .any(|expected| value_matches(value, expected))
            }),
            MappedMatcher::Ne(expected) => value.is_none_or(|value| {
                !expected
                    .iter()
                    .any(|expected| value_matches(value, expected))
            }),
            MappedMatcher::Regex(regex) => value
                .and_then(mapped_value_as_str)
                .is_some_and(|raw| regex.is_match(raw)),
            MappedMatcher::Exists => value.is_some(),
            MappedMatcher::Missing => value.is_none(),
            MappedMatcher::Lt(expected) => value
                .and_then(|value| compare_value(value, expected))
                .is_some_and(|ordering| ordering.is_lt()),
            MappedMatcher::Gt(expected) => value
                .and_then(|value| compare_value(value, expected))
                .is_some_and(|ordering| ordering.is_gt()),
        }
    }
}

#[derive(Clone)]
enum MappedMatcher {
    Eq(Vec<String>),
    Ne(Vec<String>),
    Regex(Regex),
    Exists,
    Missing,
    Lt(String),
    Gt(String),
}

impl MappedMatcher {
    fn cache_key(&self) -> String {
        match self {
            Self::Eq(values) => format!("={}", values.join("|")),
            Self::Ne(values) => format!("#{}", values.join("|")),
            Self::Regex(regex) => format!("~{}", regex.as_str()),
            Self::Exists => "*".to_string(),
            Self::Missing => "#".to_string(),
            Self::Lt(value) => format!("<{value}"),
            Self::Gt(value) => format!(">{value}"),
        }
    }
}


fn parse_mapped_query(template: &str) -> Result<MappedQuery> {
    if template.lines().any(|line| {
        let token = line.trim();
        matches!(
            token,
            "/where/" | "/have/" | "/with/" | "/without/" | "/or/"
        )
    }) {
        return parse_mapped_quantified_query(template).map(MappedQuery::Quantified);
    }

    parse_mapped_plan(template).map(MappedQuery::Plan)
}

fn parse_mapped_plan(template: &str) -> Result<MappedRelationPlan> {
    let mut atoms: Vec<SimpleAtom> = Vec::new();
    let mut names: HashMap<String, usize> = HashMap::new();
    let mut pending_relations = Vec::new();
    // Op edges discovered structurally (op-prefixed atoms, lonely operators) are
    // emitted directly with resolved atom indices.
    let mut direct_relations: Vec<MappedRelation> = Vec::new();
    // TF `atomStack`: indent -> last atom index seen at that indent in the current
    // nesting. Used to determine the operand of operator edges and to detect
    // illegal "lonely" relations.
    let mut atom_stack: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();

    for line in template
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('%'))
    {
        let trimmed = line.trim();
        let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();
        let parts = split_query_whitespace(trimmed);

        // 1. Explicit relation line: `left OP right`.
        if parts.len() == 3 {
            if let Some(operator) = parse_mapped_relation_operator(&parts[1]) {
                pending_relations.push(PendingMappedRelation {
                    left: parts[0].clone(),
                    operator,
                    right: parts[2].clone(),
                });
                continue;
            }
        }

        // 2. Lines whose first token is a relation operator: either a lonely
        //    operator (single token) or an operator-prefixed atom. This is
        //    detected BEFORE the feature-continuation branch so that lines such as
        //    `:= word` are parsed as atoms rather than mis-read as constraints.
        if let Some(first) = parts.first() {
            if let Some(operator) = parse_mapped_atom_operator(first) {
                if parts.len() == 1 {
                    handle_lonely_operator(
                        indent,
                        operator,
                        &atom_stack,
                        &mut direct_relations,
                    )?;
                    continue;
                }
                let atom = parse_simple_atom(line, indent, true)?;
                let q = atoms.len();
                if let Some(name) = &atom.name {
                    names.insert(name.clone(), q);
                }
                atoms.push(atom);
                register_atom(indent, Some(operator), q, &mut atom_stack, &mut direct_relations)?;
                continue;
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
                    });
                    pending_relations.push(PendingMappedRelation {
                        left: format!("\0ref{reference_index}"),
                        operator: MappedRelationOperator::Equal,
                        right: first.clone(),
                    });
                    names.insert(format!("\0ref{reference_index}"), reference_index);
                    register_atom(indent, None, reference_index, &mut atom_stack, &mut direct_relations)?;
                    continue;
                }
            }
        }

        // 4. Feature-continuation line (every token is an explicit constraint).
        if !atoms.is_empty() {
            if let Some(constraints) = parse_mapped_feature_continuation_line(trimmed)? {
                let last_index = atoms.len() - 1;
                atoms[last_index].constraints.extend(constraints);
                continue;
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
fn register_atom(
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
fn handle_lonely_operator(
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

fn mapped_relation_endpoint(
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

/// Compute, per atom, whether its candidate set needs precomputed slot intervals.
/// Indented atoms always do (for embedding pruning); additionally, the right
/// operand of a first-slot windowing relation needs them for the binary-search
/// narrowing in `first_slot_window`.
fn atoms_needing_intervals(plan: &MappedRelationPlan) -> Vec<bool> {
    let mut needs: Vec<bool> = plan.atoms.iter().map(|atom| atom.indent > 0).collect();
    for relation in &plan.relations {
        if !matches!(
            relation.operator,
            MappedRelationOperator::AdjacentBefore
                | MappedRelationOperator::SlotBefore
                | MappedRelationOperator::NearBefore(_)
        ) {
            continue;
        }
        if let MappedRelationEndpoint::Atom(right) = relation.right {
            if right < needs.len() {
                needs[right] = true;
            }
        }
    }
    needs
}

fn nearest_bound_parent_index(atoms: &[SimpleAtom], index: usize) -> Option<usize> {
    let indent = atoms[index].indent;
    if indent == 0 {
        return None;
    }
    (0..index)
        .rev()
        .find(|previous| atoms[*previous].indent < indent)
}

fn mapped_relation_endpoint_node(
    current: &[u32],
    endpoint: &MappedRelationEndpoint,
    root: Option<u32>,
) -> Option<u32> {
    match endpoint {
        MappedRelationEndpoint::Atom(index) => current.get(*index).copied(),
        MappedRelationEndpoint::Parent => root,
    }
}

fn parse_mapped_quantified_query(template: &str) -> Result<MappedQuantifiedQuery> {
    let parsed = parse_quantified_template(template)?;
    let base = parse_mapped_plan(&parsed.base)?;
    if base.atoms.is_empty() {
        return Err(CfError::InvalidQuery(
            "mapped quantified search requires at least one base atom".to_string(),
        ));
    }
    let (clean_atom, parent_name) = clean_quantified_atom(&parsed.base)?;
    let mut blocks = Vec::new();
    for block in parsed.blocks {
        let alternatives = block
            .alternatives
            .iter()
            .map(|alternative| substitute_parent_ref(alternative, &parent_name))
            .collect::<Vec<_>>();
        let consequents = block
            .consequents
            .iter()
            .map(|consequent| substitute_parent_ref(consequent, &parent_name))
            .collect::<Vec<_>>();
        blocks.push(TextQuantifierBlock {
            kind: block.kind,
            alternatives,
            consequents,
        });
    }
    Ok(MappedQuantifiedQuery {
        root_atom_index: base.atoms.len() - 1,
        base,
        clean_atom,
        blocks,
    })
}

/// Derive the quantified atom line from the base template (TF: the atom
/// immediately preceding the quantifier, i.e. the last base atom), ensuring it has
/// a name so `..` parent references can be rewritten to it. Returns the rewritten
/// atom line and the parent name.
fn clean_quantified_atom(base: &str) -> Result<(String, String)> {
    let mut last_atom: Option<(String, String)> = None;
    for line in base.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }
        let parts = split_query_whitespace(trimmed);
        // Skip relation lines and operator-led lines; they are not atom heads.
        if parts.len() == 3 && parse_mapped_relation_operator(&parts[1]).is_some() {
            continue;
        }
        let Some(first) = parts.first() else {
            continue;
        };
        if parse_mapped_atom_operator(first).is_some() {
            continue;
        }
        last_atom = Some(if let Some((name, _)) = first.split_once(':') {
            (trimmed.to_string(), name.to_string())
        } else {
            let parent_name = "__cf_parent__".to_string();
            let mut rewritten = parts.clone();
            rewritten[0] = format!("{parent_name}:{first}");
            (rewritten.join(" "), parent_name)
        });
    }
    last_atom.ok_or_else(|| {
        CfError::InvalidQuery(
            "mapped quantified search requires at least one base atom".to_string(),
        )
    })
}

/// Replace standalone `..` parent-reference tokens with `parent_name`, but only at
/// the current quantifier depth (tokens inside nested quantifier blocks are left
/// untouched so the recursive parse can resolve them against the inner parent).
fn substitute_parent_ref(template: &str, parent_name: &str) -> String {
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

/// Combine the clean root-atom line with a quantifier sub-template by indenting the
/// sub-template two spaces so its atoms become children of the root atom.
fn combine_quantifier_subtemplate(clean_atom: &str, sub: &str) -> String {
    let mut lines = vec![clean_atom.to_string()];
    for line in sub.lines() {
        if line.trim().is_empty() {
            continue;
        }
        lines.push(format!("  {line}"));
    }
    lines.join("\n")
}

struct ParsedQuantifiedTemplate {
    base: String,
    blocks: Vec<ParsedQuantifierBlock>,
}

struct ParsedQuantifierBlock {
    kind: MappedQuantifierKind,
    alternatives: Vec<String>,
    consequents: Vec<String>,
}

fn parse_quantified_template(template: &str) -> Result<ParsedQuantifiedTemplate> {
    let mut base_lines = Vec::new();
    let mut blocks = Vec::new();
    let mut current_kind: Option<MappedQuantifierKind> = None;
    let mut current_lines = Vec::new();
    let mut current_alternatives = Vec::new();
    let mut current_consequents = Vec::new();
    let mut collecting_consequents = false;
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
                    "/where/" => MappedQuantifierKind::Where,
                    "/without/" => MappedQuantifierKind::Without,
                    _ => MappedQuantifierKind::With,
                });
                collecting_consequents = false;
            }
            "/have/" => {
                if nested_depth > 0 {
                    current_lines.push(line.to_string());
                    continue;
                }
                let Some(kind) = current_kind else {
                    return Err(CfError::InvalidQuery(
                        "mapped quantifier continuation without quantifier".to_string(),
                    ));
                };
                if !matches!(
                    kind,
                    MappedQuantifierKind::With | MappedQuantifierKind::Where
                ) {
                    return Err(CfError::InvalidQuery(
                        "/have/ is only supported after /where/ or /with/".to_string(),
                    ));
                }
                if matches!(kind, MappedQuantifierKind::Where) {
                    push_quantifier_alternative(&mut current_alternatives, &current_lines);
                    current_lines.clear();
                    collecting_consequents = true;
                } else {
                    push_quantifier_alternative(&mut current_alternatives, &current_lines);
                    blocks.push(ParsedQuantifierBlock {
                        kind,
                        alternatives: std::mem::take(&mut current_alternatives),
                        consequents: Vec::new(),
                    });
                    current_lines.clear();
                    current_kind = Some(MappedQuantifierKind::With);
                }
            }
            "/or/" => {
                if nested_depth > 0 {
                    current_lines.push(line.to_string());
                    continue;
                }
                if current_kind.is_none() {
                    return Err(CfError::InvalidQuery(
                        "mapped quantifier alternative without quantifier".to_string(),
                    ));
                }
                if collecting_consequents {
                    push_quantifier_alternative(&mut current_consequents, &current_lines);
                } else {
                    push_quantifier_alternative(&mut current_alternatives, &current_lines);
                }
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
                        "mapped quantifier terminator without quantifier".to_string(),
                    ));
                };
                if collecting_consequents {
                    push_quantifier_alternative(&mut current_consequents, &current_lines);
                } else {
                    push_quantifier_alternative(&mut current_alternatives, &current_lines);
                }
                if matches!(kind, MappedQuantifierKind::Where)
                    && (current_alternatives.is_empty() || current_consequents.is_empty())
                {
                    return Err(CfError::InvalidQuery(
                        "/where/ requires antecedent and /have/ consequent templates".to_string(),
                    ));
                }
                blocks.push(ParsedQuantifierBlock {
                    kind,
                    alternatives: std::mem::take(&mut current_alternatives),
                    consequents: std::mem::take(&mut current_consequents),
                });
                current_lines.clear();
                collecting_consequents = false;
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
            "unterminated mapped quantified block".to_string(),
        ));
    }
    Ok(ParsedQuantifiedTemplate {
        base: base_lines.join("\n"),
        blocks,
    })
}

fn push_quantifier_alternative(alternatives: &mut Vec<String>, lines: &[String]) {
    if lines.iter().any(|line| !line.trim().is_empty()) {
        alternatives.push(normalize_parent_reference_relations(
            &normalize_indentation(lines),
        ));
    }
}

fn combine_quantifier_templates(first: &str, second: &str) -> String {
    match (first.trim().is_empty(), second.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => second.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first}\n{second}"),
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

fn parse_simple_atom(line: &str, indent: usize, skip_operator: bool) -> Result<SimpleAtom> {
    let parts = split_query_whitespace(line);
    let mut parts = parts.into_iter();
    if skip_operator {
        // discard the leading relation-operator token
        let Some(operator) = parts.next() else {
            return Err(CfError::InvalidQuery("empty mapped query".to_string()));
        };
        if parts.len() == 0 {
            return Err(CfError::InvalidQuery(format!(
                "missing mapped atom after operator {operator:?}"
            )));
        }
    }
    let Some(first) = parts.next() else {
        return Err(CfError::InvalidQuery("empty mapped query".to_string()));
    };
    let node_type = first
        .split_once(':')
        .map(|(_, node_type)| node_type)
        .unwrap_or(&first)
        .to_string();
    let name = first.split_once(':').map(|(name, _)| name.to_string());
    let mut constraints = Vec::new();
    for part in parts {
        constraints.push(parse_simple_constraint(&part)?);
    }
    Ok(SimpleAtom {
        indent,
        name,
        node_type,
        constraints,
    })
}

fn parse_mapped_atom_operator(raw: &str) -> Option<MappedRelationOperator> {
    parse_mapped_relation_operator(raw)
}

fn parse_mapped_feature_continuation_line(line: &str) -> Result<Option<Vec<SimpleConstraint>>> {
    let mut constraints = Vec::new();
    for part in split_query_whitespace(line) {
        if !looks_like_explicit_mapped_feature_constraint(&part) {
            return Ok(None);
        }
        let constraint = parse_simple_constraint(&part)?;
        constraints.push(constraint);
    }
    if constraints.is_empty() {
        Ok(None)
    } else {
        Ok(Some(constraints))
    }
}

fn looks_like_explicit_mapped_feature_constraint(part: &str) -> bool {
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

fn parse_simple_constraint(part: &str) -> Result<SimpleConstraint> {
    // Mirror TF (tf/search/syntax.py:parseFeatureVals): the feature name is the
    // leading run of name characters and the FIRST unescaped operator that follows
    // determines the comparison. A naive `split_once('=')` would mis-handle forms
    // such as `vbe#H=` (operator `#`, value `H=`) by splitting on `=` first.
    match find_first_constraint_operator(part) {
        Some((index, operator)) => {
            let feature = part[..index].to_string();
            let value = &part[index + operator.len_utf8()..];
            let matcher = match operator {
                '=' => MappedMatcher::Eq(parse_alternative_strings(value)?),
                '#' => {
                    if value.is_empty() {
                        MappedMatcher::Missing
                    } else {
                        MappedMatcher::Ne(parse_alternative_strings(value)?)
                    }
                }
                '~' => MappedMatcher::Regex(Regex::new(value).map_err(|source| {
                    CfError::InvalidQuery(format!("invalid mapped regex {value:?}: {source}"))
                })?),
                '<' => MappedMatcher::Lt(value.to_string()),
                '>' => MappedMatcher::Gt(value.to_string()),
                _ => unreachable!("unexpected constraint operator {operator:?}"),
            };
            Ok(SimpleConstraint { feature, matcher })
        }
        None => {
            // No operator: either `feature*` (exists) or a bare feature (exists).
            if let Some(feature) = part.strip_suffix('*') {
                if feature.is_empty() {
                    return Err(CfError::InvalidQuery(format!(
                        "unsupported mapped constraint syntax {part:?}"
                    )));
                }
                return Ok(SimpleConstraint {
                    feature: feature.to_string(),
                    matcher: MappedMatcher::Exists,
                });
            }
            if part.is_empty() {
                return Err(CfError::InvalidQuery(format!(
                    "unsupported mapped constraint syntax {part:?}"
                )));
            }
            Ok(SimpleConstraint {
                feature: part.to_string(),
                matcher: MappedMatcher::Exists,
            })
        }
    }
}

/// Scan left-to-right for the first unescaped constraint operator (`= # < > ~`),
/// skipping escaped characters so that escaped literals in values are preserved.
fn find_first_constraint_operator(part: &str) -> Option<(usize, char)> {
    let mut escaped = false;
    for (index, ch) in part.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '=' | '#' | '<' | '>' | '~') {
            return Some((index, ch));
        }
    }
    None
}

fn parse_alternative_strings(value: &str) -> Result<Vec<String>> {
    let values: Vec<_> = split_unescaped(value, '|')
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| unescape_query_value(&candidate))
        .collect();
    if values.is_empty() {
        return Err(CfError::InvalidQuery(
            "mapped equality constraint must include at least one value".to_string(),
        ));
    }
    Ok(values)
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

fn value_matches(actual: MappedNodeValue<'_>, expected: &str) -> bool {
    match actual {
        MappedNodeValue::Str(actual) => actual == expected,
        MappedNodeValue::Int(actual) => expected.parse::<i64>() == Ok(actual),
    }
}

fn mapped_edge_value_matches(
    actual: MappedNodeValue<'_>,
    matcher: &MappedEdgeValueMatcher,
) -> bool {
    match matcher {
        MappedEdgeValueMatcher::Eq(expected_values) => expected_values
            .iter()
            .any(|expected| value_matches(actual, expected)),
        MappedEdgeValueMatcher::Ne(expected_values) => expected_values
            .iter()
            .all(|expected| !value_matches(actual, expected)),
        MappedEdgeValueMatcher::Regex(pattern) => match actual {
            MappedNodeValue::Str(actual) => pattern.is_match(actual),
            MappedNodeValue::Int(actual) => pattern.is_match(&actual.to_string()),
        },
    }
}

fn mapped_value_as_str(value: MappedNodeValue<'_>) -> Option<&str> {
    match value {
        MappedNodeValue::Str(value) => Some(value),
        MappedNodeValue::Int(_) => None,
    }
}

fn compare_value(actual: MappedNodeValue<'_>, expected: &str) -> Option<std::cmp::Ordering> {
    match actual {
        MappedNodeValue::Str(actual) => Some(actual.cmp(expected)),
        MappedNodeValue::Int(actual) => expected
            .parse::<i64>()
            .ok()
            .map(|expected| actual.cmp(&expected)),
    }
}

fn mapped_values_equal(left: MappedNodeValue<'_>, right: MappedNodeValue<'_>) -> bool {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => left == right,
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => left == right,
        (MappedNodeValue::Str(left), MappedNodeValue::Int(right)) => {
            left.parse::<i64>().ok() == Some(right)
        }
        (MappedNodeValue::Int(left), MappedNodeValue::Str(right)) => {
            right.parse::<i64>().ok() == Some(left)
        }
    }
}

fn mapped_value_compare(
    left: MappedNodeValue<'_>,
    right: MappedNodeValue<'_>,
) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (MappedNodeValue::Str(left), MappedNodeValue::Str(right)) => Some(left.cmp(right)),
        (MappedNodeValue::Int(left), MappedNodeValue::Int(right)) => Some(left.cmp(&right)),
        (MappedNodeValue::Str(left), MappedNodeValue::Int(right)) => {
            left.parse::<i64>().ok().map(|left| left.cmp(&right))
        }
        (MappedNodeValue::Int(left), MappedNodeValue::Str(right)) => {
            right.parse::<i64>().ok().map(|right| left.cmp(&right))
        }
    }
}

fn parse_mapped_relation_operator(operator: &str) -> Option<MappedRelationOperator> {
    match operator {
        "=" => Some(MappedRelationOperator::Equal),
        "<" => Some(MappedRelationOperator::Before),
        ">" => Some(MappedRelationOperator::After),
        "#" => Some(MappedRelationOperator::NotEqual),
        "<:" => Some(MappedRelationOperator::AdjacentBefore),
        "==" => Some(MappedRelationOperator::SameSlots),
        "##" => Some(MappedRelationOperator::DifferentSlots),
        "&&" => Some(MappedRelationOperator::Overlaps),
        "||" => Some(MappedRelationOperator::Disjoint),
        "<<" => Some(MappedRelationOperator::SlotBefore),
        ">>" => Some(MappedRelationOperator::SlotAfter),
        ":>" => Some(MappedRelationOperator::AdjacentAfter),
        "=:" => Some(MappedRelationOperator::SameFirstSlot),
        ":=" => Some(MappedRelationOperator::SameLastSlot),
        "::" => Some(MappedRelationOperator::SameBoundary),
        "[[" => Some(MappedRelationOperator::Embeds),
        "]]" => Some(MappedRelationOperator::EmbeddedIn),
        _ => None,
    }
    .or_else(|| parse_mapped_near_relation_operator(operator))
    .or_else(|| {
        operator
            .strip_prefix('-')
            .and_then(|rest| rest.strip_suffix('>'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_mapped_edge_forward_operator)
    })
    .or_else(|| {
        operator
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('-'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_mapped_edge_backward_operator)
    })
    .or_else(|| {
        operator
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
            .filter(|edge_name| !edge_name.is_empty())
            .map(parse_mapped_edge_either_operator)
    })
    .or_else(|| parse_mapped_feature_relation_operator(operator))
}

fn parse_mapped_near_relation_operator(operator: &str) -> Option<MappedRelationOperator> {
    parse_surrounded_u32(operator, '=', ':')
        .map(MappedRelationOperator::NearFirstSlot)
        .or_else(|| {
            parse_surrounded_u32(operator, ':', '=').map(MappedRelationOperator::NearLastSlot)
        })
        .or_else(|| {
            parse_surrounded_u32(operator, ':', ':').map(MappedRelationOperator::NearBoundary)
        })
        .or_else(|| {
            parse_surrounded_u32(operator, '<', ':').map(MappedRelationOperator::NearBefore)
        })
        .or_else(|| parse_surrounded_u32(operator, ':', '>').map(MappedRelationOperator::NearAfter))
}

fn parse_surrounded_u32(operator: &str, prefix: char, suffix: char) -> Option<u32> {
    operator
        .strip_prefix(prefix)?
        .strip_suffix(suffix)?
        .parse::<u32>()
        .ok()
}

fn parse_mapped_edge_forward_operator(raw: &str) -> MappedRelationOperator {
    if let Some((edge_name, matcher)) = parse_mapped_edge_value_matcher(raw) {
        return MappedRelationOperator::EdgeForwardValue(edge_name, matcher);
    }
    MappedRelationOperator::EdgeForward(raw.to_string())
}

fn slots_within(left: u32, right: u32, distance: u32) -> bool {
    left.abs_diff(right) <= distance
}

fn parse_mapped_edge_backward_operator(raw: &str) -> MappedRelationOperator {
    if let Some((edge_name, matcher)) = parse_mapped_edge_value_matcher(raw) {
        return MappedRelationOperator::EdgeBackwardValue(edge_name, matcher);
    }
    MappedRelationOperator::EdgeBackward(raw.to_string())
}

fn parse_mapped_edge_either_operator(raw: &str) -> MappedRelationOperator {
    if let Some((edge_name, matcher)) = parse_mapped_edge_value_matcher(raw) {
        return MappedRelationOperator::EdgeEitherValue(edge_name, matcher);
    }
    MappedRelationOperator::EdgeEither(raw.to_string())
}

fn parse_mapped_edge_value_matcher(raw: &str) -> Option<(String, MappedEdgeValueMatcher)> {
    if let Some((edge_name, value)) = raw.split_once('=') {
        if !edge_name.is_empty() {
            return Some((
                edge_name.to_string(),
                MappedEdgeValueMatcher::Eq(
                    split_unescaped(value, '|')
                        .into_iter()
                        .map(|value| unescape_query_value(&value))
                        .collect(),
                ),
            ));
        }
    }
    if let Some((edge_name, value)) = raw.split_once('#') {
        if !edge_name.is_empty() {
            return Some((
                edge_name.to_string(),
                MappedEdgeValueMatcher::Ne(
                    split_unescaped(value, '|')
                        .into_iter()
                        .map(|value| unescape_query_value(&value))
                        .collect(),
                ),
            ));
        }
    }
    if let Some((edge_name, pattern)) = raw.split_once('~') {
        if !edge_name.is_empty() {
            return Regex::new(&unescape_query_value(pattern))
                .ok()
                .map(|pattern| {
                    (
                        edge_name.to_string(),
                        MappedEdgeValueMatcher::Regex(pattern),
                    )
                });
        }
    }
    None
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

fn parse_mapped_feature_relation_operator(operator: &str) -> Option<MappedRelationOperator> {
    let inner = operator.strip_prefix('.')?.strip_suffix('.')?;
    if inner.is_empty() {
        return None;
    }
    if let Some(operator) = parse_mapped_feature_regex_relation_operator(inner) {
        return Some(operator);
    }
    for (raw_operator, comparison) in [
        ('=', MappedFeatureRelationComparison::Eq),
        ('#', MappedFeatureRelationComparison::Ne),
        ('<', MappedFeatureRelationComparison::Lt),
        ('>', MappedFeatureRelationComparison::Gt),
    ] {
        if let Some((left_feature, right_feature)) = inner.split_once(raw_operator) {
            if left_feature.is_empty() || right_feature.is_empty() {
                return None;
            }
            return Some(MappedRelationOperator::FeatureCompare {
                left_feature: left_feature.to_string(),
                comparison,
                right_feature: right_feature.to_string(),
            });
        }
    }
    Some(MappedRelationOperator::FeatureCompare {
        left_feature: inner.to_string(),
        comparison: MappedFeatureRelationComparison::Eq,
        right_feature: inner.to_string(),
    })
}

fn parse_mapped_feature_regex_relation_operator(inner: &str) -> Option<MappedRelationOperator> {
    let (left_feature, rest) = inner.split_once('~')?;
    let (pattern, right_feature) = rest.rsplit_once('~')?;
    if left_feature.is_empty() || pattern.is_empty() || right_feature.is_empty() {
        return None;
    }
    Some(MappedRelationOperator::FeatureRegexCompare {
        left_feature: left_feature.to_string(),
        pattern: Regex::new(pattern).ok()?,
        right_feature: right_feature.to_string(),
    })
}
