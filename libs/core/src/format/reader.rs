//! Parses the table of contents of a `.cfr` file without copying payloads.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::config::CFR_VERSION;
use crate::error::{CfError, Result};
use crate::format::{
    BOUNDARY_MAGIC, CompiledConfigFeature, CompiledEdgeFeature, CompiledMetadata,
    CompiledNodeFeature, EDGE_VALUES_MAGIC, FEATURE_METADATA_MAGIC, FORMAT_FAMILY, LEV_DOWN_MAGIC,
    LEV_UP_MAGIC, MAGIC, NodeFeatureEncoding, REVERSE_EDGES_MAGIC, ROW_INDEX_MAGIC, SECTIONS_MAGIC,
    STRUCTURE_MAGIC, bounded_capacity, invalid_compiled, map_file,
};
use crate::index::edge_index::ReverseEdgeLayout;
use crate::index::row_index::{RowIndexKind, RowIndexLayout};
use crate::index::{SectionsData, StructureData, StructureHeading};
use crate::value::FeatureValue;

pub fn inspect_compiled(path: impl AsRef<Path>) -> Result<CompiledMetadata> {
    let path = path.as_ref();
    let mmap = map_file(path)?;
    inspect_compiled_bytes(path, &mmap)
}

pub(crate) fn inspect_compiled_bytes(path: &Path, bytes: &[u8]) -> Result<CompiledMetadata> {
    let mut reader = Reader::new(path, bytes);
    reader.expect_magic()?;

    let mut node_features = Vec::new();
    let node_count = reader.read_u32()? as usize;
    for _ in 0..node_count {
        node_features.push(reader.inspect_node_feature()?);
    }

    let mut edge_features = Vec::new();
    let edge_count = reader.read_u32()? as usize;
    for _ in 0..edge_count {
        edge_features.push(reader.inspect_edge_feature()?);
    }

    let (order_len, order_start) = reader.inspect_u32_vec()?;
    let (rank_len, rank_start) = reader.inspect_u32_vec()?;
    let config_features = reader.inspect_config_features()?;
    let (node_metadata, edge_metadata) = reader.read_feature_metadata_section()?;
    for feature in &mut node_features {
        if let Some(metadata) = node_metadata.get(&feature.name) {
            feature.metadata = metadata.clone();
        }
    }
    let edge_values = reader.inspect_edge_values_section()?;
    let structure_start = if reader.peek_magic(STRUCTURE_MAGIC) {
        Some(reader.inspect_structure_section()?)
    } else {
        None
    };

    reader.expect_section_magic(LEV_UP_MAGIC)?;
    let lev_up_start = reader.inspect_csr()?;
    reader.expect_section_magic(LEV_DOWN_MAGIC)?;
    let lev_down_start = reader.inspect_csr()?;
    reader.expect_section_magic(BOUNDARY_MAGIC)?;
    let boundary_first_start = reader.inspect_csr()?;
    let boundary_last_start = reader.inspect_csr()?;

    let reverse_edges: HashMap<String, ReverseEdgeLayout> = reader
        .inspect_reverse_edges_section()?
        .into_iter()
        .collect();
    let mut row_indexes: HashMap<(RowIndexKind, String), RowIndexLayout> = reader
        .inspect_row_index_section()?
        .into_iter()
        .map(|(kind, name, layout)| ((kind, name), layout))
        .collect();
    for feature in &mut node_features {
        feature.row_index = row_indexes.remove(&(RowIndexKind::Node, feature.name.clone()));
        if feature.encoding == NodeFeatureEncoding::Mixed && feature.row_index.is_none() {
            return Err(invalid_compiled(
                path,
                format!("mixed feature {} has no row offsets", feature.name),
            ));
        }
    }
    let edge_features = edge_features
        .into_iter()
        .map(|entry| {
            let missing = |section: &str| {
                invalid_compiled(
                    path,
                    format!("edge feature {} has no {section}", entry.name),
                )
            };
            let reverse_index = *reverse_edges
                .get(&entry.name)
                .ok_or_else(|| missing("reverse index"))?;
            let row_index = row_indexes
                .remove(&(RowIndexKind::Edge, entry.name.clone()))
                .ok_or_else(|| missing("row offsets"))?;
            let (edge_value_count, edge_values_start) = edge_values
                .get(&entry.name)
                .map_or((0, None), |(count, start)| (*count, Some(*start)));
            Ok(CompiledEdgeFeature {
                metadata: edge_metadata.get(&entry.name).cloned().unwrap_or_default(),
                name: entry.name,
                row_count: entry.row_count,
                payload_start: entry.payload_start,
                payload_end: entry.payload_end,
                edge_value_count,
                edge_values_start,
                reverse_index,
                row_index,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // `CFRSECT1` is the last section: record where it starts and leave the
    // parse to the first `sections_data()` call.
    let sections_start = if reader.is_done() {
        None
    } else {
        reader.expect_section_magic(SECTIONS_MAGIC)?;
        Some(reader.offset)
    };

    Ok(CompiledMetadata {
        byte_len: bytes.len(),
        node_features,
        edge_features,
        config_features,
        order_len,
        rank_len,
        order_start,
        rank_start,
        structure_start,
        lev_up_start,
        lev_down_start,
        boundary_first_start,
        boundary_last_start,
        sections_start,
    })
}

/// Feature metadata (`key -> value`) by feature name.
type MetadataByFeature = BTreeMap<String, BTreeMap<String, Option<String>>>;

/// An edge feature's entry in the feature table, before the sections that
/// complete its [`CompiledEdgeFeature`] are read.
struct EdgeFeatureEntry {
    name: String,
    row_count: usize,
    payload_start: usize,
    payload_end: usize,
}

pub(crate) struct Reader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    /// A reader positioned at `offset`, for parsing a section on demand.
    pub(crate) fn at(path: &'a Path, bytes: &'a [u8], offset: usize) -> Self {
        Self {
            path,
            bytes,
            offset,
        }
    }

    pub(crate) fn new(path: &'a Path, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            offset: 0,
        }
    }

    /// Checks the header magic. A cache of another `.cfr` format version is
    /// rejected with [`CfError::IncompatibleCache`] so the caller can recompile.
    fn expect_magic(&mut self) -> Result<()> {
        let magic = self.take(MAGIC.len())?;
        if magic == MAGIC {
            return Ok(());
        }
        if magic.starts_with(FORMAT_FAMILY) {
            return Err(CfError::IncompatibleCache {
                path: self.path.to_path_buf(),
                found: String::from_utf8_lossy(&magic[FORMAT_FAMILY.len()..]).into_owned(),
                expected: CFR_VERSION,
            });
        }
        Err(self.invalid("invalid compiled corpus magic"))
    }

    /// Consumes the magic of a mandatory section.
    fn expect_section_magic(&mut self, magic: &[u8; 8]) -> Result<()> {
        if self.take(magic.len())? != magic {
            return Err(self.invalid(format!(
                "missing section {}",
                String::from_utf8_lossy(magic).trim_end_matches('\0')
            )));
        }
        Ok(())
    }

    fn inspect_node_feature(&mut self) -> Result<CompiledNodeFeature> {
        let name = self.read_string()?;
        let encoding_tag = self.read_u8()?;
        let row_count = self.read_u32()? as usize;
        let aux_count = self.read_u32()? as usize;
        let payload_len = self.read_u32()? as usize;
        let payload_start = self.offset;
        self.skip_bytes(payload_len)?;
        let (encoding, string_pool_count) = match encoding_tag {
            0 => (NodeFeatureEncoding::StringPool, Some(aux_count)),
            1 => (NodeFeatureEncoding::Mixed, None),
            _ => return Err(self.invalid("invalid node feature encoding")),
        };
        Ok(CompiledNodeFeature {
            name,
            encoding,
            row_count,
            string_pool_count,
            payload_start,
            payload_end: self.offset,
            metadata: BTreeMap::new(),
            row_index: None,
        })
    }

    fn inspect_config_features(&mut self) -> Result<Vec<CompiledConfigFeature>> {
        let feature_count = self.read_u32()? as usize;
        let mut features = Vec::with_capacity(self.capacity_for(feature_count, 8));
        for _ in 0..feature_count {
            let name = self.read_string()?;
            let payload_start = self.offset;
            let metadata_count = self.read_u32()? as usize;
            for _ in 0..metadata_count {
                self.skip_string()?;
                match self.read_u8()? {
                    0 => {}
                    1 => self.skip_string()?,
                    _ => return Err(self.invalid("invalid config metadata value tag")),
                }
            }
            features.push(CompiledConfigFeature {
                name,
                metadata_count,
                payload_start,
                payload_end: self.offset,
            });
        }
        Ok(features)
    }

    /// Reads `CFRMETA1`: the metadata of every node feature, then of every
    /// edge feature, by feature name.
    fn read_feature_metadata_section(&mut self) -> Result<(MetadataByFeature, MetadataByFeature)> {
        let magic = self.take(FEATURE_METADATA_MAGIC.len())?;
        if magic != FEATURE_METADATA_MAGIC {
            return Err(self.invalid("invalid feature metadata section magic"));
        }
        let node_count = self.read_u32()? as usize;
        let mut node_metadata = BTreeMap::new();
        for _ in 0..node_count {
            let name = self.read_string()?;
            node_metadata.insert(name, self.read_metadata_map()?);
        }
        let edge_count = self.read_u32()? as usize;
        let mut edge_metadata = BTreeMap::new();
        for _ in 0..edge_count {
            let name = self.read_string()?;
            edge_metadata.insert(name, self.read_metadata_map()?);
        }
        Ok((node_metadata, edge_metadata))
    }

    fn inspect_edge_values_section(&mut self) -> Result<HashMap<String, (usize, usize)>> {
        let magic = self.take(EDGE_VALUES_MAGIC.len())?;
        if magic != EDGE_VALUES_MAGIC {
            return Err(self.invalid("invalid edge value metadata magic"));
        }
        let feature_count = self.read_u32()? as usize;
        let mut edge_values = HashMap::with_capacity(self.capacity_for(feature_count, 8));
        for _ in 0..feature_count {
            let name = self.read_string()?;
            let value_count = self.read_u32()? as usize;
            let values_start = self.offset;
            for _ in 0..value_count {
                self.skip_bytes(8)?;
                self.skip_feature_value()?;
            }
            edge_values.insert(name, (value_count, values_start));
        }
        Ok(edge_values)
    }

    fn inspect_structure_section(&mut self) -> Result<usize> {
        let start = self.offset;
        self.read_structure_section()?;
        Ok(start)
    }

    fn peek_magic(&self, magic: &[u8; 8]) -> bool {
        self.bytes
            .get(self.offset..self.offset.saturating_add(magic.len()))
            == Some(&magic[..])
    }

    /// Parses the `CFRROWS1` entries (vectors are bounds-checked, not copied).
    fn inspect_row_index_section(&mut self) -> Result<Vec<(RowIndexKind, String, RowIndexLayout)>> {
        self.expect_section_magic(ROW_INDEX_MAGIC)?;
        let count = self.read_u32()? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let kind = RowIndexKind::from_tag(self.read_u8()?)
                .ok_or_else(|| self.invalid("invalid row index kind"))?;
            let name = self.read_string()?;
            let (layout, end) = RowIndexLayout::inspect(self.path, self.bytes, self.offset)?;
            self.offset = end;
            entries.push((kind, name, layout));
        }
        Ok(entries)
    }

    /// Parses the `CFRREVE1` section header and each feature's reverse-index
    /// layout (payloads are validated for bounds, not copied).
    fn inspect_reverse_edges_section(&mut self) -> Result<Vec<(String, ReverseEdgeLayout)>> {
        self.expect_section_magic(REVERSE_EDGES_MAGIC)?;
        let count = self.read_u32()? as usize;
        let mut layouts = Vec::with_capacity(count);
        for _ in 0..count {
            let name = self.read_string()?;
            let (layout, end) = ReverseEdgeLayout::inspect(self.path, self.bytes, self.offset)?;
            self.offset = end;
            layouts.push((name, layout));
        }
        Ok(layouts)
    }

    /// Advances past one CSR (`row_count`, offsets vec, data vec), returning the
    /// offset of its `row_count` field (the start consumed by `read_csr_row`).
    fn inspect_csr(&mut self) -> Result<usize> {
        let start = self.offset;
        let _row_count = self.read_u32()?;
        let offsets_len = self.read_u32()? as usize;
        self.skip_bytes(
            offsets_len
                .checked_mul(4)
                .ok_or_else(|| self.invalid("compiled CSR offsets byte count overflow"))?,
        )?;
        let data_len = self.read_u32()? as usize;
        self.skip_bytes(
            data_len
                .checked_mul(4)
                .ok_or_else(|| self.invalid("compiled CSR data byte count overflow"))?,
        )?;
        Ok(start)
    }

    pub(crate) fn read_sections_section(&mut self) -> Result<SectionsData> {
        let node_from_seq_count = self.read_u32()? as usize;
        let mut node_from_seq = BTreeMap::new();
        for _ in 0..node_from_seq_count {
            let seq = self.read_u32_vec()?;
            let node = self.read_u32()?;
            node_from_seq.insert(seq, node);
        }
        let seq_from_node_count = self.read_u32()? as usize;
        let mut seq_from_node = BTreeMap::new();
        for _ in 0..seq_from_node_count {
            let node = self.read_u32()?;
            let seq = self.read_u32_vec()?;
            seq_from_node.insert(node, seq);
        }
        let sec1_count = self.read_u32()? as usize;
        let mut sec1 = BTreeMap::new();
        for _ in 0..sec1_count {
            let node = self.read_u32()?;
            let heading_count = self.read_u32()? as usize;
            let mut headings = BTreeMap::new();
            for _ in 0..heading_count {
                let heading = self.read_string()?;
                let target = self.read_u32()?;
                headings.insert(heading, target);
            }
            sec1.insert(node, headings);
        }
        let sec2_count = self.read_u32()? as usize;
        let mut sec2 = BTreeMap::new();
        for _ in 0..sec2_count {
            let node = self.read_u32()?;
            let level1_count = self.read_u32()? as usize;
            let mut level1 = BTreeMap::new();
            for _ in 0..level1_count {
                let heading1 = self.read_string()?;
                let level2_count = self.read_u32()? as usize;
                let mut level2 = BTreeMap::new();
                for _ in 0..level2_count {
                    let heading2 = self.read_string()?;
                    let target = self.read_u32()?;
                    level2.insert(heading2, target);
                }
                level1.insert(heading1, level2);
            }
            sec2.insert(node, level1);
        }
        Ok(SectionsData {
            sec1,
            sec2,
            seq_from_node,
            node_from_seq,
        })
    }

    pub(crate) fn read_structure_section(&mut self) -> Result<StructureData> {
        self.expect_section_magic(STRUCTURE_MAGIC)?;
        let top = self.read_u32_vec()?;

        let heading_count = self.read_u32()? as usize;
        let mut heading_from_node = BTreeMap::new();
        for _ in 0..heading_count {
            let node = self.read_u32()?;
            heading_from_node.insert(node, self.read_heading()?);
        }

        let node_heading_count = self.read_u32()? as usize;
        let mut node_from_heading = BTreeMap::new();
        for _ in 0..node_heading_count {
            let heading = self.read_heading()?;
            let node = self.read_u32()?;
            node_from_heading.insert(heading, node);
        }

        let multiple_count = self.read_u32()? as usize;
        let mut multiple = BTreeMap::new();
        for _ in 0..multiple_count {
            let heading = self.read_heading()?;
            multiple.insert(heading, self.read_u32_vec()?);
        }

        let up_count = self.read_u32()? as usize;
        let mut up = BTreeMap::new();
        for _ in 0..up_count {
            let node = self.read_u32()?;
            let parent = self.read_u32()?;
            up.insert(node, parent);
        }

        let down_count = self.read_u32()? as usize;
        let mut down = BTreeMap::new();
        for _ in 0..down_count {
            let parent = self.read_u32()?;
            down.insert(parent, self.read_u32_vec()?);
        }

        Ok(StructureData {
            heading_from_node,
            node_from_heading,
            multiple,
            top,
            up,
            down,
        })
    }

    fn read_metadata_map(&mut self) -> Result<BTreeMap<String, Option<String>>> {
        let metadata_count = self.read_u32()? as usize;
        let mut metadata = BTreeMap::new();
        for _ in 0..metadata_count {
            let key = self.read_string()?;
            let value = match self.read_u8()? {
                0 => None,
                1 => Some(self.read_string()?),
                _ => return Err(self.invalid("invalid feature metadata value tag")),
            };
            metadata.insert(key, value);
        }
        Ok(metadata)
    }

    fn inspect_edge_feature(&mut self) -> Result<EdgeFeatureEntry> {
        let name = self.read_string()?;
        let row_count = self.read_u32()? as usize;
        let payload_len = self.read_u32()? as usize;
        let payload_start = self.offset;
        self.skip_bytes(payload_len)?;
        Ok(EdgeFeatureEntry {
            name,
            row_count,
            payload_start,
            payload_end: self.offset,
        })
    }

    /// Reads a [`write_optional_feature_value`] encoding.
    fn read_optional_feature_value(&mut self) -> Result<Option<FeatureValue>> {
        match self.read_u8()? {
            0 => Ok(Some(FeatureValue::string(self.read_string()?))),
            1 => {
                let mut bytes = [0_u8; 8];
                bytes.copy_from_slice(self.take(8)?);
                Ok(Some(FeatureValue::Int(i64::from_le_bytes(bytes))))
            }
            2 => Ok(None),
            _ => Err(self.invalid("invalid optional feature value tag")),
        }
    }

    fn skip_feature_value(&mut self) -> Result<()> {
        match self.read_u8()? {
            0 => self.skip_string(),
            1 => self.skip_bytes(8),
            _ => Err(self.invalid("invalid feature value tag")),
        }
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_string)
            .map_err(|_| self.invalid("invalid utf-8 string"))
    }

    fn read_u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u32_vec(&mut self) -> Result<Vec<u32>> {
        let len = self.read_u32()? as usize;
        let mut values = Vec::with_capacity(self.capacity_for(len, 4));
        for _ in 0..len {
            values.push(self.read_u32()?);
        }
        Ok(values)
    }

    fn read_heading(&mut self) -> Result<Vec<StructureHeading>> {
        let len = self.read_u32()? as usize;
        let mut heading = Vec::with_capacity(self.capacity_for(len, 8));
        for _ in 0..len {
            let node_type = self.read_string()?;
            heading.push(StructureHeading {
                node_type,
                heading: self.read_optional_feature_value()?,
            });
        }
        Ok(heading)
    }

    fn inspect_u32_vec(&mut self) -> Result<(usize, usize)> {
        let len = self.read_u32()? as usize;
        let start = self.offset;
        self.skip_bytes(
            len.checked_mul(4)
                .ok_or_else(|| self.invalid("compiled u32 vector byte count overflow"))?,
        )?;
        Ok((len, start))
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| self.invalid("compiled corpus offset overflow"))?;
        if end > self.bytes.len() {
            return Err(self.invalid("unexpected end of compiled corpus"));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    /// Capacity hint for a collection whose length comes from an untrusted
    /// count field at the current offset; see [`bounded_capacity`].
    fn capacity_for(&self, declared: usize, min_item_bytes: usize) -> usize {
        bounded_capacity(
            declared,
            self.bytes.len().saturating_sub(self.offset),
            min_item_bytes,
        )
    }

    fn skip_bytes(&mut self, len: usize) -> Result<()> {
        self.take(len).map(|_| ())
    }

    fn skip_string(&mut self) -> Result<()> {
        let len = self.read_u32()? as usize;
        self.skip_bytes(len)
    }

    fn is_done(&self) -> bool {
        self.offset >= self.bytes.len()
    }

    fn invalid(&self, message: impl Into<String>) -> CfError {
        CfError::Parse {
            path: self.path.to_path_buf(),
            line: 0,
            message: message.into(),
        }
    }
}
