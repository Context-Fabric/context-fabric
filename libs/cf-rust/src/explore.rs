use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::{CfError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeatureInventory {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub configs: Vec<String>,
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

    pub fn categories(&self) -> BTreeMap<String, Vec<String>> {
        BTreeMap::from([
            ("configs".to_string(), self.configs.clone()),
            ("edges".to_string(), self.edges.clone()),
            ("nodes".to_string(), self.nodes.clone()),
        ])
    }
}

pub fn explore_features(path: impl AsRef<Path>) -> Result<FeatureInventory> {
    let path = path.as_ref();
    let mut inventory = FeatureInventory::default();
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
        match read_feature_header(&feature_path)? {
            FeatureHeader::Node => inventory.nodes.push(name.to_string()),
            FeatureHeader::Edge => inventory.edges.push(name.to_string()),
            FeatureHeader::Config => inventory.configs.push(name.to_string()),
        }
    }
    inventory.nodes.sort();
    inventory.edges.sort();
    inventory.configs.sort();
    Ok(inventory)
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
