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
        self.search_plan_with_root(plan, otype, oslots, sets, limit, None)
    }

    fn search_plan_with_root(
        &self,
        plan: &MappedRelationPlan,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
        root: Option<u32>,
    ) -> Result<Vec<Vec<u32>>> {
        if plan.relations.is_empty() && plan.atoms.len() == 1 && plan.atoms[0].indent == 0 {
            let atom = &plan.atoms[0];
            let constraints = self.load_constraints(atom)?;
            return self.search_atom(atom, otype, &constraints, sets, limit);
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
            .map(|rows| rows.iter().map(|row| row[0]).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let candidates = candidate_nodes
            .into_iter()
            .enumerate()
            .map(|(index, nodes)| {
                self.build_candidate_set(nodes, plan.atoms[index].indent > 0, otype, oslots)
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
                let default_operator = MappedRelationOperator::Embeds;
                let operator = atoms[index].operator.as_ref().unwrap_or(&default_operator);
                return Ok(self.relation_operator_holds(operator, *parent, node, otype, oslots));
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
        let default_operator = MappedRelationOperator::Embeds;
        Ok(
            match plan.atoms[index]
                .operator
                .as_ref()
                .unwrap_or(&default_operator)
            {
                MappedRelationOperator::Embeds => {
                    candidates[index].nodes_within_slot_interval(parent_first, parent_last)
                }
                MappedRelationOperator::EmbeddedIn => {
                    candidates[index].nodes_containing_slot_interval(parent_first, parent_last)
                }
                _ => candidates[index].nodes(),
            },
        )
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
        let blocks = self.precompute_quantifier_blocks(quantified, otype, oslots, sets)?;
        let base_limits = match limit {
            Some(limit) => BaseLimitSteps::limited(limit),
            None => BaseLimitSteps::unlimited(),
        };
        let mut slot_cache = HashMap::new();
        let mut results = Vec::new();
        for base_limit in base_limits {
            let base_rows = self.search_plan(&quantified.base, otype, oslots, sets, base_limit)?;
            results.clear();
            for row in &base_rows {
                let root = row[0];
                let root_slots = self.cached_node_slots(&mut slot_cache, oslots, otype, root)?;
                let root_interval = slot_interval(&root_slots);
                let mut keep = true;
                for block in &blocks {
                    let mut exists = false;
                    for alternative in &block.alternatives {
                        if self.exists_contained_precomputed_alternative(
                            root,
                            root_interval,
                            alternative,
                            &mut slot_cache,
                            otype,
                            oslots,
                        )? {
                            exists = true;
                            break;
                        }
                    }
                    keep = match block.kind {
                        MappedQuantifierKind::With => exists,
                        MappedQuantifierKind::Without => !exists,
                        MappedQuantifierKind::Where => self.mapped_where_block_holds(
                            root,
                            block,
                            &mut slot_cache,
                            otype,
                            oslots,
                            sets,
                        )?,
                    };
                    if !keep {
                        break;
                    }
                }
                if keep {
                    results.push(row.clone());
                    if limit.is_some_and(|limit| results.len() >= limit) {
                        break;
                    }
                }
            }
            if limit.is_none()
                || limit.is_some_and(|limit| results.len() >= limit)
                || base_limit.is_some_and(|base_limit| base_rows.len() < base_limit)
            {
                break;
            }
        }
        Ok(results)
    }

    fn mapped_where_block_holds(
        &self,
        root: u32,
        block: &PrecomputedQuantifierBlock,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<bool> {
        let root_slots = self.cached_node_slots(slot_cache, oslots, otype, root)?;
        let root_interval = slot_interval(&root_slots);
        for (antecedent, combined_plans) in &block.where_pairs {
            let antecedent_rows =
                self.search_plan_with_root(antecedent, otype, oslots, sets, None, Some(root))?;
            let antecedent_rows = antecedent_rows
                .into_iter()
                .filter(|row| {
                    row.iter().all(|node| {
                        self.node_contained_in_interval(
                            *node,
                            root_interval,
                            slot_cache,
                            otype,
                            oslots,
                        )
                        .unwrap_or(false)
                    })
                })
                .collect::<Vec<_>>();
            if antecedent_rows.is_empty() {
                continue;
            }

            let antecedent_width = antecedent.atoms.len();
            let mut satisfied = HashSet::new();
            for combined_plan in combined_plans {
                let combined_rows = self.search_plan_with_root(
                    combined_plan,
                    otype,
                    oslots,
                    sets,
                    None,
                    Some(root),
                )?;
                for row in combined_rows {
                    satisfied.insert(row.into_iter().take(antecedent_width).collect::<Vec<_>>());
                }
            }
            if antecedent_rows
                .into_iter()
                .any(|row| !satisfied.contains(&row))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn node_contained_in_interval(
        &self,
        node: u32,
        root_interval: Option<(u32, u32)>,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<bool> {
        let Some(root_interval) = root_interval else {
            return Ok(false);
        };
        let slots = self.cached_node_slots(slot_cache, oslots, otype, node)?;
        Ok(interval_contains(root_interval, slot_interval(&slots)))
    }

    fn precompute_quantifier_blocks(
        &self,
        quantified: &MappedQuantifiedQuery,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<PrecomputedQuantifierBlock>> {
        quantified
            .blocks
            .iter()
            .map(|block| {
                let alternatives = block
                    .alternatives
                    .iter()
                    .map(|alternative| {
                        self.precompute_quantifier_alternative(alternative, otype, oslots, sets)
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(PrecomputedQuantifierBlock {
                    kind: block.kind,
                    alternatives,
                    where_pairs: block.where_pairs.clone(),
                })
            })
            .collect()
    }

    fn precompute_quantifier_alternative(
        &self,
        alternative: &MappedQuantifierAlternative,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<PrecomputedQuantifierAlternative> {
        match alternative {
            MappedQuantifierAlternative::Simple(plan) => self
                .precompute_alternative(plan, otype, oslots, sets)
                .map(PrecomputedQuantifierAlternative::Simple),
            MappedQuantifierAlternative::Quantified(quantified) => Ok(
                PrecomputedQuantifierAlternative::Quantified(quantified.clone()),
            ),
        }
    }

    fn precompute_alternative(
        &self,
        plan: &MappedRelationPlan,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<PrecomputedAlternative> {
        if plan.has_parent_relation_endpoint() {
            return Ok(PrecomputedAlternative::ParentRelationPlan(plan.clone()));
        }
        if plan.relations.is_empty() && plan.atoms.iter().all(|atom| atom.indent == 0) {
            let parent_atoms = plan
                .atoms
                .iter()
                .filter(|atom| atom.is_parent_reference())
                .cloned()
                .collect::<Vec<_>>();
            let candidates = plan
                .atoms
                .iter()
                .filter(|atom| !atom.is_parent_reference())
                .map(|atom| {
                    let constraints = self.load_constraints(atom)?;
                    let rows = self.search_atom(atom, otype, &constraints, sets, None)?;
                    let mut atom_candidates = Vec::new();
                    for row in rows {
                        let node = row[0];
                        let slots = self.node_slots(oslots, otype, node)?;
                        if let Some((first_slot, last_slot)) = slot_interval(&slots) {
                            atom_candidates.push(PrecomputedCandidate {
                                node,
                                first_slot,
                                last_slot,
                            });
                        }
                    }
                    atom_candidates.sort_by_key(|candidate| candidate.first_slot);
                    Ok(atom_candidates)
                })
                .collect::<Result<Vec<_>>>()?;
            return Ok(PrecomputedAlternative::IndependentAtoms {
                parent_atoms,
                candidates,
            });
        }

        let rows = self.search_plan(plan, otype, oslots, sets, None)?;
        let rows = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|node| {
                        let slots = self.node_slots(oslots, otype, node)?;
                        let Some((first_slot, last_slot)) = slot_interval(&slots) else {
                            return Err(CfError::InvalidQuery(format!(
                                "mapped quantified alternative node {node} has no slots"
                            )));
                        };
                        Ok(PrecomputedCandidate {
                            node,
                            first_slot,
                            last_slot,
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(PrecomputedAlternative::RelationRows { rows })
    }

    fn exists_contained_precomputed(
        &self,
        root: u32,
        root_interval: Option<(u32, u32)>,
        alternative: &PrecomputedAlternative,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<bool> {
        let Some((root_first, root_last)) = root_interval else {
            return Ok(false);
        };
        match alternative {
            PrecomputedAlternative::IndependentAtoms {
                parent_atoms,
                candidates,
            } => {
                for atom in parent_atoms {
                    let constraints = self.load_constraints(atom)?;
                    if !self.node_matches(root, atom, otype, &constraints, None)? {
                        return Ok(false);
                    }
                }
                for candidates in candidates {
                    let mut found_contained = false;
                    let start =
                        candidates.partition_point(|candidate| candidate.first_slot < root_first);
                    let end =
                        candidates.partition_point(|candidate| candidate.first_slot <= root_last);
                    for candidate in &candidates[start..end] {
                        if candidate.last_slot > root_last {
                            continue;
                        }
                        if self.contains_by_cached_slots(
                            slot_cache,
                            oslots,
                            otype,
                            root,
                            candidate.node,
                        )? {
                            found_contained = true;
                            break;
                        }
                    }
                    if !found_contained {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            PrecomputedAlternative::RelationRows { rows } => {
                for row in rows {
                    let mut row_contained = true;
                    for candidate in row {
                        if candidate.first_slot < root_first || candidate.last_slot > root_last {
                            row_contained = false;
                            break;
                        }
                        if !self.contains_by_cached_slots(
                            slot_cache,
                            oslots,
                            otype,
                            root,
                            candidate.node,
                        )? {
                            row_contained = false;
                            break;
                        }
                    }
                    if row_contained {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PrecomputedAlternative::ParentRelationPlan(plan) => {
                let rows =
                    self.search_plan_with_root(plan, otype, oslots, None, None, Some(root))?;
                for row in rows {
                    let mut row_contained = true;
                    for node in row {
                        let slots = self.node_slots(oslots, otype, node)?;
                        let Some((first_slot, last_slot)) = slot_interval(&slots) else {
                            row_contained = false;
                            break;
                        };
                        if first_slot < root_first || last_slot > root_last {
                            row_contained = false;
                            break;
                        }
                        if !self.contains_by_cached_slots(slot_cache, oslots, otype, root, node)? {
                            row_contained = false;
                            break;
                        }
                    }
                    if row_contained {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    fn exists_contained_precomputed_alternative(
        &self,
        root: u32,
        root_interval: Option<(u32, u32)>,
        alternative: &PrecomputedQuantifierAlternative,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<bool> {
        match alternative {
            PrecomputedQuantifierAlternative::Simple(alternative) => self
                .exists_contained_precomputed(
                    root,
                    root_interval,
                    alternative,
                    slot_cache,
                    otype,
                    oslots,
                ),
            PrecomputedQuantifierAlternative::Quantified(quantified) => {
                self.exists_contained_mapped_quantified(root, quantified, slot_cache, otype, oslots)
            }
        }
    }

    fn exists_contained_mapped_quantified(
        &self,
        root: u32,
        quantified: &MappedQuantifiedQuery,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<bool> {
        let root_slots = self.cached_node_slots(slot_cache, oslots, otype, root)?;
        let Some(root_interval) = slot_interval(&root_slots) else {
            return Ok(false);
        };
        let base_rows = self.search_plan(&quantified.base, otype, oslots, None, None)?;
        let blocks = self.precompute_quantifier_blocks(quantified, otype, oslots, None)?;
        for row in base_rows {
            let Some(nested_root) = row.first().copied() else {
                continue;
            };
            if !self.contains_by_cached_slots(slot_cache, oslots, otype, root, nested_root)? {
                continue;
            }
            let nested_slots = self.cached_node_slots(slot_cache, oslots, otype, nested_root)?;
            let nested_interval = slot_interval(&nested_slots);
            if !interval_contains(root_interval, nested_interval) {
                continue;
            }
            let mut keep = true;
            for block in &blocks {
                let mut exists = false;
                for alternative in &block.alternatives {
                    if self.exists_contained_precomputed_alternative(
                        nested_root,
                        nested_interval,
                        alternative,
                        slot_cache,
                        otype,
                        oslots,
                    )? {
                        exists = true;
                        break;
                    }
                }
                keep = match block.kind {
                    MappedQuantifierKind::With => exists,
                    MappedQuantifierKind::Without => !exists,
                    MappedQuantifierKind::Where => self.mapped_where_block_holds(
                        nested_root,
                        block,
                        slot_cache,
                        otype,
                        oslots,
                        None,
                    )?,
                };
                if !keep {
                    break;
                }
            }
            if keep {
                return Ok(true);
            }
        }
        Ok(false)
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
            MappedRelationOperator::NearBefore(distance) => self
                .last_slot(oslots, otype, left)
                .zip(self.first_slot(oslots, otype, right))
                .is_some_and(|(left_last, right_first)| {
                    left_last < right_first
                        && right_first.saturating_sub(left_last) <= distance.saturating_add(1)
                }),
            MappedRelationOperator::NearAfter(distance) => self
                .first_slot(oslots, otype, left)
                .zip(self.last_slot(oslots, otype, right))
                .is_some_and(|(left_first, right_last)| {
                    right_last < left_first
                        && left_first.saturating_sub(right_last) <= distance.saturating_add(1)
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
        let Ok(Some(slot_type)) = otype.str_value(1) else {
            return false;
        };
        let Ok(Some(parent_targets)) = oslots.targets(parent) else {
            return false;
        };
        let Ok(parent_slots) = parent_targets.collect::<Result<Vec<_>>>() else {
            return false;
        };
        let Ok(child_type) = otype.str_value(child) else {
            return false;
        };
        if child_type == Some(slot_type) {
            return parent_slots.binary_search(&child).is_ok();
        }
        let Ok(Some(child_targets)) = oslots.targets(child) else {
            return false;
        };
        for slot in child_targets {
            let Ok(slot) = slot else {
                return false;
            };
            if parent_slots.binary_search(&slot).is_err() {
                return false;
            }
        }
        true
    }

    fn contains_by_cached_slots(
        &self,
        slot_cache: &mut HashMap<u32, Vec<u32>>,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        parent: u32,
        child: u32,
    ) -> Result<bool> {
        let parent_slots = self.cached_node_slots(slot_cache, oslots, otype, parent)?;
        let child_slots = self.cached_node_slots(slot_cache, oslots, otype, child)?;
        if child_slots.is_empty() {
            return Ok(false);
        }
        Ok(child_slots
            .iter()
            .all(|slot| parent_slots.binary_search(slot).is_ok()))
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

    fn nodes_containing_slot_interval(&self, first_slot: u32, last_slot: u32) -> Vec<u32> {
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|candidate_first| candidate_first <= first_slot)
        });
        self.by_first_slot[..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index];
                let candidate_last = entry.last_slot?;
                (candidate_last >= last_slot).then_some(entry.node)
            })
            .collect()
    }
}

struct CandidateEntry {
    node: u32,
    first_slot: Option<u32>,
    last_slot: Option<u32>,
}

struct BaseLimitSteps {
    current: Option<usize>,
    yielded_unlimited: bool,
}

impl BaseLimitSteps {
    fn limited(result_limit: usize) -> Self {
        Self {
            current: Some((result_limit * 64).max(1024)),
            yielded_unlimited: false,
        }
    }

    fn unlimited() -> Self {
        Self {
            current: None,
            yielded_unlimited: false,
        }
    }
}

impl Iterator for BaseLimitSteps {
    type Item = Option<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            Some(limit) => {
                let step = Some(limit);
                self.current = limit.checked_mul(2);
                Some(step)
            }
            None if !self.yielded_unlimited => {
                self.yielded_unlimited = true;
                Some(None)
            }
            None => None,
        }
    }
}

#[derive(Clone)]
struct MappedQuantifiedQuery {
    base: MappedRelationPlan,
    blocks: Vec<MappedQuantifierBlock>,
}

#[derive(Clone)]
struct MappedQuantifierBlock {
    kind: MappedQuantifierKind,
    alternatives: Vec<MappedQuantifierAlternative>,
    where_pairs: Vec<(MappedRelationPlan, Vec<MappedRelationPlan>)>,
}

#[derive(Clone)]
enum MappedQuantifierAlternative {
    Simple(MappedRelationPlan),
    Quantified(Box<MappedQuantifiedQuery>),
}

struct PrecomputedQuantifierBlock {
    kind: MappedQuantifierKind,
    alternatives: Vec<PrecomputedQuantifierAlternative>,
    where_pairs: Vec<(MappedRelationPlan, Vec<MappedRelationPlan>)>,
}

enum PrecomputedQuantifierAlternative {
    Simple(PrecomputedAlternative),
    Quantified(Box<MappedQuantifiedQuery>),
}

enum PrecomputedAlternative {
    IndependentAtoms {
        parent_atoms: Vec<SimpleAtom>,
        candidates: Vec<Vec<PrecomputedCandidate>>,
    },
    RelationRows {
        rows: Vec<Vec<PrecomputedCandidate>>,
    },
    ParentRelationPlan(MappedRelationPlan),
}

struct PrecomputedCandidate {
    node: u32,
    first_slot: u32,
    last_slot: u32,
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

impl MappedRelationPlan {
    fn has_parent_relation_endpoint(&self) -> bool {
        self.relations.iter().any(|relation| {
            matches!(relation.left, MappedRelationEndpoint::Parent)
                || matches!(relation.right, MappedRelationEndpoint::Parent)
        })
    }
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
    operator: Option<MappedRelationOperator>,
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
        let operator = self
            .operator
            .as_ref()
            .map(|operator| operator.cache_key())
            .unwrap_or_default();
        format!("{}{} {}", self.node_type, operator, constraints.join(" "))
    }
}

impl MappedRelationOperator {
    fn cache_key(&self) -> String {
        match self {
            Self::Equal => "=".to_string(),
            Self::Before => "<".to_string(),
            Self::After => ">".to_string(),
            Self::NotEqual => "#".to_string(),
            Self::AdjacentBefore => "<:".to_string(),
            Self::SameSlots => "==".to_string(),
            Self::DifferentSlots => "##".to_string(),
            Self::Overlaps => "&&".to_string(),
            Self::Disjoint => "||".to_string(),
            Self::SlotBefore => "<<".to_string(),
            Self::SlotAfter => ">>".to_string(),
            Self::AdjacentAfter => ":>".to_string(),
            Self::SameFirstSlot => "=:".to_string(),
            Self::SameLastSlot => ":=".to_string(),
            Self::SameBoundary => "::".to_string(),
            Self::NearFirstSlot(distance) => format!("={distance}:"),
            Self::NearLastSlot(distance) => format!(":{distance}="),
            Self::NearBoundary(distance) => format!(":{distance}:"),
            Self::NearBefore(distance) => format!("<{distance}:"),
            Self::NearAfter(distance) => format!(":{distance}>"),
            Self::Embeds => "[[".to_string(),
            Self::EmbeddedIn => "]]".to_string(),
            Self::EdgeForward(edge_name) => format!("-{edge_name}>"),
            Self::EdgeBackward(edge_name) => format!("<{edge_name}-"),
            Self::EdgeEither(edge_name) => format!("<{edge_name}>"),
            Self::EdgeForwardValue(edge_name, matcher) => {
                format!("-{edge_name}{}>", matcher.cache_key())
            }
            Self::EdgeBackwardValue(edge_name, matcher) => {
                format!("<{edge_name}{}-", matcher.cache_key())
            }
            Self::EdgeEitherValue(edge_name, matcher) => {
                format!("<{edge_name}{}>", matcher.cache_key())
            }
            Self::FeatureCompare {
                left_feature,
                comparison,
                right_feature,
            } => format!(".{left_feature}{}{right_feature}.", comparison.cache_key()),
            Self::FeatureRegexCompare {
                left_feature,
                pattern,
                right_feature,
            } => format!(".{left_feature}~{}~{right_feature}.", pattern.as_str()),
        }
    }
}

impl MappedEdgeValueMatcher {
    fn cache_key(&self) -> String {
        match self {
            Self::Eq(values) => format!("={}", values.join("|")),
            Self::Ne(values) => format!("#{}", values.join("|")),
            Self::Regex(pattern) => format!("~{}", pattern.as_str()),
        }
    }
}

impl MappedFeatureRelationComparison {
    fn cache_key(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "#",
            Self::Lt => "<",
            Self::Gt => ">",
        }
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

fn slot_interval(slots: &[u32]) -> Option<(u32, u32)> {
    Some((*slots.first()?, *slots.last()?))
}

fn interval_contains(parent: (u32, u32), child: Option<(u32, u32)>) -> bool {
    child.is_some_and(|(first, last)| parent.0 <= first && last <= parent.1)
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
    let mut pending_atom_operator: Option<MappedRelationOperator> = None;

    for line in template
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('%'))
    {
        let trimmed = line.trim();
        if let Some(operator) = parse_mapped_atom_operator(trimmed) {
            pending_atom_operator = Some(operator);
            continue;
        }
        let relation_parts = split_query_whitespace(trimmed);
        if relation_parts.len() == 3 {
            if let Some(operator) = parse_mapped_relation_operator(&relation_parts[1]) {
                pending_relations.push(PendingMappedRelation {
                    left: relation_parts[0].clone(),
                    operator,
                    right: relation_parts[2].clone(),
                });
                continue;
            }
        }
        if pending_atom_operator.is_none() && !atoms.is_empty() {
            if let Some(constraints) = parse_mapped_feature_continuation_line(trimmed)? {
                let last_index = atoms.len() - 1;
                atoms[last_index].constraints.extend(constraints);
                continue;
            }
        }

        let parts = split_query_whitespace(trimmed);
        let mut parts = parts.into_iter();
        let Some(first) = parts.next() else {
            continue;
        };
        if !first.contains(':') {
            if let Some(atom_index) = names.get(&first).copied() {
                let constraints = parts
                    .map(|part| parse_simple_constraint(&part))
                    .collect::<Result<Vec<_>>>()?;
                let node_type = atoms[atom_index].node_type.clone();
                let reference_index = atoms.len();
                atoms.push(SimpleAtom {
                    indent: line.chars().take_while(|ch| ch.is_whitespace()).count(),
                    operator: pending_atom_operator.take(),
                    name: None,
                    node_type,
                    constraints,
                });
                pending_relations.push(PendingMappedRelation {
                    left: format!("\0ref{reference_index}"),
                    operator: MappedRelationOperator::Equal,
                    right: first,
                });
                names.insert(format!("\0ref{reference_index}"), reference_index);
                continue;
            }
        }

        let atom = parse_simple_atom_with_operator(line, pending_atom_operator.take())?;
        if let Some(name) = &atom.name {
            names.insert(name.clone(), atoms.len());
        }
        atoms.push(atom);
    }

    if atoms.is_empty() {
        return Err(CfError::InvalidQuery("empty mapped query".to_string()));
    }
    let relations = pending_relations
        .into_iter()
        .map(|relation| {
            let left = mapped_relation_endpoint(&names, &relation.left)?;
            let right = mapped_relation_endpoint(&names, &relation.right)?;
            Ok(MappedRelation {
                left,
                operator: relation.operator,
                right,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(MappedRelationPlan { atoms, relations })
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
    let mut blocks = Vec::new();
    for block in parsed.blocks {
        let mut alternatives = Vec::new();
        for alternative in &block.alternatives {
            if contains_quantifier_token(&alternative) {
                alternatives.push(MappedQuantifierAlternative::Quantified(Box::new(
                    parse_mapped_quantified_query(alternative)?,
                )));
            } else {
                alternatives.push(MappedQuantifierAlternative::Simple(parse_mapped_plan(
                    alternative,
                )?));
            }
        }
        let where_pairs = if matches!(block.kind, MappedQuantifierKind::Where) {
            block
                .alternatives
                .iter()
                .map(|antecedent| {
                    let antecedent_plan = parse_mapped_plan(antecedent)?;
                    let combined_plans = block
                        .consequents
                        .iter()
                        .map(|consequent| {
                            parse_mapped_plan(&combine_quantifier_templates(antecedent, consequent))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok((antecedent_plan, combined_plans))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        blocks.push(MappedQuantifierBlock {
            kind: block.kind,
            alternatives,
            where_pairs,
        });
    }
    Ok(MappedQuantifiedQuery { base, blocks })
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

fn parse_simple_atom_with_operator(
    line: &str,
    pending_operator: Option<MappedRelationOperator>,
) -> Result<SimpleAtom> {
    let indent = line.chars().take_while(|ch| ch.is_whitespace()).count();
    let parts = split_query_whitespace(line);
    let mut parts = parts.into_iter();
    let Some(first) = parts.next() else {
        return Err(CfError::InvalidQuery("empty mapped query".to_string()));
    };
    let (operator, first) = match parse_mapped_atom_operator(&first) {
        Some(operator) => {
            let Some(atom) = parts.next() else {
                return Err(CfError::InvalidQuery(format!(
                    "missing mapped atom after operator {first:?}"
                )));
            };
            (Some(operator), atom)
        }
        None => (None, first),
    };
    let operator = operator.or(pending_operator);
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
        operator,
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
    if let Some(feature) = part.strip_suffix('*') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Exists,
        });
    }
    if let Some(feature) = part.strip_suffix('#') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Missing,
        });
    }
    if let Some((feature, value)) = part.split_once('=') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Eq(parse_alternative_strings(value)?),
        });
    }
    if let Some((feature, value)) = part.split_once('#') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Ne(parse_alternative_strings(value)?),
        });
    }
    if let Some((feature, pattern)) = part.split_once('~') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Regex(Regex::new(pattern).map_err(|source| {
                CfError::InvalidQuery(format!("invalid mapped regex {pattern:?}: {source}"))
            })?),
        });
    }
    if let Some((feature, value)) = part.split_once('<') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Lt(value.to_string()),
        });
    }
    if let Some((feature, value)) = part.split_once('>') {
        return Ok(SimpleConstraint {
            feature: feature.to_string(),
            matcher: MappedMatcher::Gt(value.to_string()),
        });
    }
    if !part.is_empty() {
        return Ok(SimpleConstraint {
            feature: part.to_string(),
            matcher: MappedMatcher::Exists,
        });
    }
    Err(CfError::InvalidQuery(format!(
        "unsupported mapped constraint syntax {part:?}"
    )))
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
