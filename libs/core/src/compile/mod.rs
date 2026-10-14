//! Compilation of Text-Fabric `.tf` sources into a `.cfr` cache: the sources
//! are loaded into an in-memory [`Corpus`], the indexes are precomputed, and
//! the result is written in the `.cfr` format.

pub(crate) mod corpus;
pub(crate) mod precompute;

pub use corpus::Corpus;

use std::path::Path;

use crate::error::Result;
use crate::format::write_compiled;

pub fn compile_features(
    tf_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    features: &[&str],
) -> Result<()> {
    let corpus = if features.is_empty() {
        Corpus::load(tf_path)?
    } else {
        Corpus::load_features(tf_path, features)?
    }
    .with_rank_arrays();
    write_compiled(&corpus, output_path.as_ref())
}

pub fn compile_loaded_corpus(corpus: &Corpus, output_path: impl AsRef<Path>) -> Result<()> {
    write_compiled(corpus, output_path.as_ref())
}

/// Whether the sorted slot list `outer` contains every slot of `inner`.
pub(crate) fn is_superset(outer: &[u32], inner: &[u32]) -> bool {
    inner.iter().all(|slot| outer.binary_search(slot).is_ok())
}
