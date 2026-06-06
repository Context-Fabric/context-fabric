use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use context_fabric_core::{
    Corpus, FeatureValue, MappedCompiledCorpus, MappedSearch, MappedSections, MappedText,
    SectionOptions, StructureHeading, StructureTree, TextOptions, compile_features,
};
use serde_json::{Value, json};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 3 || args.len() > 4 {
        return Err("usage: cf_rust_dump [--mapped] <tf_path> <probes.jsonl>".into());
    }

    let mapped = args.get(1).is_some_and(|arg| arg == "--mapped");
    let offset = if mapped { 2 } else { 1 };
    let tf_path = Path::new(&args[offset]);
    let probes_path = Path::new(&args[offset + 1]);
    let probe_lines = BufReader::new(File::open(probes_path)?)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;
    let probes = probe_lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line))
        .collect::<Result<Vec<_>, _>>()?;
    let selected_features = mapped_feature_set(&probes);
    let selected_feature_refs = selected_features
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let corpus = Corpus::load_features(tf_path, &selected_feature_refs)?;
    let mapped_corpus = if mapped {
        let output = temp_compiled_path();
        compile_features(tf_path, &output, &selected_feature_refs)?;
        Some(MappedCompiledCorpus::open(&output)?)
    } else {
        None
    };
    for probe in probes {
        let id = probe
            .get("id")
            .and_then(Value::as_str)
            .ok_or("probe missing string id")?;
        let kind = probe
            .get("kind")
            .and_then(Value::as_str)
            .ok_or("probe missing string kind")?;
        let args = probe.get("args").unwrap_or(&Value::Null);
        let result = if let Some(mapped_corpus) = mapped_corpus.as_ref() {
            run_mapped_probe(&corpus, mapped_corpus, kind, args)?
        } else {
            run_probe(&corpus, kind, args)?
        };
        println!(
            "{}",
            serde_json::to_string(&json!({"id": id, "result": result}))?
        );
    }
    Ok(())
}

fn mapped_feature_set(probes: &[Value]) -> Vec<String> {
    let mut features = std::collections::BTreeSet::from([
        "book".to_string(),
        "book@en".to_string(),
        "book@he".to_string(),
        "chapter".to_string(),
        "verse".to_string(),
        "word".to_string(),
        "text".to_string(),
        "before".to_string(),
        "after".to_string(),
        "letters".to_string(),
        "punc".to_string(),
        "title".to_string(),
        "number".to_string(),
        "g_word_utf8".to_string(),
        "qere_utf8".to_string(),
        "trailer_utf8".to_string(),
        "qere_trailer_utf8".to_string(),
    ]);
    for probe in probes {
        if let Some(template) = probe
            .get("args")
            .and_then(|args| args.get("template"))
            .and_then(Value::as_str)
        {
            for token in template.split(|ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '@')
            {
                if matches!(
                    token,
                    "sp" | "vt" | "vs" | "gn" | "nu" | "language" | "function" | "typ" | "kind"
                ) {
                    features.insert(token.to_string());
                }
            }
        }
    }
    features.into_iter().collect()
}

fn temp_compiled_path() -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "cf-rust-dump-{}-{}.cfr",
        std::process::id(),
        unique_suffix()
    ));
    path
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn run_probe(
    corpus: &Corpus,
    kind: &str,
    args: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    match kind {
        "text" => {
            let node = required_u32(args, "node")?;
            let format = args.get("format").and_then(Value::as_str);
            let descend = args.get("descend").and_then(Value::as_bool);
            Ok(json!(corpus.text_with_options(
                node,
                &TextOptions::new(format.map(str::to_string), descend)
            )))
        }
        "section_from_node" => {
            let node = required_u32(args, "node")?;
            let lang = args.get("lang").and_then(Value::as_str).unwrap_or("");
            Ok(json!(corpus.section_from_node_lang(
                node,
                &SectionOptions::default(),
                lang
            )))
        }
        "node_from_section" => {
            let section = args
                .get("section")
                .and_then(Value::as_array)
                .ok_or("node_from_section requires section array")?
                .iter()
                .map(value_to_feature_value)
                .collect::<Result<Vec<_>, _>>()?;
            let lang = args.get("lang").and_then(Value::as_str).unwrap_or("");
            Ok(json!(corpus.node_from_section_lang(&section, lang)))
        }
        "heading_from_node" => {
            let node = required_u32(args, "node")?;
            Ok(match corpus.heading_from_node(node) {
                Some(heading) => heading_to_json(&heading),
                None => Value::Null,
            })
        }
        "node_from_heading" => {
            let heading = args
                .get("heading")
                .and_then(Value::as_array)
                .ok_or("node_from_heading requires heading array")?
                .iter()
                .map(value_to_heading)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!(corpus.node_from_heading(&heading)))
        }
        "structure" => Ok(match corpus.structure(optional_u32(args, "node")) {
            Some(tree) => structure_tree_to_json(&tree),
            None => Value::Null,
        }),
        "top" => Ok(json!(corpus.top())),
        "structure_pretty" => Ok(json!(
            corpus.structure_pretty(
                optional_u32(args, "node"),
                args.get("full_heading")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            )
        )),
        "book_name" => {
            let node = required_u32(args, "node")?;
            let lang = args.get("lang").and_then(Value::as_str).unwrap_or("");
            Ok(json!(corpus.bookName(node, lang)))
        }
        "book_node" => {
            let name = args
                .get("name")
                .and_then(Value::as_str)
                .ok_or("book_node requires name string")?;
            let lang = args.get("lang").and_then(Value::as_str).unwrap_or("");
            Ok(json!(corpus.bookNode(name, lang)))
        }
        "locality_up" => {
            let node = required_u32(args, "node")?;
            let node_type = args.get("node_type").and_then(Value::as_str);
            Ok(json!(corpus.u(node, node_type)))
        }
        "search" => {
            let template = args
                .get("template")
                .and_then(Value::as_str)
                .ok_or("search requires template string")?;
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            Ok(json!(corpus.search().search(template, limit)?))
        }
        _ => Err(format!("unsupported probe kind {kind:?}").into()),
    }
}

fn run_mapped_probe(
    corpus: &Corpus,
    mapped: &MappedCompiledCorpus,
    kind: &str,
    args: &Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    match kind {
        "text" => {
            let node = required_u32(args, "node")?;
            let format = args.get("format").and_then(Value::as_str);
            let descend = args.get("descend").and_then(Value::as_bool);
            Ok(json!(MappedText::new(mapped)?.text_with_options(
                node,
                &TextOptions::new(format.map(str::to_string), descend)
            )?))
        }
        "section_from_node" => {
            let node = required_u32(args, "node")?;
            if args.get("lang").is_some() {
                return run_probe(corpus, kind, args);
            }
            Ok(json!(
                MappedSections::new(mapped)?.section_from_node(node, &SectionOptions::default())?
            ))
        }
        "node_from_section" => {
            if args.get("lang").is_some() {
                return run_probe(corpus, kind, args);
            }
            let section = args
                .get("section")
                .and_then(Value::as_array)
                .ok_or("node_from_section requires section array")?
                .iter()
                .map(value_to_feature_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!(
                MappedSections::new(mapped)?.node_from_section(&section)?
            ))
        }
        "locality_up" => run_probe(corpus, kind, args),
        "search" => {
            let template = args
                .get("template")
                .and_then(Value::as_str)
                .ok_or("search requires template string")?;
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            Ok(json!(MappedSearch::new(mapped).search(template, limit)?))
        }
        _ => run_probe(corpus, kind, args),
    }
}

fn required_u32(args: &Value, key: &str) -> Result<u32, Box<dyn std::error::Error>> {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| format!("missing integer arg {key:?}").into())
}

fn optional_u32(args: &Value, key: &str) -> Option<u32> {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as u32)
}

fn value_to_heading(value: &Value) -> Result<StructureHeading, Box<dyn std::error::Error>> {
    let items = value
        .as_array()
        .ok_or("heading item must be a two-value array")?;
    if items.len() != 2 {
        return Err("heading item must have two values".into());
    }
    let node_type = items[0]
        .as_str()
        .ok_or("heading item first value must be a string")?
        .to_string();
    let heading = if let Some(text) = items[1].as_str() {
        text.to_string()
    } else if let Some(number) = items[1].as_i64() {
        number.to_string()
    } else {
        return Err("heading item second value must be a string or integer".into());
    };
    Ok(StructureHeading { node_type, heading })
}

fn value_to_feature_value(value: &Value) -> Result<FeatureValue, Box<dyn std::error::Error>> {
    if let Some(text) = value.as_str() {
        Ok(FeatureValue::string(text))
    } else if let Some(number) = value.as_i64() {
        Ok(FeatureValue::Int(number))
    } else {
        Err("section values must be strings or integers".into())
    }
}

fn structure_tree_to_json(tree: &StructureTree) -> Value {
    match tree {
        StructureTree::Forest(children) => {
            Value::Array(children.iter().map(structure_tree_to_json).collect())
        }
        StructureTree::Node { node, children } => json!([
            node,
            children
                .iter()
                .map(structure_tree_to_json)
                .collect::<Vec<_>>()
        ]),
    }
}

fn heading_to_json(heading: &[StructureHeading]) -> Value {
    Value::Array(
        heading
            .iter()
            .map(|item| {
                let value = item
                    .heading
                    .parse::<i64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| Value::from(item.heading.clone()));
                json!([item.node_type, value])
            })
            .collect(),
    )
}
