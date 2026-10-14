//! The backtracking join: binds atoms one at a time, each driven by an
//! already-bound neighbour through relations, embedding, or edges.

use std::collections::{HashMap, HashSet};

use crate::error::{CfError, Result};
use crate::mapped::{EdgeFeatureView, MappedCompiledCorpus};
use crate::search::candidates::CandidateSet;
use crate::search::plan::{
    MappedRelationEndpoint, MappedRelationOperator, MappedRelationPlan, nearest_bound_parent_index,
};
use crate::search::{MappedSearch, SearchSets};

/// Where the backtracking join emits complete assignments: materialized rows
/// for `search`/`fetch`, a bare counter for `count`. Counting skips the
/// per-result `Vec` allocation entirely (424k rows on `q_feat_empty_qere`).
pub(super) enum JoinSink {
    Rows(Vec<Vec<u32>>),
    Count(usize),
}

impl JoinSink {
    pub(super) fn emit(&mut self, bound: &[Option<u32>]) {
        match self {
            Self::Rows(rows) => {
                rows.push(bound.iter().map(|node| node.expect("bound")).collect());
            }
            Self::Count(count) => *count += 1,
        }
    }

    pub(super) fn emitted(&self) -> usize {
        match self {
            Self::Rows(rows) => rows.len(),
            Self::Count(count) => *count,
        }
    }
}

impl<'a> MappedSearch<'a> {
    pub(super) fn search_plan(
        &self,
        plan: &MappedRelationPlan,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        match self.search_plan_with_root(plan, sets, limit, None, false)? {
            JoinSink::Rows(rows) => Ok(rows),
            JoinSink::Count(_) => unreachable!("rows mode requested"),
        }
    }

    /// Run the plan in count-only mode: the join executes identically but emits
    /// a counter instead of materializing result rows. With `limit`, counting
    /// stops early — `count(limit)` is `min(total, limit)` either way.
    pub(super) fn count_plan(
        &self,
        plan: &MappedRelationPlan,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<usize> {
        match self.search_plan_with_root(plan, sets, limit, None, true)? {
            JoinSink::Count(count) => Ok(count),
            JoinSink::Rows(rows) => Ok(rows.len()),
        }
    }

    pub(super) fn search_plan_with_root(
        &self,
        plan: &MappedRelationPlan,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
        // Bound node for the `..` parent reference (`MappedRelationEndpoint::Parent`)
        // when this plan is a sub-search; `None` for top-level plans.
        root: Option<u32>,
        count_only: bool,
    ) -> Result<JoinSink> {
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
                let nodes = self.search_atom(atom, &constraints, sets, None)?;
                candidate_row_cache.insert(cache_key, nodes.clone());
                nodes
            };
            if !atom.quantifiers.is_empty() {
                let (clean, _) = atom.clean.as_ref().ok_or_else(|| {
                    CfError::InvalidQuery("quantified atom missing clean line".to_string())
                })?;
                let mut yarn: HashSet<u32> = nodes.iter().copied().collect();
                for block in &atom.quantifiers {
                    yarn = self.apply_quantifier_block(clean, block, yarn, sets)?;
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
            let nodes = candidate_nodes.pop().unwrap_or_default();
            let capped = limit.unwrap_or(nodes.len()).min(nodes.len());
            if count_only {
                return Ok(JoinSink::Count(capped));
            }
            return Ok(JoinSink::Rows(
                nodes
                    .into_iter()
                    .take(capped)
                    .map(|node| vec![node])
                    .collect(),
            ));
        }

        // 2. Candidate sets with slot intervals (always; driving needs them).
        let candidates = candidate_nodes
            .into_iter()
            .map(|nodes| self.build_candidate_set(nodes))
            .collect::<Result<Vec<_>>>()?;

        // 3. Embedding parent per atom (nearest enclosing smaller indent).
        let embeds: Vec<Option<usize>> = (0..n)
            .map(|i| nearest_bound_parent_index(&plan.atoms, i))
            .collect();

        // 4. Edge feature views for any edge relations: drivers read forward rows
        //    and the reverse index straight from the map.
        let edge_views = self.edge_views(plan)?;

        // 5. Connectivity-greedy evaluation order so every non-seed atom is driven
        //    from an already-bound neighbour rather than enumerated wholesale.
        let order = compute_join_order(plan, &candidates, &embeds, root);

        // 6. Backtracking join driven by relations/embeddings.
        Join {
            search: self,
            plan,
            candidates,
            embeds,
            edge_views,
            order,
            limit,
            root,
            bound: vec![None; n],
            results: if count_only {
                JoinSink::Count(0)
            } else {
                JoinSink::Rows(Vec::new())
            },
            slots_cache: HashMap::new(),
        }
        .run()
    }

    /// Slot-containment check (`parent` embeds `child`), mirroring
    /// `contains_by_slots` but caching the parent's (potentially huge) slot list so
    /// it is materialized once per container — critical when many children share a
    /// container such as `lex`. A slot embeds nothing: `self.oslots.targets` is empty for
    /// slot parents (unlike `node_slots`, which returns slot-identity).
    pub(super) fn contains_cached(
        &self,
        slots_cache: &mut HashMap<u32, Vec<u32>>,
        parent: u32,
        child: u32,
    ) -> Result<bool> {
        let child_slots = self.node_slots(child)?;
        if child_slots.is_empty() {
            return Ok(false);
        }
        let parent_slots = self.cached_oslots_targets(slots_cache, parent)?;
        if parent_slots.is_empty() {
            return Ok(false);
        }
        Ok(child_slots
            .iter()
            .all(|slot| parent_slots.binary_search(slot).is_ok()))
    }

    pub(super) fn cached_oslots_targets<'c>(
        &self,
        slots_cache: &'c mut HashMap<u32, Vec<u32>>,
        node: u32,
    ) -> Result<&'c Vec<u32>> {
        if let std::collections::hash_map::Entry::Vacant(e) = slots_cache.entry(node) {
            let targets = match self.oslots.targets(node)? {
                Some(targets) => targets.collect::<Result<Vec<_>>>()?,
                None => Vec::new(),
            };
            e.insert(targets);
        }
        Ok(slots_cache.get(&node).expect("just inserted"))
    }

    /// The node's `(first_slot, last_slot)` without materializing its slot list:
    /// identity for slots, two CSR reads after one binary search otherwise.
    pub(super) fn fast_interval(&self, node: u32) -> Result<Option<(u32, u32)>> {
        if node == 0 {
            return Ok(None);
        }
        if node <= self.corpus.max_slot()? {
            return Ok(Some((node, node)));
        }
        self.oslots.first_last_targets(node)
    }

    pub(super) fn edge_views(&self, plan: &MappedRelationPlan) -> Result<EdgeViews<'a>> {
        let mut views = EdgeViews::new();
        for relation in &plan.relations {
            match &relation.operator {
                MappedRelationOperator::EdgeForward(name)
                | MappedRelationOperator::EdgeBackward(name)
                | MappedRelationOperator::EdgeEither(name)
                | MappedRelationOperator::EdgeForwardValue(name, _)
                | MappedRelationOperator::EdgeBackwardValue(name, _)
                | MappedRelationOperator::EdgeEitherValue(name, _) => {
                    if !views.contains_key(name.as_str())
                        && let Some(edge) = self.corpus.edge_feature(name)?
                    {
                        views.insert(name.clone(), edge);
                    }
                }
                _ => {}
            }
        }
        Ok(views)
    }
}

/// One backtracking join: binds the plan's atoms in join order, each from the
/// candidates its bound neighbours drive, and emits every complete binding.
struct Join<'s, 'a> {
    search: &'s MappedSearch<'a>,
    plan: &'s MappedRelationPlan,
    candidates: Vec<CandidateSet>,
    /// The embedding parent of each atom (its nearest enclosing atom).
    embeds: Vec<Option<usize>>,
    edge_views: EdgeViews<'a>,
    /// Atom indices in binding order.
    order: Vec<usize>,
    limit: Option<usize>,
    /// The bound node of the `..` parent reference in a sub-search.
    root: Option<u32>,
    bound: Vec<Option<u32>>,
    results: JoinSink,
    /// Slots of container nodes, read once per container.
    slots_cache: HashMap<u32, Vec<u32>>,
}

impl Join<'_, '_> {
    fn run(mut self) -> Result<JoinSink> {
        self.extend(0)?;
        Ok(self.results)
    }

    fn limit_reached(&self) -> bool {
        self.limit
            .is_some_and(|limit| self.results.emitted() >= limit)
    }

    /// Binds the atom at `position` in the join order to each of its driven
    /// candidates that satisfies the constraints it completes, and recurses.
    fn extend(&mut self, position: usize) -> Result<()> {
        if self.limit_reached() {
            return Ok(());
        }
        if position == self.order.len() {
            // Every constraint was verified as its last endpoint bound, so a
            // fully-bound assignment is a match. Emit in atom order.
            self.results.emit(&self.bound);
            return Ok(());
        }
        let atom = self.order[position];
        for node in self.drive_candidates(atom)? {
            self.bound[atom] = Some(node);
            if self.constraints_hold(atom)? {
                self.extend(position + 1)?;
            }
            self.bound[atom] = None;
            if self.limit_reached() {
                break;
            }
        }
        Ok(())
    }

    /// Check every constraint that becomes fully bound by binding `atom`: relations
    /// where both endpoints are now known, plus the embedding to its bound parent
    /// and from any bound child. Each constraint is thus verified exactly once, when
    /// its last endpoint binds.
    fn constraints_hold(&mut self, atom: usize) -> Result<bool> {
        let node = self.bound[atom].expect("atom just bound");
        for relation in &self.plan.relations {
            let involves = matches!(relation.left, MappedRelationEndpoint::Atom(a) if a == atom)
                || matches!(relation.right, MappedRelationEndpoint::Atom(a) if a == atom);
            if !involves {
                continue;
            }
            let (Some(left), Some(right)) = (
                endpoint_node(&relation.left, &self.bound, self.root),
                endpoint_node(&relation.right, &self.bound, self.root),
            ) else {
                continue;
            };
            // Slot-containment verifications collect the (potentially huge) container
            // slot list; route them through the cache so each container's slots are
            // read once. Common with `lex` containers shared across many words.
            let ok = match relation.operator {
                MappedRelationOperator::Embeds => {
                    self.search
                        .contains_cached(&mut self.slots_cache, left, right)?
                }
                MappedRelationOperator::EmbeddedIn => {
                    self.search
                        .contains_cached(&mut self.slots_cache, right, left)?
                }
                _ => self.search.relation_holds(relation, left, right),
            };
            if !ok {
                return Ok(false);
            }
        }
        if let Some(parent) = self.embeds[atom]
            && let Some(parent_node) = self.bound[parent]
            && !self
                .search
                .contains_cached(&mut self.slots_cache, parent_node, node)?
        {
            return Ok(false);
        }
        for (child, child_parent) in self.embeds.iter().enumerate() {
            if *child_parent == Some(atom)
                && let Some(child_node) = self.bound[child]
                && !self
                    .search
                    .contains_cached(&mut self.slots_cache, node, child_node)?
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Generate the candidate nodes for `atom` by intersecting every driver that a
    /// currently-bound neighbour provides (embedding both directions, edge rows,
    /// slot-window relations). With no driver the full candidate set is returned
    /// (only the seed atom hits this).
    fn drive_candidates(&self, atom: usize) -> Result<Vec<u32>> {
        let cset = &self.candidates[atom];
        let mut drivers: Vec<Vec<u32>> = Vec::new();

        // Forward embedding: `atom` is contained in its bound parent.
        if let Some(parent) = self.embeds[atom]
            && let Some(parent_node) = self.bound[parent]
        {
            let Some((pf, pl)) = self.search.fast_interval(parent_node)? else {
                return Ok(Vec::new());
            };
            drivers.push(cset.nodes_within_slot_interval(pf, pl));
        }
        // Reverse embedding: `atom` contains a bound child.
        for (child, child_parent) in self.embeds.iter().enumerate() {
            if *child_parent == Some(atom)
                && let Some(child_node) = self.bound[child]
            {
                let Some((cf, cl)) = self.search.fast_interval(child_node)? else {
                    return Ok(Vec::new());
                };
                drivers.push(containers_of(self.search.corpus, cset, cf, cl)?);
            }
        }
        // Relations with a bound other endpoint.
        for relation in &self.plan.relations {
            let (is_left, other_ep) = if matches!(relation.left, MappedRelationEndpoint::Atom(a) if a == atom)
            {
                (true, &relation.right)
            } else if matches!(relation.right, MappedRelationEndpoint::Atom(a) if a == atom) {
                (false, &relation.left)
            } else {
                continue;
            };
            let Some(other) = endpoint_node(other_ep, &self.bound, self.root) else {
                continue;
            };
            if let Some(driver) = self.relation_driver(&relation.operator, is_left, other, cset)? {
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
    fn relation_driver(
        &self,
        operator: &MappedRelationOperator,
        is_left: bool,
        other: u32,
        cset: &CandidateSet,
    ) -> Result<Option<Vec<u32>>> {
        use crate::search::plan::MappedRelationOperator::*;
        let interval = self.search.fast_interval(other)?;
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
        let driver =
            match operator {
                Equal => {
                    if cset.contains(other) {
                        vec![other]
                    } else {
                        vec![]
                    }
                }
                Embeds => {
                    if is_left {
                        containers_of(self.search.corpus, cset, of, ol)?
                    } else {
                        cset.nodes_within_slot_interval(of, ol)
                    }
                }
                EmbeddedIn => {
                    if is_left {
                        cset.nodes_within_slot_interval(of, ol)
                    } else {
                        containers_of(self.search.corpus, cset, of, ol)?
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
                NearFirstSlot(k) => cset
                    .nodes_with_first_in(clamp_lo(i64::from(of), *k), clamp_hi(i64::from(of), *k)),
                NearLastSlot(k) => cset
                    .nodes_with_last_in(clamp_lo(i64::from(ol), *k), clamp_hi(i64::from(ol), *k)),
                SameBoundary | SameSlots => cset.nodes_with_first_in(of, of),
                NearBoundary(k) => cset
                    .nodes_with_first_in(clamp_lo(i64::from(of), *k), clamp_hi(i64::from(of), *k)),
                // EdgeForward: edge runs left -> right. If `atom` is the right operand
                // (is_left = false) it is a forward target of `other`; if it is the left
                // operand it is a backward source of `other`.
                EdgeForward(name) | EdgeForwardValue(name, _) => {
                    self.edge_driver(name, other, !is_left, cset)?
                }
                // EdgeBackward: edge runs right -> left (the relation holds when an edge
                // goes from the right operand to the left). If `atom` is the left operand
                // it is a forward target of `other`; otherwise a backward source.
                EdgeBackward(name) | EdgeBackwardValue(name, _) => {
                    self.edge_driver(name, other, is_left, cset)?
                }
                EdgeEither(name) | EdgeEitherValue(name, _) => {
                    let mut nodes = self.edge_driver(name, other, is_left, cset)?;
                    nodes.extend(self.edge_driver(name, other, !is_left, cset)?);
                    nodes.sort_unstable();
                    nodes.dedup();
                    nodes
                }
                // `<<` / `>>` produce open-ended ~half-corpus windows; intersecting such
                // a large driver per seed is far more expensive than verifying the
                // relation, so they drive nothing (verified incrementally instead).
                SlotBefore
                | SlotAfter
                | NotEqual
                | DifferentSlots
                | Disjoint
                | Before
                | After
                | FeatureCompare { .. }
                | FeatureRegexCompare { .. } => return Ok(None),
            };
        Ok(Some(driver))
    }

    /// Drive one side of an edge relation: when `forward` is true the candidates are
    /// the edge targets of `other` (forward row), else its edge sources (reverse
    /// index). Both come back in canonical order without duplicates.
    fn edge_driver(
        &self,
        name: &str,
        other: u32,
        forward: bool,
        cset: &CandidateSet,
    ) -> Result<Vec<u32>> {
        let Some(edge) = self.edge_views.get(name) else {
            return Ok(Vec::new());
        };
        let mut nodes = if forward {
            let mut targets = edge.f(other)?;
            targets.dedup();
            targets
        } else {
            edge.t(other)?
        };
        nodes.retain(|node| cset.contains(*node));
        Ok(nodes)
    }
}

/// The edge features a query's relations name, resolved once per query. Used
/// to drive a join from the (sparse) edge rows rather than verifying it over the
/// candidate cross product.
pub(super) type EdgeViews<'a> = HashMap<String, EdgeFeatureView<'a>>;

pub(super) fn endpoint_node(
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
pub(super) fn operator_drive_strength(operator: &MappedRelationOperator) -> u32 {
    use crate::search::plan::MappedRelationOperator::*;
    match operator {
        NotEqual
        | DifferentSlots
        | Disjoint
        | Before
        | After
        | FeatureCompare { .. }
        | FeatureRegexCompare { .. } => 0,
        SlotBefore | SlotAfter | Overlaps => 1,
        _ => 2,
    }
}

/// Candidate container nodes for a child interval `[first, last]`: the
/// embedders of its first slot from the `levUp` index, falling back to the
/// interval scan for a slot outside the index.
pub(super) fn containers_of(
    corpus: &MappedCompiledCorpus,
    cset: &CandidateSet,
    first: u32,
    last: u32,
) -> Result<Vec<u32>> {
    if let Some(nodes) = lev_up_containers(corpus, cset, first)? {
        return Ok(nodes);
    }
    Ok(cset.nodes_containing(first, last))
}

/// Containers of the slot `first` drawn from the precomputed `levUp` CSR row
/// (every embedder of that slot), filtered to the atom's candidates and sorted
/// ascending.
pub(super) fn lev_up_containers(
    corpus: &MappedCompiledCorpus,
    cset: &CandidateSet,
    first: u32,
) -> Result<Option<Vec<u32>>> {
    let Some(row) = corpus.lev_up_row(first)? else {
        return Ok(None);
    };
    let mut nodes: Vec<u32> = row
        .into_iter()
        .filter(|node| cset.contains(*node))
        .collect();
    nodes.sort_unstable();
    Ok(Some(nodes))
}

pub(super) fn add1(value: u32) -> Option<u32> {
    value.checked_add(1)
}

pub(super) fn sub1(value: u32) -> Option<u32> {
    value.checked_sub(1)
}

/// Clamp `target - k` to a valid `u32` lower bound (≥ 0).
pub(super) fn clamp_lo(target: i64, k: u32) -> u32 {
    (target - i64::from(k)).max(0) as u32
}

/// Clamp `target + k` to a valid `u32` upper bound.
pub(super) fn clamp_hi(target: i64, k: u32) -> u32 {
    (target + i64::from(k)).clamp(0, i64::from(u32::MAX)) as u32
}

/// Best driving strength for placing `i` given the already-placed atoms (and the
/// bound parent reference). 0 means `i` is not yet reachable by any driver.
/// Embedding (both directions) is a tight driver (strength 2).
pub(super) fn connection_strength(
    i: usize,
    placed: &[bool],
    plan: &MappedRelationPlan,
    embeds: &[Option<usize>],
    root: Option<u32>,
) -> u32 {
    let mut best = 0;
    if let Some(parent) = embeds[i]
        && placed[parent]
    {
        best = 2;
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
pub(super) fn compute_join_order(
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
                Some((b, bs))
                    if (bs, std::cmp::Reverse(sizes[b]))
                        >= (strength, std::cmp::Reverse(sizes[i])) =>
                {
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
