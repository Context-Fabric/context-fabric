//! Writes features as Text-Fabric `.tf` files (`TF.save`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::error::{CfError, Result};
use crate::feature::{TfFeature, tf_from_value};

impl TfFeature {
    /// Writes the feature as a `.tf` file at `path`: the `@node` / `@edge` /
    /// `@config` header, the metadata, a blank line, then one explicit row per
    /// node (node features) or per edge (edge features).
    pub fn write_tf(&self, path: &Path) -> Result<()> {
        fs::write(path, self.to_tf()).map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    fn to_tf(&self) -> String {
        let mut output = String::new();
        match self {
            TfFeature::Node(feature) => {
                output.push_str("@node\n");
                write_metadata(&mut output, &feature.metadata);
                output.push('\n');
                for (node, value) in feature.items() {
                    let _ = writeln!(output, "{node}\t{}", tf_from_value(&value));
                }
            }
            TfFeature::Edge(feature) => {
                output.push_str("@edge\n");
                if feature.has_edge_values() || !feature.edge_values.is_empty() {
                    output.push_str("@edgeValues\n");
                }
                write_metadata(&mut output, &feature.metadata);
                output.push('\n');
                for (source, targets) in feature.items() {
                    for target in targets {
                        match feature.edge_value(source, target) {
                            Some(value) => {
                                let _ = writeln!(
                                    output,
                                    "{source}\t{target}\t{}",
                                    tf_from_value(value)
                                );
                            }
                            None => {
                                let _ = writeln!(output, "{source}\t{target}");
                            }
                        }
                    }
                }
            }
            TfFeature::Config { metadata, .. } => {
                output.push_str("@config\n");
                write_metadata(&mut output, metadata);
                output.push('\n');
            }
        }
        output
    }
}

fn write_metadata(output: &mut String, metadata: &BTreeMap<String, Option<String>>) {
    for (key, value) in metadata {
        // `edgeValues` is the standalone `@edgeValues` flag line (TF convention),
        // never a `key=value` metadata line.
        if key == "edgeValues" {
            continue;
        }
        let _ = match value {
            Some(value) => writeln!(output, "@{key}={value}"),
            None => writeln!(output, "@{key}"),
        };
    }
}
