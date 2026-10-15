//! The backtracking join: binds atoms one at a time, each driven by an
//! already-bound neighbour through relations, embedding, or edges. The join
//! is resumable, so matches can be produced in batches of any size.

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;

use crate::error::{CfError, Result};
use crate::mapped::{EdgeFeatureView, MappedCompiledCorpus};
use crate::search::candidates::CandidateSet;
use crate::search::interrupt::InterruptPoll;
use crate::search::plan::{MappedRelationOperator, MappedRelationPlan, nearest_bound_parent_index};
use crate::search::{MappedSearch, SearchSets};

/// A join in progress: everything it needs between batches of matches, owned
/// so that it can be resumed. Made by `MappedSearch::start_join` and advanced
/// with a search over the same corpus.
pub(super) enum JoinState {
    /// A template of one atom without relations: its candidates are the
    /// matches.
    Single { nodes: Vec<u32>, next: usize },
    /// The backtracking join over several atoms.
    Multi(Box<MultiJoin>),
}

/// The backtracking join's state: atoms are bound in join order, each from the
/// candidates its bound neighbours drive; `frames[d]` holds the remaining
/// candidates of the atom at position `d`.
pub(super) struct MultiJoin {
    plan: MappedRelationPlan,
    candidates: Vec<CandidateSet>,
    /// The embedding parent of each atom (its nearest enclosing atom).
    embeds: Vec<Option<usize>>,
    /// Atom indices in binding order.
    order: Vec<usize>,
    bound: Vec<Option<u32>>,
    frames: Vec<Frame>,
    started: bool,
    /// Slots of container nodes, read once per container.
    slots_cache: HashMap<u32, Vec<u32>>,
}

/// The candidates of one join position and the next one to try.
struct Frame {
    candidates: Vec<u32>,
    next: usize,
}

impl JoinState {
    /// The atoms in binding order, each with its number of candidates.
    pub(super) fn binding_order(&self) -> Vec<(usize, usize)> {
        match self {
            Self::Single { nodes, .. } => vec![(0, nodes.len())],
            Self::Multi(join) => join
                .order
                .iter()
                .map(|&atom| (atom, join.candidates[atom].len()))
                .collect(),
        }
    }

    /// Visit matches in order, each as its bound nodes in atom order, until
    /// `visit` breaks or the join is exhausted. Returns whether it is
    /// exhausted.
    pub(super) fn advance(
        &mut self,
        search: &MappedSearch<'_>,
        visit: &mut dyn FnMut(&[u32]) -> ControlFlow<()>,
    ) -> Result<bool> {
        match self {
            Self::Single { nodes, next } => {
                let mut poll = search.interrupt_poll();
                while let Some(&node) = nodes.get(*next) {
                    poll.step()?;
                    *next += 1;
                    if visit(&[node]).is_break() {
                        return Ok(*next >= nodes.len());
                    }
                }
                Ok(true)
            }
            Self::Multi(join) => {
                let mut cursor = JoinCursor {
                    search,
                    edge_views: search.edge_views(&join.plan)?,
                    join,
                    poll: search.interrupt_poll(),
                    row: Vec::new(),
                };
                while cursor.next_match()? {
                    cursor.row.clear();
                    cursor
                        .row
                        .extend(cursor.join.bound.iter().map(|node| node.expect("bound")));
                    if visit(&cursor.row).is_break() {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}

impl<'a> MappedSearch<'a> {
    /// Prepare the join of `plan`: each atom's candidates (filtered by its
    /// node type, features and quantifiers) and the binding order.
    pub(super) fn start_join(
        &self,
        plan: MappedRelationPlan,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<JoinState> {
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

        // Single-atom fast path: the candidates are the matches.
        if plan.relations.is_empty() && n == 1 {
            let nodes = candidate_nodes.pop().unwrap_or_default();
            return Ok(JoinState::Single { nodes, next: 0 });
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

        // 4. Connectivity-greedy evaluation order so every non-seed atom is driven
        //    from an already-bound neighbour rather than enumerated wholesale.
        let order = compute_join_order(&plan, &candidates, &embeds);

        Ok(JoinState::Multi(Box::new(MultiJoin {
            plan,
            candidates,
            embeds,
            order,
            bound: vec![None; n],
            frames: Vec::with_capacity(n),
            started: false,
            slots_cache: HashMap::new(),
        })))
    }

    /// Every match of `plan`, materialized (quantifier sub-searches).
    pub(super) fn join_rows(
        &self,
        plan: MappedRelationPlan,
        sets: Option<&SearchSets<'_>>,
    ) -> Result<Vec<Vec<u32>>> {
        let mut rows = Vec::new();
        self.start_join(plan, sets)?.advance(self, &mut |row| {
            rows.push(row.to_vec());
            ControlFlow::Continue(())
        })?;
        Ok(rows)
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

/// A [`MultiJoin`] being advanced, with the borrowed views it reads.
struct JoinCursor<'j, 's, 'a> {
    search: &'s MappedSearch<'a>,
    edge_views: EdgeViews<'a>,
    join: &'j mut MultiJoin,
    poll: InterruptPoll<'s>,
    /// The current match in atom order.
    row: Vec<u32>,
}

impl JoinCursor<'_, '_, '_> {
    /// Bind the next complete assignment; `false` once the join is exhausted.
    ///
    /// Depth-first over the join order: the deepest frame tries its next
    /// candidate; a candidate that satisfies every constraint it completes
    /// either completes a match (last position) or opens the next frame.
    fn next_match(&mut self) -> Result<bool> {
        if !self.join.started {
            self.join.started = true;
            let first = self.drive_candidates(self.join.order[0])?;
            self.join.frames.push(Frame {
                candidates: first,
                next: 0,
            });
        }
        while let Some(depth) = self.join.frames.len().checked_sub(1) {
            // Poll before taking a candidate, so that an interrupted join
            // resumes exactly where it stopped.
            self.poll.step()?;
            let atom = self.join.order[depth];
            self.join.bound[atom] = None;
            let frame = &mut self.join.frames[depth];
            let Some(&node) = frame.candidates.get(frame.next) else {
                self.join.frames.pop();
                continue;
            };
            frame.next += 1;
            self.join.bound[atom] = Some(node);
            if !self.constraints_hold(atom)? {
                continue;
            }
            if depth + 1 == self.join.order.len() {
                // Every constraint was verified as its last endpoint bound, so
                // a fully-bound assignment is a match.
                return Ok(true);
            }
            let candidates = self.drive_candidates(self.join.order[depth + 1])?;
            self.join.frames.push(Frame {
                candidates,
                next: 0,
            });
        }
        Ok(false)
    }

    /// Check every constraint that becomes fully bound by binding `atom`: relations
    /// where both endpoints are now known, plus the embedding to its bound parent
    /// and from any bound child. Each constraint is thus verified exactly once, when
    /// its last endpoint binds.
    fn constraints_hold(&mut self, atom: usize) -> Result<bool> {
        let join = &mut *self.join;
        let node = join.bound[atom].expect("atom just bound");
        for relation in &join.plan.relations {
            if relation.left != atom && relation.right != atom {
                continue;
            }
            let (Some(left), Some(right)) = (join.bound[relation.left], join.bound[relation.right])
            else {
                continue;
            };
            // Slot-containment verifications collect the (potentially huge) container
            // slot list; route them through the cache so each container's slots are
            // read once. Common with `lex` containers shared across many words.
            let ok = match relation.operator {
                MappedRelationOperator::Embeds => {
                    self.search
                        .contains_cached(&mut join.slots_cache, left, right)?
                }
                MappedRelationOperator::EmbeddedIn => {
                    self.search
                        .contains_cached(&mut join.slots_cache, right, left)?
                }
                _ => self.search.relation_holds(relation, left, right),
            };
            if !ok {
                return Ok(false);
            }
        }
        if let Some(parent) = join.embeds[atom]
            && let Some(parent_node) = join.bound[parent]
            && !self
                .search
                .contains_cached(&mut join.slots_cache, parent_node, node)?
        {
            return Ok(false);
        }
        for (child, child_parent) in join.embeds.iter().enumerate() {
            if *child_parent == Some(atom)
                && let Some(child_node) = join.bound[child]
                && !self
                    .search
                    .contains_cached(&mut join.slots_cache, node, child_node)?
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
        let join = &*self.join;
        let cset = &join.candidates[atom];
        let mut drivers: Vec<Vec<u32>> = Vec::new();

        // Forward embedding: `atom` is contained in its bound parent.
        if let Some(parent) = join.embeds[atom]
            && let Some(parent_node) = join.bound[parent]
        {
            let Some((pf, pl)) = self.search.fast_interval(parent_node)? else {
                return Ok(Vec::new());
            };
            drivers.push(cset.nodes_within_slot_interval(pf, pl));
        }
        // Reverse embedding: `atom` contains a bound child.
        for (child, child_parent) in join.embeds.iter().enumerate() {
            if *child_parent == Some(atom)
                && let Some(child_node) = join.bound[child]
            {
                let Some((cf, cl)) = self.search.fast_interval(child_node)? else {
                    return Ok(Vec::new());
                };
                drivers.push(containers_of(self.search.corpus, cset, cf, cl)?);
            }
        }
        // Relations with a bound other endpoint.
        for relation in &join.plan.relations {
            let (is_left, other) = if relation.left == atom {
                (true, relation.right)
            } else if relation.right == atom {
                (false, relation.left)
            } else {
                continue;
            };
            let Some(other) = join.bound[other] else {
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

/// Best driving strength for placing `i` given the already-placed atoms. 0
/// means `i` is not yet reachable by any driver.
/// Embedding (both directions) is a tight driver (strength 2).
pub(super) fn connection_strength(
    i: usize,
    placed: &[bool],
    plan: &MappedRelationPlan,
    embeds: &[Option<usize>],
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
        let other = if relation.left == i {
            relation.right
        } else if relation.right == i {
            relation.left
        } else {
            continue;
        };
        if placed[other] {
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
            let strength = connection_strength(i, &placed, plan, embeds);
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
