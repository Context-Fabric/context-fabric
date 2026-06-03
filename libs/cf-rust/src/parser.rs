use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::{CfError, Result};
use crate::feature::{EdgeFeature, NodeFeature, TfFeature, parse_value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureKind {
    Node,
    Edge,
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfFeatureMetadata {
    pub name: String,
    pub kind: TfFeatureKind,
    pub metadata: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TfFeatureKind {
    Node,
    Edge,
    Config,
}

pub fn parse_tf_file(path: impl AsRef<Path>) -> Result<TfFeature> {
    let path = path.as_ref();
    let ParsedHeader {
        name,
        kind,
        metadata,
        lines,
    } = read_header_and_metadata(path)?;
    let data_lines = lines.map(|(zero_index, line)| {
        let line_no = zero_index + 1;
        line.map(|line| (line_no, line))
            .map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })
    });

    match kind {
        FeatureKind::Node => parse_node_feature(name, metadata, data_lines, path),
        FeatureKind::Edge => parse_edge_feature(name, metadata, data_lines, path),
        FeatureKind::Config => Ok(TfFeature::Config { name, metadata }),
    }
}

pub fn parse_tf_file_metadata(path: impl AsRef<Path>) -> Result<TfFeatureMetadata> {
    let path = path.as_ref();
    let ParsedHeader {
        name,
        kind,
        metadata,
        ..
    } = read_header_and_metadata(path)?;
    Ok(TfFeatureMetadata {
        name,
        kind: kind.into(),
        metadata,
    })
}

struct ParsedHeader {
    name: String,
    kind: FeatureKind,
    metadata: BTreeMap<String, Option<String>>,
    lines: std::iter::Enumerate<std::io::Lines<BufReader<File>>>,
}

fn read_header_and_metadata(path: &Path) -> Result<ParsedHeader> {
    let file = File::open(path).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut lines = BufReader::new(file).lines().enumerate();

    let (_, first) = lines.next().ok_or_else(|| CfError::Parse {
        path: path.to_path_buf(),
        line: 1,
        message: "empty feature file".to_string(),
    })?;
    let first = first.map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let kind = match first.trim_end() {
        "@node" => FeatureKind::Node,
        "@edge" => FeatureKind::Edge,
        "@config" => FeatureKind::Config,
        other => {
            return Err(CfError::Parse {
                path: path.to_path_buf(),
                line: 1,
                message: format!("missing @node/@edge/@config header, got {other:?}"),
            });
        }
    };

    let mut metadata = BTreeMap::new();
    for (zero_index, line) in lines.by_ref() {
        let line_no = zero_index + 1;
        let line = line.map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix('@') {
            if let Some((key, value)) = rest.split_once('=') {
                metadata.insert(key.to_string(), Some(value.to_string()));
            } else {
                metadata.insert(rest.to_string(), None);
            }
            continue;
        }
        return Err(CfError::Parse {
            path: path.to_path_buf(),
            line: line_no,
            message: "missing blank line after metadata".to_string(),
        });
    }

    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();

    normalize_metadata(&mut metadata);

    Ok(ParsedHeader {
        name,
        kind,
        metadata,
        lines,
    })
}

fn normalize_metadata(metadata: &mut BTreeMap<String, Option<String>>) {
    if metadata.contains_key("description") {
        return;
    }
    let Some(desc) = metadata.remove("desc").flatten() else {
        return;
    };
    let eg = metadata.remove("eg").flatten();
    let description = match eg {
        Some(eg) if !eg.is_empty() => format!("{desc} ({eg})"),
        _ => desc,
    };
    metadata.insert("description".to_string(), Some(description));
}

impl From<FeatureKind> for TfFeatureKind {
    fn from(kind: FeatureKind) -> Self {
        match kind {
            FeatureKind::Node => Self::Node,
            FeatureKind::Edge => Self::Edge,
            FeatureKind::Config => Self::Config,
        }
    }
}

fn parse_node_feature<I>(
    name: String,
    metadata: BTreeMap<String, Option<String>>,
    data_lines: I,
    path: &Path,
) -> Result<TfFeature>
where
    I: IntoIterator<Item = Result<(usize, String)>>,
{
    let value_type = metadata_value(&metadata, "valueType").unwrap_or("str");
    let mut values = HashMap::new();
    let mut implicit_node = 1_u32;
    for line_result in data_lines {
        let (line_no, line) = line_result?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let (nodes, raw_value) = if fields.len() == 1 {
            (vec![implicit_node], fields[0])
        } else if fields.len() == 2 {
            (parse_node_spec(fields[0], path, line_no)?, fields[1])
        } else {
            return Err(CfError::Parse {
                path: path.to_path_buf(),
                line: line_no,
                message: "node feature line has too many fields".to_string(),
            });
        };
        implicit_node = nodes.iter().max().copied().unwrap_or(implicit_node) + 1;
        if let Some(value) = parse_value(raw_value, value_type, path, line_no)? {
            for node in nodes {
                values.insert(node, value.clone());
            }
        }
    }
    let build_index = name != "otype";
    Ok(TfFeature::Node(NodeFeature::new_with_indexing(
        name,
        metadata,
        values,
        build_index,
    )))
}

fn parse_edge_feature<I>(
    name: String,
    metadata: BTreeMap<String, Option<String>>,
    data_lines: I,
    path: &Path,
) -> Result<TfFeature>
where
    I: IntoIterator<Item = Result<(usize, String)>>,
{
    let has_edge_values = metadata.contains_key("edgeValues");
    let value_type = metadata_value(&metadata, "valueType").unwrap_or("str");
    let mut values: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut edge_values = HashMap::new();
    let mut implicit_node = 1_u32;
    for line_result in data_lines {
        let (line_no, line) = line_result?;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() > 3 {
            return Err(CfError::Parse {
                path: path.to_path_buf(),
                line: line_no,
                message: "edge feature line has too many fields".to_string(),
            });
        }
        let (sources, targets, raw_value) = if fields.len() >= 2 {
            (
                parse_node_spec(fields[0], path, line_no)?,
                parse_node_spec(fields[1], path, line_no)?,
                fields.get(2).copied(),
            )
        } else if fields.len() == 1 {
            (
                vec![implicit_node],
                parse_node_spec(fields[0], path, line_no)?,
                None,
            )
        } else {
            return Err(CfError::Parse {
                path: path.to_path_buf(),
                line: line_no,
                message: "empty edge line".to_string(),
            });
        };
        implicit_node = sources.iter().max().copied().unwrap_or(implicit_node) + 1;
        let parsed_value = if has_edge_values {
            raw_value
                .map(|raw| parse_value(raw, value_type, path, line_no))
                .transpose()?
                .flatten()
        } else {
            None
        };
        for source in sources {
            values
                .entry(source)
                .or_default()
                .extend(targets.iter().copied());
            if let Some(value) = &parsed_value {
                for target in &targets {
                    edge_values.insert((source, *target), value.clone());
                }
            }
        }
    }
    Ok(TfFeature::Edge(EdgeFeature::new_with_values(
        name,
        metadata,
        values,
        edge_values,
    )))
}

fn metadata_value<'a>(
    metadata: &'a BTreeMap<String, Option<String>>,
    key: &str,
) -> Option<&'a str> {
    metadata.get(key).and_then(Option::as_deref).or_else(|| {
        if key == "valueType" {
            metadata.get("value_type").and_then(Option::as_deref)
        } else {
            None
        }
    })
}

pub fn parse_node_spec(raw: &str, path: &Path, line: usize) -> Result<Vec<u32>> {
    let mut nodes = Vec::new();
    for part in raw.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            let start = parse_node(start, path, line)?;
            let end = parse_node(end, path, line)?;
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            nodes.extend(start..=end);
        } else {
            nodes.push(parse_node(part, path, line)?);
        }
    }
    Ok(nodes)
}

fn parse_node(raw: &str, path: &Path, line: usize) -> Result<u32> {
    raw.parse::<u32>().map_err(|_| CfError::Parse {
        path: path.to_path_buf(),
        line,
        message: format!("invalid node spec {raw:?}"),
    })
}
