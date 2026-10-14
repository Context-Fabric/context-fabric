//! Canonical node order: ranks, sorting, and chunk sort keys.

use std::path::Path;

use crate::error::Result;
use crate::format::{MappedU32s, invalid_compiled, read_u32_from};
use crate::mapped::MappedCompiledCorpus;

impl MappedCompiledCorpus {
    /// Every node in canonical order, borrowed from the compiled `order`
    /// array. `None` when the cache has no order array covering every node.
    pub fn canonical_order(&self) -> Result<Option<MappedU32s<'_>>> {
        let order_start = self.metadata.order_start;
        if self.metadata.order_len != self.metadata.rank_len {
            return Ok(None);
        }
        let bytes = self
            .mmap
            .get(order_start..order_start + self.metadata.order_len * 4)
            .ok_or_else(|| invalid_compiled(&self.path, "compiled order array out of bounds"))?;
        Ok(Some(MappedU32s::new(bytes)))
    }

    pub fn order(&self) -> Result<Vec<u32>> {
        let order_start = self.metadata.order_start;
        (0..self.metadata.order_len)
            .map(|index| read_u32_from(&self.path, &self.mmap, order_start + (index * 4)))
            .collect()
    }

    pub fn rank(&self) -> Result<Vec<u32>> {
        let rank_start = self.metadata.rank_start;
        (0..self.metadata.rank_len)
            .map(|index| read_u32_from(&self.path, &self.mmap, rank_start + (index * 4)))
            .collect()
    }

    pub fn sort_key(&self, node: u32) -> Result<Option<u32>> {
        let rank_start = self.metadata.rank_start;
        if node == 0 || node as usize > self.metadata.rank_len {
            return Ok(None);
        }
        read_u32_from(
            &self.path,
            &self.mmap,
            rank_start + ((node as usize - 1) * 4),
        )
        .map(Some)
    }

    pub fn sort_key_tuple(&self, nodes: &[u32]) -> Result<Vec<Option<u32>>> {
        nodes.iter().map(|node| self.sort_key(*node)).collect()
    }

    pub fn sort_nodes(&self, nodes: &mut [u32]) -> Result<()> {
        let rank_start = self.metadata.rank_start;
        nodes.sort_unstable_by_key(|node| {
            if *node == 0 || *node as usize > self.metadata.rank_len {
                return u32::MAX;
            }
            read_u32_from(
                &self.path,
                &self.mmap,
                rank_start + ((*node as usize - 1) * 4),
            )
            .unwrap_or(u32::MAX)
        });
        Ok(())
    }

    pub fn sort_key_chunk(&self, chunk: Chunk) -> Result<ChunkPositionKey> {
        Ok(ChunkPositionKey::new(
            chunk.start,
            self.type_rank(chunk.node)?,
            chunk.end,
            chunk.node,
        ))
    }

    pub fn sort_key_chunk_length(&self, chunk: Chunk) -> Result<ChunkLengthKey> {
        Ok(ChunkLengthKey::new(
            chunk.end.saturating_sub(chunk.start),
            chunk.start,
            self.type_rank(chunk.node)?,
            chunk.node,
        ))
    }

    fn type_rank(&self, node: u32) -> Result<u32> {
        let node_type = self
            .string_pool_node_feature("otype")?
            .and_then(|otype| otype.str_value(node).transpose())
            .transpose()?
            .map(str::to_string);
        let levels = self.mapped_levels()?;
        let fallback = levels
            .last()
            .map(|level| level.node_type.as_str())
            .unwrap_or_default();
        let node_type = node_type.as_deref().unwrap_or(fallback);
        Ok(levels
            .iter()
            .rev()
            .position(|level| level.node_type == node_type)
            .map(|rank| rank as u32)
            .unwrap_or(u32::MAX))
    }
}

/// Reads a single node's rank directly from the mmap'd rank array, avoiding
/// materialization of the whole rank Vec.
pub(crate) fn mmap_rank(
    path: &Path,
    bytes: &[u8],
    rank_start: usize,
    rank_len: usize,
    node: u32,
) -> Option<u32> {
    if node == 0 || node as usize > rank_len {
        return None;
    }
    read_u32_from(path, bytes, rank_start + ((node as usize - 1) * 4)).ok()
}

/// Sorts nodes into canonical (rank) order using ranks read from the mmap.
pub(crate) fn sort_nodes_by_mmap_rank(
    path: &Path,
    bytes: &[u8],
    rank_start: usize,
    rank_len: usize,
    nodes: &mut [u32],
) {
    let key = |node: &u32| {
        (
            mmap_rank(path, bytes, rank_start, rank_len, *node).unwrap_or(*node),
            *node,
        )
    };
    // Nodes of one type are usually numbered in canonical order already; one
    // sequential pass is far cheaper than sorting by random rank reads.
    if !nodes.is_sorted_by_key(key) {
        nodes.sort_unstable_by_key(key);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub node: u32,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkPositionKey {
    start: u32,
    reverse_type_rank: std::cmp::Reverse<u32>,
    reverse_end: std::cmp::Reverse<u32>,
    node: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkLengthKey {
    reverse_size: std::cmp::Reverse<u32>,
    start: u32,
    type_rank: u32,
    node: u32,
}

impl ChunkPositionKey {
    pub(crate) fn new(start: u32, type_rank: u32, end: u32, node: u32) -> Self {
        Self {
            start,
            reverse_type_rank: std::cmp::Reverse(type_rank),
            reverse_end: std::cmp::Reverse(end),
            node,
        }
    }
}

impl ChunkLengthKey {
    pub(crate) fn new(size: u32, start: u32, type_rank: u32, node: u32) -> Self {
        Self {
            reverse_size: std::cmp::Reverse(size),
            start,
            type_rank,
            node,
        }
    }
}
