//! Writes a compiled corpus to a `.cfr` file.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use crate::atomic_file::write_atomically;
use crate::compile::{Corpus, precompute};
use crate::error::{CfError, Result};
use crate::format::{
    BOUNDARY_MAGIC, EDGE_VALUES_MAGIC, FEATURE_METADATA_MAGIC, LEV_DOWN_MAGIC, LEV_UP_MAGIC, MAGIC,
    REVERSE_EDGES_MAGIC, ROW_INDEX_MAGIC, SECTIONS_MAGIC, STRUCTURE_MAGIC, invalid_compiled,
};
use crate::index::edge_index::ReverseEdgeCsr;
use crate::index::row_index::RowIndexKind;
use crate::index::{SectionsData, StructureHeading};
use crate::tf::{EdgeFeature, NodeFeature};
use crate::value::FeatureValue;

/// Writes the compiled cache atomically (see [`write_atomically`]).
///
/// Never rewriting the destination in place matters because readers memory-map
/// it: truncating a mapped file makes those readers see foreign bytes (or fault
/// with SIGBUS past the new end). With a rename, open mappings keep the old
/// inode, concurrent compilers each publish a complete file (last one wins),
/// and a failed or interrupted compile leaves the previous cache untouched.
pub(crate) fn write_compiled(corpus: &Corpus, output_path: &Path) -> Result<()> {
    write_atomically(output_path, |file| {
        write_compiled_contents(corpus, file, output_path)
    })
}

fn write_compiled_contents(corpus: &Corpus, file: &File, output_path: &Path) -> Result<()> {
    let mut writer = BufWriter::new(file);
    writer.write_all(MAGIC).map_err(|source| CfError::Io {
        path: output_path.to_path_buf(),
        source,
    })?;

    let mut row_indexes = Vec::new();
    write_u32(&mut writer, corpus.node_features.len() as u32, output_path)?;
    for feature in corpus.node_features.values() {
        if let Some(offsets) = write_node_feature(&mut writer, feature, output_path)? {
            row_indexes.push((RowIndexKind::Node, feature.name.as_str(), offsets));
        }
    }

    write_u32(&mut writer, corpus.edge_features.len() as u32, output_path)?;
    for feature in corpus.edge_features.values() {
        let offsets = write_edge_feature(&mut writer, feature, output_path)?;
        row_indexes.push((RowIndexKind::Edge, feature.name.as_str(), offsets));
    }
    write_u32_vec(&mut writer, &corpus.order(), output_path)?;
    write_u32_vec(&mut writer, &corpus.rank(), output_path)?;
    write_config_features(&mut writer, &corpus.config_features, output_path)?;
    write_feature_metadata_section(
        &mut writer,
        &corpus.node_features,
        &corpus.edge_features,
        output_path,
    )?;
    write_edge_values_section(&mut writer, &corpus.edge_features, output_path)?;
    write_structure_section(&mut writer, corpus, output_path)?;
    write_index_sections(&mut writer, corpus, &row_indexes, output_path)?;
    writer.flush().map_err(|source| CfError::Io {
        path: output_path.to_path_buf(),
        source,
    })
}

/// Builds a CSR `(offsets, data)` pair from per-row slices. `offsets` has
/// `rows.len() + 1` entries; row `i` occupies `data[offsets[i]..offsets[i+1]]`.
fn build_csr(rows: &[Vec<u32>]) -> (Vec<u32>, Vec<u32>) {
    let mut offsets = Vec::with_capacity(rows.len() + 1);
    let mut data = Vec::new();
    offsets.push(0);
    for row in rows {
        data.extend_from_slice(row);
        offsets.push(data.len() as u32);
    }
    (offsets, data)
}

fn write_csr<W: Write>(writer: &mut W, rows: &[Vec<u32>], path: &Path) -> Result<()> {
    let (offsets, data) = build_csr(rows);
    write_u32(writer, rows.len() as u32, path)?;
    write_u32_vec(writer, &offsets, path)?;
    write_u32_vec(writer, &data, path)?;
    Ok(())
}

/// One `CFRROWS1` entry: feature table, feature name, row offsets.
type RowIndexEntry<'a> = (RowIndexKind, &'a str, Vec<u32>);

/// Appends the precomputed index sections: `CFRLEVU1`, `CFRLEVD1`, `CFRBND1`,
/// `CFRREVE1` and `CFRROWS1`, then `CFRSECT1` when sections are configured.
/// Plain u32 LE, no varint, so accessors random-access individual rows out of
/// the mmap with no decode pass. `CFRSECT1` stays last: opening a cache records
/// its offset without parsing it.
fn write_index_sections<W: Write>(
    writer: &mut W,
    corpus: &Corpus,
    row_indexes: &[RowIndexEntry<'_>],
    path: &Path,
) -> Result<()> {
    let max_slot = corpus.max_slot();
    let max_node = corpus.max_node();
    let slot_sets: BTreeMap<u32, Vec<u32>> = corpus.oslots_items().into_iter().collect();
    let rank = corpus.rank();

    let lev_up = precompute::lev_up(&slot_sets, &rank, max_slot, max_node);
    write_magic(writer, LEV_UP_MAGIC, path)?;
    write_csr(writer, &lev_up, path)?;

    // levDown is indexed by non-slot nodes (`max_slot+1..=max_node`); store it
    // as a full `1..=max_node` CSR with empty rows for slots so the accessor is
    // uniform with levUp. The offsets overhead is ~max_slot u32 (negligible).
    let lev_down = precompute::lev_down(&lev_up, &rank, max_slot, max_node);
    let mut lev_down_full = vec![Vec::new(); max_node as usize];
    for (index, row) in lev_down.iter().enumerate() {
        let node = max_slot as usize + 1 + index;
        if node >= 1 && node <= max_node as usize {
            lev_down_full[node - 1] = row.clone();
        }
    }
    write_magic(writer, LEV_DOWN_MAGIC, path)?;
    write_csr(writer, &lev_down_full, path)?;

    let boundary = precompute::boundary(&slot_sets, &rank, max_slot);
    write_magic(writer, BOUNDARY_MAGIC, path)?;
    write_csr(writer, &boundary.first_slots, path)?;
    write_csr(writer, &boundary.last_slots, path)?;

    write_reverse_edges_section(writer, corpus, path)?;
    write_row_index_section(writer, row_indexes, path)?;

    if max_node > 0
        && let Some(sections) = precompute::sections_with(corpus, &lev_up, &lev_down)
    {
        write_magic(writer, SECTIONS_MAGIC, path)?;
        write_sections_section(writer, &sections, path)?;
    }
    Ok(())
}

fn write_magic<W: Write>(writer: &mut W, magic: &[u8; 8], path: &Path) -> Result<()> {
    writer.write_all(magic).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Appends `CFRROWS1` (layout in [`crate::row_index`]).
fn write_row_index_section<W: Write>(
    writer: &mut W,
    row_indexes: &[RowIndexEntry<'_>],
    path: &Path,
) -> Result<()> {
    writer
        .write_all(ROW_INDEX_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32(writer, row_indexes.len() as u32, path)?;
    for (kind, name, offsets) in row_indexes {
        writer
            .write_all(&[*kind as u8])
            .map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        write_string(writer, name, path)?;
        write_u32_vec(writer, offsets, path)?;
    }
    Ok(())
}

/// Appends `CFRREVE1`: the feature count, then per edge feature its name and
/// its [`ReverseEdgeCsr`].
fn write_reverse_edges_section<W: Write>(
    writer: &mut W,
    corpus: &Corpus,
    path: &Path,
) -> Result<()> {
    let rank = corpus.rank();
    writer
        .write_all(REVERSE_EDGES_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32(writer, corpus.edge_features.len() as u32, path)?;
    for feature in corpus.edge_features.values() {
        write_string(writer, &feature.name, path)?;
        ReverseEdgeCsr::build(&feature.values, &rank).write(writer, path)?;
    }
    Ok(())
}

fn write_sections_section<W: Write>(
    writer: &mut W,
    sections: &SectionsData,
    path: &Path,
) -> Result<()> {
    // node_from_seq: BTreeMap<Vec<u32>, u32>
    write_u32(writer, sections.node_from_seq.len() as u32, path)?;
    for (seq, node) in &sections.node_from_seq {
        write_u32_vec(writer, seq, path)?;
        write_u32(writer, *node, path)?;
    }
    // seq_from_node: BTreeMap<u32, Vec<u32>>
    write_u32(writer, sections.seq_from_node.len() as u32, path)?;
    for (node, seq) in &sections.seq_from_node {
        write_u32(writer, *node, path)?;
        write_u32_vec(writer, seq, path)?;
    }
    // sec1: BTreeMap<u32, BTreeMap<String, u32>>
    write_u32(writer, sections.sec1.len() as u32, path)?;
    for (node, headings) in &sections.sec1 {
        write_u32(writer, *node, path)?;
        write_u32(writer, headings.len() as u32, path)?;
        for (heading, target) in headings {
            write_string(writer, heading, path)?;
            write_u32(writer, *target, path)?;
        }
    }
    // sec2: BTreeMap<u32, BTreeMap<String, BTreeMap<String, u32>>>
    write_u32(writer, sections.sec2.len() as u32, path)?;
    for (node, level1) in &sections.sec2 {
        write_u32(writer, *node, path)?;
        write_u32(writer, level1.len() as u32, path)?;
        for (heading1, level2) in level1 {
            write_string(writer, heading1, path)?;
            write_u32(writer, level2.len() as u32, path)?;
            for (heading2, target) in level2 {
                write_string(writer, heading2, path)?;
                write_u32(writer, *target, path)?;
            }
        }
    }
    Ok(())
}

fn write_config_features<W: Write>(
    writer: &mut W,
    features: &BTreeMap<String, BTreeMap<String, Option<String>>>,
    path: &Path,
) -> Result<()> {
    write_u32(writer, features.len() as u32, path)?;
    for (name, metadata) in features {
        write_string(writer, name, path)?;
        write_u32(writer, metadata.len() as u32, path)?;
        for (key, value) in metadata {
            write_string(writer, key, path)?;
            match value {
                Some(value) => {
                    writer.write_all(&[1]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    write_string(writer, value, path)?;
                }
                None => {
                    writer.write_all(&[0]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn write_feature_metadata_section<W: Write>(
    writer: &mut W,
    node_features: &BTreeMap<String, NodeFeature>,
    edge_features: &BTreeMap<String, EdgeFeature>,
    path: &Path,
) -> Result<()> {
    writer
        .write_all(FEATURE_METADATA_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32(writer, node_features.len() as u32, path)?;
    for feature in node_features.values() {
        write_string(writer, &feature.name, path)?;
        write_metadata_map(writer, &feature.metadata, path)?;
    }
    write_u32(writer, edge_features.len() as u32, path)?;
    for feature in edge_features.values() {
        write_string(writer, &feature.name, path)?;
        write_metadata_map(writer, &feature.metadata, path)?;
    }
    Ok(())
}

fn write_edge_values_section<W: Write>(
    writer: &mut W,
    edge_features: &BTreeMap<String, EdgeFeature>,
    path: &Path,
) -> Result<()> {
    writer
        .write_all(EDGE_VALUES_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    let valued_features = edge_features
        .values()
        .filter(|feature| !feature.edge_values.is_empty())
        .collect::<Vec<_>>();
    write_u32(writer, valued_features.len() as u32, path)?;
    for feature in valued_features {
        write_string(writer, &feature.name, path)?;
        write_u32(writer, feature.edge_values.len() as u32, path)?;
        let mut rows = feature.edge_values.iter().collect::<Vec<_>>();
        rows.sort_unstable_by_key(|((source, target), _)| (*source, *target));
        for ((source, target), value) in rows {
            write_u32(writer, *source, path)?;
            write_u32(writer, *target, path)?;
            write_feature_value(writer, value, path)?;
        }
    }
    Ok(())
}

fn write_structure_section<W: Write>(writer: &mut W, corpus: &Corpus, path: &Path) -> Result<()> {
    let Some(data) = precompute::structure(corpus) else {
        return Ok(());
    };
    writer
        .write_all(STRUCTURE_MAGIC)
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32_vec(writer, &data.top, path)?;
    write_u32(writer, data.heading_from_node.len() as u32, path)?;
    for (node, heading) in &data.heading_from_node {
        write_u32(writer, *node, path)?;
        write_heading(writer, heading, path)?;
    }
    write_u32(writer, data.node_from_heading.len() as u32, path)?;
    for (heading, node) in &data.node_from_heading {
        write_heading(writer, heading, path)?;
        write_u32(writer, *node, path)?;
    }
    write_u32(writer, data.multiple.len() as u32, path)?;
    for (heading, nodes) in &data.multiple {
        write_heading(writer, heading, path)?;
        write_u32_vec(writer, nodes, path)?;
    }
    write_u32(writer, data.up.len() as u32, path)?;
    for (node, parent) in &data.up {
        write_u32(writer, *node, path)?;
        write_u32(writer, *parent, path)?;
    }
    write_u32(writer, data.down.len() as u32, path)?;
    for (parent, children) in &data.down {
        write_u32(writer, *parent, path)?;
        write_u32_vec(writer, children, path)?;
    }
    Ok(())
}

fn write_heading<W: Write>(
    writer: &mut W,
    heading: &[StructureHeading],
    path: &Path,
) -> Result<()> {
    write_u32(writer, heading.len() as u32, path)?;
    for item in heading {
        write_string(writer, &item.node_type, path)?;
        write_optional_feature_value(writer, item.heading.as_ref(), path)?;
    }
    Ok(())
}

fn write_metadata_map<W: Write>(
    writer: &mut W,
    metadata: &BTreeMap<String, Option<String>>,
    path: &Path,
) -> Result<()> {
    write_u32(writer, metadata.len() as u32, path)?;
    for (key, value) in metadata {
        write_string(writer, key, path)?;
        match value {
            Some(value) => {
                writer.write_all(&[1]).map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
                write_string(writer, value, path)?;
            }
            None => {
                writer.write_all(&[0]).map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
            }
        }
    }
    Ok(())
}

/// Writes one node feature; returns the row offsets (relative to the payload
/// start) of a mixed feature, whose rows vary in length, for `CFRROWS1`.
fn write_node_feature<W: Write>(
    writer: &mut W,
    feature: &NodeFeature,
    path: &Path,
) -> Result<Option<Vec<u32>>> {
    write_string(writer, &feature.name, path)?;
    let all_strings = feature
        .values
        .values()
        .all(|value| matches!(value, FeatureValue::Str(_)));
    writer
        .write_all(&[if all_strings { 0 } else { 1 }])
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    write_u32(writer, feature.values.len() as u32, path)?;

    let mut payload = Vec::new();
    let mut row_offsets = None;
    if all_strings {
        let mut pool: Vec<Arc<str>> = Vec::new();
        let mut pool_ids: HashMap<Arc<str>, u32> = HashMap::new();
        for value in feature.values.values() {
            let FeatureValue::Str(value) = value else {
                unreachable!();
            };
            if !pool_ids.contains_key(value) {
                let id = pool.len() as u32;
                pool.push(value.clone());
                pool_ids.insert(value.clone(), id);
            }
        }
        write_u32(writer, pool.len() as u32, path)?;
        write_u32(&mut payload, pool.len() as u32, path)?;
        for value in &pool {
            write_string(&mut payload, value, path)?;
        }

        write_u32(&mut payload, feature.values.len() as u32, path)?;
        let mut rows: Vec<_> = feature.values.iter().collect();
        rows.sort_unstable_by_key(|(node, _)| **node);
        for (node, value) in rows {
            let FeatureValue::Str(value) = value else {
                unreachable!();
            };
            write_u32(&mut payload, *node, path)?;
            write_u32(&mut payload, pool_ids[value], path)?;
        }
    } else {
        write_u32(writer, 0, path)?;
        write_u32(&mut payload, feature.values.len() as u32, path)?;
        let mut rows: Vec<_> = feature.values.iter().collect();
        rows.sort_unstable_by_key(|(node, _)| **node);
        let mut offsets = Vec::with_capacity(rows.len());
        for (node, value) in rows {
            offsets.push(payload_offset(&payload, path)?);
            write_u32(&mut payload, *node, path)?;
            match value {
                FeatureValue::Str(value) => {
                    payload.write_all(&[0]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    write_string(&mut payload, value, path)?;
                }
                FeatureValue::Int(value) => {
                    payload.write_all(&[1]).map_err(|source| CfError::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
                    payload
                        .write_all(&value.to_le_bytes())
                        .map_err(|source| CfError::Io {
                            path: path.to_path_buf(),
                            source,
                        })?;
                }
            }
        }
        row_offsets = Some(offsets);
    }
    write_u32(writer, payload.len() as u32, path)?;
    writer.write_all(&payload).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(row_offsets)
}

/// Writes one edge feature; returns its row offsets (relative to the payload
/// start) for `CFRROWS1`.
fn write_edge_feature<W: Write>(
    writer: &mut W,
    feature: &EdgeFeature,
    path: &Path,
) -> Result<Vec<u32>> {
    write_string(writer, &feature.name, path)?;
    write_u32(writer, feature.values.len() as u32, path)?;
    let mut payload = Vec::new();
    write_u32(&mut payload, feature.values.len() as u32, path)?;
    let mut rows: Vec<_> = feature.values.iter().collect();
    rows.sort_unstable_by_key(|(node, _)| **node);
    let mut row_offsets = Vec::with_capacity(rows.len());
    for (node, targets) in rows {
        row_offsets.push(payload_offset(&payload, path)?);
        write_u32(&mut payload, *node, path)?;
        write_u32(&mut payload, targets.len() as u32, path)?;
        for target in targets {
            write_u32(&mut payload, *target, path)?;
        }
    }
    write_u32(writer, payload.len() as u32, path)?;
    writer.write_all(&payload).map_err(|source| CfError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(row_offsets)
}

/// The current payload length as a u32 row offset.
fn payload_offset(payload: &[u8], path: &Path) -> Result<u32> {
    u32::try_from(payload.len())
        .map_err(|_| invalid_compiled(path, "feature payload exceeds the 4 GiB row-offset range"))
}

/// Writes `None` as tag `2`, otherwise the [`write_feature_value`] encoding.
fn write_optional_feature_value<W: Write>(
    writer: &mut W,
    value: Option<&FeatureValue>,
    path: &Path,
) -> Result<()> {
    match value {
        Some(value) => write_feature_value(writer, value, path),
        None => writer.write_all(&[2]).map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn write_feature_value<W: Write>(writer: &mut W, value: &FeatureValue, path: &Path) -> Result<()> {
    match value {
        FeatureValue::Str(value) => {
            writer.write_all(&[0]).map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            write_string(writer, value, path)?;
        }
        FeatureValue::Int(value) => {
            writer.write_all(&[1]).map_err(|source| CfError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            writer
                .write_all(&value.to_le_bytes())
                .map_err(|source| CfError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
    }
    Ok(())
}

fn write_string<W: Write>(writer: &mut W, value: &str, path: &Path) -> Result<()> {
    write_u32(writer, value.len() as u32, path)?;
    writer
        .write_all(value.as_bytes())
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })
}

fn write_u32<W: Write>(writer: &mut W, value: u32, path: &Path) -> Result<()> {
    writer
        .write_all(&value.to_le_bytes())
        .map_err(|source| CfError::Io {
            path: path.to_path_buf(),
            source,
        })
}

pub(crate) fn write_u32_vec<W: Write>(writer: &mut W, values: &[u32], path: &Path) -> Result<()> {
    write_u32(writer, values.len() as u32, path)?;
    for value in values {
        write_u32(writer, *value, path)?;
    }
    Ok(())
}
