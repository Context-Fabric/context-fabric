//! [`MappedCompiledCorpus`]: a memory-mapped `.cfr` file and its feature views.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

use memmap2::Mmap;
use serde::Serialize;

use crate::config::OTEXT;
use crate::error::{CfError, Result};
use crate::format::{
    CompiledMetadata, NodeFeatureEncoding, bounded_capacity, invalid_compiled, map_file,
    read_csr_row, read_u32_from,
};
use crate::index::{Boundary, SectionsData, StructureData};
use crate::mapped::edge_features::{EdgeParts, build_edge_parts};
use crate::mapped::node_features::{
    MixedParts, StringPoolParts, build_mixed_parts, build_string_pool_parts,
};
use crate::mapped::{
    ConfigFeatureView, EdgeFeatureView, MappedNodeFeatureView, MixedNodeFeatureView,
    OwnedEdgeFeature, OwnedMixedFeature, OwnedNodeFeature, OwnedStringPoolFeature,
    StringPoolNodeFeatureView,
};
use crate::tf::Otext;

/// Per-corpus cache of parsed feature parts, keyed by feature name. Holds
/// `Arc`s so callers clone a pointer (cheap) under a short-lived lock and then
/// build a thin view borrowing the mmap bytes.
#[derive(Default)]
struct ViewCaches {
    string_pool: HashMap<String, Arc<StringPoolParts>>,
    mixed: HashMap<String, Arc<MixedParts>>,
    edge: HashMap<String, Arc<EdgeParts>>,
}

pub struct MappedCompiledCorpus {
    pub(crate) path: PathBuf,
    pub(crate) mmap: Mmap,
    #[cfg(target_os = "macos")]
    _shared_mmap_hint: Mmap,
    pub(crate) metadata: CompiledMetadata,
    caches: RwLock<ViewCaches>,
    pub(crate) sections_cache: OnceLock<Option<SectionsData>>,
    pub(crate) structure_cache: OnceLock<Option<StructureData>>,
    /// The `otext` config feature, parsed on first use.
    otext_cache: OnceLock<Otext>,
    /// Computed once: `compute_mapped_levels` scans every `otype` row and reads
    /// each non-slot node's `oslots` row (~1M edge reads on BHSA), far too slow
    /// to repeat per `freqList`/`otypeRank` call.
    levels_cache: OnceLock<Vec<MappedLevel>>,
    /// Node ids per node type in ascending node order, built by one `otype` pass
    /// on first use. Search atoms seed from these instead of re-scanning all
    /// `otype` rows (with a string compare each) per query.
    type_nodes_cache: OnceLock<HashMap<String, Arc<Vec<u32>>>>,
    /// The last slot node, computed once by one `otype` pool-id pass. Locality,
    /// search and text all branch on it per call, so it must never be rescanned.
    max_slot_cache: OnceLock<u32>,
}

impl std::fmt::Debug for MappedCompiledCorpus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MappedCompiledCorpus")
            .field("path", &self.path)
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub(crate) struct MappedLevel {
    pub(crate) node_type: String,
    average_slots: f64,
    pub(crate) min_node: u32,
    pub(crate) max_node: u32,
}

impl MappedCompiledCorpus {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mmap = map_file(path)?;
        let metadata = crate::format::inspect_compiled_bytes(path, &mmap)?;
        #[cfg(target_os = "macos")]
        let shared_mmap_hint = map_file(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            mmap,
            #[cfg(target_os = "macos")]
            _shared_mmap_hint: shared_mmap_hint,
            metadata,
            caches: RwLock::new(ViewCaches::default()),
            sections_cache: OnceLock::new(),
            structure_cache: OnceLock::new(),
            otext_cache: OnceLock::new(),
            levels_cache: OnceLock::new(),
            type_nodes_cache: OnceLock::new(),
            max_slot_cache: OnceLock::new(),
        })
    }

    pub fn metadata(&self) -> &CompiledMetadata {
        &self.metadata
    }

    /// The `otext` config feature: sections, structure and text formats (empty
    /// when the corpus has none).
    pub fn otext(&self) -> Result<&Otext> {
        if let Some(otext) = self.otext_cache.get() {
            return Ok(otext);
        }
        let otext = match self.config_feature(OTEXT)? {
            Some(view) => Otext::from_metadata(&view.metadata()?),
            None => Otext::default(),
        };
        Ok(self.otext_cache.get_or_init(|| otext))
    }

    pub fn all_node_features(&self, warp: bool) -> Vec<String> {
        self.metadata
            .node_features
            .iter()
            .filter(|feature| warp || feature.name != "otype")
            .map(|feature| feature.name.clone())
            .collect()
    }

    pub fn all_edge_features(&self, warp: bool) -> Vec<String> {
        self.metadata
            .edge_features
            .iter()
            .filter(|feature| warp || feature.name != "oslots")
            .map(|feature| feature.name.clone())
            .collect()
    }

    pub fn computed_feature(&self, name: &str) -> Result<Option<ComputedFeatureData>> {
        match name {
            "levels" => Ok(Some(ComputedFeatureData::Levels(
                self.mapped_levels()?
                    .into_iter()
                    .map(|level| {
                        (
                            level.node_type,
                            level.average_slots,
                            level.min_node,
                            level.max_node,
                        )
                    })
                    .collect(),
            ))),
            "order" => self
                .order()
                .map(|order| Some(ComputedFeatureData::Order(order))),
            "rank" => self
                .rank()
                .map(|rank| Some(ComputedFeatureData::Rank(rank))),
            "boundary" => Ok(Some(ComputedFeatureData::Boundary(
                crate::index::Boundary {
                    first_slots: self.csr_all_rows(self.metadata.boundary_first_start)?,
                    last_slots: self.csr_all_rows(self.metadata.boundary_last_start)?,
                },
            ))),
            "levUp" => Ok(Some(ComputedFeatureData::LevUp(
                self.csr_all_rows(self.metadata.lev_up_start)?,
            ))),
            "levDown" => {
                // levDown is materialized for non-slot nodes only, matching
                // `precompute::lev_down` (rows for `max_slot+1..=max_node`).
                let full = self.csr_all_rows(self.metadata.lev_down_start)?;
                let max_slot = self.max_slot()? as usize;
                let rows = full.into_iter().skip(max_slot).collect();
                Ok(Some(ComputedFeatureData::LevDown(rows)))
            }
            _ => Ok(None),
        }
    }

    /// Reads every row of a CSR section (starting at its `row_count` field) into
    /// a `Vec<Vec<u32>>`.
    fn csr_all_rows(&self, start: usize) -> Result<Vec<Vec<u32>>> {
        let row_count = read_u32_from(&self.path, &self.mmap, start)? as usize;
        let mut rows = Vec::with_capacity(bounded_capacity(
            row_count,
            self.mmap.len().saturating_sub(start),
            4,
        ));
        for index in 1..=row_count as u32 {
            rows.push(read_csr_row(&self.path, &self.mmap, start, index)?.unwrap_or_default());
        }
        Ok(rows)
    }

    pub fn otype_rank(&self) -> Result<BTreeMap<String, u32>> {
        Ok(self
            .mapped_levels()?
            .into_iter()
            .rev()
            .enumerate()
            .map(|(rank, level)| (level.node_type, rank as u32))
            .collect())
    }

    pub fn max_node(&self) -> u32 {
        self.metadata.rank_len as u32
    }

    pub fn slot_type(&self) -> Result<String> {
        let otype = self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        Ok(otype.str_value(1)?.unwrap_or_default().to_string())
    }

    /// Node ids of one type in ascending node order, served from a cache built
    /// by a single `otype` pass (u32 reads only) on first use. Unlike
    /// [`nodes_of_type`](Self::nodes_of_type) (canonical rank order), this
    /// preserves row order, matching a plain `otype.rows()` type scan.
    pub fn typed_nodes(&self, node_type: &str) -> Result<Option<Arc<Vec<u32>>>> {
        if let Some(by_type) = self.type_nodes_cache.get() {
            return Ok(by_type.get(node_type).cloned());
        }
        let otype = self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let pool_values = otype.distinct_values()?;
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); pool_values.len()];
        otype.for_each_id_row(&mut |node, pool_id| {
            let Some(bucket) = buckets.get_mut(pool_id as usize) else {
                return Err(invalid_compiled(&self.path, "string pool id out of bounds"));
            };
            bucket.push(node);
            Ok(true)
        })?;
        let by_type: HashMap<String, Arc<Vec<u32>>> = pool_values
            .into_iter()
            .zip(buckets)
            .map(|(value, bucket)| (value.to_string(), Arc::new(bucket)))
            .collect();
        let _ = self.type_nodes_cache.set(by_type);
        Ok(self
            .type_nodes_cache
            .get()
            .expect("type nodes just initialized")
            .get(node_type)
            .cloned())
    }

    /// The highest slot node (TF `maxSlot`). In TF's data model the slots are
    /// exactly the nodes `1..=maxSlot`, all of node 1's type, so the boundary
    /// is found by binary search over `otype` pool ids (a few reads, no scan)
    /// and cached per corpus handle.
    pub fn max_slot(&self) -> Result<u32> {
        if let Some(max_slot) = self.max_slot_cache.get() {
            return Ok(*max_slot);
        }
        let otype = self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let max_slot = match otype.pool_id(1)? {
            Some(slot_id) => {
                // Invariant: `otype(node) == slot type` holds for 1..=max_slot
                // and fails above it. `low` is always a slot.
                let (mut low, mut high) = (1_u32, otype.last_node()?.unwrap_or(1));
                while low < high {
                    let mid = low + (high - low).div_ceil(2);
                    if otype.pool_id(mid)? == Some(slot_id) {
                        low = mid;
                    } else {
                        high = mid - 1;
                    }
                }
                low
            }
            None => 0,
        };
        Ok(*self.max_slot_cache.get_or_init(|| max_slot))
    }

    fn string_pool_parts(&self, name: &str) -> Result<Option<Arc<StringPoolParts>>> {
        let Some(feature) = self.metadata.node_feature(name) else {
            return Ok(None);
        };
        if feature.encoding != NodeFeatureEncoding::StringPool {
            return Ok(None);
        }
        if let Some(parts) = self
            .caches
            .read()
            .expect("view cache poisoned")
            .string_pool
            .get(name)
        {
            return Ok(Some(Arc::clone(parts)));
        }
        let parts = Arc::new(build_string_pool_parts(&self.path, &self.mmap, feature)?);
        self.caches
            .write()
            .expect("view cache poisoned")
            .string_pool
            .insert(name.to_string(), Arc::clone(&parts));
        Ok(Some(parts))
    }

    fn mixed_parts(&self, name: &str) -> Result<Option<Arc<MixedParts>>> {
        let Some(feature) = self.metadata.node_feature(name) else {
            return Ok(None);
        };
        if feature.encoding != NodeFeatureEncoding::Mixed {
            return Ok(None);
        }
        if let Some(parts) = self
            .caches
            .read()
            .expect("view cache poisoned")
            .mixed
            .get(name)
        {
            return Ok(Some(Arc::clone(parts)));
        }
        let parts = Arc::new(build_mixed_parts(&self.path, &self.mmap, feature)?);
        self.caches
            .write()
            .expect("view cache poisoned")
            .mixed
            .insert(name.to_string(), Arc::clone(&parts));
        Ok(Some(parts))
    }

    fn edge_parts(&self, name: &str) -> Result<Option<Arc<EdgeParts>>> {
        let Some(feature) = self.metadata.edge_feature(name) else {
            return Ok(None);
        };
        if let Some(parts) = self
            .caches
            .read()
            .expect("view cache poisoned")
            .edge
            .get(name)
        {
            return Ok(Some(Arc::clone(parts)));
        }
        let parts = Arc::new(build_edge_parts(&self.path, &self.mmap, feature)?);
        self.caches
            .write()
            .expect("view cache poisoned")
            .edge
            .insert(name.to_string(), Arc::clone(&parts));
        Ok(Some(parts))
    }

    pub fn string_pool_node_feature(
        &self,
        name: &str,
    ) -> Result<Option<StringPoolNodeFeatureView<'_>>> {
        Ok(self
            .string_pool_parts(name)?
            .map(|parts| StringPoolNodeFeatureView {
                path: &self.path,
                bytes: &self.mmap,
                parts,
                rank_start: self.metadata.rank_start,
                rank_len: self.metadata.rank_len,
            }))
    }

    pub fn mixed_node_feature(&self, name: &str) -> Result<Option<MixedNodeFeatureView<'_>>> {
        Ok(self.mixed_parts(name)?.map(|parts| MixedNodeFeatureView {
            path: &self.path,
            bytes: &self.mmap,
            parts,
            rank_start: self.metadata.rank_start,
            rank_len: self.metadata.rank_len,
        }))
    }

    pub fn node_feature(&self, name: &str) -> Result<Option<MappedNodeFeatureView<'_>>> {
        let Some(feature) = self.metadata.node_feature(name) else {
            return Ok(None);
        };
        match feature.encoding {
            NodeFeatureEncoding::StringPool => self
                .string_pool_node_feature(name)
                .map(|feature| feature.map(MappedNodeFeatureView::StringPool)),
            NodeFeatureEncoding::Mixed => self
                .mixed_node_feature(name)
                .map(|feature| feature.map(MappedNodeFeatureView::Mixed)),
        }
    }

    pub fn edge_feature(&self, name: &str) -> Result<Option<EdgeFeatureView<'_>>> {
        Ok(self.edge_parts(name)?.map(|parts| EdgeFeatureView {
            path: &self.path,
            bytes: &self.mmap,
            parts,
            rank_start: self.metadata.rank_start,
            rank_len: self.metadata.rank_len,
        }))
    }

    /// Resolve a string-pool node feature into an *owned* handle that holds the
    /// corpus `Arc` and the parsed `Arc<Parts>`. Long-lived binding objects can
    /// cache the handle (e.g. in a `OnceLock`) and then call `view()` per access,
    /// skipping the per-call `RwLock` + `HashMap` lookup + parts rebuild. The
    /// thin `view()` only does an `Arc` pointer clone.
    pub fn owned_string_pool_feature(
        self: &Arc<Self>,
        name: &str,
    ) -> Result<Option<OwnedStringPoolFeature>> {
        Ok(self
            .string_pool_parts(name)?
            .map(|parts| OwnedStringPoolFeature {
                corpus: Arc::clone(self),
                parts,
            }))
    }

    /// Owned counterpart of [`mixed_node_feature`](Self::mixed_node_feature).
    pub fn owned_mixed_feature(self: &Arc<Self>, name: &str) -> Result<Option<OwnedMixedFeature>> {
        Ok(self.mixed_parts(name)?.map(|parts| OwnedMixedFeature {
            corpus: Arc::clone(self),
            parts,
        }))
    }

    /// Owned counterpart of [`node_feature`](Self::node_feature): the feature's
    /// handle in whichever encoding it was compiled with.
    pub fn owned_node_feature(self: &Arc<Self>, name: &str) -> Result<Option<OwnedNodeFeature>> {
        if let Some(feature) = self.owned_string_pool_feature(name)? {
            return Ok(Some(OwnedNodeFeature::StringPool(feature)));
        }
        Ok(self.owned_mixed_feature(name)?.map(OwnedNodeFeature::Mixed))
    }

    /// Owned counterpart of [`edge_feature`](Self::edge_feature).
    pub fn owned_edge_feature(self: &Arc<Self>, name: &str) -> Result<Option<OwnedEdgeFeature>> {
        Ok(self.edge_parts(name)?.map(|parts| OwnedEdgeFeature {
            corpus: Arc::clone(self),
            parts,
        }))
    }

    pub fn config_feature(&self, name: &str) -> Result<Option<ConfigFeatureView<'_>>> {
        let Some(feature) = self.metadata.config_feature(name) else {
            return Ok(None);
        };
        ConfigFeatureView::new(&self.path, &self.mmap, feature).map(Some)
    }

    /// Cached levels. The clone per call is ~a dozen small structs — negligible
    /// next to the one-time full-corpus scan in `compute_mapped_levels`.
    pub(crate) fn mapped_levels(&self) -> Result<Vec<MappedLevel>> {
        if let Some(levels) = self.levels_cache.get() {
            return Ok(levels.clone());
        }
        let levels = self.compute_mapped_levels()?;
        let _ = self.levels_cache.set(levels.clone());
        Ok(levels)
    }

    fn compute_mapped_levels(&self) -> Result<Vec<MappedLevel>> {
        let otype = self
            .string_pool_node_feature("otype")?
            .ok_or_else(|| CfError::MissingFeature("otype".to_string()))?;
        let oslots = self
            .edge_feature("oslots")?
            .ok_or_else(|| CfError::MissingFeature("oslots".to_string()))?;
        let slot_type = otype.str_value(1)?.unwrap_or_default().to_string();
        let mut rows = BTreeMap::<String, (usize, usize, u32, u32)>::new();

        for row in otype.rows() {
            let (node, node_type) = row?;
            let slot_count = if node_type == slot_type {
                1
            } else {
                oslots.s(node)?.len()
            };
            rows.entry(node_type.to_string())
                .and_modify(|(count, total_slots, min_node, max_node)| {
                    *count += 1;
                    *total_slots += slot_count;
                    *min_node = (*min_node).min(node);
                    *max_node = (*max_node).max(node);
                })
                .or_insert((1, slot_count, node, node));
        }

        let mut levels = rows
            .into_iter()
            .map(
                |(node_type, (count, total_slots, min_node, max_node))| MappedLevel {
                    node_type,
                    average_slots: total_slots as f64 / count.max(1) as f64,
                    min_node,
                    max_node,
                },
            )
            .collect::<Vec<_>>();
        levels.sort_by(|left, right| {
            right
                .average_slots
                .total_cmp(&left.average_slots)
                .then_with(|| right.min_node.cmp(&left.min_node))
        });

        // Honor `@levelConstraints` (TF `prepare.py`) so the otype ranks derived
        // from this list (and hence canonical tie-breaks) match Text-Fabric.
        if let Some(spec) = &self.otext()?.level_constraints {
            crate::index::apply_level_constraints(&mut levels, spec, |level| {
                level.node_type.as_str()
            });
        }
        Ok(levels)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ComputedFeatureData {
    Levels(Vec<(String, f64, u32, u32)>),
    Order(Vec<u32>),
    Rank(Vec<u32>),
    Boundary(Boundary),
    LevUp(Vec<Vec<u32>>),
    LevDown(Vec<Vec<u32>>),
}
