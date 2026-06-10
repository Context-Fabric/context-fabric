use std::path::PathBuf;
use std::time::Instant;

use context_fabric_core::{Corpus, MappedCompiledCorpus, MappedSearch, compile_features};

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/.corpora/bhsa/tf")
        });
    let load_start = Instant::now();
    let corpus =
        Corpus::load_features(&path, &["otype", "oslots", "sp"]).expect("BHSA corpus should load");
    let load_ms = load_start.elapsed();
    // Search runs on the mapped engine; compile the cache as one-time setup.
    let cache = std::env::temp_dir().join("cf_rust_bhsa.cfr");
    compile_features(&path, &cache, &["otype", "oslots", "sp"]).expect("compile BHSA cache");
    let mapped = MappedCompiledCorpus::open(&cache).expect("open BHSA cache");
    let query_start = Instant::now();
    let verbs = MappedSearch::new(&mapped)
        .search("word sp=verb", None)
        .expect("query should run");
    let query_ms = query_start.elapsed();
    println!(
        "load_ms={:.3} query_ms={:.3} nodes={} words={} verbs={}",
        load_ms.as_secs_f64() * 1000.0,
        query_ms.as_secs_f64() * 1000.0,
        corpus.max_node,
        corpus.nodes_of_type("word").len(),
        verbs.len()
    );
}
