use crate::compiled::{
    EdgeFeatureView, MappedCompiledCorpus, MappedNodeValue, StringPoolNodeFeatureView,
};
use crate::corpus::{Boundary, SectionOptions, WalkEvent};
use crate::error::{CfError, Result};
use crate::feature::FeatureValue;
use std::collections::HashSet;

pub struct MappedSections<'a> {
    corpus: &'a MappedCompiledCorpus,
    otype: StringPoolNodeFeatureView<'a>,
    oslots: EdgeFeatureView<'a>,
    slot_type: String,
    section_types: Vec<String>,
    section_features: Vec<String>,
    structure_types: Vec<String>,
    structure_features: Vec<String>,
}

impl<'a> MappedSections<'a> {
    pub fn new(corpus: &'a MappedCompiledCorpus) -> Result<Self> {
        let otype = corpus
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype
            .str_value(1)?
            .ok_or_else(|| CfError::MissingFeature("otype slot type".to_string()))?
            .to_string();
        let otext = corpus.config_feature("otext")?;
        let section_types = otext
            .as_ref()
            .and_then(|metadata| metadata.get("sectionTypes").transpose())
            .transpose()?
            .flatten()
            .map(parse_csv_config)
            .unwrap_or_default();
        let section_features = otext
            .as_ref()
            .and_then(|metadata| metadata.get("sectionFeatures").transpose())
            .transpose()?
            .flatten()
            .map(parse_csv_config)
            .unwrap_or_default();
        let structure_types = otext
            .as_ref()
            .and_then(|metadata| metadata.get("structureTypes").transpose())
            .transpose()?
            .flatten()
            .map(parse_csv_config)
            .unwrap_or_default();
        let structure_features = otext
            .as_ref()
            .and_then(|metadata| metadata.get("structureFeatures").transpose())
            .transpose()?
            .flatten()
            .map(parse_csv_config)
            .unwrap_or_default();

        Ok(Self {
            corpus,
            otype,
            oslots,
            slot_type,
            section_types,
            section_features,
            structure_types,
            structure_features,
        })
    }

    pub fn section_types(&self) -> &[String] {
        &self.section_types
    }

    #[allow(non_snake_case)]
    pub fn sectionTypes(&self) -> &[String] {
        self.section_types()
    }

    pub fn section_features(&self) -> &[String] {
        &self.section_features
    }

    #[allow(non_snake_case)]
    pub fn sectionFeatures(&self) -> &[String] {
        self.section_features()
    }

    pub fn structure_types(&self) -> &[String] {
        &self.structure_types
    }

    #[allow(non_snake_case)]
    pub fn structureTypes(&self) -> &[String] {
        self.structure_types()
    }

    pub fn structure_features(&self) -> &[String] {
        &self.structure_features
    }

    #[allow(non_snake_case)]
    pub fn structureFeatures(&self) -> &[String] {
        self.structure_features()
    }

    pub fn section_tuple(&self, node: u32, options: &SectionOptions) -> Result<Vec<Option<u32>>> {
        if self.section_types.is_empty() || node == 0 || self.node_type(node)?.is_none() {
            return Ok(Vec::new());
        }

        let node_type = self.node_type(node)?.unwrap_or_default();
        let reference_slot = if node_type == self.slot_type {
            node
        } else {
            let slots = self.slots(node)?;
            let Some(slot) = (if options.last_slot {
                slots.last()
            } else {
                slots.first()
            }) else {
                return Ok(Vec::new());
            };
            *slot
        };

        let mut sections = Vec::new();
        for (index, section_type) in self.section_types.iter().enumerate() {
            let section_node = if node_type == section_type {
                Some(node)
            } else {
                self.up_first(reference_slot, section_type)?
            };
            if section_node.is_some() || options.fillup || index == 0 {
                sections.push(section_node);
                if node_type == section_type && !options.fillup {
                    break;
                }
            } else {
                break;
            }
        }

        if let Some(level) = options.level {
            sections.truncate(level);
        }
        Ok(sections)
    }

    #[allow(non_snake_case)]
    pub fn sectionTuple(&self, node: u32, options: &SectionOptions) -> Result<Vec<Option<u32>>> {
        self.section_tuple(node, options)
    }

    pub fn section_from_node(
        &self,
        node: u32,
        options: &SectionOptions,
    ) -> Result<Vec<Option<FeatureValue>>> {
        self.section_tuple(node, options)?
            .into_iter()
            .enumerate()
            .map(|(index, section_node)| {
                let Some(section_node) = section_node else {
                    return Ok(None);
                };
                let Some(feature_name) = self.section_features.get(index) else {
                    return Ok(None);
                };
                self.node_feature_value(feature_name, section_node)
            })
            .collect()
    }

    #[allow(non_snake_case)]
    pub fn sectionFromNode(
        &self,
        node: u32,
        options: &SectionOptions,
    ) -> Result<Vec<Option<FeatureValue>>> {
        self.section_from_node(node, options)
    }

    pub fn section_ref(&self, node: u32) -> Result<String> {
        Ok(self
            .section_from_node(node, &SectionOptions::default())?
            .into_iter()
            .filter_map(|value| value.map(section_value_to_string))
            .collect::<Vec<_>>()
            .join(" "))
    }

    pub fn node_from_section(&self, section: &[FeatureValue]) -> Result<Option<u32>> {
        if section.is_empty()
            || section.len() > self.section_types.len()
            || section.len() > self.section_features.len()
        {
            return Ok(None);
        }

        let target_type = &self.section_types[section.len() - 1];
        for node in self.otype.s(target_type)? {
            let matches = section
                .iter()
                .enumerate()
                .map(|(index, expected)| {
                    let options = SectionOptions {
                        fillup: true,
                        level: Some(index + 1),
                        ..SectionOptions::default()
                    };
                    Ok(self
                        .section_from_node(node, &options)?
                        .get(index)
                        .and_then(Option::as_ref)
                        == Some(expected))
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .all(|matches| matches);
            if matches {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    #[allow(non_snake_case)]
    pub fn nodeFromSection(&self, section: &[FeatureValue]) -> Result<Option<u32>> {
        self.node_from_section(section)
    }

    pub fn node_type(&self, node: u32) -> Result<Option<&'a str>> {
        self.otype.str_value(node)
    }

    pub fn is_slot(&self, node: u32) -> Result<bool> {
        Ok(self.node_type(node)? == Some(self.slot_type.as_str()))
    }

    pub fn slots(&self, node: u32) -> Result<Vec<u32>> {
        if self.is_slot(node)? {
            return Ok(vec![node]);
        }
        self.oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    pub fn contains(&self, parent: u32, child: u32) -> Result<bool> {
        if self.is_slot(child)? {
            return self.contains_slot(parent, child);
        }
        let child_slots = self.slots(child)?;
        if child_slots.is_empty() {
            return Ok(false);
        }
        let parent_slots = self.slots(parent)?;
        Ok(child_slots
            .iter()
            .all(|slot| parent_slots.binary_search(slot).is_ok()))
    }

    pub fn up(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.up_types(node, Some(&[node_type])),
            None => self.up_types(node, None),
        }
    }

    pub fn u(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.up(node, node_type)
    }

    pub fn intersecting(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.intersecting_types(node, Some(&[node_type])),
            None => self.intersecting_types(node, None),
        }
    }

    pub fn i(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.intersecting(node, node_type)
    }

    pub fn intersecting_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let max_slot = self.max_slot()?;
        if node <= max_slot || node > self.max_node() {
            return Ok(Vec::new());
        }
        let slots = self.slots(node)?;
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        let slot_set = slots.iter().copied().collect::<HashSet<_>>();
        let mut result = Vec::new();
        if self.node_type_allowed(1, node_types)? {
            result.extend(slots.iter().copied());
        }
        for row in self.otype.rows() {
            let (candidate, node_type) = row?;
            if candidate == node || node_type == self.slot_type {
                continue;
            }
            if !self.node_type_allowed(candidate, node_types)? {
                continue;
            }
            if self
                .slots(candidate)?
                .iter()
                .any(|slot| slot_set.contains(slot))
            {
                result.push(candidate);
            }
        }
        self.sort_nodes(&mut result)?;
        result.reverse();
        Ok(result)
    }

    pub fn up_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let mut result = Vec::new();
        for row in self.otype.rows() {
            let (candidate, node_type) = row?;
            if candidate == node || node_type == self.slot_type {
                continue;
            }
            if !self.contains(candidate, node)? {
                continue;
            }
            if !self.node_type_allowed(candidate, node_types)? {
                continue;
            }
            result.push(candidate);
        }
        self.sort_nodes(&mut result)?;
        result.reverse();
        Ok(result)
    }

    pub fn u_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        self.up_types(node, node_types)
    }

    pub fn down(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.down_types(node, Some(&[node_type])),
            None => self.down_types(node, None),
        }
    }

    pub fn d(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.down(node, node_type)
    }

    pub fn down_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let slots = self
            .oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))?;
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        let mut result = Vec::new();
        if node_types.is_none_or(|expected| expected.contains(&self.slot_type.as_str())) {
            result.extend_from_slice(&slots);
        }
        for row in self.otype.rows() {
            let (candidate, node_type) = row?;
            if candidate == node || node_type == self.slot_type {
                continue;
            }
            if !self.contains(node, candidate)? {
                continue;
            }
            if !self.node_type_allowed(candidate, node_types)? {
                continue;
            }
            result.push(candidate);
        }
        self.sort_nodes(&mut result)?;
        Ok(result)
    }

    pub fn d_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        self.down_types(node, node_types)
    }

    pub fn next(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.next_types(node, Some(&[node_type])),
            None => self.next_types(node, None),
        }
    }

    pub fn n(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.next(node, node_type)
    }

    pub fn next_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let max_slot = self.max_slot()?;
        let max_node = self.max_node();
        if node == 0 || node == max_slot || node > max_node {
            return Ok(Vec::new());
        }
        let Some(next_slot) = self.adjacent_slot(node, true)? else {
            return Ok(Vec::new());
        };
        if next_slot > max_slot {
            return Ok(Vec::new());
        }

        let mut result = vec![next_slot];
        result.extend(self.nodes_starting_at(next_slot)?);
        self.filter_locality_nodes(result, node_types)
    }

    pub fn previous(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        match node_type {
            Some(node_type) => self.previous_types(node, Some(&[node_type])),
            None => self.previous_types(node, None),
        }
    }

    pub fn p(&self, node: u32, node_type: Option<&str>) -> Result<Vec<u32>> {
        self.previous(node, node_type)
    }

    pub fn previous_types(&self, node: u32, node_types: Option<&[&str]>) -> Result<Vec<u32>> {
        let max_node = self.max_node();
        if node <= 1 || node > max_node {
            return Ok(Vec::new());
        }
        let Some(previous_slot) = self.adjacent_slot(node, false)? else {
            return Ok(Vec::new());
        };
        if previous_slot == 0 {
            return Ok(Vec::new());
        }

        let mut result = self.nodes_ending_at(previous_slot)?;
        result.push(previous_slot);
        self.filter_locality_nodes(result, node_types)
    }

    pub fn all_nodes(&self) -> Result<Vec<u32>> {
        let mut nodes: Vec<_> = (1..=self.max_node()).collect();
        self.sort_nodes(&mut nodes)?;
        Ok(nodes)
    }

    pub fn node_type_items(&self) -> Result<Vec<(u32, String)>> {
        let mut rows = Vec::new();
        for node in 1..=self.max_node() {
            if let Some(node_type) = self.node_type(node)? {
                rows.push((node, node_type.to_string()));
            }
        }
        Ok(rows)
    }

    pub fn otype_items(&self) -> Result<Vec<(u32, String)>> {
        self.node_type_items()
    }

    #[allow(non_snake_case)]
    pub fn otypeItems(&self) -> Result<Vec<(u32, String)>> {
        self.node_type_items()
    }

    #[allow(non_snake_case)]
    pub fn allNodes(&self) -> Result<Vec<u32>> {
        self.all_nodes()
    }

    pub fn walk(&self, nodes: Option<&[u32]>) -> Result<Vec<u32>> {
        match nodes {
            Some(nodes) => {
                let mut nodes = nodes.to_vec();
                self.sort_nodes(&mut nodes)?;
                Ok(nodes)
            }
            None => self.all_nodes(),
        }
    }

    pub fn walk_events(&self, nodes: Option<&[u32]>) -> Result<Vec<WalkEvent>> {
        let walk_nodes = self.walk(nodes)?;
        let walk_node_set: Option<HashSet<u32>> =
            nodes.map(|nodes| nodes.iter().copied().collect());
        let mut events = Vec::new();

        for node in walk_nodes {
            if self.node_type(node)? == Some(self.slot_type.as_str()) {
                events.push(WalkEvent::Slot(node));
                let mut ending_nodes = self.nodes_ending_at(node)?;
                ending_nodes.reverse();
                for ending_node in ending_nodes {
                    if walk_node_set
                        .as_ref()
                        .is_none_or(|node_set| node_set.contains(&ending_node))
                    {
                        events.push(WalkEvent::End(ending_node));
                    }
                }
            } else {
                events.push(WalkEvent::Start(node));
            }
        }

        Ok(events)
    }

    #[allow(non_snake_case)]
    pub fn walkEvents(&self, nodes: Option<&[u32]>) -> Result<Vec<WalkEvent>> {
        self.walk_events(nodes)
    }

    pub fn boundary(&self) -> Result<Boundary> {
        let max_slot = self.max_slot()?;
        let mut first_slots = Vec::with_capacity(max_slot as usize);
        let mut last_slots = Vec::with_capacity(max_slot as usize);
        for slot in 1..=max_slot {
            first_slots.push(self.nodes_starting_at(slot)?);
            last_slots.push(self.nodes_ending_at(slot)?);
        }
        Ok(Boundary {
            first_slots,
            last_slots,
        })
    }

    fn up_first(&self, reference_slot: u32, section_type: &str) -> Result<Option<u32>> {
        for node in self.otype.s(section_type)? {
            if self.contains_slot(node, reference_slot)? {
                return Ok(Some(node));
            }
        }
        Ok(None)
    }

    fn contains_slot(&self, parent: u32, slot: u32) -> Result<bool> {
        self.oslots
            .targets(parent)?
            .map(|targets| {
                for target in targets {
                    if target? == slot {
                        return Ok(true);
                    }
                }
                Ok(false)
            })
            .unwrap_or(Ok(false))
    }

    fn node_type_allowed(&self, node: u32, node_types: Option<&[&str]>) -> Result<bool> {
        let Some(node_types) = node_types else {
            return Ok(true);
        };
        Ok(self
            .node_type(node)?
            .is_some_and(|node_type| node_types.contains(&node_type)))
    }

    fn filter_locality_nodes(
        &self,
        nodes: Vec<u32>,
        node_types: Option<&[&str]>,
    ) -> Result<Vec<u32>> {
        let Some(node_types) = node_types else {
            return Ok(nodes);
        };
        nodes
            .into_iter()
            .filter_map(
                |node| match self.node_type_allowed(node, Some(node_types)) {
                    Ok(true) => Some(Ok(node)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .collect()
    }

    fn adjacent_slot(&self, node: u32, forward: bool) -> Result<Option<u32>> {
        if node <= self.max_slot()? {
            return Ok(if forward {
                node.checked_add(1)
            } else {
                node.checked_sub(1)
            });
        }
        let slots = self
            .oslots
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))?;
        Ok(if forward {
            slots.last().and_then(|slot| slot.checked_add(1))
        } else {
            slots.first().and_then(|slot| slot.checked_sub(1))
        })
    }

    fn nodes_starting_at(&self, slot: u32) -> Result<Vec<u32>> {
        let mut nodes = Vec::new();
        for row in self.otype.rows() {
            let (node, node_type) = row?;
            if node_type == self.slot_type {
                continue;
            }
            let slots = self
                .oslots
                .targets(node)?
                .map(|targets| targets.collect())
                .unwrap_or_else(|| Ok(Vec::new()))?;
            if slots.first() == Some(&slot) {
                nodes.push(node);
            }
        }
        self.sort_nodes_reverse(&mut nodes)?;
        Ok(nodes)
    }

    fn nodes_ending_at(&self, slot: u32) -> Result<Vec<u32>> {
        let mut nodes = Vec::new();
        for row in self.otype.rows() {
            let (node, node_type) = row?;
            if node_type == self.slot_type {
                continue;
            }
            let slots = self
                .oslots
                .targets(node)?
                .map(|targets| targets.collect())
                .unwrap_or_else(|| Ok(Vec::new()))?;
            if slots.last() == Some(&slot) {
                nodes.push(node);
            }
        }
        self.sort_nodes(&mut nodes)?;
        Ok(nodes)
    }

    fn max_slot(&self) -> Result<u32> {
        Ok(self
            .otype
            .s(&self.slot_type)?
            .last()
            .copied()
            .unwrap_or_default())
    }

    fn max_node(&self) -> u32 {
        self.corpus.metadata().rank_len as u32
    }

    fn sort_nodes(&self, nodes: &mut [u32]) -> Result<()> {
        let mut keyed = nodes
            .iter()
            .map(|node| Ok((self.corpus.sort_key(*node)?.unwrap_or(u32::MAX), *node)))
            .collect::<Result<Vec<_>>>()?;
        keyed.sort_unstable_by_key(|(rank, _)| *rank);
        for (slot, (_, node)) in nodes.iter_mut().zip(keyed) {
            *slot = node;
        }
        Ok(())
    }

    fn sort_nodes_reverse(&self, nodes: &mut [u32]) -> Result<()> {
        let mut keyed = nodes
            .iter()
            .map(|node| Ok((self.corpus.sort_key(*node)?.unwrap_or(u32::MAX), *node)))
            .collect::<Result<Vec<_>>>()?;
        keyed.sort_unstable_by_key(|(rank, _)| std::cmp::Reverse(*rank));
        for (slot, (_, node)) in nodes.iter_mut().zip(keyed) {
            *slot = node;
        }
        Ok(())
    }

    pub fn node_feature_value(
        &self,
        feature_name: &str,
        node: u32,
    ) -> Result<Option<FeatureValue>> {
        if let Some(feature) = self.corpus.string_pool_node_feature(feature_name)? {
            return Ok(feature.str_value(node)?.map(FeatureValue::string));
        }
        if let Some(feature) = self.corpus.mixed_node_feature(feature_name)? {
            return Ok(feature.value(node)?.map(mapped_value_to_feature_value));
        }
        Ok(None)
    }
}

fn mapped_value_to_feature_value(value: MappedNodeValue<'_>) -> FeatureValue {
    match value {
        MappedNodeValue::Str(value) => FeatureValue::string(value),
        MappedNodeValue::Int(value) => FeatureValue::Int(value),
    }
}

fn section_value_to_string(value: FeatureValue) -> String {
    match value {
        FeatureValue::Str(value) => value.to_string(),
        FeatureValue::Int(value) => value.to_string(),
    }
}

fn parse_csv_config(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|item| {
            let item = item.trim();
            (!item.is_empty()).then(|| item.to_string())
        })
        .collect()
}
