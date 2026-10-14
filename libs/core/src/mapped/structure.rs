//! Structure navigation (`T.structure`, headings) from the stored structure section.

use crate::error::Result;
use crate::format::Reader;
use crate::index::{StructureData, StructureHeading, StructureTree};
use crate::mapped::MappedCompiledCorpus;

impl MappedCompiledCorpus {
    /// The precomputed structure (`C.structure` in Text-Fabric), parsed from the
    /// compiled section on first use; `None` when no usable structure is
    /// configured.
    pub fn structure_data(&self) -> Result<Option<&StructureData>> {
        if let Some(cached) = self.structure_cache.get() {
            return Ok(cached.as_ref());
        }
        let value = match self.metadata.structure_start {
            Some(structure_start) => {
                Some(Reader::at(&self.path, &self.mmap, structure_start).read_structure_section()?)
            }
            None => None,
        };
        let _ = self.structure_cache.set(value);
        Ok(self
            .structure_cache
            .get()
            .expect("structure cache just initialized")
            .as_ref())
    }

    /// The configured `(structure type, structure feature)` pairs (`T.headings`),
    /// empty when no usable structure is configured.
    pub fn structure_headings(&self) -> Result<Vec<(String, String)>> {
        if self.structure_data()?.is_none() {
            return Ok(Vec::new());
        }
        let types = self.otext_list("structureTypes")?;
        let features = self.otext_list("structureFeatures")?;
        Ok(types.into_iter().zip(features).collect())
    }

    /// Whether `node`'s type is one of the configured structure types; Text-Fabric's
    /// structure accessors answer `None` for every other node.
    pub fn is_structure_node(&self, node: u32) -> Result<bool> {
        let Some(otype) = self.string_pool_node_feature("otype")? else {
            return Ok(false);
        };
        let Some(node_type) = otype.str_value(node)? else {
            return Ok(false);
        };
        Ok(self
            .otext_list("structureTypes")?
            .iter()
            .any(|structure_type| structure_type == node_type))
    }

    fn otext_list(&self, key: &str) -> Result<Vec<String>> {
        Ok(self
            .config_feature("otext")?
            .and_then(|otext| otext.metadata_value(key).transpose())
            .transpose()?
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    /// `T.structure(node)`; see [`StructureData::tree`].
    pub fn structure(&self, node: Option<u32>) -> Result<Option<StructureTree>> {
        let Some(data) = self.structure_data()? else {
            return Ok(None);
        };
        if let Some(node) = node
            && !self.is_structure_node(node)?
        {
            return Ok(None);
        }
        Ok(data.tree(node, &|_| true))
    }

    pub fn top(&self) -> Result<Option<Vec<u32>>> {
        Ok(self.structure_data()?.map(|data| data.top.clone()))
    }

    /// `T.up`: the parent structural node; `None` without structure, for nodes
    /// of other types, and for top-level nodes.
    pub fn structure_up(&self, node: u32) -> Result<Option<u32>> {
        let Some(data) = self.structure_data()? else {
            return Ok(None);
        };
        Ok(if self.is_structure_node(node)? {
            data.parent(node)
        } else {
            None
        })
    }

    /// `T.down`: the structural children; `None` without structure and for nodes
    /// of other types.
    pub fn structure_down(&self, node: u32) -> Result<Option<Vec<u32>>> {
        let Some(data) = self.structure_data()? else {
            return Ok(None);
        };
        Ok(self
            .is_structure_node(node)?
            .then(|| data.children(node).to_vec()))
    }

    pub fn heading_from_node(&self, node: u32) -> Result<Option<Vec<StructureHeading>>> {
        Ok(self
            .structure_data()?
            .and_then(|data| data.heading_from_node.get(&node).cloned()))
    }

    pub fn node_from_heading(&self, heading: &[StructureHeading]) -> Result<Option<u32>> {
        Ok(self
            .structure_data()?
            .and_then(|data| data.node_from_heading.get(heading).copied()))
    }

    /// `T.structurePretty(node, fullHeading)`; see [`StructureData::pretty`].
    pub fn structure_pretty(
        &self,
        node: Option<u32>,
        full_heading: bool,
    ) -> Result<Option<String>> {
        let Some(data) = self.structure_data()? else {
            return Ok(None);
        };
        if let Some(node) = node
            && !self.is_structure_node(node)?
        {
            return Ok(None);
        }
        Ok(data.pretty(node, full_heading, &|_| true))
    }
}
