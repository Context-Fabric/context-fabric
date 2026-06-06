use std::path::PathBuf;
use std::time::Instant;

use context_fabric_core::compiled::{compile_features, load_compiled};

fn main() {
    let tf_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/.corpora/bhsa/tf")
        });
    let cache_path = std::env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/bhsa-core.cfr"));

    let compile_ms = if cache_path.exists() {
        None
    } else {
        let compile_start = Instant::now();
        compile_features(&tf_path, &cache_path, &["otype", "oslots", "sp"])
            .expect("BHSA corpus should compile");
        Some(compile_start.elapsed())
    };

    let load_start = Instant::now();
    let corpus = load_compiled(&cache_path).expect("compiled BHSA corpus should load");
    let load_ms = load_start.elapsed();

    let query_start = Instant::now();
    let verbs = corpus
        .search()
        .search("word sp=verb", None)
        .expect("query should run");
    let query_ms = query_start.elapsed();

    println!(
        "compile_ms={} load_ms={:.3} query_ms={:.3} nodes={} words={} verbs={}",
        compile_ms
            .map(|duration| format!("{:.3}", duration.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "cached".to_string()),
        load_ms.as_secs_f64() * 1000.0,
        query_ms.as_secs_f64() * 1000.0,
        corpus.max_node,
        corpus.nodes_of_type("word").len(),
        verbs.len()
    );
}
