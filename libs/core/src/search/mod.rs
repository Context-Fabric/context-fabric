//! Search templates over a mapped corpus: parsing, planning, and the join.

mod candidates;
mod constraint;
mod join;
mod plan;
mod quantifier;
mod relations;
mod study;
mod syntax;
mod template;

pub use study::SearchStudy;
pub use syntax::{RELATIONS_LEGEND, SEARCH_FAIL_FACTOR, SearchSets};

use crate::config::{OSLOTS, OTYPE};
use crate::error::{CfError, Result};
use crate::mapped::{EdgeFeatureView, MappedCompiledCorpus, StringPoolNodeFeatureView};
use crate::search::plan::parse_mapped_plan;

/// Runs search templates over a mapped corpus.
pub struct MappedSearch<'a> {
    corpus: &'a MappedCompiledCorpus,
    /// The warp features every search reads: node types and slot containment.
    otype: StringPoolNodeFeatureView<'a>,
    oslots: EdgeFeatureView<'a>,
}

impl<'a> MappedSearch<'a> {
    pub fn new(corpus: &'a MappedCompiledCorpus) -> Result<Self> {
        let otype = corpus
            .string_pool_node_feature(OTYPE)?
            .ok_or_else(|| CfError::MissingFeature(OTYPE.to_string()))?;
        let oslots = corpus
            .edge_feature(OSLOTS)?
            .ok_or_else(|| CfError::MissingFeature(OSLOTS.to_string()))?;
        Ok(Self {
            corpus,
            otype,
            oslots,
        })
    }

    pub fn search(&self, template: &str, limit: Option<usize>) -> Result<Vec<Vec<u32>>> {
        self.search_inner(template, None, limit)
    }

    pub fn count(&self, template: &str, limit: Option<usize>) -> Result<usize> {
        self.count_inner(template, None, limit)
    }

    pub fn count_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<usize> {
        self.count_inner(template, Some(sets), limit)
    }

    fn count_inner(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<usize> {
        let plan = parse_mapped_plan(template)?;
        self.count_plan(&plan, sets, limit)
    }

    pub fn search_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.search_inner(template, Some(sets), limit)
    }

    pub fn search_first_nodes(&self, template: &str, limit: Option<usize>) -> Result<Vec<u32>> {
        Ok(self.study(template)?.fetch_first_nodes(limit))
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

    pub fn search_prefixes(
        &self,
        template: &str,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        Ok(self.study(template)?.fetch_prefixes(width, limit))
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

    pub fn show_plan(&self, template: &str, details: bool) -> Result<String> {
        Ok(self.study(template)?.show_plan(details))
    }

    pub fn show_plan_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        details: bool,
    ) -> Result<String> {
        Ok(self.study_with_sets(template, sets)?.show_plan(details))
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

        self.search_plan(&plan, sets, limit)
    }
}
