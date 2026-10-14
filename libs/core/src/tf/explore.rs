//! Feature inventory of `.tf` directories without loading feature data.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{CfError, Result};
use crate::tf::{TfFeatureKind, parse_tf_file_metadata};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeatureInventory {
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub configs: Vec<String>,
    pub computeds: Vec<String>,
    pub paths: BTreeMap<String, Vec<PathBuf>>,
    pub metadata: BTreeMap<String, BTreeMap<String, Option<String>>>,
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
            let header = parse_tf_file_metadata(&feature_path)?;
            match header.kind {
                TfFeatureKind::Node => push_unique(&mut inventory.nodes, &name),
                TfFeatureKind::Edge => push_unique(&mut inventory.edges, &name),
                TfFeatureKind::Config => push_unique(&mut inventory.configs, &name),
            }
            inventory
                .paths
                .entry(name.clone())
                .or_default()
                .push(feature_path.clone());
            inventory.metadata.insert(name, header.metadata);
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
