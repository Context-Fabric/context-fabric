use std::path::PathBuf;
use std::time::Instant;

use cf_rust::Corpus;

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/.corpora/bhsa/tf")
        });
    let limit = std::env::args()
        .nth(2)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5);

    let load_start = Instant::now();
    let corpus = Corpus::load(&path).expect("BHSA corpus should load all features");
    let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let query_start = Instant::now();
    let results = corpus
        .search()
        .search("word sp=verb", Some(limit))
        .expect("query should run");
    let query_ms = query_start.elapsed().as_secs_f64() * 1000.0;

    println!(
        "loaded rust_features={} load_ms={:.3} nodes={} slots={}",
        corpus.node_features.len() + corpus.edge_features.len() + corpus.config_features.len(),
        load_ms,
        corpus.max_node,
        corpus.max_slot
    );
    println!(
        "ok query=word_sp_verb results={} elapsed_ms={:.3}",
        results.len(),
        query_ms
    );
}
