use std::path::PathBuf;
use std::time::Instant;

use cf_rust::Corpus;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/.corpora")
        });

    let mut failures = Vec::new();
    let entries = std::fs::read_dir(&root).expect("corpora root should be readable");
    for entry in entries {
        let entry = entry.expect("corpus directory entry should be readable");
        let corpus_root = entry.path();
        let tf_path = corpus_root.join("tf");
        if !tf_path.is_dir() {
            continue;
        }
        let name = corpus_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>");
        let start = Instant::now();
        match Corpus::load_features(&tf_path, &["otype", "oslots"]) {
            Ok(corpus) => {
                let query = corpus.slot_type.clone();
                let results = corpus
                    .search()
                    .search(&query, Some(5))
                    .expect("slot-type query should run");
                println!(
                    "ok corpus={} load_ms={:.3} nodes={} slots={} query={} results={}",
                    name,
                    start.elapsed().as_secs_f64() * 1000.0,
                    corpus.max_node,
                    corpus.max_slot,
                    query,
                    results.len()
                );
            }
            Err(error) => {
                println!("fail corpus={} error={}", name, error);
                failures.push(name.to_string());
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("failed corpora: {}", failures.join(", "));
        std::process::exit(1);
    }
}
