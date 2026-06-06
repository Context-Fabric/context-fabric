use std::collections::BTreeMap;

use crate::corpus::{Corpus, SectionOptions, StructureInfo, StructureTree, TextOptions, WalkEvent};
use crate::feature::FeatureValue;
use crate::precompute::StructureHeading;

#[derive(Debug, Clone, Copy)]
pub struct Nodes<'a> {
    pub corpus: &'a Corpus,
}

impl<'a> Nodes<'a> {
    pub fn new(corpus: &'a Corpus) -> Self {
        Self { corpus }
    }

    #[allow(non_snake_case)]
    pub fn otypeRank(&self) -> BTreeMap<String, u32> {
        self.corpus.otype_rank()
    }

    pub fn otype_rank(&self) -> BTreeMap<String, u32> {
        self.corpus.otype_rank()
    }

    #[allow(non_snake_case)]
    pub fn sortKey(&self, node: u32) -> usize {
        self.corpus.sort_key(node)
    }

    pub fn sort_key(&self, node: u32) -> usize {
        self.corpus.sort_key(node)
    }

    #[allow(non_snake_case)]
    pub fn sortKeyTuple(&self, nodes: &[u32]) -> Vec<usize> {
        self.corpus.sort_key_tuple(nodes)
    }

    pub fn sort_key_tuple(&self, nodes: &[u32]) -> Vec<usize> {
        self.corpus.sort_key_tuple(nodes)
    }

    #[allow(non_snake_case)]
    pub fn sortNodes(&self, nodes: impl IntoIterator<Item = u32>) -> Vec<u32> {
        self.corpus.sorted_nodes(nodes)
    }

    pub fn sort_nodes(&self, nodes: impl IntoIterator<Item = u32>) -> Vec<u32> {
        self.corpus.sorted_nodes(nodes)
    }

    pub fn walk(&self, nodes: Option<&[u32]>) -> Vec<u32> {
        self.corpus.walk(nodes)
    }

    pub fn walk_events(&self, nodes: Option<&[u32]>) -> Vec<WalkEvent> {
        self.corpus.walk_events(nodes)
    }

    #[allow(non_snake_case)]
    pub fn walkEvents(&self, nodes: Option<&[u32]>) -> Vec<WalkEvent> {
        self.corpus.walk_events(nodes)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Locality<'a> {
    pub corpus: &'a Corpus,
}

impl<'a> Locality<'a> {
    pub fn new(corpus: &'a Corpus) -> Self {
        Self { corpus }
    }

    pub fn i(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.i(node, node_type)
    }

    pub fn intersecting(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.intersecting(node, node_type)
    }

    pub fn intersecting_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.corpus.intersecting_types(node, node_types)
    }

    pub fn u(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.u(node, node_type)
    }

    pub fn up(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.up(node, node_type)
    }

    pub fn up_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.corpus.up_types(node, node_types)
    }

    pub fn d(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.d(node, node_type)
    }

    pub fn down(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.down(node, node_type)
    }

    pub fn down_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.corpus.down_types(node, node_types)
    }

    pub fn n(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.n(node, node_type)
    }

    pub fn next(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.next(node, node_type)
    }

    pub fn next_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.corpus.next_types(node, node_types)
    }

    pub fn p(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.p(node, node_type)
    }

    pub fn previous(&self, node: u32, node_type: Option<&str>) -> Vec<u32> {
        self.corpus.previous(node, node_type)
    }

    pub fn previous_types(&self, node: u32, node_types: Option<&[&str]>) -> Vec<u32> {
        self.corpus.previous_types(node, node_types)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Text<'a> {
    pub corpus: &'a Corpus,
}

impl<'a> Text<'a> {
    pub fn new(corpus: &'a Corpus) -> Self {
        Self { corpus }
    }

    pub fn text(&self, node: u32, format: Option<&str>) -> String {
        self.corpus.text(node, format)
    }

    pub fn text_with_options(&self, node: u32, options: &TextOptions) -> String {
        self.corpus.text_with_options(node, options)
    }

    #[allow(non_snake_case)]
    pub fn textWithOptions(&self, node: u32, options: &TextOptions) -> String {
        self.text_with_options(node, options)
    }

    pub fn text_nodes(&self, nodes: &[u32], format: Option<&str>) -> String {
        self.corpus.text_nodes(nodes, format)
    }

    #[allow(non_snake_case)]
    pub fn textNodes(&self, nodes: &[u32], format: Option<&str>) -> String {
        self.corpus.text_nodes(nodes, format)
    }

    pub fn languages(&self) -> BTreeMap<String, BTreeMap<String, String>> {
        self.corpus.languages()
    }

    pub fn section_from_node(
        &self,
        node: u32,
        options: &SectionOptions,
        lang: &str,
    ) -> Vec<Option<FeatureValue>> {
        self.corpus.section_from_node_lang(node, options, lang)
    }

    #[allow(non_snake_case)]
    pub fn sectionFromNode(
        &self,
        node: u32,
        options: &SectionOptions,
        lang: &str,
    ) -> Vec<Option<FeatureValue>> {
        self.section_from_node(node, options, lang)
    }

    pub fn node_from_section(&self, section: &[FeatureValue], lang: &str) -> Option<u32> {
        self.corpus.node_from_section_lang(section, lang)
    }

    #[allow(non_snake_case)]
    pub fn nodeFromSection(&self, section: &[FeatureValue], lang: &str) -> Option<u32> {
        self.node_from_section(section, lang)
    }

    pub fn name_from_node(&self, lang: &str) -> BTreeMap<u32, String> {
        self.corpus.name_from_node(lang)
    }

    #[allow(non_snake_case)]
    pub fn nameFromNode(&self, lang: &str) -> BTreeMap<u32, String> {
        self.name_from_node(lang)
    }

    pub fn node_from_name(&self, lang: &str) -> BTreeMap<(String, String), u32> {
        self.corpus.node_from_name(lang)
    }

    #[allow(non_snake_case)]
    pub fn nodeFromName(&self, lang: &str) -> BTreeMap<(String, String), u32> {
        self.node_from_name(lang)
    }

    #[allow(non_snake_case)]
    pub fn bookName(&self, node: u32, lang: &str) -> Option<String> {
        self.corpus.bookName(node, lang)
    }

    #[allow(non_snake_case)]
    pub fn bookNode(&self, name: &str, lang: &str) -> Option<u32> {
        self.corpus.bookNode(name, lang)
    }

    pub fn structure(&self, node: Option<u32>) -> Option<StructureTree> {
        self.corpus.structure(node)
    }

    pub fn structure_pretty(&self, node: Option<u32>, full_heading: bool) -> Option<String> {
        self.corpus.structure_pretty(node, full_heading)
    }

    #[allow(non_snake_case)]
    pub fn structurePretty(&self, node: Option<u32>, full_heading: bool) -> Option<String> {
        self.structure_pretty(node, full_heading)
    }

    pub fn structure_info(&self) -> Option<StructureInfo> {
        self.corpus.structure_info()
    }

    #[allow(non_snake_case)]
    pub fn structureInfo(&self) -> Option<StructureInfo> {
        self.structure_info()
    }

    pub fn top(&self) -> Option<Vec<u32>> {
        self.corpus.top()
    }

    pub fn up(&self, node: u32) -> Option<u32> {
        self.corpus.structure_up(node)
    }

    pub fn down(&self, node: u32) -> Option<Vec<u32>> {
        self.corpus.structure_down(node)
    }

    pub fn heading_from_node(&self, node: u32) -> Option<Vec<StructureHeading>> {
        self.corpus.heading_from_node(node)
    }

    #[allow(non_snake_case)]
    pub fn headingFromNode(&self, node: u32) -> Option<Vec<StructureHeading>> {
        self.heading_from_node(node)
    }

    pub fn node_from_heading(&self, heading: &[StructureHeading]) -> Option<u32> {
        self.corpus.node_from_heading(heading)
    }

    #[allow(non_snake_case)]
    pub fn nodeFromHeading(&self, heading: &[StructureHeading]) -> Option<u32> {
        self.node_from_heading(heading)
    }
}
