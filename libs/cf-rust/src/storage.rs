use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CfError, Result};

pub const MISSING_STR_INDEX: usize = usize::MAX;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CsrValue {
    Int(i64),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmapArray {
    values: Vec<i64>,
    shape: Vec<usize>,
}

impl MmapArray {
    pub fn values(&self) -> &[i64] {
        &self.values
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path)?;
        parse_npy_i64_values(&bytes, path)
    }
}

#[derive(Debug, Clone)]
pub struct MmapManager {
    cfm_path: std::path::PathBuf,
    arrays: BTreeMap<String, MmapArray>,
    meta: Option<Value>,
}

impl MmapManager {
    pub fn new(cfm_path: impl AsRef<Path>) -> Self {
        Self {
            cfm_path: cfm_path.as_ref().to_path_buf(),
            arrays: BTreeMap::new(),
            meta: None,
        }
    }

    pub fn meta(&mut self) -> Result<&Value> {
        if self.meta.is_none() {
            let path = self.cfm_path.join("meta.json");
            self.meta = Some(serde_json::from_slice(&fs::read(path)?)?);
        }
        Ok(self.meta.as_ref().expect("metadata was just loaded"))
    }

    pub fn max_slot(&mut self) -> Result<u64> {
        Ok(self
            .meta()?
            .get("max_slot")
            .and_then(Value::as_u64)
            .unwrap_or(0))
    }

    pub fn max_node(&mut self) -> Result<u64> {
        Ok(self
            .meta()?
            .get("max_node")
            .and_then(Value::as_u64)
            .unwrap_or(0))
    }

    pub fn slot_type(&mut self) -> Result<Option<String>> {
        Ok(self
            .meta()?
            .get("slot_type")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    pub fn node_types(&mut self) -> Result<Vec<String>> {
        Ok(self
            .meta()?
            .get("node_types")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn get_array(&mut self, path_parts: &[&str]) -> Result<&MmapArray> {
        let key = path_parts.join("/");
        if !self.arrays.contains_key(&key) {
            let path = self.path_with_extension(path_parts, "npy");
            self.arrays.insert(key.clone(), MmapArray::load(path)?);
        }
        Ok(self.arrays.get(&key).expect("array was just loaded"))
    }

    pub fn get_json(&self, path_parts: &[&str]) -> Result<Value> {
        let path = self.path_with_extension(path_parts, "json");
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    pub fn exists(&self) -> bool {
        self.cfm_path.join("meta.json").exists()
    }

    pub fn close(&mut self) {
        self.arrays.clear();
        self.meta = None;
    }

    pub fn cached_arrays_count(&self) -> usize {
        self.arrays.len()
    }

    fn path_with_extension(&self, path_parts: &[&str], extension: &str) -> std::path::PathBuf {
        let (file_name, directories) = path_parts
            .split_last()
            .expect("at least one path component is required");
        let mut path = self.cfm_path.clone();
        for part in directories {
            path.push(part);
        }
        path.push(format!("{file_name}.{extension}"));
        path
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringPool {
    pub strings: Vec<String>,
    indices: Vec<Option<usize>>,
}

impl StringPool {
    pub fn from_dict(data: &BTreeMap<u32, String>, max_node: u32) -> Self {
        let mut value_to_index = BTreeMap::new();
        let mut strings = Vec::new();
        let mut indices = vec![None; max_node as usize + 1];

        for (node, value) in data {
            if *node == 0 || *node > max_node {
                continue;
            }
            let index = *value_to_index.entry(value.clone()).or_insert_with(|| {
                strings.push(value.clone());
                strings.len() - 1
            });
            indices[*node as usize] = Some(index);
        }

        Self { strings, indices }
    }

    pub fn get(&self, node: i64) -> Option<&str> {
        if node <= 0 {
            return None;
        }
        self.indices
            .get(node as usize)
            .and_then(|index| index.and_then(|index| self.strings.get(index)))
            .map(String::as_str)
    }

    pub fn get_value_index(&self, value: &str) -> Option<usize> {
        self.strings.iter().position(|candidate| candidate == value)
    }

    pub fn filter_by_value(&self, nodes: &[u32], expected: &str) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.get(*node as i64) == Some(expected))
            .collect()
    }

    pub fn filter_by_values(&self, nodes: &[u32], expected: &BTreeSet<String>) -> Vec<u32> {
        if expected.is_empty() {
            return Vec::new();
        }
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.get(*node as i64)
                    .is_some_and(|value| expected.contains(value))
            })
            .collect()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntFeatureArray {
    values: Vec<Option<i64>>,
}

impl IntFeatureArray {
    pub fn from_dict(data: &BTreeMap<u32, i64>, max_node: u32) -> Self {
        let mut values = vec![None; max_node as usize + 1];
        for (node, value) in data {
            if *node == 0 || *node > max_node {
                continue;
            }
            values[*node as usize] = Some(*value);
        }
        Self { values }
    }

    pub fn get(&self, node: i64) -> Option<i64> {
        if node <= 0 {
            return None;
        }
        self.values.get(node as usize).copied().flatten()
    }

    pub fn filter_by_value(&self, nodes: &[u32], expected: i64) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.get(*node as i64) == Some(expected))
            .collect()
    }

    pub fn filter_by_values(&self, nodes: &[u32], expected: &BTreeSet<i64>) -> Vec<u32> {
        if expected.is_empty() {
            return Vec::new();
        }
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.get(*node as i64)
                    .is_some_and(|value| expected.contains(&value))
            })
            .collect()
    }

    pub fn filter_less_than(&self, nodes: &[u32], threshold: i64) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.get(*node as i64)
                    .is_some_and(|value| value < threshold)
            })
            .collect()
    }

    pub fn filter_greater_than(&self, nodes: &[u32], threshold: i64) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| {
                self.get(*node as i64)
                    .is_some_and(|value| value > threshold)
            })
            .collect()
    }

    pub fn filter_has_value(&self, nodes: &[u32]) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.get(*node as i64).is_some())
            .collect()
    }

    pub fn filter_missing_value(&self, nodes: &[u32]) -> Vec<u32> {
        nodes
            .iter()
            .copied()
            .filter(|node| self.get(*node as i64).is_none())
            .collect()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSRArray {
    rows: Vec<Vec<u32>>,
    #[serde(skip)]
    cached_rows: Option<Vec<Vec<u32>>>,
}

impl CSRArray {
    pub fn from_sequences(sequences: &[Vec<u32>]) -> Self {
        Self {
            rows: sequences.to_vec(),
            cached_rows: None,
        }
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn row(&self, index: usize) -> Option<&[u32]> {
        self.active_rows().get(index).map(Vec::as_slice)
    }

    pub fn get_as_tuple(&self, index: usize) -> Vec<u32> {
        self.row(index).unwrap_or(&[]).to_vec()
    }

    pub fn get_all_targets<I>(&self, sources: I) -> BTreeSet<u32>
    where
        I: IntoIterator<Item = i64>,
    {
        let mut targets = BTreeSet::new();
        for source in sources {
            if let Some(row) = self.row_for_source(source) {
                targets.extend(row.iter().copied());
            }
        }
        targets
    }

    pub fn filter_sources_with_targets_in<I, J>(
        &self,
        sources: I,
        targets: J,
    ) -> (BTreeSet<u32>, BTreeSet<u32>)
    where
        I: IntoIterator<Item = i64>,
        J: IntoIterator<Item = u32>,
    {
        let target_set: BTreeSet<u32> = targets.into_iter().collect();
        if target_set.is_empty() {
            return (BTreeSet::new(), BTreeSet::new());
        }

        let mut matched_sources = BTreeSet::new();
        let mut matched_targets = BTreeSet::new();
        for source in sources {
            let Some(row) = self.row_for_source(source) else {
                continue;
            };
            let source_matches: Vec<u32> = row
                .iter()
                .copied()
                .filter(|target| target_set.contains(target))
                .collect();
            if !source_matches.is_empty() {
                matched_sources.insert(source as u32);
                matched_targets.extend(source_matches);
            }
        }
        (matched_sources, matched_targets)
    }

    pub fn is_cached(&self) -> bool {
        self.cached_rows.is_some()
    }

    pub fn preload_to_ram(&mut self) {
        if self.cached_rows.is_none() {
            self.cached_rows = Some(self.rows.clone());
        }
    }

    pub fn release_cache(&mut self) {
        self.cached_rows = None;
    }

    pub fn memory_usage_bytes(&self) -> usize {
        self.cached_rows
            .as_ref()
            .map(|rows| {
                rows.iter()
                    .map(|row| row.len() * std::mem::size_of::<u32>())
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    fn active_rows(&self) -> &[Vec<u32>] {
        self.cached_rows.as_deref().unwrap_or(&self.rows)
    }

    fn row_for_source(&self, source: i64) -> Option<&[u32]> {
        if source <= 0 {
            return None;
        }
        self.row((source - 1) as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CSRArrayWithValues {
    rows: Vec<BTreeMap<u32, CsrValue>>,
}

impl CSRArrayWithValues {
    pub fn from_dict_of_dicts(
        data: &BTreeMap<u32, BTreeMap<u32, CsrValue>>,
        num_rows: usize,
    ) -> Self {
        let mut rows = vec![BTreeMap::new(); num_rows];
        for (row, values) in data {
            if let Some(target_row) = rows.get_mut(*row as usize) {
                *target_row = values.clone();
            }
        }
        Self { rows }
    }

    pub fn from_int_dict_of_dicts(
        data: &BTreeMap<u32, BTreeMap<u32, i64>>,
        num_rows: usize,
    ) -> Self {
        let converted = data
            .iter()
            .map(|(row, values)| {
                (
                    *row,
                    values
                        .iter()
                        .map(|(target, value)| (*target, CsrValue::Int(*value)))
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        Self::from_dict_of_dicts(&converted, num_rows)
    }

    pub fn from_string_dict_of_dicts(
        data: &BTreeMap<u32, BTreeMap<u32, String>>,
        num_rows: usize,
    ) -> Self {
        let converted = data
            .iter()
            .map(|(row, values)| {
                (
                    *row,
                    values
                        .iter()
                        .map(|(target, value)| (*target, CsrValue::Str(value.clone())))
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        Self::from_dict_of_dicts(&converted, num_rows)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn row(&self, index: usize) -> Option<(Vec<u32>, Vec<CsrValue>)> {
        self.rows.get(index).map(|row| {
            (
                row.keys().copied().collect(),
                row.values().cloned().collect(),
            )
        })
    }

    pub fn get_as_dict(&self, index: usize) -> BTreeMap<u32, CsrValue> {
        self.rows.get(index).cloned().unwrap_or_default()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, serde_json::to_vec(self)?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }
}

fn parse_npy_i64_values(bytes: &[u8], path: &Path) -> Result<MmapArray> {
    if bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
        return Err(npy_error(path, "missing NumPy magic header"));
    }

    let major = bytes[6];
    let header_len_size = match major {
        1 => 2,
        2 | 3 => 4,
        _ => return Err(npy_error(path, "unsupported NumPy format version")),
    };
    let header_start = 8 + header_len_size;
    if bytes.len() < header_start {
        return Err(npy_error(path, "truncated NumPy header"));
    }

    let header_len = match header_len_size {
        2 => u16::from_le_bytes([bytes[8], bytes[9]]) as usize,
        _ => u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize,
    };
    let data_start = header_start + header_len;
    if bytes.len() < data_start {
        return Err(npy_error(path, "truncated NumPy data"));
    }

    let header = std::str::from_utf8(&bytes[header_start..data_start])
        .map_err(|_| npy_error(path, "NumPy header is not UTF-8"))?;
    if header.contains("'fortran_order': True") || header.contains("\"fortran_order\": True") {
        return Err(npy_error(path, "Fortran-order arrays are not supported"));
    }
    let descr = extract_npy_string_field(header, "descr")
        .ok_or_else(|| npy_error(path, "missing NumPy descr"))?;
    let shape = extract_npy_shape(header).ok_or_else(|| npy_error(path, "missing NumPy shape"))?;
    let count = shape.iter().product::<usize>();
    let data = &bytes[data_start..];

    let values = match descr.as_str() {
        "|u1" | "<u1" => {
            if data.len() < count {
                return Err(npy_error(path, "truncated uint8 data"));
            }
            data.iter().take(count).map(|value| *value as i64).collect()
        }
        "|i1" | "<i1" => {
            if data.len() < count {
                return Err(npy_error(path, "truncated int8 data"));
            }
            data.iter()
                .take(count)
                .map(|value| (*value as i8) as i64)
                .collect()
        }
        "<u4" => {
            let needed = count * 4;
            if data.len() < needed {
                return Err(npy_error(path, "truncated uint32 data"));
            }
            data.chunks_exact(4)
                .take(count)
                .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()) as i64)
                .collect()
        }
        "<i4" => {
            let needed = count * 4;
            if data.len() < needed {
                return Err(npy_error(path, "truncated int32 data"));
            }
            data.chunks_exact(4)
                .take(count)
                .map(|chunk| i32::from_le_bytes(chunk.try_into().unwrap()) as i64)
                .collect()
        }
        "<i8" => {
            let needed = count * 8;
            if data.len() < needed {
                return Err(npy_error(path, "truncated int64 data"));
            }
            data.chunks_exact(8)
                .take(count)
                .map(|chunk| i64::from_le_bytes(chunk.try_into().unwrap()))
                .collect()
        }
        _ => return Err(npy_error(path, "unsupported NumPy dtype")),
    };

    Ok(MmapArray { values, shape })
}

fn extract_npy_string_field(header: &str, name: &str) -> Option<String> {
    let field = format!("'{name}':");
    let start = header.find(&field)? + field.len();
    let rest = header[start..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &rest[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn extract_npy_shape(header: &str) -> Option<Vec<usize>> {
    let field = "'shape':";
    let start = header.find(field)? + field.len();
    let rest = header[start..].trim_start();
    let open = rest.find('(')?;
    let close = rest[open + 1..].find(')')? + open + 1;
    let values = rest[open + 1..close]
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                None
            } else {
                part.parse::<usize>().ok()
            }
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

fn npy_error(path: &Path, message: &str) -> CfError {
    CfError::Parse {
        path: path.to_path_buf(),
        line: 0,
        message: message.to_string(),
    }
}
