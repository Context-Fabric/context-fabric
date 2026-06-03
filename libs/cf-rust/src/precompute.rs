use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::corpus::{Boundary, Corpus};
use crate::feature::FeatureValue;

pub type LevelsData = Vec<(String, f64, u32, u32)>;
pub type OrderData = Vec<u32>;
pub type RankData = Vec<u32>;
pub type LevUpData = Vec<Vec<u32>>;
pub type LevDownData = Vec<Vec<u32>>;
pub type CharactersData = BTreeMap<String, Vec<(String, usize)>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionsData {
    pub sec1: BTreeMap<u32, BTreeMap<String, u32>>,
    pub sec2: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>>,
    pub seq_from_node: BTreeMap<u32, Vec<u32>>,
    pub node_from_seq: BTreeMap<Vec<u32>, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StructureHeading {
    pub node_type: String,
    pub heading: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureData {
    pub heading_from_node: BTreeMap<u32, Vec<StructureHeading>>,
    pub node_from_heading: BTreeMap<Vec<StructureHeading>, u32>,
    pub multiple: BTreeMap<Vec<StructureHeading>, Vec<u32>>,
    pub top: Vec<u32>,
    pub up: BTreeMap<u32, u32>,
    pub down: BTreeMap<u32, Vec<u32>>,
}

pub fn levels(
    node_types: &BTreeMap<u32, String>,
    oslots: &BTreeMap<u32, Vec<u32>>,
    max_slot: u32,
    slot_type: &str,
    configured_order: Option<&str>,
) -> LevelsData {
    let mut nodes_by_type: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for slot in 1..=max_slot {
        nodes_by_type
            .entry(slot_type.to_string())
            .or_default()
            .push(slot);
    }
    for (node, node_type) in node_types {
        if *node > max_slot {
            nodes_by_type
                .entry(node_type.clone())
                .or_default()
                .push(*node);
        }
    }

    let mut rows = nodes_by_type
        .into_iter()
        .map(|(node_type, mut nodes)| {
            nodes.sort_unstable();
            let slot_count: usize = nodes
                .iter()
                .map(|node| {
                    if *node <= max_slot {
                        1
                    } else {
                        oslots.get(node).map(Vec::len).unwrap_or_default()
                    }
                })
                .sum();
            let average_slots = slot_count as f64 / nodes.len().max(1) as f64;
            let min_node = nodes.first().copied().unwrap_or_default();
            let max_node = nodes.last().copied().unwrap_or_default();
            (node_type, average_slots, min_node, max_node)
        })
        .collect::<Vec<_>>();

    if let Some(order) = configured_order {
        let ranks: HashMap<&str, usize> = order
            .split(',')
            .map(str::trim)
            .enumerate()
            .map(|(index, node_type)| (node_type, index))
            .collect();
        rows.sort_by(|left, right| {
            match (ranks.get(left.0.as_str()), ranks.get(right.0.as_str())) {
                (Some(left_rank), Some(right_rank)) => left_rank.cmp(right_rank),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => right
                    .1
                    .partial_cmp(&left.1)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| left.0.cmp(&right.0)),
            }
        });
    } else {
        rows.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
    }
    rows
}

pub fn order(
    node_types: &BTreeMap<u32, String>,
    oslots: &BTreeMap<u32, Vec<u32>>,
    levels: &[(String, f64, u32, u32)],
    max_slot: u32,
    max_node: u32,
    slot_type: &str,
) -> OrderData {
    let type_ranks = type_ranks(levels);
    let mut nodes = (1..=max_node).collect::<Vec<_>>();
    nodes.sort_unstable_by(|left, right| {
        compare_nodes(
            *left,
            *right,
            node_types,
            oslots,
            &type_ranks,
            max_slot,
            slot_type,
        )
    });
    nodes
}

pub fn rank(max_node: u32, order: &[u32]) -> RankData {
    let mut ranks = vec![0; max_node as usize];
    for (rank, node) in order.iter().copied().enumerate() {
        if node > 0 && node <= max_node {
            ranks[(node - 1) as usize] = rank as u32;
        }
    }
    ranks
}

pub fn lev_up(
    oslots: &BTreeMap<u32, Vec<u32>>,
    rank: &[u32],
    max_slot: u32,
    max_node: u32,
) -> LevUpData {
    let mut inverse: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (node, slots) in oslots {
        for slot in slots {
            inverse.entry(*slot).or_default().push(*node);
        }
    }

    let mut embedders = Vec::with_capacity(max_node as usize);
    for node in 1..=max_node {
        let slots = node_slots(node, oslots, max_slot);
        if slots.is_empty() {
            embedders.push(Vec::new());
            continue;
        }
        let mut common = inverse
            .get(&slots[0])
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        for slot in slots.iter().skip(1) {
            let slot_embedders = inverse
                .get(slot)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            common = common.intersection(&slot_embedders).copied().collect();
        }
        let mut row = common
            .into_iter()
            .filter(|embedder| *embedder != node)
            .collect::<Vec<_>>();
        row.sort_unstable_by_key(|embedder| {
            std::cmp::Reverse(
                rank.get((*embedder).saturating_sub(1) as usize)
                    .copied()
                    .unwrap_or_default(),
            )
        });
        embedders.push(row);
    }
    embedders
}

#[allow(non_snake_case)]
pub fn levUp(
    oslots: &BTreeMap<u32, Vec<u32>>,
    rank: &[u32],
    max_slot: u32,
    max_node: u32,
) -> LevUpData {
    lev_up(oslots, rank, max_slot, max_node)
}

pub fn lev_down(lev_up: &[Vec<u32>], rank: &[u32], max_slot: u32, max_node: u32) -> LevDownData {
    let mut inverse: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (index, embedders) in lev_up.iter().enumerate() {
        let node = index as u32 + 1;
        if node <= max_slot {
            continue;
        }
        for embedder in embedders {
            inverse.entry(*embedder).or_default().push(node);
        }
    }

    (max_slot + 1..=max_node)
        .map(|node| {
            let mut row = inverse.remove(&node).unwrap_or_default();
            row.sort_unstable_by_key(|embeddee| {
                rank.get((*embeddee).saturating_sub(1) as usize)
                    .copied()
                    .unwrap_or(u32::MAX)
            });
            row
        })
        .collect()
}

#[allow(non_snake_case)]
pub fn levDown(lev_up: &[Vec<u32>], rank: &[u32], max_slot: u32, max_node: u32) -> LevDownData {
    lev_down(lev_up, rank, max_slot, max_node)
}

pub fn boundary(oslots: &BTreeMap<u32, Vec<u32>>, rank: &[u32], max_slot: u32) -> Boundary {
    let mut first_slots = vec![Vec::new(); max_slot as usize];
    let mut last_slots = vec![Vec::new(); max_slot as usize];

    for (node, slots) in oslots {
        let (Some(first), Some(last)) = (slots.first(), slots.last()) else {
            continue;
        };
        if let Some(nodes) = first_slots.get_mut((*first - 1) as usize) {
            nodes.push(*node);
        }
        if let Some(nodes) = last_slots.get_mut((*last - 1) as usize) {
            nodes.push(*node);
        }
    }

    for nodes in &mut first_slots {
        nodes.sort_unstable_by_key(|node| {
            std::cmp::Reverse(
                rank.get((*node).saturating_sub(1) as usize)
                    .copied()
                    .unwrap_or_default(),
            )
        });
    }
    for nodes in &mut last_slots {
        nodes.sort_unstable_by_key(|node| {
            rank.get((*node).saturating_sub(1) as usize)
                .copied()
                .unwrap_or(u32::MAX)
        });
    }

    Boundary {
        first_slots,
        last_slots,
    }
}

pub fn sections(corpus: &Corpus) -> Option<SectionsData> {
    let section_types = corpus.section_types();
    let section_features = corpus.section_features();
    if section_types.len() < 2 || section_features.len() < 2 {
        return None;
    }

    let mut sec1: BTreeMap<u32, BTreeMap<String, u32>> = BTreeMap::new();
    let mut sec2: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>> = BTreeMap::new();
    let mut seq_from_node: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut node_from_seq: BTreeMap<Vec<u32>, u32> = BTreeMap::new();

    let Some(level_1_feature) = corpus.node_feature(&section_features[1]) else {
        return Some(SectionsData {
            sec1,
            sec2,
            seq_from_node,
            node_from_seq,
        });
    };
    let level_2_feature = section_features
        .get(2)
        .and_then(|feature_name| corpus.node_feature(feature_name));

    for &level_1_node in corpus.nodes_of_type(&section_types[1]) {
        let Some(level_0_node) = first_embedder_of_type(corpus, level_1_node, &section_types[0])
        else {
            continue;
        };
        let heading_1 = level_1_feature
            .v(level_1_node)
            .map(section_heading_key)
            .unwrap_or_default();
        sec1.entry(level_0_node)
            .or_default()
            .entry(heading_1)
            .or_insert(level_1_node);
    }

    if section_types.len() >= 3 {
        if let Some(level_2_feature) = level_2_feature {
            for &level_2_node in corpus.nodes_of_type(&section_types[2]) {
                let Some(level_0_node) =
                    first_embedder_of_type(corpus, level_2_node, &section_types[0])
                else {
                    continue;
                };
                let Some(level_1_node) =
                    first_embedder_of_type(corpus, level_2_node, &section_types[1])
                else {
                    continue;
                };
                let heading_1 = level_1_feature
                    .v(level_1_node)
                    .map(section_heading_key)
                    .unwrap_or_default();
                let heading_2 = level_2_feature
                    .v(level_2_node)
                    .map(section_heading_key)
                    .unwrap_or_default();
                sec2.entry(level_0_node)
                    .or_default()
                    .entry(heading_1)
                    .or_default()
                    .insert(heading_2, level_2_node);
            }
        }
    }

    if section_types.len() == 3 {
        let mut seq_0 = 0;
        for &level_0_node in corpus.nodes_of_type(&section_types[0]) {
            seq_0 += 1;
            seq_from_node.insert(level_0_node, vec![seq_0]);
            node_from_seq.insert(vec![seq_0], level_0_node);

            let mut seq_1 = 0;
            for level_1_node in corpus.d(level_0_node, Some(&section_types[1])) {
                seq_1 += 1;
                seq_from_node.insert(level_1_node, vec![seq_0, seq_1]);
                node_from_seq.insert(vec![seq_0, seq_1], level_1_node);

                let mut seq_2 = 0;
                for level_2_node in corpus.d(level_1_node, Some(&section_types[2])) {
                    seq_2 += 1;
                    seq_from_node.insert(level_2_node, vec![seq_0, seq_1, seq_2]);
                    node_from_seq.insert(vec![seq_0, seq_1, seq_2], level_2_node);
                }
            }
        }
    }

    Some(SectionsData {
        sec1,
        sec2,
        seq_from_node,
        node_from_seq,
    })
}

#[allow(non_snake_case)]
pub fn sectionsFromApi(corpus: &Corpus) -> Option<SectionsData> {
    sections(corpus)
}

pub fn structure(corpus: &Corpus) -> Option<StructureData> {
    let structure_types = corpus.structure_types();
    let structure_features = corpus.structure_features();
    if structure_types.is_empty() {
        return None;
    }
    if structure_types.len() != structure_features.len() {
        return Some(empty_structure_data());
    }
    let unique_types = structure_types
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if unique_types.len() != structure_types.len() {
        return Some(empty_structure_data());
    }

    let mut higher_types: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for (index, high_type) in structure_types.iter().enumerate() {
        for low_type in structure_types.iter().skip(index) {
            higher_types
                .entry(low_type.clone())
                .or_default()
                .insert(high_type.clone());
        }
    }

    let mut heading_from_node: BTreeMap<u32, Vec<StructureHeading>> = BTreeMap::new();
    let mut node_from_heading: BTreeMap<Vec<StructureHeading>, u32> = BTreeMap::new();
    let mut multiple: BTreeMap<Vec<StructureHeading>, Vec<u32>> = BTreeMap::new();

    for node in corpus.max_slot() + 1..=corpus.max_node() {
        let Some(node_type) = corpus.node_type(node) else {
            continue;
        };
        if !unique_types.contains(&node_type.to_string()) {
            continue;
        }
        let Some(allowed_higher_types) = higher_types.get(node_type) else {
            continue;
        };
        let mut nodes = vec![node];
        nodes.extend(corpus.u(node, None).into_iter().filter(|embedder| {
            corpus
                .node_type(*embedder)
                .is_some_and(|embedder_type| allowed_higher_types.contains(embedder_type))
        }));
        let heading = nodes
            .into_iter()
            .rev()
            .filter_map(|heading_node| {
                let heading_type = corpus.node_type(heading_node)?;
                let feature_index = structure_types
                    .iter()
                    .position(|structure_type| structure_type == heading_type)?;
                let feature_name = structure_features.get(feature_index)?;
                let heading_value = corpus
                    .node_feature(feature_name)
                    .and_then(|feature| feature.v(heading_node))
                    .map(section_heading_key)
                    .unwrap_or_default();
                Some(StructureHeading {
                    node_type: heading_type.to_string(),
                    heading: heading_value,
                })
            })
            .collect::<Vec<_>>();

        if let Some(previous) = node_from_heading.insert(heading.clone(), node) {
            multiple.entry(heading.clone()).or_default().push(previous);
            multiple.entry(heading.clone()).or_default().push(node);
        }
        heading_from_node.insert(node, heading);
    }

    for nodes in multiple.values_mut() {
        *nodes = corpus.sortNodes(nodes.iter().copied());
        nodes.dedup();
    }

    let top = corpus.sortNodes(
        heading_from_node
            .iter()
            .filter_map(|(node, heading)| (heading.len() == 1).then_some(*node)),
    );

    let mut up = BTreeMap::new();
    for (node, heading) in &heading_from_node {
        if heading.len() == 1 {
            continue;
        }
        for end in (1..heading.len()).rev() {
            let parent_heading = heading[..end].to_vec();
            if let Some(parent) = node_from_heading.get(&parent_heading) {
                up.insert(*node, *parent);
                break;
            }
        }
    }

    let mut down: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (node, parent) in &up {
        down.entry(*parent).or_default().push(*node);
    }
    for children in down.values_mut() {
        *children = corpus.sortNodes(children.iter().copied());
    }

    Some(StructureData {
        heading_from_node,
        node_from_heading,
        multiple,
        top,
        up,
        down,
    })
}

pub fn characters(
    text_formats: &BTreeMap<String, Vec<String>>,
    text_features: &BTreeMap<String, BTreeMap<u32, String>>,
) -> CharactersData {
    let mut character_counts_by_feature: BTreeMap<String, BTreeMap<String, usize>> =
        BTreeMap::new();

    for (feature_name, values) in text_features {
        let mut counts = BTreeMap::new();
        for value in values.values() {
            for character in value.chars() {
                *counts.entry(character.to_string()).or_default() += 1;
            }
        }
        character_counts_by_feature.insert(feature_name.clone(), counts);
    }

    let mut character_counts_by_format = BTreeMap::new();
    for (format_name, feature_names) in text_formats {
        let mut counts = BTreeMap::new();
        for feature_name in feature_names {
            if let Some(feature_counts) = character_counts_by_feature.get(feature_name) {
                for (character, count) in feature_counts {
                    *counts.entry(character.clone()).or_default() += count;
                }
            }
        }
        character_counts_by_format.insert(format_name.clone(), counts.into_iter().collect());
    }

    character_counts_by_format
}

fn first_embedder_of_type(corpus: &Corpus, node: u32, node_type: &str) -> Option<u32> {
    corpus
        .u(node, None)
        .into_iter()
        .find(|embedder| corpus.node_type(*embedder) == Some(node_type))
}

fn section_heading_key(value: &FeatureValue) -> String {
    match value {
        FeatureValue::Str(value) => value.to_string(),
        FeatureValue::Int(value) => value.to_string(),
    }
}

fn empty_structure_data() -> StructureData {
    StructureData {
        heading_from_node: BTreeMap::new(),
        node_from_heading: BTreeMap::new(),
        multiple: BTreeMap::new(),
        top: Vec::new(),
        up: BTreeMap::new(),
        down: BTreeMap::new(),
    }
}

fn type_ranks(levels: &[(String, f64, u32, u32)]) -> HashMap<String, u32> {
    levels
        .iter()
        .rev()
        .enumerate()
        .map(|(rank, (node_type, _, _, _))| (node_type.clone(), rank as u32))
        .collect()
}

fn compare_nodes(
    left: u32,
    right: u32,
    node_types: &BTreeMap<u32, String>,
    oslots: &BTreeMap<u32, Vec<u32>>,
    type_ranks: &HashMap<String, u32>,
    max_slot: u32,
    slot_type: &str,
) -> Ordering {
    if left == right {
        return Ordering::Equal;
    }
    let left_slots = node_slots(left, oslots, max_slot);
    let right_slots = node_slots(right, oslots, max_slot);
    let left_rank = node_type_rank(left, node_types, type_ranks, max_slot, slot_type);
    let right_rank = node_type_rank(right, node_types, type_ranks, max_slot, slot_type);

    if left_slots == right_slots {
        return if left_rank == right_rank {
            left.cmp(&right)
        } else if left_rank > right_rank {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    if is_superset(&left_slots, &right_slots) {
        return Ordering::Less;
    }
    if is_superset(&right_slots, &left_slots) {
        return Ordering::Greater;
    }

    let left_min = min_difference(&left_slots, &right_slots).unwrap_or(u32::MAX);
    let right_min = min_difference(&right_slots, &left_slots).unwrap_or(u32::MAX);
    left_min.cmp(&right_min)
}

fn node_type_rank(
    node: u32,
    node_types: &BTreeMap<u32, String>,
    type_ranks: &HashMap<String, u32>,
    max_slot: u32,
    slot_type: &str,
) -> u32 {
    let node_type = if node <= max_slot {
        slot_type
    } else {
        node_types
            .get(&node)
            .map(String::as_str)
            .unwrap_or(slot_type)
    };
    type_ranks.get(node_type).copied().unwrap_or(u32::MAX)
}

fn node_slots(node: u32, oslots: &BTreeMap<u32, Vec<u32>>, max_slot: u32) -> Vec<u32> {
    if node <= max_slot {
        vec![node]
    } else {
        oslots.get(&node).cloned().unwrap_or_default()
    }
}

fn is_superset(left: &[u32], right: &[u32]) -> bool {
    right.iter().all(|value| left.binary_search(value).is_ok())
}

fn min_difference(left: &[u32], right: &[u32]) -> Option<u32> {
    left.iter()
        .copied()
        .find(|value| right.binary_search(value).is_err())
}
