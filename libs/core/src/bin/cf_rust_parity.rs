use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use context_fabric_core::compiled::MappedCompiledCorpus;
use context_fabric_core::{MappedSearch, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct ParityFile {
    queries: Vec<ParityQuery>,
}

#[derive(Deserialize, Clone)]
struct ParityQuery {
    id: String,
    template: String,
    tf_count: usize,
}

fn main() -> Result<()> {
    let cfr = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/cf_bhsa_v3_full.cfr"));
    let json = std::env::args().nth(2).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/parity_queries.json")
    });
    let filter = std::env::args().nth(3).filter(|s| !s.is_empty()); // optional comma-separated ids
    let budget_secs: u64 = std::env::args()
        .nth(4)
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(120);

    let file: ParityFile = serde_json::from_slice(&std::fs::read(&json).unwrap()).unwrap();
    let ids: Option<Vec<String>> =
        filter.map(|f| f.split(',').map(|s| s.trim().to_string()).collect());

    let corpus = Arc::new(MappedCompiledCorpus::open(&cfr)?);

    println!(
        "{:<32} {:>10} {:>10} {:>10} {:>9}",
        "id", "tf_count", "cf_count", "seconds", "status"
    );
    let mut fails = 0usize;
    for q in &file.queries {
        if let Some(ids) = &ids {
            if !ids.contains(&q.id) {
                continue;
            }
        }
        let (tx, rx) = mpsc::channel();
        let corpus_thread = Arc::clone(&corpus);
        let template = q.template.clone();
        let handle = std::thread::spawn(move || {
            let search = MappedSearch::new(&corpus_thread);
            let start = Instant::now();
            let res = search.count(&template, None);
            let _ = tx.send((res, start.elapsed()));
        });
        match rx.recv_timeout(Duration::from_secs(budget_secs)) {
            Ok((Ok(count), elapsed)) => {
                let ok = count == q.tf_count;
                if !ok {
                    fails += 1;
                }
                println!(
                    "{:<32} {:>10} {:>10} {:>10.3} {:>9}",
                    q.id,
                    q.tf_count,
                    count,
                    elapsed.as_secs_f64(),
                    if ok { "OK" } else { "WRONG" }
                );
                let _ = handle.join();
            }
            Ok((Err(e), _)) => {
                fails += 1;
                println!(
                    "{:<32} {:>10} {:>10} {:>10} {:>9}",
                    q.id, q.tf_count, "ERR", "-", "ERROR"
                );
                eprintln!("  error {}: {}", q.id, e);
                let _ = handle.join();
            }
            Err(_) => {
                fails += 1;
                println!(
                    "{:<32} {:>10} {:>10} {:>10} {:>9}",
                    q.id, q.tf_count, "-", ">budget", "TIMEOUT"
                );
                // leak the worker thread; it cannot be cancelled
            }
        }
    }
    if fails > 0 {
        eprintln!("{fails} failing queries");
    }
    Ok(())
}
