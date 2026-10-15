//! [`PreparedSearch`]: a checked and planned search that produces its matches
//! on demand.

use std::collections::HashSet;
use std::ops::ControlFlow;

use crate::error::Result;
use crate::search::MappedSearch;
use crate::search::join::JoinState;

/// A checked and planned search, ready to produce its matches in order.
///
/// Matches are produced on demand, in batches of any size, so a search with
/// very many matches costs memory only for the batch being read. Advance it
/// only with a [`MappedSearch`] over the corpus that prepared it.
pub struct PreparedSearch {
    template: String,
    /// The node type of each atom.
    atom_types: Vec<String>,
    relation_count: usize,
    has_quantifiers: bool,
    join: JoinState,
}

impl PreparedSearch {
    pub(super) fn new(
        template: String,
        atom_types: Vec<String>,
        relation_count: usize,
        has_quantifiers: bool,
        join: JoinState,
    ) -> Self {
        Self {
            template,
            atom_types,
            relation_count,
            has_quantifiers,
            join,
        }
    }

    /// Visit the next matches, each as its nodes in atom order, until `visit`
    /// breaks or the matches run out. Returns whether they ran out.
    pub fn advance(
        &mut self,
        search: &MappedSearch<'_>,
        visit: &mut dyn FnMut(&[u32]) -> ControlFlow<()>,
    ) -> Result<bool> {
        self.join.advance(search, visit)
    }

    /// The next `max` matches (fewer when they run out).
    pub fn next_rows(&mut self, search: &MappedSearch<'_>, max: usize) -> Result<Vec<Vec<u32>>> {
        let mut rows = Vec::new();
        if max == 0 {
            return Ok(rows);
        }
        self.advance(search, &mut |row| {
            rows.push(row.to_vec());
            if rows.len() >= max {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
        Ok(rows)
    }

    /// The number of remaining matches, counting at most `limit`.
    pub fn count(&mut self, search: &MappedSearch<'_>, limit: Option<usize>) -> Result<usize> {
        let limit = limit.unwrap_or(usize::MAX);
        let mut count = 0;
        if limit == 0 {
            return Ok(count);
        }
        self.advance(search, &mut |_| {
            count += 1;
            if count >= limit {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })?;
        Ok(count)
    }

    /// The distinct first nodes of the remaining matches, in order of first
    /// appearance, at most `limit` of them.
    pub fn first_nodes(
        &mut self,
        search: &MappedSearch<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<u32>> {
        let limit = limit.unwrap_or(usize::MAX);
        let mut seen: HashSet<u32> = HashSet::new();
        let mut nodes = Vec::new();
        if limit == 0 {
            return Ok(nodes);
        }
        self.advance(search, &mut |row| {
            if seen.insert(row[0]) {
                nodes.push(row[0]);
                if nodes.len() >= limit {
                    return ControlFlow::Break(());
                }
            }
            ControlFlow::Continue(())
        })?;
        Ok(nodes)
    }

    /// The distinct `width`-node prefixes of the remaining matches, in order
    /// of first appearance, at most `limit` of them.
    pub fn prefixes(
        &mut self,
        search: &MappedSearch<'_>,
        width: usize,
        limit: Option<usize>,
    ) -> Result<Vec<Vec<u32>>> {
        let limit = limit.unwrap_or(usize::MAX);
        let mut seen: HashSet<Vec<u32>> = HashSet::new();
        let mut prefixes = Vec::new();
        if width == 0 || limit == 0 {
            return Ok(prefixes);
        }
        self.advance(search, &mut |row| {
            let prefix = &row[..width.min(row.len())];
            if !seen.contains(prefix) {
                seen.insert(prefix.to_vec());
                prefixes.push(prefix.to_vec());
                if prefixes.len() >= limit {
                    return ControlFlow::Break(());
                }
            }
            ControlFlow::Continue(())
        })?;
        Ok(prefixes)
    }

    /// A description of the plan: the template, its atoms and relations, and
    /// with `details`, the binding order with each atom's candidate count.
    pub fn describe(&self, details: bool) -> String {
        let mut lines = vec![
            format!("template: {}", self.template.escape_default()),
            format!("atoms: {}", self.atom_types.len()),
            format!("relations: {}", self.relation_count),
            format!("quantifiers: {}", self.has_quantifiers),
        ];
        if details {
            lines.push("binding order:".to_string());
            for (position, (atom, candidates)) in self.join.binding_order().into_iter().enumerate()
            {
                lines.push(format!(
                    "  {}: atom {} {} ({candidates} candidates)",
                    position + 1,
                    atom + 1,
                    self.atom_types[atom]
                ));
            }
        }
        lines.join("\n")
    }
}
