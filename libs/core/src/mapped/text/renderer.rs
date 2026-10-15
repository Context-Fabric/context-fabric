//! [`TextRenderer`]: renders nodes with the `otext` formats (`T.text`).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, RwLock};

use crate::config::DEFAULT_FORMAT;
use crate::error::{CfError, Result};
use crate::mapped::text::TextOptions;
use crate::mapped::text::template::{FormatTemplate, TemplatePart, split_format};
use crate::mapped::{
    MappedCompiledCorpus, MappedNodeValue, OwnedEdgeFeature, OwnedNodeFeature,
    OwnedStringPoolFeature, SectionsContext,
};

/// One step of a compiled format.
enum TextOp {
    /// Literal text.
    Literal(String),
    /// A placeholder: indices into the format's feature handles, and the
    /// default text.
    Placeholder {
        features: Vec<usize>,
        default: String,
    },
}

/// A format compiled for rendering: the node type it renders, its steps, and
/// the feature handles its placeholders read (`None` for a feature the corpus
/// lacks, which never has a value).
struct CompiledFormat {
    target_type: String,
    ops: Vec<TextOp>,
    handles: Vec<Option<OwnedNodeFeature>>,
}

/// Renders nodes with the corpus's text formats. Built once per API object:
/// it holds the `otype`/`oslots` handles and the raw format templates, and
/// compiles each format (parsing it and resolving its feature handles) on its
/// first use, so steady-state rendering is a loop over cached steps.
pub struct TextRenderer {
    corpus: Arc<MappedCompiledCorpus>,
    otype: OwnedStringPoolFeature,
    oslots: OwnedEdgeFeature,
    slot_type: String,
    /// Every node type: the distinct values of `otype`.
    node_types: HashSet<String>,
    compiled: RwLock<HashMap<String, Arc<CompiledFormat>>>,
    /// Locality, for descending to a format's target type.
    sections: SectionsContext,
}

impl TextRenderer {
    pub fn new(corpus: &Arc<MappedCompiledCorpus>) -> Result<Self> {
        let otype = corpus
            .owned_string_pool_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = corpus
            .owned_edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype
            .str_value(1)?
            .ok_or_else(|| CfError::MissingFeature("otype slot type".to_string()))?;
        let node_types = otype
            .view()
            .distinct_values()?
            .into_iter()
            .map(str::to_string)
            .collect();
        Ok(Self {
            corpus: Arc::clone(corpus),
            otype,
            oslots,
            slot_type,
            node_types,
            compiled: RwLock::new(HashMap::new()),
            sections: SectionsContext::new(corpus)?,
        })
    }

    /// `T.formats`: every format name with the node type it renders.
    pub fn formats(&self) -> Result<BTreeMap<String, String>> {
        Ok(self
            .corpus
            .otext()?
            .formats
            .iter()
            .map(|(name, format)| (name.clone(), self.split_format(format).0))
            .collect())
    }

    /// `T.splitFormat`: `type#template` split into the node type it renders
    /// and the template.
    pub fn split_format(&self, format: &str) -> (String, String) {
        let (node_type, template) = split_format(&self.node_types, &self.slot_type, format);
        (node_type, template.to_string())
    }

    /// `T.splitDefaultFormat`: the node type of a format named `<type>-default`.
    pub fn split_default_format(&self, name: &str) -> Option<String> {
        let (node_type, suffix) = name.rsplit_once('-')?;
        (suffix == "default" && self.node_types.contains(node_type)).then(|| node_type.to_string())
    }

    /// `T.text(node, fmt, descend)` for one node.
    pub fn text(&self, node: u32, options: &TextOptions) -> Result<String> {
        let mut rendered = String::new();
        self.render_into(node, options, &mut rendered)?;
        Ok(rendered)
    }

    /// Appends the text of `node` to `out`, so callers rendering many nodes
    /// can reuse one buffer.
    pub fn render_into(&self, node: u32, options: &TextOptions, out: &mut String) -> Result<()> {
        let Some((nodes, format)) = self.resolve(node, options)? else {
            return Ok(());
        };
        for text_node in nodes {
            self.render_node(text_node, &format, out)?;
        }
        Ok(())
    }

    /// The nodes `T.text(node, fmt, descend)` renders one by one: `node` itself,
    /// or its descendants of the format's target type when descending. Empty
    /// when no format applies to `node`.
    pub fn rendered_nodes(&self, node: u32, options: &TextOptions) -> Result<Vec<u32>> {
        Ok(self
            .resolve(node, options)?
            .map(|(nodes, _)| nodes)
            .unwrap_or_default())
    }

    /// The format `T.text` applies to `node` and the nodes it is applied to,
    /// or `None` when no format applies.
    fn resolve(
        &self,
        node: u32,
        options: &TextOptions,
    ) -> Result<Option<(Vec<u32>, Arc<CompiledFormat>)>> {
        let otype = self.otype.view();
        let Some(node_type) = otype.str_value(node)? else {
            return Ok(None);
        };
        let formats = &self.corpus.otext()?.formats;
        // An explicit format, else the node type's `<type>-default` format
        // (which renders the node itself), else the default format.
        let (format_name, node_default) = match options.format.as_deref() {
            Some(format) => (
                format.strip_prefix("fmt:").unwrap_or(format).to_string(),
                false,
            ),
            None => {
                let node_default = format!("{node_type}-default");
                if formats.contains_key(&node_default) {
                    (node_default, true)
                } else {
                    (DEFAULT_FORMAT.to_string(), false)
                }
            }
        };
        let Some(format) = self.compiled_format(&format_name)? else {
            return Ok(None);
        };
        let descend = options.descend.unwrap_or(!node_default);
        let nodes = if !descend || format.target_type == node_type {
            vec![node]
        } else if format.target_type == self.slot_type {
            self.slots(node)?
        } else {
            self.sections
                .sections()
                .d(node, Some(&format.target_type))?
        };
        Ok(Some((nodes, format)))
    }

    /// The named format, compiled on first use; `None` when it is not defined.
    fn compiled_format(&self, name: &str) -> Result<Option<Arc<CompiledFormat>>> {
        if let Some(format) = self
            .compiled
            .read()
            .expect("text format cache poisoned")
            .get(name)
        {
            return Ok(Some(Arc::clone(format)));
        }
        let Some(format) = self.corpus.otext()?.formats.get(name) else {
            return Ok(None);
        };
        let format = Arc::new(self.compile(format)?);
        self.compiled
            .write()
            .expect("text format cache poisoned")
            .insert(name.to_string(), Arc::clone(&format));
        Ok(Some(format))
    }

    fn compile(&self, format: &str) -> Result<CompiledFormat> {
        let (target_type, template) = split_format(&self.node_types, &self.slot_type, format);
        let mut handles = Vec::new();
        let mut handle_index: HashMap<String, usize> = HashMap::new();
        let mut ops = Vec::new();
        for part in FormatTemplate::parse(template).parts {
            ops.push(match part {
                TemplatePart::Literal(text) => TextOp::Literal(text),
                TemplatePart::Placeholder { features, default } => {
                    let mut indices = Vec::with_capacity(features.len());
                    for feature in features {
                        let index = match handle_index.get(&feature) {
                            Some(index) => *index,
                            None => {
                                handles.push(self.corpus.owned_node_feature(&feature)?);
                                handle_index.insert(feature, handles.len() - 1);
                                handles.len() - 1
                            }
                        };
                        indices.push(index);
                    }
                    TextOp::Placeholder {
                        features: indices,
                        default,
                    }
                }
            });
        }
        Ok(CompiledFormat {
            target_type,
            ops,
            handles,
        })
    }

    fn render_node(&self, node: u32, format: &CompiledFormat, out: &mut String) -> Result<()> {
        for op in &format.ops {
            match op {
                TextOp::Literal(literal) => out.push_str(literal),
                TextOp::Placeholder { features, default } => {
                    match placeholder_value(&format.handles, node, features)? {
                        Some(MappedNodeValue::Str(text)) => out.push_str(text),
                        Some(MappedNodeValue::Int(number)) => {
                            let _ = write!(out, "{number}");
                        }
                        None => out.push_str(default),
                    }
                }
            }
        }
        Ok(())
    }

    fn slots(&self, node: u32) -> Result<Vec<u32>> {
        if self.otype.view().str_value(node)? == Some(self.slot_type.as_str()) {
            return Ok(vec![node]);
        }
        self.oslots
            .view()
            .targets(node)?
            .map(|targets| targets.collect())
            .unwrap_or_else(|| Ok(Vec::new()))
    }
}

/// The value a placeholder renders for `node`, or `None` for its default.
/// Like Text-Fabric (`tf/core/text.py` `_makeFunc`): with one or two features
/// the first *present* value wins, even when empty; with more, the first
/// present value wins only when non-empty.
fn placeholder_value<'h>(
    handles: &'h [Option<OwnedNodeFeature>],
    node: u32,
    features: &[usize],
) -> Result<Option<MappedNodeValue<'h>>> {
    let value_of = |index: usize| -> Result<Option<MappedNodeValue<'h>>> {
        match &handles[index] {
            Some(handle) => handle.value(node),
            None => Ok(None),
        }
    };
    for index in features {
        if let Some(value) = value_of(*index)? {
            let empty = matches!(value, MappedNodeValue::Str(text) if text.is_empty());
            return Ok((features.len() <= 2 || !empty).then_some(value));
        }
    }
    Ok(None)
}
