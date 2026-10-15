//! Locality (`L.u/d/n/p/i`), sections and canonical ordering over a mapped
//! corpus.
//!
//! Locality is checked against [`Reference`], a direct implementation of the
//! Text-Fabric definitions over every node's slot set, on the mini corpus and
//! on the three-level corpus.

use std::collections::BTreeSet;
use std::sync::Arc;

mod common;

use common::*;
use context_fabric_core::*;

/// Text-Fabric locality computed naively from slot sets, ordered by the
/// canonical ranks of the mapped corpus.
struct Reference {
    slots: Vec<BTreeSet<u32>>,
    node_types: Vec<String>,
    ranks: Vec<u32>,
    max_slot: u32,
}

impl Reference {
    fn new(corpus: &MappedCompiledCorpus) -> Self {
        let max_node = corpus.max_node();
        let max_slot = corpus.max_slot().unwrap();
        let otype = corpus.string_pool_node_feature("otype").unwrap().unwrap();
        let oslots = corpus.edge_feature("oslots").unwrap().unwrap();
        let mut slots = vec![BTreeSet::new()];
        let mut node_types = vec![String::new()];
        let mut ranks = vec![0];
        for node in 1..=max_node {
            slots.push(if node <= max_slot {
                BTreeSet::from([node])
            } else {
                oslots.f(node).unwrap().into_iter().collect()
            });
            node_types.push(otype.str_value(node).unwrap().unwrap().to_string());
            ranks.push(corpus.sort_key(node).unwrap().unwrap());
        }
        Self {
            slots,
            node_types,
            ranks,
            max_slot,
        }
    }

    fn nodes(&self) -> impl Iterator<Item = u32> + '_ {
        1..self.slots.len() as u32
    }

    fn filtered(&self, nodes: Vec<u32>, node_type: Option<&str>) -> Vec<u32> {
        nodes
            .into_iter()
            .filter(|node| node_type.is_none_or(|wanted| self.node_types[*node as usize] == wanted))
            .collect()
    }

    fn ascending(&self, mut nodes: Vec<u32>) -> Vec<u32> {
        nodes.sort_by_key(|node| self.ranks[*node as usize]);
        nodes
    }

    fn descending(&self, mut nodes: Vec<u32>) -> Vec<u32> {
        nodes.sort_by_key(|node| std::cmp::Reverse(self.ranks[*node as usize]));
        nodes
    }

    /// Non-slot nodes (other than `node`) whose slots contain all of `node`'s,
    /// innermost first.
    fn up(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        let own = &self.slots[node as usize];
        let found = self
            .nodes()
            .filter(|other| *other != node && *other > self.max_slot)
            .filter(|other| own.is_subset(&self.slots[*other as usize]))
            .collect();
        self.filtered(self.descending(found), node_type)
    }

    /// Nodes (other than `node`) whose slots all lie within `node`'s, in
    /// canonical order.
    fn down(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        if node <= self.max_slot {
            return Vec::new();
        }
        let own = &self.slots[node as usize];
        let found = self
            .nodes()
            .filter(|other| *other != node && self.slots[*other as usize].is_subset(own))
            .collect();
        self.filtered(self.ascending(found), node_type)
    }

    /// Nodes (other than `node`) sharing a slot with `node`, in canonical order.
    fn intersecting(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        if node <= self.max_slot {
            return Vec::new();
        }
        let own = &self.slots[node as usize];
        let found = self
            .nodes()
            .filter(|other| *other != node && !own.is_disjoint(&self.slots[*other as usize]))
            .collect();
        self.filtered(self.ascending(found), node_type)
    }

    /// The slot right after `node`, then the nodes starting there (outermost
    /// last).
    fn next(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        let Some(slot) = self.slots[node as usize].last().map(|last| last + 1) else {
            return Vec::new();
        };
        if slot > self.max_slot {
            return Vec::new();
        }
        let starting = self
            .nodes()
            .filter(|other| *other > self.max_slot)
            .filter(|other| self.slots[*other as usize].first() == Some(&slot))
            .collect();
        let mut found = vec![slot];
        found.extend(self.descending(starting));
        self.filtered(found, node_type)
    }

    /// The nodes ending right before `node` (outermost first), then that slot.
    fn previous(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        let Some(slot) = self.slots[node as usize].first().map(|first| first - 1) else {
            return Vec::new();
        };
        if slot == 0 {
            return Vec::new();
        }
        let ending = self
            .nodes()
            .filter(|other| *other > self.max_slot)
            .filter(|other| self.slots[*other as usize].last() == Some(&slot))
            .collect();
        let mut found = self.ascending(ending);
        found.push(slot);
        self.filtered(found, node_type)
    }
}

fn assert_locality_matches_reference(corpus: &MappedCompiledCorpus, node_types: &[Option<&str>]) {
    let sections = MappedSections::new(corpus).unwrap();
    let reference = Reference::new(corpus);
    for node in reference.nodes() {
        for node_type in node_types.iter().copied() {
            let label = format!("node {node}, type {node_type:?}");
            assert_eq!(
                sections.u(node, node_type).unwrap(),
                reference.up(node, node_type),
                "L.u {label}"
            );
            assert_eq!(
                sections.d(node, node_type).unwrap(),
                reference.down(node, node_type),
                "L.d {label}"
            );
            assert_eq!(
                sections.n(node, node_type).unwrap(),
                reference.next(node, node_type),
                "L.n {label}"
            );
            assert_eq!(
                sections.p(node, node_type).unwrap(),
                reference.previous(node, node_type),
                "L.p {label}"
            );
            assert_eq!(
                sections.i(node, node_type).unwrap(),
                reference.intersecting(node, node_type),
                "L.i {label}"
            );
        }
    }
}

#[test]
fn mini_corpus_locality_matches_the_reference() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = mapped_mini_corpus(&temp);
    assert_locality_matches_reference(
        &corpus,
        &[None, Some("word"), Some("phrase"), Some("sentence")],
    );
}

#[test]
fn three_level_corpus_locality_matches_the_reference() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = mapped_three_level_corpus(&temp);
    assert_locality_matches_reference(
        &corpus,
        &[
            None,
            Some("word"),
            Some("verse"),
            Some("chapter"),
            Some("book"),
        ],
    );
}

#[test]
fn sections_resolve_in_both_directions() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = Arc::new(mapped_mini_corpus(&temp));
    let context = SectionsContext::new(&corpus).unwrap();
    let sections = context.sections();

    assert_eq!(sections.section_types(), ["sentence", "phrase"]);
    assert_eq!(sections.section_features(), ["sentence_id", "phrase_id"]);
    assert_eq!(
        sections
            .section_tuple(1, &SectionOptions::default())
            .unwrap(),
        vec![Some(8), Some(6)]
    );
    assert_eq!(
        sections
            .section_from_node(1, &SectionOptions::default())
            .unwrap(),
        vec![Some(FeatureValue::string("S1")), Some(FeatureValue::Int(1))]
    );
    let lookup =
        |section: &[FeatureValue]| context.node_from_section_langed(section, None).unwrap();
    assert_eq!(lookup(&[FeatureValue::string("S1")]), Some(8));
    assert_eq!(
        lookup(&[FeatureValue::string("S1"), FeatureValue::Int(1)]),
        Some(6)
    );
    assert_eq!(
        lookup(&[FeatureValue::string("S1"), FeatureValue::Int(99)]),
        None
    );
}

/// Every section node round-trips through its own section heading.
#[test]
fn three_level_sections_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = Arc::new(mapped_three_level_corpus(&temp));
    let context = SectionsContext::new(&corpus).unwrap();
    let sections = context.sections();
    let lookup =
        |section: &[FeatureValue]| context.node_from_section_langed(section, None).unwrap();

    for node in 7..=10 {
        let heading: Vec<FeatureValue> = sections
            .section_from_node(node, &SectionOptions::default())
            .unwrap()
            .into_iter()
            .map(Option::unwrap)
            .collect();
        assert_eq!(lookup(&heading), Some(node), "{heading:?}");
    }

    let genesis = FeatureValue::string("Genesis");
    assert_eq!(lookup(std::slice::from_ref(&genesis)), Some(10));
    assert_eq!(lookup(&[genesis.clone(), FeatureValue::Int(1)]), Some(9));
    assert_eq!(
        lookup(&[genesis.clone(), FeatureValue::Int(1), FeatureValue::Int(2)]),
        Some(8)
    );
    assert_eq!(lookup(&[FeatureValue::string("Nonexistent")]), None);
    assert_eq!(lookup(&[genesis, FeatureValue::Int(99)]), None);
}

#[test]
fn chunks_sort_by_position_and_by_length() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = mapped_mini_corpus(&temp);
    let chunk = |node, start, end| Chunk { node, start, end };
    let chunks = vec![
        chunk(1, 1, 1),
        chunk(6, 1, 3),
        chunk(8, 1, 5),
        chunk(7, 4, 5),
        chunk(4, 4, 4),
    ];
    let order = |chunks: &[Chunk]| chunks.iter().map(|chunk| chunk.node).collect::<Vec<_>>();

    let mut by_position = chunks.clone();
    by_position.sort_by_key(|chunk| corpus.sort_key_chunk(*chunk).unwrap());
    // By first slot, longer chunks first.
    assert_eq!(order(&by_position), [8, 6, 1, 7, 4]);

    let mut by_length = chunks;
    by_length.sort_by_key(|chunk| corpus.sort_key_chunk_length(*chunk).unwrap());
    // Longer chunks first, then by position.
    assert_eq!(order(&by_length), [8, 6, 7, 1, 4]);
}
