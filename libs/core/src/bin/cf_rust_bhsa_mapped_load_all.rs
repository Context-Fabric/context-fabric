use std::path::PathBuf;
use std::time::Instant;

use context_fabric_core::compiled::{MappedCompiledCorpus, compile_features};
use context_fabric_core::{MappedSearch, Result};

fn main() -> Result<()> {
    let tf_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/.corpora/bhsa/tf")
        });
    let cache_path = std::env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/bhsa-all-features.cfr")
        });
    let limit = std::env::args()
        .nth(3)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5);

    let compile_ms = if cache_path.exists() {
        None
    } else {
        let compile_start = Instant::now();
        compile_features(&tf_path, &cache_path, &[])?;
        Some(compile_start.elapsed().as_secs_f64() * 1000.0)
    };

    let load_start = Instant::now();
    let mapped = MappedCompiledCorpus::open(&cache_path)?;
    let load_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let query_start = Instant::now();
    let results = MappedSearch::new(&mapped).search("word sp=verb", Some(limit))?;
    let query_ms = query_start.elapsed().as_secs_f64() * 1000.0;
    let metadata = mapped.metadata();

    println!(
        "loaded rust_mapped_features={} compile_ms={} load_ms={:.3} nodes={}",
        metadata.node_features.len()
            + metadata.edge_features.len()
            + metadata.config_features.len(),
        compile_ms
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "cached".to_string()),
        load_ms,
        metadata.order_len
    );
    println!(
        "ok query=word_sp_verb results={} elapsed_ms={:.3}",
        results.len(),
        query_ms
    );

    Ok(())
}
