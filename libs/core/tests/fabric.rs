//! The `Fabric` facade: locations, exploration, loading and compiling.

use std::fs;

mod common;

use common::*;
use context_fabric_core::*;

#[test]
fn fabric_facade_explores_loads_compiles_and_opens_mapped_corpora() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let fabric = Fabric::new(&source);

    let inventory = fabric.explore().unwrap();
    assert!(inventory.nodes.contains(&"word".to_string()));
    assert!(inventory.edges.contains(&"parent".to_string()));

    let loaded = fabric.load_all().unwrap();
    assert_eq!(loaded.max_node(), 8);

    let selective = fabric.load("word, pos").unwrap();
    assert!(selective.node_feature("word").unwrap().is_some());
    assert!(selective.node_feature("pos").unwrap().is_some());
    assert!(selective.node_feature("number").unwrap().is_some());
    assert!(selective.edge_feature("oslots").unwrap().is_some());

    let additive = fabric.load("word").unwrap();
    assert!(additive.node_feature("word").unwrap().is_some());
    assert!(additive.node_feature("pos").unwrap().is_some());
    assert!(additive.node_feature("number").unwrap().is_some());
    let selective_from_slice = fabric.load(["word", "number"]).unwrap();
    assert!(selective_from_slice.node_feature("word").unwrap().is_some());
    assert!(
        selective_from_slice
            .node_feature("number")
            .unwrap()
            .is_some()
    );
    assert!(selective_from_slice.node_feature("pos").unwrap().is_some());

    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");
    fabric.compile(&cache_path, Vec::<&str>::new()).unwrap();

    let mapped = fabric.open_mapped(&cache_path).unwrap();
    assert_eq!(
        MappedSearch::new(&mapped)
            .unwrap()
            .search("word word=hello", None)
            .unwrap(),
        vec![vec![1]]
    );
}

#[test]
fn fabric_load_all_creates_reuses_and_refreshes_mapped_cache() {
    let temp_dir = tempfile::tempdir().unwrap();
    let corpus_dir = copy_mini_corpus(&temp_dir.path().join("mini"));
    let fabric = Fabric::new(&corpus_dir);
    let cache_path = fabric.cfr_cache_path().unwrap();
    assert_eq!(
        cache_path.parent().unwrap(),
        corpus_dir.join(".cfr").join(CFR_VERSION)
    );
    let name = cache_path.file_name().unwrap().to_string_lossy();
    assert!(
        name.starts_with("corpus.") && name.ends_with(".cfr"),
        "{name}"
    );
    assert!(!cache_path.exists());

    let first = fabric.load_all().unwrap();
    assert_eq!(first.max_node(), 8);
    assert!(cache_path.exists());
    let first_mtime = fs::metadata(&cache_path).unwrap().modified().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = fabric.load_all().unwrap();
    assert_eq!(second.max_node(), 8);
    let second_mtime = fs::metadata(&cache_path).unwrap().modified().unwrap();
    assert_eq!(first_mtime, second_mtime);

    std::thread::sleep(std::time::Duration::from_millis(20));
    let word_path = corpus_dir.join("word.tf");
    let word_contents = fs::read_to_string(&word_path).unwrap();
    fs::write(&word_path, word_contents).unwrap();
    let refreshed_path = fabric.cfr_cache_path().unwrap();
    assert_ne!(refreshed_path, cache_path);
    let third = fabric.load_all().unwrap();
    assert_eq!(third.max_node(), 8);
    assert!(refreshed_path.exists());
    // The stale cache is pruned where the platform allows deleting a mapped
    // file; the corpora mapped from it keep working either way.
    #[cfg(unix)]
    assert!(!cache_path.exists());
    assert_eq!(first.max_node(), 8);
}

#[test]
fn fabric_modules_use_last_module_wins_and_report_ignored_feature_paths() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n@description=base words\n\nbase1\nbase2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("otext.tf"),
        "@config\n@fmt:text-orig-full={word}\n\n",
    )
    .unwrap();

    let extra = dir.path().join("extra");
    fs::create_dir(&extra).unwrap();
    fs::write(
        extra.join("word.tf"),
        "@node\n@valueType=str\n@description=override words\n\nextra1\nextra2\n",
    )
    .unwrap();

    let fabric = Fabric::with_modules(dir.path(), ["", "extra"]);
    let inventory = fabric.explore().unwrap();
    assert_eq!(
        inventory.paths["word"],
        [dir.path().join("word.tf"), extra.join("word.tf")]
    );
    assert_eq!(
        inventory.metadata["word"]
            .get("description")
            .and_then(Option::as_deref),
        Some("override words")
    );
    assert_eq!(
        fabric.ignored_feature_paths().unwrap()["word"],
        vec![dir.path().join("word.tf")]
    );

    let corpus = fabric.load_all().unwrap();
    assert_eq!(
        corpus
            .string_pool_node_feature("word")
            .unwrap()
            .unwrap()
            .str_value(1)
            .unwrap(),
        Some("extra1")
    );
    let corpus = std::sync::Arc::new(corpus);
    assert_eq!(
        TextRenderer::new(&corpus)
            .unwrap()
            .text(2, &TextOptions::default())
            .unwrap(),
        "extra2"
    );
}

#[test]
fn public_fabric_constructor_metadata_matches_python_public_state() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let fabric = Fabric::new(&source);

    assert_eq!(fabric.banner(), BANNER);
    assert_eq!(fabric.version(), VERSION);
    assert!(fabric.good());
    assert_eq!(fabric.locations(), std::slice::from_ref(&source));
    assert_eq!(fabric.modules(), &[String::new()]);
    assert_eq!(fabric.location_rep(), source.display().to_string());

    let from_locations = Fabric::from_locations([source.clone()]);
    assert_eq!(from_locations.locations(), std::slice::from_ref(&source));
    assert_eq!(from_locations.load_all().unwrap().max_node(), 8);

    let with_modules = Fabric::with_modules(&source, ["", "extra"]);
    assert_eq!(
        with_modules.modules(),
        &[String::new(), "extra".to_string()]
    );
    assert_eq!(with_modules.load_all().unwrap().max_node(), 8);
}
