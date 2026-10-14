//! Row access to the precomputed `levUp`, `levDown`, `boundary` and sections indexes.

use crate::error::Result;
use crate::format::{MappedU32s, Reader, csr_row, read_csr_row};
use crate::index::SectionsData;
use crate::mapped::MappedCompiledCorpus;

impl MappedCompiledCorpus {
    /// [`lev_up_row`](Self::lev_up_row) borrowed from the map, without
    /// materializing a `Vec`.
    pub fn lev_up_values(&self, node: u32) -> Result<Option<MappedU32s<'_>>> {
        csr_row(&self.path, &self.mmap, self.metadata.lev_up_start, node)
    }

    /// Embedders of `node` in descending rank order (`levUp` CSR row), read
    /// directly from the mmap.
    pub fn lev_up_row(&self, node: u32) -> Result<Option<Vec<u32>>> {
        read_csr_row(&self.path, &self.mmap, self.metadata.lev_up_start, node)
    }

    /// Embeddees of `node` (`levDown` CSR row), read directly from the mmap.
    pub fn lev_down_row(&self, node: u32) -> Result<Option<Vec<u32>>> {
        read_csr_row(&self.path, &self.mmap, self.metadata.lev_down_start, node)
    }

    /// Nodes whose first slot is `slot` (`boundary` first CSR row).
    pub fn boundary_first(&self, slot: u32) -> Result<Option<Vec<u32>>> {
        read_csr_row(
            &self.path,
            &self.mmap,
            self.metadata.boundary_first_start,
            slot,
        )
    }

    /// Nodes whose last slot is `slot` (`boundary` last CSR row).
    pub fn boundary_last(&self, slot: u32) -> Result<Option<Vec<u32>>> {
        read_csr_row(
            &self.path,
            &self.mmap,
            self.metadata.boundary_last_start,
            slot,
        )
    }

    /// Lazily-loaded `CFRSECT1` sections lookup. Loaded once into RAM (< 2 MB on
    /// BHSA) and cached. Returns `None` for caches without a sections section.
    pub fn sections_data(&self) -> Result<Option<&SectionsData>> {
        if let Some(cached) = self.sections_cache.get() {
            return Ok(cached.as_ref());
        }
        let value = match self.metadata.sections_start {
            Some(start) => {
                let mut reader = Reader::at(&self.path, &self.mmap, start);
                Some(reader.read_sections_section()?)
            }
            None => None,
        };
        let _ = self.sections_cache.set(value);
        Ok(self
            .sections_cache
            .get()
            .expect("sections cache just initialized")
            .as_ref())
    }
}
