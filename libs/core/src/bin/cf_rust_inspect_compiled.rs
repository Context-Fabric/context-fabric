use std::env;

use context_fabric_core::Result;
use context_fabric_core::compiled::{MappedCompiledCorpus, inspect_compiled};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .expect("usage: cf_rust_inspect_compiled <compiled-cache>");
    let metadata = inspect_compiled(&path)?;
    println!(
        "bytes={} node_features={} edge_features={} order_len={} rank_len={}",
        metadata.byte_len,
        metadata.node_features.len(),
        metadata.edge_features.len(),
        metadata.order_len,
        metadata.rank_len
    );
    for feature in &metadata.node_features {
        println!(
            "node name={} encoding={:?} rows={} pool={:?} payload={}..{}",
            feature.name,
            feature.encoding,
            feature.row_count,
            feature.string_pool_count,
            feature.payload_start,
            feature.payload_end
        );
    }
    for feature in &metadata.edge_features {
        println!(
            "edge name={} rows={} payload={}..{}",
            feature.name, feature.row_count, feature.payload_start, feature.payload_end
        );
    }

    println!(
        "v3 sections: present={} lev_up={:?} lev_down={:?} boundary_first={:?} boundary_last={:?} sections={:?} (v3_start={:?})",
        metadata.v3_start.is_some(),
        metadata.lev_up_start,
        metadata.lev_down_start,
        metadata.boundary_first_start,
        metadata.boundary_last_start,
        metadata.sections_start,
        metadata.v3_start,
    );

    if let Some(feature_name) = env::args().nth(2) {
        let mapped = MappedCompiledCorpus::open(&path)?;
        if let Some(feature) = mapped.string_pool_node_feature(&feature_name)? {
            for raw_node in env::args().skip(3) {
                let node: u32 = raw_node
                    .parse()
                    .expect("node arguments must be unsigned integers");
                println!(
                    "value feature={} node={} value={:?}",
                    feature_name,
                    node,
                    feature.str_value(node)?
                );
            }
        } else if let Some(feature) = mapped.edge_feature(&feature_name)? {
            for raw_node in env::args().skip(3) {
                let node: u32 = raw_node
                    .parse()
                    .expect("node arguments must be unsigned integers");
                let targets = feature
                    .targets(node)?
                    .map(|targets| targets.collect::<Result<Vec<_>>>())
                    .transpose()?;
                let summary = targets.as_ref().map(|targets| {
                    format!(
                        "count={} first={:?} last={:?}",
                        targets.len(),
                        targets.first(),
                        targets.last()
                    )
                });
                println!(
                    "targets feature={} node={} {}",
                    feature_name,
                    node,
                    summary.unwrap_or_else(|| "missing".to_string())
                );
            }
        } else {
            println!("mapped feature {feature_name} not found");
        }
    }
    Ok(())
}
