//! Search templates over a mapped corpus: parsing, planning, and the join.
//!
//! [`MappedSearch::prepare`] plans a template and returns a
//! [`PreparedSearch`] that produces the matches on demand.

mod candidates;
mod constraint;
mod interrupt;
mod join;
mod plan;
mod prepared;
mod quantifier;
mod relations;
mod syntax;
mod template;

pub use interrupt::Interrupt;
pub use prepared::PreparedSearch;
pub use syntax::{RELATIONS_LEGEND, SEARCH_FAIL_FACTOR, SearchSets};

use crate::config::{OSLOTS, OTYPE};
use crate::error::{CfError, Result};
use crate::mapped::{EdgeFeatureView, MappedCompiledCorpus, StringPoolNodeFeatureView};
use crate::search::interrupt::InterruptPoll;
use crate::search::plan::parse_mapped_plan;

/// Runs search templates over a mapped corpus.
pub struct MappedSearch<'a> {
    corpus: &'a MappedCompiledCorpus,
    /// The warp features every search reads: node types and slot containment.
    otype: StringPoolNodeFeatureView<'a>,
    oslots: EdgeFeatureView<'a>,
    /// Polled while a join runs; stops the search when it reports.
    interrupt: Option<&'a dyn Interrupt>,
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
            interrupt: None,
        })
    }

    /// Stop running searches with [`CfError::Interrupted`] once `interrupt`
    /// reports an interruption.
    pub fn with_interrupt(mut self, interrupt: &'a dyn Interrupt) -> Self {
        self.interrupt = Some(interrupt);
        self
    }

    pub(super) fn interrupt_poll(&self) -> InterruptPoll<'a> {
        InterruptPoll::new(self.interrupt)
    }

    /// Plan `template`.
    pub fn prepare(&self, template: &str, sets: Option<&SearchSets<'_>>) -> Result<PreparedSearch> {
        let plan = parse_mapped_plan(template)?;
        let atom_types = plan
            .atoms
            .iter()
            .map(|atom| atom.node_type.clone())
            .collect();
        let relation_count = plan.relations.len();
        let has_quantifiers = plan.atoms.iter().any(|atom| !atom.quantifiers.is_empty());
        let join = self.start_join(plan, sets)?;
        Ok(PreparedSearch::new(
            template.to_string(),
            atom_types,
            relation_count,
            has_quantifiers,
            join,
        ))
    }

    /// The matches of `template`, at most `limit` of them.
    pub fn search(&self, template: &str, limit: Option<usize>) -> Result<Vec<Vec<u32>>> {
        self.search_inner(template, None, limit)
    }

    pub fn search_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.search_inner(template, Some(sets), limit)
    }

    fn search_inner(
        &self,
        template: &str,
        sets: Option<&SearchSets<'_>>,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.prepare(template, sets)?
            .next_rows(self, limit.unwrap_or(usize::MAX))
    }

    /// The number of matches of `template`, counting at most `limit`.
    pub fn count(&self, template: &str, limit: Option<usize>) -> Result<usize> {
        self.prepare(template, None)?.count(self, limit)
    }

    pub fn count_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<usize> {
        self.prepare(template, Some(sets))?.count(self, limit)
    }

    /// The distinct first nodes of the matches of `template`.
    pub fn search_first_nodes(&self, template: &str, limit: Option<usize>) -> Result<Vec<u32>> {
        self.prepare(template, None)?.first_nodes(self, limit)
    }

    pub fn search_first_nodes_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        self.prepare(template, Some(sets))?.first_nodes(self, limit)
    }

    /// The distinct `width`-node prefixes of the matches of `template`.
    pub fn search_prefixes(
        &self,
        template: &str,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.prepare(template, None)?.prefixes(self, width, limit)
    }

    pub fn search_prefixes_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        self.prepare(template, Some(sets))?
            .prefixes(self, width, limit)
    }

    /// A description of the plan of `template` (see
    /// [`PreparedSearch::describe`]).
    pub fn show_plan(&self, template: &str, details: bool) -> Result<String> {
        Ok(self.prepare(template, None)?.describe(details))
    }

    pub fn show_plan_with_sets(
        &self,
        template: &str,
        sets: &SearchSets<'_>,
        details: bool,
    ) -> Result<String> {
        Ok(self.prepare(template, Some(sets))?.describe(details))
    }
}
