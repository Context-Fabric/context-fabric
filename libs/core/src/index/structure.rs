//! The structure index (`T.structure`, headings) and its tree rendering.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::value::FeatureValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum StructureTree {
    Forest(Vec<StructureTree>),
    Node {
        node: u32,
        children: Vec<StructureTree>,
    },
}

/// One level of a structural heading: a node type and that node's value for the
/// structure feature configured for its type (`None` when the node has no value),
/// like a `(nodeType, value)` pair in Text-Fabric's `T.headingFromNode`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct StructureHeading {
    pub node_type: String,
    pub heading: Option<FeatureValue>,
}

impl StructureHeading {
    /// Text-Fabric's `T.structurePretty` rendering, `type:value` (`None` for a
    /// missing value).
    pub fn repr(heading: &[StructureHeading]) -> String {
        heading
            .iter()
            .map(|part| {
                let value = match &part.heading {
                    Some(FeatureValue::Str(value)) => value.to_string(),
                    Some(FeatureValue::Int(value)) => value.to_string(),
                    None => "None".to_string(),
                };
                format!("{}:{value}", part.node_type)
            })
            .collect::<Vec<_>>()
            .join("-")
    }
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

/// Structure navigation over [`StructureData`], following Text-Fabric's
/// `tf/core/text.py` (`T.structure`, `T.structurePretty`, `T.up`, `T.down`).
///
/// Every method takes `is_structure_node`, which tells whether a node's type is
/// one of the configured structure types; Text-Fabric answers `None` for other
/// nodes.
impl StructureData {
    /// `T.up`: the parent structural node, `None` for top-level nodes.
    pub fn parent(&self, node: u32) -> Option<u32> {
        self.up.get(&node).copied()
    }

    /// `T.down`: the structural children in canonical order.
    pub fn children(&self, node: u32) -> &[u32] {
        self.down.get(&node).map(Vec::as_slice).unwrap_or_default()
    }

    /// `T.structure(node)`: the subtree below `node`, or the forest below the top
    /// nodes when `node` is `None`.
    pub fn tree(
        &self,
        node: Option<u32>,
        is_structure_node: &dyn Fn(u32) -> bool,
    ) -> Option<StructureTree> {
        match node {
            None => Some(StructureTree::Forest(
                self.top.iter().map(|top| self.subtree(*top)).collect(),
            )),
            Some(node) if is_structure_node(node) => Some(self.subtree(node)),
            Some(_) => None,
        }
    }

    fn subtree(&self, node: u32) -> StructureTree {
        StructureTree::Node {
            node,
            children: self
                .children(node)
                .iter()
                .map(|child| self.subtree(*child))
                .collect(),
        }
    }

    /// `T.structurePretty(node, fullHeading)`.
    ///
    /// Text-Fabric indents by nesting depth of the tuple returned by
    /// `T.structure`: two spaces per tuple level, so the forest of the whole
    /// corpus starts at four spaces and a single subtree at two; every structure
    /// level adds four.
    pub fn pretty(
        &self,
        node: Option<u32>,
        full_heading: bool,
        is_structure_node: &dyn Fn(u32) -> bool,
    ) -> Option<String> {
        let tree = self.tree(node, is_structure_node)?;
        let mut lines = Vec::new();
        match &tree {
            StructureTree::Forest(trees) => {
                for tree in trees {
                    self.pretty_lines(tree, full_heading, "    ", &mut lines);
                }
            }
            StructureTree::Node { .. } => self.pretty_lines(&tree, full_heading, "  ", &mut lines),
        }
        Some(lines.join("\n"))
    }

    fn pretty_lines(
        &self,
        tree: &StructureTree,
        full_heading: bool,
        indent: &str,
        lines: &mut Vec<String>,
    ) {
        let StructureTree::Node { node, children } = tree else {
            return;
        };
        let heading = self
            .heading_from_node
            .get(node)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let parts = if full_heading {
            heading
        } else {
            heading.last().map(std::slice::from_ref).unwrap_or_default()
        };
        lines.push(format!("{indent}{}", StructureHeading::repr(parts)));
        let child_indent = format!("{indent}    ");
        for child in children {
            self.pretty_lines(child, full_heading, &child_indent, lines);
        }
    }
}
