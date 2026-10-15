//! The `.cfr` compiled-corpus file format: layout, section magics, and the
//! reader and writer.

/// Header magic: [`FORMAT_FAMILY`] followed by the two-digit format version
/// ([`crate::config::CFR_VERSION`]). Any other version is rejected on open.
const MAGIC: &[u8; 8] = b"CFRUST04";

const FORMAT_FAMILY: &[u8; 6] = b"CFRUST";

const FEATURE_METADATA_MAGIC: &[u8; 8] = b"CFRMETA1";

const EDGE_VALUES_MAGIC: &[u8; 8] = b"CFREDGE1";

const STRUCTURE_MAGIC: &[u8; 8] = b"CFRSTRC3";

// Precomputed CSR indexes (mmap-resident, 0 RSS).
const LEV_UP_MAGIC: &[u8; 8] = b"CFRLEVU1";

const LEV_DOWN_MAGIC: &[u8; 8] = b"CFRLEVD1";

const BOUNDARY_MAGIC: &[u8; 8] = b"CFRBND1\0";

const SECTIONS_MAGIC: &[u8; 8] = b"CFRSECT1";

// Per edge feature reverse (target -> sources) CSR, see `edge_index`.
const REVERSE_EDGES_MAGIC: &[u8; 8] = b"CFRREVE1";

// Per mixed/edge feature row offsets, see `row_index`.
const ROW_INDEX_MAGIC: &[u8; 8] = b"CFRROWS1";

mod bytes;
mod metadata;
mod reader;
mod writer;

pub(crate) use bytes::*;
pub use metadata::*;
pub use reader::inspect_compiled;
pub(crate) use reader::{Reader, inspect_compiled_bytes};
pub(crate) use writer::{write_compiled, write_compiled_to, write_u32_vec};
