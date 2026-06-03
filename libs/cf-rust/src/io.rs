use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::compiled::compile_features;
use crate::config::CFM_VERSION;
use crate::error::Result;
use crate::feature::{EdgeFeature, NodeFeature, TfFeature, tf_from_value};
use crate::parser::{TfFeatureKind, parse_tf_file, parse_tf_file_metadata};

pub const DATA_TYPES: &[&str] = &["str", "int"];
pub const ERROR_CUTOFF: usize = 20;
pub const MEM_MSG: &str = "CF is out of memory!\nIf this happens and your computer has more than 3GB RAM on board:\n* close all other programs and try again.\n";
pub const FATAL_MSG: &str = "There was a fatal error! The message is:\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TfDataContent {
    Node(NodeFeature),
    Edge(EdgeFeature),
    Config {
        name: String,
        metadata: BTreeMap<String, Option<String>>,
    },
}

#[derive(Debug, Clone)]
pub struct TfData {
    pub path: String,
    pub dir_name: String,
    pub file_name: String,
    pub extension: String,
    pub edge_values: bool,
    pub is_edge: Option<bool>,
    pub is_config: Option<bool>,
    pub metadata: BTreeMap<String, Option<String>>,
    pub data: Option<TfDataContent>,
    pub data_loaded: bool,
    pub data_error: bool,
    pub data_type: String,
}

pub type Data = TfData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compiler {
    source_dir: PathBuf,
}

impl Compiler {
    pub fn new(source_dir: impl AsRef<Path>) -> Self {
        Self {
            source_dir: source_dir.as_ref().to_path_buf(),
        }
    }

    pub fn source_dir(&self) -> &Path {
        &self.source_dir
    }

    pub fn default_output_path(&self) -> PathBuf {
        default_compiled_output_path(&self.source_dir)
    }

    pub fn compile(&self, output_path: Option<&Path>) -> Result<bool> {
        let output_path = output_path
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.default_output_path());
        compile_corpus_to_path(&self.source_dir, &output_path)
    }
}

pub fn default_compiled_output_path(source_dir: impl AsRef<Path>) -> PathBuf {
    source_dir
        .as_ref()
        .join(".cfm")
        .join(CFM_VERSION)
        .join("corpus.cfr")
}

pub fn compile_corpus(source_dir: impl AsRef<Path>, output_path: Option<&Path>) -> Result<bool> {
    let output_path = output_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_compiled_output_path(&source_dir));
    compile_corpus_to_path(source_dir, output_path)
}

fn compile_corpus_to_path(
    source_dir: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<bool> {
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    compile_features(source_dir, output_path, &[])?;
    Ok(true)
}

impl TfData {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path_ref = path.as_ref();
        let dir_name = path_ref
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned())
            .unwrap_or_default();
        let file_name = path_ref
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let extension = path_ref
            .extension()
            .map(|extension| format!(".{}", extension.to_string_lossy()))
            .unwrap_or_default();

        Self {
            path: path_ref.to_string_lossy().into_owned(),
            dir_name,
            file_name,
            extension,
            edge_values: false,
            is_edge: None,
            is_config: None,
            metadata: BTreeMap::new(),
            data: None,
            data_loaded: false,
            data_error: false,
            data_type: "str".to_string(),
        }
    }

    pub fn load(&mut self, meta_only: bool) -> bool {
        let path = PathBuf::from(&self.path);
        if !path.exists() {
            self.data_error = true;
            return false;
        }

        let result = if meta_only {
            parse_tf_file_metadata(&path).map(|metadata| {
                self.apply_kind(metadata.kind);
                self.metadata = metadata.metadata;
                self.set_data_type();
                self.data = None;
            })
        } else {
            parse_tf_file(&path).map(|feature| self.set_feature(feature))
        };

        match result {
            Ok(()) => {
                if !meta_only && !matches!(self.is_config, Some(true)) {
                    self.data_loaded = true;
                }
                true
            }
            Err(_) => {
                self.data_error = true;
                false
            }
        }
    }

    pub fn load_meta_only(&mut self) -> bool {
        self.load(true)
    }

    #[allow(non_snake_case)]
    pub fn loadMetaOnly(&mut self) -> bool {
        self.load_meta_only()
    }

    pub fn unload(&mut self) {
        self.data = None;
        self.data_loaded = false;
    }

    pub fn save(&self) -> bool {
        self.write_tf().is_ok()
    }

    pub fn set_data_type(&mut self) {
        if self.is_config == Some(true) {
            return;
        }
        self.data_type = self
            .metadata
            .get("valueType")
            .and_then(Option::as_deref)
            .or_else(|| self.metadata.get("value_type").and_then(Option::as_deref))
            .filter(|value_type| DATA_TYPES.contains(value_type))
            .unwrap_or("str")
            .to_string();
    }

    #[allow(non_snake_case)]
    pub fn setDataType(&mut self) {
        self.set_data_type()
    }

    #[allow(non_snake_case)]
    pub fn dataLoaded(&self) -> bool {
        self.data_loaded
    }

    #[allow(non_snake_case)]
    pub fn dataError(&self) -> bool {
        self.data_error
    }

    #[allow(non_snake_case)]
    pub fn dirName(&self) -> &str {
        &self.dir_name
    }

    #[allow(non_snake_case)]
    pub fn fileName(&self) -> &str {
        &self.file_name
    }

    #[allow(non_snake_case)]
    pub fn edgeValues(&self) -> bool {
        self.edge_values
    }

    #[allow(non_snake_case)]
    pub fn isEdge(&self) -> Option<bool> {
        self.is_edge
    }

    #[allow(non_snake_case)]
    pub fn isConfig(&self) -> Option<bool> {
        self.is_config
    }

    #[allow(non_snake_case)]
    pub fn metaData(&self) -> &BTreeMap<String, Option<String>> {
        &self.metadata
    }

    #[allow(non_snake_case)]
    pub fn dataType(&self) -> &str {
        &self.data_type
    }

    fn set_feature(&mut self, feature: TfFeature) {
        match feature {
            TfFeature::Node(feature) => {
                self.is_edge = Some(false);
                self.is_config = Some(false);
                self.edge_values = false;
                self.metadata = feature.metadata.clone();
                self.set_data_type();
                self.data = Some(TfDataContent::Node(feature));
            }
            TfFeature::Edge(feature) => {
                self.is_edge = Some(true);
                self.is_config = Some(false);
                self.edge_values = feature.has_edge_values();
                self.metadata = feature.metadata.clone();
                self.set_data_type();
                self.data = Some(TfDataContent::Edge(feature));
            }
            TfFeature::Config { name, metadata } => {
                self.is_edge = Some(false);
                self.is_config = Some(true);
                self.edge_values = false;
                self.metadata = metadata.clone();
                self.data = Some(TfDataContent::Config { name, metadata });
            }
        }
    }

    fn apply_kind(&mut self, kind: TfFeatureKind) {
        match kind {
            TfFeatureKind::Node => {
                self.is_edge = Some(false);
                self.is_config = Some(false);
            }
            TfFeatureKind::Edge => {
                self.is_edge = Some(true);
                self.is_config = Some(false);
            }
            TfFeatureKind::Config => {
                self.is_edge = Some(false);
                self.is_config = Some(true);
            }
        }
    }

    fn write_tf(&self) -> Result<()> {
        let mut output = String::new();
        match &self.data {
            Some(TfDataContent::Node(feature)) => {
                output.push_str("@node\n");
                write_metadata(&mut output, &feature.metadata);
                output.push('\n');
                for (node, value) in feature.items() {
                    output.push_str(&format!("{node}\t{}\n", tf_from_value(&value)));
                }
            }
            Some(TfDataContent::Edge(feature)) => {
                output.push_str("@edge\n");
                write_metadata(&mut output, &feature.metadata);
                output.push('\n');
                for (source, targets) in feature.items() {
                    for target in targets {
                        match feature.edge_value(source, target) {
                            Some(value) => {
                                output.push_str(&format!(
                                    "{source}\t{target}\t{}\n",
                                    tf_from_value(value)
                                ));
                            }
                            None => {
                                output.push_str(&format!("{source}\t{target}\n"));
                            }
                        }
                    }
                }
            }
            Some(TfDataContent::Config { metadata, .. }) => {
                output.push_str("@config\n");
                write_metadata(&mut output, metadata);
                output.push('\n');
            }
            None => {
                let header = if self.is_config == Some(true) {
                    "@config"
                } else if self.is_edge == Some(true) {
                    "@edge"
                } else {
                    "@node"
                };
                output.push_str(header);
                output.push('\n');
                write_metadata(&mut output, &self.metadata);
                output.push('\n');
            }
        }
        fs::write(&self.path, output)?;
        Ok(())
    }
}

fn write_metadata(output: &mut String, metadata: &BTreeMap<String, Option<String>>) {
    for (key, value) in metadata {
        match value {
            Some(value) => output.push_str(&format!("@{key}={value}\n")),
            None => output.push_str(&format!("@{key}\n")),
        }
    }
}

impl From<TfFeature> for TfDataContent {
    fn from(feature: TfFeature) -> Self {
        match feature {
            TfFeature::Node(feature) => Self::Node(feature),
            TfFeature::Edge(feature) => Self::Edge(feature),
            TfFeature::Config { name, metadata } => Self::Config { name, metadata },
        }
    }
}
