use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::error::{CfError, Result};
use crate::parser::parse_tf_file_metadata;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeatureInventory {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub configs: Vec<String>,
    pub computeds: Vec<String>,
    pub paths: BTreeMap<String, Vec<PathBuf>>,
    pub metadata: BTreeMap<String, BTreeMap<String, Option<String>>>,
}

impl FeatureInventory {
    pub fn nodes(&self) -> &[String] {
        &self.nodes
    }

    pub fn edges(&self) -> &[String] {
        &self.edges
    }

    pub fn configs(&self) -> &[String] {
        &self.configs
    }

    pub fn computeds(&self) -> &[String] {
        &self.computeds
    }

    pub fn categories(&self) -> BTreeMap<String, Vec<String>> {
        BTreeMap::from([
            ("computeds".to_string(), self.computeds.clone()),
            ("configs".to_string(), self.configs.clone()),
            ("edges".to_string(), self.edges.clone()),
            ("nodes".to_string(), self.nodes.clone()),
        ])
    }

    pub fn feature_paths(&self, name: &str) -> Option<&[PathBuf]> {
        self.paths.get(name).map(Vec::as_slice)
    }

    pub fn feature_metadata(&self, name: &str) -> Option<&BTreeMap<String, Option<String>>> {
        self.metadata.get(name)
    }
}

pub fn explore_features(path: impl AsRef<Path>) -> Result<FeatureInventory> {
    explore_feature_paths([path.as_ref().to_path_buf()])
}

pub fn explore_feature_paths<I, P>(paths: I) -> Result<FeatureInventory>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut inventory = FeatureInventory::default();
    inventory.computeds = computed_feature_names();
    for path in paths {
        let path = path.as_ref();
        for entry in fs::read_dir(path).map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })? {
            let entry = entry.map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            let feature_path = entry.path();
            if feature_path.extension().and_then(|ext| ext.to_str()) != Some("tf") {
                continue;
            }
            let Some(name) = feature_path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let name = name.to_string();
            match read_feature_header(&feature_path)? {
                FeatureHeader::Node => push_unique(&mut inventory.nodes, &name),
                FeatureHeader::Edge => push_unique(&mut inventory.edges, &name),
                FeatureHeader::Config => push_unique(&mut inventory.configs, &name),
            }
            inventory
                .paths
                .entry(name.clone())
                .or_default()
                .push(feature_path.clone());
            inventory
                .metadata
                .insert(name, parse_tf_file_metadata(&feature_path)?.metadata);
        }
    }
    inventory.nodes.sort();
    inventory.edges.sort();
    inventory.configs.sort();
    Ok(inventory)
}

fn computed_feature_names() -> Vec<String> {
    ["boundary", "levels", "order", "rank"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureHeader {
    Node,
    Edge,
    Config,
}

fn read_feature_header(path: &Path) -> Result<FeatureHeader> {
    let file = File::open(path).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut lines = BufReader::new(file).lines();
    let first = lines
        .next()
        .ok_or_else(|| CfError::Parse {
            path: path.to_path_buf(),
            line: 1,
            message: "empty feature file".to_string(),
        })?
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    match first.trim_end() {
        "@node" => Ok(FeatureHeader::Node),
        "@edge" => Ok(FeatureHeader::Edge),
        "@config" => Ok(FeatureHeader::Config),
        other => Err(CfError::Parse {
            path: path.to_path_buf(),
            line: 1,
            message: format!("missing @node/@edge/@config header, got {other:?}"),
        }),
    }
}
