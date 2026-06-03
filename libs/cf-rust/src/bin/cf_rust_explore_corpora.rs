use std::path::PathBuf;
use std::time::Instant;

use cf_rust::explore_features;

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
        match explore_features(&tf_path) {
            Ok(inventory) => {
                let has_otype = inventory.nodes.iter().any(|feature| feature == "otype");
                let has_oslots = inventory.edges.iter().any(|feature| feature == "oslots");
                let status = if has_otype && has_oslots {
                    "ok"
                } else {
                    "fail"
                };
                println!(
                    "{} corpus={} explore_ms={:.3} nodes={} edges={} configs={} has_otype={} has_oslots={}",
                    status,
                    name,
                    start.elapsed().as_secs_f64() * 1000.0,
                    inventory.nodes.len(),
                    inventory.edges.len(),
                    inventory.configs.len(),
                    has_otype,
                    has_oslots
                );
                if status == "fail" {
                    failures.push(name.to_string());
                }
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
