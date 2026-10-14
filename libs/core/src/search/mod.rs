//! Search templates over a mapped corpus: parsing, planning, and the join.

mod engine;
mod study;
mod syntax;

pub use engine::MappedSearch;
pub use study::SearchStudy;
pub use syntax::{RELATIONS_LEGEND, SEARCH_FAIL_FACTOR, SearchSets};
