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
        Ok(self.otext()?.structure_headings())
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
            .otext()?
            .structure_types
            .iter()
            .any(|structure_type| structure_type == node_type))
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

    /// `T.structureInfo()` — a human-readable summary of how structure is
    /// configured, mirroring text-fabric's `T.structureInfo` (`tf/core/text.py:655`),
    /// backed by `structure_data()`.
    ///
    /// TF prints this summary and returns `None`; here we return the formatted
    /// string so callers can print it (or assert on it). When no structure is
    /// configured we return TF's "No structural elements configured" line.
    pub fn structure_info(&self) -> Result<String> {
        let Some(data) = self.structure_data()? else {
            return Ok("No structural elements configured".to_string());
        };
        let headings = self.structure_headings()?;
        let n_structure = data.heading_from_node.len();
        let mut out = String::new();
        out.push_str("A heading is a tuple of pairs (node type, feature value)\n");
        out.push_str(
            "\tof node types and features that have been configured as structural elements\n",
        );
        out.push_str(&format!(
            "These {} structural elements have been configured\n",
            headings.len()
        ));
        for (node_type, feature) in &headings {
            out.push_str(&format!(
                "\tnode type {node_type:<10} with heading feature {feature}\n"
            ));
        }
        out.push_str("You can get them as a tuple with T.headings.\n");
        out.push_str(&format!(
            "\nThere are {n_structure} structural elements in the dataset.\n"
        ));
        if !data.multiple.is_empty() {
            let n_multiple = data.multiple.len();
            let t_multiple: usize = data.multiple.values().map(Vec::len).sum();
            out.push_str(&format!(
                "WARNING: {n_multiple} structure headings with hdMult occurrences (total {t_multiple})\n"
            ));
            for (key, nodes) in data.multiple.iter().take(10) {
                let key_rep = StructureHeading::repr(key);
                let n_nodes = nodes.len();
                out.push_str(&format!("\t{key_rep} has {n_nodes} occurrences\n"));
                let sample = nodes
                    .iter()
                    .take(5)
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("\t\t{sample}\n"));
                if n_nodes > 5 {
                    out.push_str(&format!("\t\tand {} more\n", n_nodes - 5));
                }
            }
        }
        Ok(out)
    }
}
