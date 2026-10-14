//! Text: rendering nodes with the `otext` formats (`T.text`) and parsing the
//! format templates.

mod characters;
mod renderer;
pub(crate) mod template;

pub use renderer::TextRenderer;
pub use template::{FormatTemplate, TemplatePart};

/// How `T.text` renders a node.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextOptions {
    /// The format name (with or without the `fmt:` prefix); `None` for the
    /// node type's `<type>-default` format, else `text-orig-full`.
    pub format: Option<String>,
    /// Whether to render the nodes of the format's target type contained in
    /// the node; `None` descends except for a node type's default format.
    pub descend: Option<bool>,
}

impl TextOptions {
    pub fn new(format: Option<impl Into<String>>, descend: Option<bool>) -> Self {
        Self {
            format: format.map(Into::into),
            descend,
        }
    }
}
