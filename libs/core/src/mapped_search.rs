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
        let plan = parse_mapped_plan(template)?;
        let otype = self
            .corpus
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = self
            .corpus
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;

        self.search_plan(&plan, &otype, &oslots, sets, limit)
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

    #[allow(clippy::too_many_arguments)]
    fn search_plan_with_root(
        &self,
        plan: &MappedRelationPlan,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
        // Bound node for the `..` parent reference (`MappedRelationEndpoint::Parent`)
        // when this plan is a sub-search; `None` for top-level plans.
        root: Option<u32>,
    ) -> Result<Vec<Vec<u32>>> {
        let n = plan.atoms.len();

        // 1. Base candidate node set per atom (feature/type filtered), then reduced
        //    by the atom's quantifier blocks via global set algebra (TF _spinAtom +
        //    _doQuantifier).
        let mut candidate_row_cache: HashMap<String, Vec<u32>> = HashMap::new();
        let mut candidate_nodes: Vec<Vec<u32>> = Vec::with_capacity(n);
        for atom in &plan.atoms {
            let cache_key = atom.candidate_cache_key();
            let mut nodes = if let Some(nodes) = candidate_row_cache.get(&cache_key) {
                nodes.clone()
            } else {
                let constraints = self.load_constraints(atom)?;
                let nodes: Vec<u32> = self
                    .search_atom(atom, otype, &constraints, sets, None)?
                    .into_iter()
                    .filter_map(|row| row.into_iter().next())
                    .collect();
                candidate_row_cache.insert(cache_key, nodes.clone());
                nodes
            };
            if !atom.quantifiers.is_empty() {
                let (clean, _) = atom.clean.as_ref().ok_or_else(|| {
                    CfError::InvalidQuery("quantified atom missing clean line".to_string())
                })?;
                let mut yarn: HashSet<u32> = nodes.iter().copied().collect();
                for block in &atom.quantifiers {
                    yarn = self.apply_quantifier_block(clean, block, yarn, otype, oslots, sets)?;
                    if yarn.is_empty() {
                        break;
                    }
                }
                nodes.retain(|node| yarn.contains(node));
            }
            candidate_nodes.push(nodes);
        }

        // Single-atom fast path.
        if plan.relations.is_empty() && n == 1 {
            let mut rows: Vec<Vec<u32>> = candidate_nodes
                .pop()
                .unwrap_or_default()
                .into_iter()
                .map(|node| vec![node])
                .collect();
            if let Some(limit) = limit {
                rows.truncate(limit);
            }
            return Ok(rows);
        }

        // 2. Candidate sets with slot intervals (always; driving needs them).
        let candidates = candidate_nodes
            .into_iter()
            .map(|nodes| self.build_candidate_set(nodes, otype, oslots))
            .collect::<Result<Vec<_>>>()?;

        // 3. Embedding parent per atom (nearest enclosing smaller indent).
        let embeds: Vec<Option<usize>> = (0..n)
            .map(|i| nearest_bound_parent_index(&plan.atoms, i))
            .collect();

        // 4. Adjacency maps for any edge relations (driven from the sparse edge rows).
        let edge_adj = self.build_edge_adjacency(plan)?;

        // 5. Slot -> container indexes for atoms that may be driven in the reverse
        //    (container-from-contained) direction. Interval containment scans grow
        //    O(candidates); a slot index keeps it O(1) and is exact even for
        //    sparse-interval container types such as `lex`.
        let container_index = self.build_container_indexes(plan, &candidates, &embeds, otype, oslots)?;

        // 6. Connectivity-greedy evaluation order so every non-seed atom is driven
        //    from an already-bound neighbour rather than enumerated wholesale.
        let order = compute_join_order(plan, &candidates, &embeds, root);


        // 7. Backtracking join driven by relations/embeddings.
        let mut results = Vec::new();
        let mut bound: Vec<Option<u32>> = vec![None; n];
        let mut slot_cache: HashMap<u32, Option<(u32, u32)>> = HashMap::new();
        let mut slots_cache: HashMap<u32, Vec<u32>> = HashMap::new();
        self.join_recurse(
            plan,
            &candidates,
            &embeds,
            &edge_adj,
            &container_index,
            &order,
            0,
            &mut bound,
            &mut results,
            limit,
            root,
            otype,
            oslots,
            &mut slot_cache,
            &mut slots_cache,
        )?;
        Ok(results)
    }

    /// Build a `slot -> [container node]` index for every atom that may be driven as
    /// a container (an embedding parent, or the container side of `[[` / `]]`).
    fn build_container_indexes(
        &self,
        plan: &MappedRelationPlan,
        candidates: &[CandidateSet],
        embeds: &[Option<usize>],
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<HashMap<usize, HashMap<u32, Vec<u32>>>> {
        let n = plan.atoms.len();
        let mut needs = vec![false; n];
        for parent in embeds.iter().flatten() {
            needs[*parent] = true;
        }
        for relation in &plan.relations {
            match (&relation.operator, &relation.left, &relation.right) {
                (MappedRelationOperator::Embeds, MappedRelationEndpoint::Atom(left), _) => {
                    needs[*left] = true;
                }
                (MappedRelationOperator::EmbeddedIn, _, MappedRelationEndpoint::Atom(right)) => {
                    needs[*right] = true;
                }
                _ => {}
            }
        }
        let mut indexes = HashMap::new();
        for (atom, needed) in needs.into_iter().enumerate() {
            if !needed {
                continue;
            }
            let mut map: HashMap<u32, Vec<u32>> = HashMap::new();
            for node in candidates[atom].nodes() {
                for slot in self.node_slots(oslots, otype, node)? {
                    map.entry(slot).or_default().push(node);
                }
            }
            indexes.insert(atom, map);
        }
        Ok(indexes)
    }

    #[allow(clippy::too_many_arguments)]
    fn join_recurse(
        &self,
        plan: &MappedRelationPlan,
        candidates: &[CandidateSet],
        embeds: &[Option<usize>],
        edge_adj: &HashMap<String, EdgeAdj>,
        container_index: &HashMap<usize, HashMap<u32, Vec<u32>>>,
        order: &[usize],
        pos: usize,
        bound: &mut [Option<u32>],
        results: &mut Vec<Vec<u32>>,
        limit: Option<usize>,
        root: Option<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        slot_cache: &mut HashMap<u32, Option<(u32, u32)>>,
        slots_cache: &mut HashMap<u32, Vec<u32>>,
    ) -> Result<()> {
        if limit.is_some_and(|limit| results.len() >= limit) {
            return Ok(());
        }
        if pos == order.len() {
            // Every constraint was verified incrementally as its second endpoint
            // bound, so a fully-bound assignment is a match. Emit in atom order.
            results.push(bound.iter().map(|node| node.expect("bound")).collect());
            return Ok(());
        }
        let atom = order[pos];
        let cands = self.drive_candidates(
            plan,
            candidates,
            embeds,
            edge_adj,
            container_index,
            atom,
            bound,
            root,
            otype,
            oslots,
            slot_cache,
        )?;
        for node in cands {
            bound[atom] = Some(node);
            if self
                .partial_constraints_ok(plan, embeds, atom, bound, root, otype, oslots, slots_cache)?
            {
                self.join_recurse(
                    plan,
                    candidates,
                    embeds,
                    edge_adj,
                    container_index,
                    order,
                    pos + 1,
                    bound,
                    results,
                    limit,
                    root,
                    otype,
                    oslots,
                    slot_cache,
                    slots_cache,
                )?;
            }
            bound[atom] = None;
            if limit.is_some_and(|limit| results.len() >= limit) {
                break;
            }
        }
        Ok(())
    }

    /// Check every constraint that becomes fully bound by binding `atom`: relations
    /// where both endpoints are now known, plus the embedding to its bound parent
    /// and from any bound child. Each constraint is thus verified exactly once, when
    /// its last endpoint binds.
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    fn partial_constraints_ok(
        &self,
        plan: &MappedRelationPlan,
        embeds: &[Option<usize>],
        atom: usize,
        bound: &[Option<u32>],
        root: Option<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        slots_cache: &mut HashMap<u32, Vec<u32>>,
    ) -> Result<bool> {
        let node = bound[atom].expect("atom just bound");
        for relation in &plan.relations {
            let involves = matches!(relation.left, MappedRelationEndpoint::Atom(a) if a == atom)
                || matches!(relation.right, MappedRelationEndpoint::Atom(a) if a == atom);
            if !involves {
                continue;
            }
            let (Some(left), Some(right)) = (
                endpoint_node(&relation.left, bound, root),
                endpoint_node(&relation.right, bound, root),
            ) else {
                continue;
            };
            // Slot-containment verifications collect the (potentially huge) container
            // slot list; route them through the cache so each container's slots are
            // read once. Common with `lex` containers shared across many words.
            let ok = match relation.operator {
                MappedRelationOperator::Embeds => {
                    self.contains_cached(slots_cache, oslots, otype, left, right)?
                }
                MappedRelationOperator::EmbeddedIn => {
                    self.contains_cached(slots_cache, oslots, otype, right, left)?
                }
                _ => self.relation_holds(relation, left, right, otype, oslots),
            };
            if !ok {
                return Ok(false);
            }
        }
        if let Some(parent) = embeds[atom] {
            if let Some(parent_node) = bound[parent] {
                if !self.contains_cached(slots_cache, oslots, otype, parent_node, node)? {
                    return Ok(false);
                }
            }
        }
        for (child, child_parent) in embeds.iter().enumerate() {
            if *child_parent == Some(atom) {
                if let Some(child_node) = bound[child] {
                    if !self.contains_cached(slots_cache, oslots, otype, node, child_node)? {
                        return Ok(false);
                    }
                }
            }
        }
        Ok(true)
    }

    /// Slot-containment check (`parent` embeds `child`), mirroring
    /// `contains_by_slots` but caching the parent's (potentially huge) slot list so
    /// it is materialized once per container — critical when many children share a
    /// container such as `lex`. A slot embeds nothing: `oslots.targets` is empty for
    /// slot parents (unlike `node_slots`, which returns slot-identity).
    fn contains_cached(
        &self,
        slots_cache: &mut HashMap<u32, Vec<u32>>,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        parent: u32,
        child: u32,
    ) -> Result<bool> {
        let child_slots = self.node_slots(oslots, otype, child)?;
        if child_slots.is_empty() {
            return Ok(false);
        }
        let parent_slots = self.cached_oslots_targets(slots_cache, oslots, parent)?;
        if parent_slots.is_empty() {
            return Ok(false);
        }
        Ok(child_slots
            .iter()
            .all(|slot| parent_slots.binary_search(slot).is_ok()))
    }

    fn cached_oslots_targets<'c>(
        &self,
        slots_cache: &'c mut HashMap<u32, Vec<u32>>,
        oslots: &EdgeFeatureView<'_>,
        node: u32,
    ) -> Result<&'c Vec<u32>> {
        if !slots_cache.contains_key(&node) {
            let targets = match oslots.targets(node)? {
                Some(targets) => targets.collect::<Result<Vec<_>>>()?,
                None => Vec::new(),
            };
            slots_cache.insert(node, targets);
        }
        Ok(slots_cache.get(&node).expect("just inserted"))
    }

    /// Generate the candidate nodes for `atom` by intersecting every driver that a
    /// currently-bound neighbour provides (embedding both directions, edge rows,
    /// slot-window relations). With no driver the full candidate set is returned
    /// (only the seed atom hits this).
    #[allow(clippy::too_many_arguments)]
    fn drive_candidates(
        &self,
        plan: &MappedRelationPlan,
        candidates: &[CandidateSet],
        embeds: &[Option<usize>],
        edge_adj: &HashMap<String, EdgeAdj>,
        container_index: &HashMap<usize, HashMap<u32, Vec<u32>>>,
        atom: usize,
        bound: &[Option<u32>],
        root: Option<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
        slot_cache: &mut HashMap<u32, Option<(u32, u32)>>,
    ) -> Result<Vec<u32>> {
        let cset = &candidates[atom];
        let cmap = container_index.get(&atom);
        let mut drivers: Vec<Vec<u32>> = Vec::new();

        // Forward embedding: `atom` is contained in its bound parent.
        if let Some(parent) = embeds[atom] {
            if let Some(parent_node) = bound[parent] {
                let Some((pf, pl)) = self.node_interval(slot_cache, oslots, otype, parent_node)?
                else {
                    return Ok(Vec::new());
                };
                drivers.push(cset.nodes_within_slot_interval(pf, pl));
            }
        }
        // Reverse embedding: `atom` contains a bound child.
        for (child, child_parent) in embeds.iter().enumerate() {
            if *child_parent == Some(atom) {
                if let Some(child_node) = bound[child] {
                    let Some((cf, cl)) = self.node_interval(slot_cache, oslots, otype, child_node)?
                    else {
                        return Ok(Vec::new());
                    };
                    drivers.push(containers_of(cmap, cset, cf, cl));
                }
            }
        }
        // Relations with a bound other endpoint.
        for relation in &plan.relations {
            let (is_left, other_ep) =
                if matches!(relation.left, MappedRelationEndpoint::Atom(a) if a == atom) {
                    (true, &relation.right)
                } else if matches!(relation.right, MappedRelationEndpoint::Atom(a) if a == atom) {
                    (false, &relation.left)
                } else {
                    continue;
                };
            let Some(other) = endpoint_node(other_ep, bound, root) else {
                continue;
            };
            if let Some(driver) = self.relation_driver(
                &relation.operator,
                is_left,
                other,
                cset,
                cmap,
                edge_adj,
                slot_cache,
                otype,
                oslots,
            )? {
                drivers.push(driver);
            }
        }

        if drivers.is_empty() {
            return Ok(cset.nodes());
        }
        drivers.sort_by_key(Vec::len);
        let mut acc: Vec<u32> = drivers.swap_remove(0);
        for driver in &drivers {
            let set: HashSet<u32> = driver.iter().copied().collect();
            acc.retain(|node| set.contains(node));
            if acc.is_empty() {
                break;
            }
        }
        Ok(acc)
    }

    /// Candidate nodes for `atom` implied by a single relation to a bound node
    /// `other`. All slot-window / embedding drivers are drawn from `cset` (so they
    /// are automatically type/feature-valid); edge and equality drivers are filtered
    /// against `cset` membership. Returns `None` for relations that cannot drive
    /// (verify-only: `#`, `##`, `||`, `<`, `>`, feature comparisons).
    #[allow(clippy::too_many_arguments)]
    fn relation_driver(
        &self,
        operator: &MappedRelationOperator,
        is_left: bool,
        other: u32,
        cset: &CandidateSet,
        cmap: Option<&HashMap<u32, Vec<u32>>>,
        edge_adj: &HashMap<String, EdgeAdj>,
        slot_cache: &mut HashMap<u32, Option<(u32, u32)>>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<Option<Vec<u32>>> {
        use MappedRelationOperator::*;
        let interval = self.node_interval(slot_cache, oslots, otype, other)?;
        let (of, ol) = match interval {
            Some(value) => value,
            None => match operator {
                Equal
                | EdgeForward(_)
                | EdgeBackward(_)
                | EdgeEither(_)
                | EdgeForwardValue(..)
                | EdgeBackwardValue(..)
                | EdgeEitherValue(..) => (0, 0),
                _ => return Ok(Some(Vec::new())),
            },
        };
        let driver = match operator {
            Equal => {
                if cset.contains(other) {
                    vec![other]
                } else {
                    vec![]
                }
            }
            Embeds => {
                if is_left {
                    containers_of(cmap, cset, of, ol)
                } else {
                    cset.nodes_within_slot_interval(of, ol)
                }
            }
            EmbeddedIn => {
                if is_left {
                    cset.nodes_within_slot_interval(of, ol)
                } else {
                    containers_of(cmap, cset, of, ol)
                }
            }
            Overlaps => cset.nodes_overlapping(of, ol),
            AdjacentBefore => {
                if is_left {
                    sub1(of).map_or_else(Vec::new, |t| cset.nodes_with_last_in(t, t))
                } else {
                    add1(ol).map_or_else(Vec::new, |t| cset.nodes_with_first_in(t, t))
                }
            }
            AdjacentAfter => {
                if is_left {
                    add1(ol).map_or_else(Vec::new, |t| cset.nodes_with_first_in(t, t))
                } else {
                    sub1(of).map_or_else(Vec::new, |t| cset.nodes_with_last_in(t, t))
                }
            }
            NearBefore(k) => {
                if is_left {
                    let t = i64::from(of) - 1;
                    cset.nodes_with_last_in(clamp_lo(t, *k), clamp_hi(t, *k))
                } else {
                    let t = i64::from(ol) + 1;
                    cset.nodes_with_first_in(clamp_lo(t, *k), clamp_hi(t, *k))
                }
            }
            NearAfter(k) => {
                if is_left {
                    let t = i64::from(ol) + 1;
                    cset.nodes_with_first_in(clamp_lo(t, *k), clamp_hi(t, *k))
                } else {
                    let t = i64::from(of) - 1;
                    cset.nodes_with_last_in(clamp_lo(t, *k), clamp_hi(t, *k))
                }
            }
            SameFirstSlot => cset.nodes_with_first_in(of, of),
            SameLastSlot => cset.nodes_with_last_in(ol, ol),
            NearFirstSlot(k) => {
                cset.nodes_with_first_in(clamp_lo(i64::from(of), *k), clamp_hi(i64::from(of), *k))
            }
            NearLastSlot(k) => {
                cset.nodes_with_last_in(clamp_lo(i64::from(ol), *k), clamp_hi(i64::from(ol), *k))
            }
            SameBoundary | SameSlots => cset.nodes_with_first_in(of, of),
            NearBoundary(k) => {
                cset.nodes_with_first_in(clamp_lo(i64::from(of), *k), clamp_hi(i64::from(of), *k))
            }
            // EdgeForward: edge runs left -> right. If `atom` is the right operand
            // (is_left = false) it is a forward target of `other`; if it is the left
            // operand it is a backward source of `other`.
            EdgeForward(name) | EdgeForwardValue(name, _) => {
                self.edge_driver(edge_adj, name, other, !is_left, cset)
            }
            // EdgeBackward: edge runs right -> left (the relation holds when an edge
            // goes from the right operand to the left). If `atom` is the left operand
            // it is a forward target of `other`; otherwise a backward source.
            EdgeBackward(name) | EdgeBackwardValue(name, _) => {
                self.edge_driver(edge_adj, name, other, is_left, cset)
            }
            EdgeEither(name) | EdgeEitherValue(name, _) => {
                let mut nodes = self.edge_driver(edge_adj, name, other, is_left, cset);
                nodes.extend(self.edge_driver(edge_adj, name, other, !is_left, cset));
                nodes.sort_unstable();
                nodes.dedup();
                nodes
            }
            // `<<` / `>>` produce open-ended ~half-corpus windows; intersecting such
            // a large driver per seed is far more expensive than verifying the
            // relation, so they drive nothing (verified incrementally instead).
            SlotBefore | SlotAfter | NotEqual | DifferentSlots | Disjoint | Before | After
            | FeatureCompare { .. } | FeatureRegexCompare { .. } => return Ok(None),
        };
        Ok(Some(driver))
    }

    /// Drive one side of an edge relation: when `forward` is true the candidates are
    /// the edge targets of `other` (binary-search rows), else the edge sources.
    fn edge_driver(
        &self,
        edge_adj: &HashMap<String, EdgeAdj>,
        name: &str,
        other: u32,
        forward: bool,
        cset: &CandidateSet,
    ) -> Vec<u32> {
        let Some(adj) = edge_adj.get(name) else {
            return Vec::new();
        };
        let map = if forward { &adj.fwd } else { &adj.bwd };
        match map.get(&other) {
            Some(nodes) => nodes.iter().copied().filter(|n| cset.contains(*n)).collect(),
            None => Vec::new(),
        }
    }

    fn node_interval(
        &self,
        slot_cache: &mut HashMap<u32, Option<(u32, u32)>>,
        oslots: &EdgeFeatureView<'_>,
        otype: &StringPoolNodeFeatureView<'_>,
        node: u32,
    ) -> Result<Option<(u32, u32)>> {
        if let Some(value) = slot_cache.get(&node) {
            return Ok(*value);
        }
        let slots = self.node_slots(oslots, otype, node)?;
        let value = match (slots.first(), slots.last()) {
            (Some(&first), Some(&last)) => Some((first, last)),
            _ => None,
        };
        slot_cache.insert(node, value);
        Ok(value)
    }

    fn build_candidate_set(
        &self,
        nodes: Vec<u32>,
        otype: &StringPoolNodeFeatureView<'_>,
        oslots: &EdgeFeatureView<'_>,
    ) -> Result<CandidateSet> {
        let entries = nodes
            .into_iter()
            .map(|node| {
                let slots = self.node_slots(oslots, otype, node)?;
                Ok(CandidateEntry {
                    node,
                    first_slot: slots.first().copied(),
                    last_slot: slots.last().copied(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(CandidateSet::new(entries))
    }

    fn build_edge_adjacency(
        &self,
        plan: &MappedRelationPlan,
    ) -> Result<HashMap<String, EdgeAdj>> {
        let mut names: HashSet<&str> = HashSet::new();
        for relation in &plan.relations {
            match &relation.operator {
                MappedRelationOperator::EdgeForward(name)
                | MappedRelationOperator::EdgeBackward(name)
                | MappedRelationOperator::EdgeEither(name)
                | MappedRelationOperator::EdgeForwardValue(name, _)
                | MappedRelationOperator::EdgeBackwardValue(name, _)
                | MappedRelationOperator::EdgeEitherValue(name, _) => {
                    names.insert(name.as_str());
                }
                _ => {}
            }
        }
        let mut out = HashMap::new();
        for name in names {
            let Some(edge) = self.corpus.edge_feature(name)? else {
                continue;
            };
            let mut fwd: HashMap<u32, Vec<u32>> = HashMap::new();
            let mut bwd: HashMap<u32, Vec<u32>> = HashMap::new();
            for (source, targets) in edge.items()? {
                for target in &targets {
                    bwd.entry(*target).or_default().push(source);
                }
                fwd.insert(source, targets);
            }
            out.insert(name.to_string(), EdgeAdj { fwd, bwd });
        }
        Ok(out)
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
        let plan = parse_mapped_plan(template)?;
        self.search_plan(&plan, otype, oslots, sets, limit)
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

/// Forward/backward adjacency for an edge feature, used to drive a join from the
/// (sparse) edge rows rather than verifying it over the candidate cross product.
struct EdgeAdj {
    fwd: HashMap<u32, Vec<u32>>,
    bwd: HashMap<u32, Vec<u32>>,
}

struct CandidateSet {
    entries: Vec<CandidateEntry>,
    by_first_slot: Vec<usize>,
    by_last_slot: Vec<usize>,
    member: HashSet<u32>,
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
        let mut by_last_slot = (0..entries.len()).collect::<Vec<_>>();
        by_last_slot.sort_by_key(|index| {
            (
                entries[*index].last_slot.unwrap_or(u32::MAX),
                entries[*index].first_slot.unwrap_or(u32::MAX),
                entries[*index].node,
            )
        });
        let member = entries.iter().map(|entry| entry.node).collect();
        Self {
            entries,
            by_first_slot,
            by_last_slot,
            member,
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn contains(&self, node: u32) -> bool {
        self.member.contains(&node)
    }

    fn nodes(&self) -> Vec<u32> {
        self.entries.iter().map(|entry| entry.node).collect()
    }

    /// Candidates with `first_slot` in `[lo, hi]` (binary search over the
    /// first-slot index). Entries without a first slot are excluded.
    fn nodes_with_first_in(&self, lo: u32, hi: u32) -> Vec<u32> {
        if lo > hi {
            return Vec::new();
        }
        let start = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|first| first < lo)
        });
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|first| first <= hi)
        });
        self.by_first_slot[start..end]
            .iter()
            .map(|index| self.entries[*index].node)
            .collect()
    }

    /// Candidates with `last_slot` in `[lo, hi]` (binary search over the last-slot
    /// index). Entries without a last slot are excluded.
    fn nodes_with_last_in(&self, lo: u32, hi: u32) -> Vec<u32> {
        if lo > hi {
            return Vec::new();
        }
        let start = self
            .by_last_slot
            .partition_point(|index| self.entries[*index].last_slot.is_some_and(|last| last < lo));
        let end = self
            .by_last_slot
            .partition_point(|index| self.entries[*index].last_slot.is_some_and(|last| last <= hi));
        self.by_last_slot[start..end]
            .iter()
            .map(|index| self.entries[*index].node)
            .collect()
    }

    /// Candidates contained in `[first_slot, last_slot]` (first >= first_slot and
    /// last <= last_slot).
    fn nodes_within_slot_interval(&self, first_slot: u32, last_slot: u32) -> Vec<u32> {
        let start = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|first| first < first_slot)
        });
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|first| first <= last_slot)
        });
        self.by_first_slot[start..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index];
                (entry.last_slot? <= last_slot).then_some(entry.node)
            })
            .collect()
    }

    /// Candidates whose interval contains `[first, last]` (first_slot <= first and
    /// last_slot >= last).
    fn nodes_containing(&self, first: u32, last: u32) -> Vec<u32> {
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|candidate_first| candidate_first <= first)
        });
        self.by_first_slot[..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index];
                (entry.last_slot? >= last).then_some(entry.node)
            })
            .collect()
    }

    /// Candidates whose interval overlaps `[first, last]` (first_slot <= last and
    /// last_slot >= first).
    fn nodes_overlapping(&self, first: u32, last: u32) -> Vec<u32> {
        let end = self.by_first_slot.partition_point(|index| {
            self.entries[*index]
                .first_slot
                .is_some_and(|candidate_first| candidate_first <= last)
        });
        self.by_first_slot[..end]
            .iter()
            .filter_map(|index| {
                let entry = &self.entries[*index];
                (entry.last_slot? >= first).then_some(entry.node)
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
    /// Quantifier blocks attached to this atom (TF: an atom's `quantifiers` list).
    /// Each reduces the atom's candidate yarn via global set algebra before the
    /// join (TF spin.py:_doQuantifier).
    quantifiers: Vec<TextQuantifierBlock>,
    /// `(clean_atom_line, parent_name)` for this atom, used to build quantifier
    /// sub-search templates. Set only when `quantifiers` is non-empty.
    clean: Option<(String, String)>,
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

    /// Re-serialize as a template token (inverse of `parse_simple_constraint`), used
    /// to rebuild a host atom's clean line for quantifier sub-searches.
    fn to_token(&self) -> String {
        let feature = &self.feature;
        match &self.matcher {
            MappedMatcher::Eq(values) => format!("{feature}={}", values.join("|")),
            MappedMatcher::Ne(values) => format!("{feature}#{}", values.join("|")),
            MappedMatcher::Regex(regex) => format!("{feature}~{}", regex.as_str()),
            MappedMatcher::Exists => feature.clone(),
            MappedMatcher::Missing => format!("{feature}#"),
            MappedMatcher::Lt(value) => format!("{feature}<{value}"),
            MappedMatcher::Gt(value) => format!("{feature}>{value}"),
        }
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
        if is_quantifier_init_token(trimmed) {
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

fn nearest_bound_parent_index(atoms: &[SimpleAtom], index: usize) -> Option<usize> {
    let indent = atoms[index].indent;
    if indent == 0 {
        return None;
    }
    (0..index)
        .rev()
        .find(|previous| atoms[*previous].indent < indent)
}

fn endpoint_node(
    endpoint: &MappedRelationEndpoint,
    bound: &[Option<u32>],
    root: Option<u32>,
) -> Option<u32> {
    match endpoint {
        MappedRelationEndpoint::Atom(index) => bound[*index],
        MappedRelationEndpoint::Parent => root,
    }
}

/// Driving selectivity of a relation operator: 0 = cannot drive (verify only),
/// 1 = open-ended slot window (`<<`, `>>`, `&&` — yields a large half-corpus set),
/// 2 = tightly bounded driver (edges, adjacency, same/near boundary, embedding).
/// Used to order the join so a non-seed atom is reached by its most selective
/// driver, never an open-ended one when a tight one exists.
fn operator_drive_strength(operator: &MappedRelationOperator) -> u32 {
    use MappedRelationOperator::*;
    match operator {
        NotEqual | DifferentSlots | Disjoint | Before | After | FeatureCompare { .. }
        | FeatureRegexCompare { .. } => 0,
        SlotBefore | SlotAfter | Overlaps => 1,
        _ => 2,
    }
}

/// Candidate container nodes for a child interval `[first, last]`. With a slot
/// index (built for container atoms) this is an exact, O(1) lookup on the child's
/// first slot followed by interval verification; otherwise it falls back to the
/// interval scan.
fn containers_of(
    cmap: Option<&HashMap<u32, Vec<u32>>>,
    cset: &CandidateSet,
    first: u32,
    last: u32,
) -> Vec<u32> {
    match cmap {
        Some(map) => match map.get(&first) {
            Some(nodes) => nodes.iter().copied().filter(|n| cset.contains(*n)).collect(),
            None => Vec::new(),
        },
        None => cset.nodes_containing(first, last),
    }
}

fn add1(value: u32) -> Option<u32> {
    value.checked_add(1)
}

fn sub1(value: u32) -> Option<u32> {
    value.checked_sub(1)
}

/// Clamp `target - k` to a valid `u32` lower bound (≥ 0).
fn clamp_lo(target: i64, k: u32) -> u32 {
    (target - i64::from(k)).max(0) as u32
}

/// Clamp `target + k` to a valid `u32` upper bound.
fn clamp_hi(target: i64, k: u32) -> u32 {
    (target + i64::from(k)).clamp(0, i64::from(u32::MAX)) as u32
}

/// Best driving strength for placing `i` given the already-placed atoms (and the
/// bound parent reference). 0 means `i` is not yet reachable by any driver.
/// Embedding (both directions) is a tight driver (strength 2).
fn connection_strength(
    i: usize,
    placed: &[bool],
    plan: &MappedRelationPlan,
    embeds: &[Option<usize>],
    root: Option<u32>,
) -> u32 {
    let mut best = 0;
    if let Some(parent) = embeds[i] {
        if placed[parent] {
            best = 2;
        }
    }
    for (child, child_parent) in embeds.iter().enumerate() {
        if *child_parent == Some(i) && placed[child] {
            best = 2;
        }
    }
    for relation in &plan.relations {
        let strength = operator_drive_strength(&relation.operator);
        if strength == 0 || strength <= best {
            continue;
        }
        let other = if matches!(relation.left, MappedRelationEndpoint::Atom(a) if a == i) {
            &relation.right
        } else if matches!(relation.right, MappedRelationEndpoint::Atom(a) if a == i) {
            &relation.left
        } else {
            continue;
        };
        let reachable = match other {
            MappedRelationEndpoint::Atom(o) => placed[*o],
            MappedRelationEndpoint::Parent => root.is_some(),
        };
        if reachable {
            best = best.max(strength);
        }
    }
    best
}

/// Greedy evaluation order: repeatedly place the atom reachable by the most
/// selective driver (breaking ties by smallest candidate set); fall back to the
/// smallest remaining set as a new seed for disconnected components.
fn compute_join_order(
    plan: &MappedRelationPlan,
    candidates: &[CandidateSet],
    embeds: &[Option<usize>],
    root: Option<u32>,
) -> Vec<usize> {
    let n = plan.atoms.len();
    let sizes: Vec<usize> = candidates.iter().map(CandidateSet::len).collect();
    let mut placed = vec![false; n];
    let mut order = Vec::with_capacity(n);
    for _ in 0..n {
        let mut best: Option<(usize, u32)> = None;
        for i in 0..n {
            if placed[i] {
                continue;
            }
            let strength = connection_strength(i, &placed, plan, embeds, root);
            if strength == 0 {
                continue;
            }
            best = Some(match best {
                Some((b, bs)) if (bs, std::cmp::Reverse(sizes[b])) >= (strength, std::cmp::Reverse(sizes[i])) => {
                    (b, bs)
                }
                _ => (i, strength),
            });
        }
        let pick = best.map(|(i, _)| i).unwrap_or_else(|| {
            (0..n)
                .filter(|i| !placed[*i])
                .min_by_key(|i| sizes[*i])
                .expect("an unplaced atom remains")
        });
        placed[pick] = true;
        order.push(pick);
    }
    order
}

const PARENT_NAME: &str = "__cf_parent__";

fn is_quantifier_init_token(token: &str) -> bool {
    matches!(token, "/where/" | "/with/" | "/without/")
}

/// A clean, named atom line for `atom` (TF cleanParent), plus the name that `..`
/// parent references inside its quantifier blocks resolve to.
fn clean_host_atom(atom: &SimpleAtom) -> (String, String) {
    let parent_name = atom
        .name
        .clone()
        .unwrap_or_else(|| PARENT_NAME.to_string());
    let mut line = format!("{parent_name}:{}", atom.node_type);
    for constraint in &atom.constraints {
        line.push(' ');
        line.push_str(&constraint.to_token());
    }
    (line, parent_name)
}

/// Left operand of an operator-prefixed line: the previous sibling at this indent,
/// else the nearest enclosing parent (TF semantics.py).
fn sibling_or_parent(
    indent: usize,
    atom_stack: &std::collections::BTreeMap<usize, usize>,
) -> Result<usize> {
    atom_stack
        .get(&indent)
        .copied()
        .or_else(|| atom_stack.range(..indent).next_back().map(|(_, idx)| *idx))
        .ok_or_else(|| {
            CfError::InvalidQuery("Lonely relation: not allowed at outermost level".to_string())
        })
}

/// Read a quantifier block starting at `lines[start]` (the keyword) until its
/// matching `/-/`, stripping `kw_indent` leading spaces from body lines (TF strips
/// by the outermost quantifier keyword's indent). Returns the parsed block and the
/// index just past the terminator.
fn collect_quantifier_block(
    lines: &[&str],
    start: usize,
    kw_indent: usize,
) -> Result<(ParsedQuantifierBlock, usize)> {
    let kind = match lines[start].trim() {
        "/where/" => MappedQuantifierKind::Where,
        "/without/" => MappedQuantifierKind::Without,
        _ => MappedQuantifierKind::With,
    };
    let mut alternatives: Vec<String> = Vec::new();
    let mut consequents: Vec<String> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    let mut collecting_consequents = false;
    let mut depth = 0usize;
    let mut i = start + 1;
    while i < lines.len() {
        let token = lines[i].trim();
        if depth == 0 {
            if token == "/-/" {
                push_block_alternative(
                    if collecting_consequents {
                        &mut consequents
                    } else {
                        &mut alternatives
                    },
                    &cur,
                );
                if matches!(kind, MappedQuantifierKind::Where)
                    && (alternatives.is_empty() || consequents.is_empty())
                {
                    return Err(CfError::InvalidQuery(
                        "/where/ requires antecedent and /have/ consequent templates".to_string(),
                    ));
                }
                return Ok((
                    ParsedQuantifierBlock {
                        kind,
                        alternatives,
                        consequents,
                    },
                    i + 1,
                ));
            }
            if token == "/have/" {
                push_block_alternative(
                    if collecting_consequents {
                        &mut consequents
                    } else {
                        &mut alternatives
                    },
                    &cur,
                );
                cur.clear();
                collecting_consequents = true;
                i += 1;
                continue;
            }
            if token == "/or/" {
                push_block_alternative(
                    if collecting_consequents {
                        &mut consequents
                    } else {
                        &mut alternatives
                    },
                    &cur,
                );
                cur.clear();
                i += 1;
                continue;
            }
            if is_quantifier_init_token(token) {
                depth += 1;
            }
            cur.push(strip_indent(lines[i], kw_indent));
            i += 1;
            continue;
        }
        // Inside a nested quantifier: pass lines through verbatim (stripped by the
        // outer keyword indent) and track nesting.
        if is_quantifier_init_token(token) {
            depth += 1;
        } else if token == "/-/" {
            depth -= 1;
        }
        cur.push(strip_indent(lines[i], kw_indent));
        i += 1;
    }
    Err(CfError::InvalidQuery(
        "unterminated mapped quantified block".to_string(),
    ))
}

fn push_block_alternative(target: &mut Vec<String>, lines: &[String]) {
    if lines.iter().any(|line| !line.trim().is_empty()) {
        target.push(lines.join("\n"));
    }
}

fn strip_indent(line: &str, n: usize) -> String {
    let lead = line.chars().take_while(|ch| ch.is_whitespace()).count();
    line.chars().skip(lead.min(n)).collect()
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

/// Combine the clean host-atom line (at indent 0) with a quantifier sub-template,
/// preserving the sub-template's keyword-relative indentation (TF joins cleanAtom +
/// quTemplates without re-indenting).
fn combine_quantifier_subtemplate(clean_atom: &str, sub: &str) -> String {
    let mut lines = vec![clean_atom.to_string()];
    for line in sub.lines() {
        if line.trim().is_empty() {
            continue;
        }
        lines.push(line.to_string());
    }
    lines.join("\n")
}

struct ParsedQuantifierBlock {
    kind: MappedQuantifierKind,
    alternatives: Vec<String>,
    consequents: Vec<String>,
}

fn combine_quantifier_templates(first: &str, second: &str) -> String {
    match (first.trim().is_empty(), second.trim().is_empty()) {
        (true, true) => String::new(),
        (true, false) => second.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first}\n{second}"),
    }
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
        quantifiers: Vec::new(),
        clean: None,
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
