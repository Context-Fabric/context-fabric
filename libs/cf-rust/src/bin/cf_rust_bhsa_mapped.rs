use std::path::PathBuf;
use std::time::Instant;

use cf_rust::compiled::{MappedCompiledCorpus, compile_features};
use cf_rust::{MappedSearch, Result};

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
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/bhsa-core.cfr"));

    let compile_ms = if cache_path.exists() {
        None
    } else {
        let compile_start = Instant::now();
        compile_features(&tf_path, &cache_path, &["otype", "oslots", "sp"])?;
        Some(compile_start.elapsed())
    };

    let load_start = Instant::now();
    let mapped = MappedCompiledCorpus::open(&cache_path)?;
    let oslots = mapped
        .edge_feature("oslots")?
        .expect("compiled corpus should include oslots");
    let load_ms = load_start.elapsed();

    let query_start = Instant::now();
    let search = MappedSearch::new(&mapped);
    let verbs = search.search("word sp=verb", None)?;
    let words = search.search("word", None)?;
    let query_ms = query_start.elapsed();

    println!(
        "compile_ms={} load_ms={:.3} query_ms={:.3} nodes={} words={} verbs={} oslots_rows={}",
        compile_ms
            .map(|duration| format!("{:.3}", duration.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "cached".to_string()),
        load_ms.as_secs_f64() * 1000.0,
        query_ms.as_secs_f64() * 1000.0,
        mapped.metadata().order_len,
        words.len(),
        verbs.len(),
        oslots.row_count()
    );
    Ok(())
}
