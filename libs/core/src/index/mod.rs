//! Precomputed indexes: built at compile time, stored in the `.cfr` file, and
//! read by the mapped engine.

pub(crate) mod edge_index;
pub(crate) mod levels;
pub(crate) mod row_index;
pub(crate) mod sections;
pub(crate) mod structure;

pub use levels::apply_level_constraints;
pub use sections::SectionsData;
pub use structure::{StructureData, StructureHeading, StructureTree};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Boundary {
    pub first_slots: Vec<Vec<u32>>,
    pub last_slots: Vec<Vec<u32>>,
}

pub type LevUpData = Vec<Vec<u32>>;

pub type LevDownData = Vec<Vec<u32>>;
