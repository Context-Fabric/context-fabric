use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use context_fabric_core::LogicalRange::{Range, Single};
use context_fabric_core::compiled::{
    MappedCompiledCorpus, MappedNodeFeatureView, MappedNodeValue, NodeFeatureEncoding,
    compile_features, inspect_compiled,
};
use context_fabric_core::explore_features;
use context_fabric_core::{
    __version__, API_VERSION, BANNER, BRANCH_DEFAULT, BRANCH_DEFAULT_NEW, CF_VERSION, CFM_VERSION,
    CONFIG_MISSING_STR_INDEX, DOI_DEFAULT, DOI_URL_PREFIX, GH, GL, HOST, INDEX_DTYPE, MISSING_INT,
    NAME, NODE_DTYPE, OINTERF, OINTERT, OMAP, ORG, OSLOTS, OTEXT, OTYPE, OVOLUME, OWORK, PORT_BASE,
    PROTOCOL, RANK_DTYPE, RELATIVE, REPO, SEARCH_FAIL_FACTOR, TRY_LIMIT_FROM, TRY_LIMIT_TO,
    TYPE_DTYPE, URL_CF_DOCS, URL_GH, URL_GH_API, URL_GH_UPLOAD, URL_GL, URL_GL_API, URL_GL_UPLOAD,
    URL_NB, VERSION, WARP, YARN_RATIO,
};
use context_fabric_core::{
    AUTO, Api, AttrDict, CSRArray, CSRArrayWithValues, CfError, Chunk, CliFlagSpec, CliFlagValue,
    Compiler, Computed, ComputedFeatureData, Computeds, Corpus, CorpusInfo, CsrValue, DATA_TYPES,
    DEEP, Data, ERROR_CUTOFF, ESCAPES, EdgeFeature, EdgeFeatures, EdgeFrequency, FATAL_MSG, Fabric,
    FeatureInfo, FeatureKind, FeatureValue, FitemizeValue, FlattenFeatureSpec, IntFeatureArray,
    LOCATIONS, LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR, LOG_LEVEL_INFO, LOG_LEVEL_WARNING,
    LevDownComputed, LevUpComputed, LoadedFeatureKind, Locality, MEM_MSG, MISSING_STR_INDEX, MSG64,
    MappedSearch, MappedSections, MappedText, MmapManager, NodeFeature, NodeFeatures, NodeInfo,
    NodeInfoOptions, NodeList, Nodes, OrderComputed, OslotsFeature, OtypeFeature, PARENT_REF,
    Projection, QCONT, QEND, QHAVE, QINIT, QOR, QTERM, QWHERE, QWITH, QWITHOUT, RankComputed,
    SILENT_D, SearchResult, SectionOptions, SetValue, SilentInput,
    StringPool, TERSE, Text, TextOptions, TfData, TfDataContent, TfFeature, TfFeatureKind,
    VAL_ESCAPES, VERBOSE, WARN32, WalkEvent, abspath, active_logging_level, api_refs, atomOpRe,
    atomRe, backendRep, camel, chDir, check32, clean_name, cleanName, collect_formats,
    collectFormats, compRe, compile_corpus, configure_logging, console_message, deep_attr_dict,
    deep_size_json, deepSize, deepdict, default_compiled_output_path, describe_corpus,
    describe_corpus_overview, describe_feature, describe_features, describe_text_formats,
    dirAllFiles, dirContents, dirCopy, dirEmpty, dirExists, dirMake, dirMove, dirNm, dirRemove,
    download, expandDir, expanduser, extNm, fileCopy, fileExists, fileMake, fileMove, fileNm,
    fileRemove, fitemize, flatten_to_set, format_meta, get_all_feature_otypes, get_cache_dir,
    get_feature_otypes, getCwd, html_esc, htmlEsc, identRe, indentLineRe, is_clean, is_int,
    is_iterable, is_quantifier_continuation, is_quantifier_init, is_quantifier_line,
    is_quantifier_terminator, is_search_name, is_search_number, is_search_white_line, isClean,
    isDir, isFile, isInt, itemize, kRe, level_map, list_corpora, list_features, log_message,
    logging_level, make_examples, make_index, make_inverse, make_inverse_val, makeIndex,
    makeInverse, makeInverseVal, math_esc, mathEsc, md_esc, mdEsc, mdhtml_esc, mdhtmlEsc,
    merge_dict, merge_dict_of_sets, nameRe, namesRe, nbytes, noneRe, normpath, numRe, opLineRe,
    opStripRe, pandas_esc, pandasEsc, parse_atom_operator_syntax, parse_atom_syntax,
    parse_comparison_syntax, parse_ident_syntax, parse_k_nearness_syntax, parse_named_atom_prefix,
    parse_none_syntax, parse_operator_line_syntax, parse_quantifier_line_syntax,
    parse_regex_feature_syntax, parse_relation_syntax, parse_tf_file, parse_tf_file_metadata,
    parse_true_syntax, prefixSlash, project, quLineRe, ranges_from_list, ranges_from_set,
    rangesFromList, rangesFromSet, reRe, read_args, readJson, readYaml, relRe, replaceExt,
    resolve_corpus_id, scanDir, search_line_indent, set_from_spec,
    set_from_str, set_from_value, set_logging_level, setDir, setFromSpec, should_log,
    silentConvert, spec_from_ranges, spec_from_ranges_logical, specFromRanges,
    specFromRangesLogical, splitExt, splitPath, strip_operator_syntax, stripExt, tf_from_value,
    tfFromValue, trueRe, tsv_esc, tsvEsc, unexpanduser, utcnow, value_from_tf, valueFromTf, var,
    version_sort, versionSort, whiteRe, writeJson, writeYaml, xml_esc, xmlEsc,
};

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn optional_corpus_tf(name: &str) -> Option<PathBuf> {
    let path = repo_path(&format!("libs/benchmarks/.corpora/{name}/tf"));
    if path.join("otype.tf").exists() {
        Some(path)
    } else if env::var_os("CF_REQUIRE_CORPORA").is_some() {
        panic!(
            "required corpus '{name}' is absent at {}",
            path.to_string_lossy()
        );
    } else {
        eprintln!(
            "skipping corpus-dependent test: '{name}' is absent at {}",
            path.to_string_lossy()
        );
        None
    }
}

fn structured_mini_corpus(temp_dir: &tempfile::TempDir) -> PathBuf {
    let structure_source = temp_dir.path().join("structured");
    dirCopy(
        &repo_path("libs/core/tests/fixtures/mini_corpus").to_string_lossy(),
        &structure_source.to_string_lossy(),
        false,
    )
    .unwrap();
    let otext_path = structure_source.join("otext.tf");
    let mut otext = fs::read_to_string(&otext_path).unwrap();
    otext = otext.replace(
        "@structureFeatures=",
        "@structureFeatures=sentence_id,phrase_id",
    );
    otext = otext.replace("@structureTypes=", "@structureTypes=sentence,phrase");
    fs::write(&otext_path, otext).unwrap();
    structure_source
}

/// Compile the bundled mini corpus to a temporary `.cfr` and open it for the
/// mapped search engine. The returned corpus borrows from `dir`, which must
/// outlive it.
fn mapped_mini_corpus(dir: &tempfile::TempDir) -> MappedCompiledCorpus {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let cache = dir.path().join("mini.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    MappedCompiledCorpus::open(&cache).unwrap()
}

// Mini corpus layout (used by the directional operator tests below):
//   words 1..5 are slots; phrase 6 = slots 1-3, phrase 7 = slots 4-5,
//   sentence 8 = slots 1-5.
#[test]
fn mapped_operator_prefixed_atoms_emit_directional_sibling_relations() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `<: word` (AdjacentBefore) between the previous sibling (w1) and the
    // operator atom (w2): w1 immediately before w2, both embedded in the sentence.
    let mut adjacent_before = search.search("sentence\n  w1:word\n  <: w2:word", None).unwrap();
    adjacent_before.sort_unstable();
    assert_eq!(
        adjacent_before,
        vec![vec![8, 1, 2], vec![8, 2, 3], vec![8, 3, 4], vec![8, 4, 5]]
    );

    // Direction is load-bearing: `:> word` (AdjacentAfter) yields the reverse
    // pairs, not the same set.
    let mut adjacent_after = search.search("sentence\n  w1:word\n  :> w2:word", None).unwrap();
    adjacent_after.sort_unstable();
    assert_eq!(
        adjacent_after,
        vec![vec![8, 2, 1], vec![8, 3, 2], vec![8, 4, 3], vec![8, 5, 4]]
    );

    // `< word` (canonical before) between siblings: prevSibling < opAtom, never
    // inverted; all strictly increasing word pairs inside the sentence.
    let before = search.search("sentence\n  w1:word\n  < w2:word", None).unwrap();
    assert_eq!(before.len(), 10);
    assert!(before.iter().all(|row| row[1] < row[2]));
    assert!(before.contains(&vec![8, 1, 5]));

    // An operator-prefixed first child takes the PARENT as the left operand and
    // keeps the embedding edge. `:= word` (SameLastSlot) under the sentence:
    // the word whose last slot equals the sentence's last slot (slot 5).
    let mut same_last = search.search("sentence\n  := word", None).unwrap();
    same_last.sort_unstable();
    assert_eq!(same_last, vec![vec![8, 5]]);

    // `=: word` (SameFirstSlot) under the sentence -> word at slot 1.
    let mut same_first = search.search("sentence\n  =: word", None).unwrap();
    same_first.sort_unstable();
    assert_eq!(same_first, vec![vec![8, 1]]);

    // Embedding is still enforced alongside the operator edge: a `:= word` whose
    // candidate is outside the parent never appears (sentence 8 is the only
    // sentence, so all matches are contained).
    assert!(search.search("sentence\n  := word", None).unwrap().iter().all(|row| row[0] == 8));

    // Lonely operators: as a first child or at the outermost level they are
    // rejected, mirroring TF.
    assert!(search.search("sentence\n  <:\n  word", None).is_err());
    assert!(search.search("<: word", None).is_err());
}

#[test]
fn mapped_compiled_corpus_serializes_structure_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let source = structured_mini_corpus(&temp_dir);
    let cache_path = temp_dir.path().join("structured.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();
    let materialized = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let structure = context_fabric_core::precompute::structure(&materialized).unwrap();

    assert!(mapped.metadata().structure_start.is_some());
    assert_eq!(mapped.top().unwrap(), materialized.top());
    assert_eq!(
        mapped.heading_from_node(7).unwrap(),
        materialized.heading_from_node(7)
    );
    assert_eq!(
        mapped.node_from_heading(&structure.heading_from_node[&7]).unwrap(),
        materialized.node_from_heading(&structure.heading_from_node[&7])
    );
    assert_eq!(
        mapped.structure(Some(8)).unwrap(),
        materialized.structure(Some(8))
    );
    assert_eq!(
        mapped.structure_pretty(Some(8), false).unwrap(),
        materialized.structure_pretty(Some(8), false)
    );
    assert_eq!(
        mapped.structure_pretty(Some(7), true).unwrap(),
        materialized.structure_pretty(Some(7), true)
    );
}

fn bhsa_tf() -> Option<PathBuf> {
    optional_corpus_tf("bhsa")
}

fn write_numpy_u8(path: &std::path::Path, values: &[u8]) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"\x93NUMPY");
    bytes.extend_from_slice(&[1, 0]);

    let mut header = format!(
        "{{'descr': '|u1', 'fortran_order': False, 'shape': ({},), }}",
        values.len()
    );
    let preamble_len = bytes.len() + 2;
    let padding = (16 - ((preamble_len + header.len() + 1) % 16)) % 16;
    header.push_str(&" ".repeat(padding));
    header.push('\n');
    bytes.extend_from_slice(&(header.len() as u16).to_le_bytes());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(values);

    fs::write(path, bytes).unwrap();
}

#[test]
fn public_config_constants_match_python_config_api_values() {
    assert_eq!(VERSION, "0.6.0rc1");
    assert_eq!(CF_VERSION, VERSION);
    assert_eq!(__version__, VERSION);
    assert_eq!(NAME, "Context-Fabric");
    assert_eq!(BANNER, "This is Context-Fabric 0.6.0rc1");
    assert_eq!(API_VERSION, 3);

    assert_eq!(OTYPE, "otype");
    assert_eq!(OSLOTS, "oslots");
    assert_eq!(OTEXT, "otext");
    assert_eq!(OVOLUME, "ovolume");
    assert_eq!(OWORK, "owork");
    assert_eq!(OINTERF, "ointerfrom");
    assert_eq!(OINTERT, "ointerto");
    assert_eq!(OMAP, "omap");
    assert_eq!(WARP, &[OTYPE, OSLOTS, OTEXT]);

    assert_eq!(ORG, "codykingham");
    assert_eq!(REPO, "context-fabric");
    assert_eq!(RELATIVE, "tf");
    assert_eq!(GH, "github");
    assert_eq!(GL, "gitlab");
    assert_eq!(URL_GH, "https://github.com");
    assert_eq!(URL_GH_API, "https://api.github.com");
    assert_eq!(URL_GH_UPLOAD, "https://uploads.github.com");
    assert_eq!(URL_GL, "https://gitlab.com");
    assert_eq!(URL_GL_API, "https://api.gitlab.com");
    assert_eq!(URL_GL_UPLOAD, "https://uploads.gitlab.com");
    assert_eq!(URL_NB, "https://nbviewer.jupyter.org");
    assert_eq!(
        URL_CF_DOCS,
        "https://github.com/Context-Fabric/context-fabric"
    );
    assert_eq!(PROTOCOL, "http://");
    assert_eq!(HOST, "localhost");
    assert_eq!(PORT_BASE, 10000);
    assert_eq!(DOI_DEFAULT, "no DOI");
    assert_eq!(DOI_URL_PREFIX, "https://doi.org");
    assert_eq!(BRANCH_DEFAULT, "master");
    assert_eq!(BRANCH_DEFAULT_NEW, "main");

    assert_eq!(YARN_RATIO, 1.25);
    assert_eq!(TRY_LIMIT_FROM, 40);
    assert_eq!(TRY_LIMIT_TO, 40);
    assert_eq!(SEARCH_FAIL_FACTOR, 4);
    assert_eq!(CFM_VERSION, "2");
    assert_eq!(NODE_DTYPE, "uint32");
    assert_eq!(RANK_DTYPE, "uint32");
    assert_eq!(INDEX_DTYPE, "uint32");
    assert_eq!(TYPE_DTYPE, "uint8");
    assert_eq!(MISSING_INT, -1);
    assert_eq!(CONFIG_MISSING_STR_INDEX, 0xFFFF_FFFF);
}

#[test]
fn public_types_module_aliases_match_python_types_surface() {
    let node: context_fabric_core::types::Node = 7;
    let feature_value: context_fabric_core::types::FeatureValue =
        context_fabric_core::types::FeatureValue::string("typed");
    let slot_range: context_fabric_core::types::SlotRange = (1, node);
    let node_array: context_fabric_core::types::NodeArray = vec![1, 2, node];
    let index_array: context_fabric_core::types::IndexArray = vec![0, 1, 2];
    let offset_array: context_fabric_core::types::OffsetArray = vec![0, 3, 6];
    let search_result: context_fabric_core::types::SearchResult = vec![node];
    let section_spec: context_fabric_core::types::SectionSpec =
        vec!["book".to_string(), "chapter".to_string()];

    let mut metadata: context_fabric_core::types::MetaData = BTreeMap::new();
    metadata.insert("valueType".to_string(), Some("str".to_string()));
    let mut feature_metadata: context_fabric_core::types::FeatureMetaData = BTreeMap::new();
    feature_metadata.insert("word".to_string(), metadata.clone());

    let mut node_feature_data: context_fabric_core::types::NodeFeatureData = BTreeMap::new();
    node_feature_data.insert(node, FeatureValue::string("hello"));

    let mut edge_feature_data: context_fabric_core::types::EdgeFeatureData = BTreeMap::new();
    edge_feature_data.insert(node, BTreeSet::from([1, 2, 3]));
    let mut edge_feature_value_data: context_fabric_core::types::EdgeFeatureValueData =
        BTreeMap::new();
    edge_feature_value_data.insert(
        node,
        BTreeMap::from([(1, FeatureValue::string("contains"))]),
    );

    let mut nodes_by_type: context_fabric_core::types::NodesByType = BTreeMap::new();
    nodes_by_type.insert("word".to_string(), node_array.clone());

    assert_eq!(slot_range, (1, 7));
    assert_eq!(feature_value.as_str(), Some("typed"));
    assert_eq!(index_array, vec![0, 1, 2]);
    assert_eq!(offset_array, vec![0, 3, 6]);
    assert_eq!(search_result, vec![7]);
    assert_eq!(section_spec[0], "book");
    assert_eq!(
        metadata.get("valueType").and_then(Option::as_deref),
        Some("str")
    );
    assert!(feature_metadata.contains_key("word"));
    assert_eq!(node_feature_data[&7].as_str(), Some("hello"));
    assert_eq!(edge_feature_data[&7], BTreeSet::from([1, 2, 3]));
    assert_eq!(edge_feature_value_data[&7][&1].as_str(), Some("contains"));
    assert_eq!(nodes_by_type["word"], vec![1, 2, 7]);
}

#[test]
fn public_api_aliases_match_python_cf_tf_behavior() {
    let fabric = Fabric::new(repo_path("libs/core/tests/fixtures/mini_corpus"));
    let api = Api::from_arc(std::sync::Arc::new(fabric), ["z_ignored", "a_ignored"]);

    assert!(api.cf_and_tf_are_same_object());
    assert!(std::sync::Arc::ptr_eq(&api.CF, &api.TF));
    assert!(std::sync::Arc::ptr_eq(&api.cf(), &api.tf()));
    assert_eq!(api.ignored, vec!["a_ignored", "z_ignored"]);

    assert_eq!(api.CF.path(), api.TF.path());
    assert!(
        api.CF
            .explore()
            .unwrap()
            .nodes
            .contains(&"word".to_string())
    );

    let refs = api_refs();
    assert!(refs.contains_key("CF"));
    assert!(refs.contains_key("TF"));
    assert_eq!(refs.get("CF"), refs.get("TF"));
}

#[test]
fn public_downloader_helpers_match_python_registry_and_path_behaviors() {
    assert!(list_corpora().is_empty());
    assert_eq!(
        resolve_corpus_id("researcher/cfabric-my-corpus").unwrap(),
        "researcher/cfabric-my-corpus"
    );
    let unknown = resolve_corpus_id("bhsa").unwrap_err().to_string();
    assert!(unknown.contains("Unknown corpus: bhsa"));
    assert!(unknown.contains("full HF repo ID"));

    let temp_dir = tempfile::tempdir().unwrap();
    let previous_cache = env::var("CFABRIC_CACHE").ok();
    unsafe {
        env::set_var("CFABRIC_CACHE", temp_dir.path());
    }
    assert_eq!(get_cache_dir(), temp_dir.path());
    unsafe {
        match previous_cache {
            Some(value) => env::set_var("CFABRIC_CACHE", value),
            None => env::remove_var("CFABRIC_CACHE"),
        }
    }
    let previous_xdg = env::var("XDG_CACHE_HOME").ok();
    unsafe {
        env::remove_var("XDG_CACHE_HOME");
    }
    let default_cache = get_cache_dir();
    #[cfg(target_os = "macos")]
    assert!(default_cache.ends_with(PathBuf::from("Library").join("Caches").join("cfabric")));
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    assert!(default_cache.ends_with(PathBuf::from(".cache").join("cfabric")));
    unsafe {
        match previous_xdg {
            Some(value) => env::set_var("XDG_CACHE_HOME", value),
            None => env::remove_var("XDG_CACHE_HOME"),
        }
    }

    let request = context_fabric_core::build_download_request(
        "researcher/cfabric-my-corpus",
        Some("v1"),
        true,
        true,
    )
    .unwrap();
    assert_eq!(request.repo_id, "researcher/cfabric-my-corpus");
    assert_eq!(request.revision.as_deref(), Some("v1"));
    assert!(request.force);
    assert!(request.compiled_only);
    assert_eq!(
        request.allow_patterns,
        Some(vec![
            ".cfm/**".to_string(),
            "corpus_info.json".to_string(),
            "README.md".to_string(),
        ])
    );

    let request = context_fabric_core::build_download_request(
        "researcher/cfabric-my-corpus",
        None,
        false,
        false,
    )
    .unwrap();
    assert_eq!(request.allow_patterns, None);

    let download_error = download("researcher/cfabric-my-corpus", None, false, false)
        .unwrap_err()
        .to_string();
    assert!(download_error.contains("Rust downloader transport is not implemented yet"));
}

#[test]
fn public_search_syntax_constants_match_python_syntax_module() {
    assert_eq!(QWHERE, "/where/");
    assert_eq!(QHAVE, "/have/");
    assert_eq!(QWITHOUT, "/without/");
    assert_eq!(QWITH, "/with/");
    assert_eq!(QOR, "/or/");
    assert_eq!(QEND, "/-/");
    assert_eq!(PARENT_REF, "..");

    assert!(QINIT.contains(&QWHERE));
    assert!(QINIT.contains(&QWITHOUT));
    assert!(QINIT.contains(&QWITH));
    assert!(QCONT.contains(&QHAVE));
    assert!(QCONT.contains(&QOR));
    assert!(QTERM.contains(&QEND));

    assert!(ESCAPES.contains(&"\\\\"));
    assert!(ESCAPES.contains(&"\\ "));
    assert!(ESCAPES.contains(&"\\t"));
    assert!(ESCAPES.contains(&"\\n"));
    assert!(ESCAPES.contains(&"\\|"));
    assert!(ESCAPES.contains(&"\\="));
    assert!(VAL_ESCAPES.contains(&"\\|"));
    assert!(VAL_ESCAPES.contains(&"\\="));

    assert!(is_quantifier_init("/where/"));
    assert!(is_quantifier_init("/with/"));
    assert!(is_quantifier_init("/without/"));
    assert!(is_quantifier_continuation("/have/"));
    assert!(is_quantifier_continuation("/or/"));
    assert!(is_quantifier_terminator("/-/"));
    assert!(is_quantifier_line("  /where/  "));
    assert!(is_quantifier_line("/-/"));
    assert!(!is_quantifier_line("word"));

    assert!(is_search_white_line(""));
    assert!(is_search_white_line("   "));
    assert!(is_search_white_line("% this is a comment"));
    assert!(is_search_white_line("  % indented comment"));
    assert!(!is_search_white_line("word"));
}

#[test]
fn public_search_syntax_recognizers_match_python_regex_examples() {
    let atom_match = atomRe.captures("  word pos=noun").unwrap();
    assert_eq!(atom_match.get(1).unwrap().as_str(), "  ");
    assert_eq!(atom_match.get(2).unwrap().as_str(), "word");
    assert_eq!(atom_match.get(3).unwrap().as_str(), "pos=noun");

    let atom_op_match = atomOpRe.captures("  << phrase").unwrap();
    assert_eq!(atom_op_match.get(2).unwrap().as_str(), "<<");
    assert_eq!(atom_op_match.get(3).unwrap().as_str(), "phrase");

    let ident_match = identRe.captures("pos=noun").unwrap();
    assert_eq!(ident_match.get(1).unwrap().as_str(), "pos");
    assert_eq!(ident_match.get(2).unwrap().as_str(), "=");
    assert_eq!(ident_match.get(3).unwrap().as_str(), "noun");
    assert_eq!(
        compRe
            .captures("chapter<10")
            .unwrap()
            .get(2)
            .unwrap()
            .as_str(),
        "<"
    );
    assert_eq!(
        indentLineRe
            .captures("  word")
            .unwrap()
            .get(1)
            .unwrap()
            .as_str(),
        "  "
    );
    assert_eq!(kRe.captures(":10:").unwrap().get(2).unwrap().as_str(), "10");
    assert!(nameRe.is_match("half-verse"));
    assert!(!nameRe.is_match("word type"));
    assert_eq!(
        namesRe
            .captures("first_word:word")
            .unwrap()
            .get(1)
            .unwrap()
            .as_str(),
        "first_word"
    );
    assert!(numRe.is_match("-42"));
    assert_eq!(
        noneRe.captures("pos#").unwrap().get(2).unwrap().as_str(),
        "#"
    );
    assert_eq!(
        trueRe.captures("gloss*").unwrap().get(1).unwrap().as_str(),
        "gloss"
    );
    assert_eq!(
        opLineRe
            .captures("  -parent>")
            .unwrap()
            .get(2)
            .unwrap()
            .as_str(),
        "-parent>"
    );
    assert_eq!(
        opStripRe
            .captures("  < word pos=noun")
            .unwrap()
            .get(1)
            .unwrap()
            .as_str(),
        "word pos=noun"
    );
    assert_eq!(
        quLineRe
            .captures("  /where/")
            .unwrap()
            .get(2)
            .unwrap()
            .as_str(),
        "/where/"
    );
    assert_eq!(
        relRe.captures("a << b").unwrap().get(3).unwrap().as_str(),
        "<<"
    );
    assert_eq!(
        reRe.captures("word~^h").unwrap().get(2).unwrap().as_str(),
        "^h"
    );
    assert!(whiteRe.is_match("  % comment"));

    let atom = parse_atom_syntax("  word pos=noun number=sg").unwrap();
    assert_eq!(atom.indent, "  ");
    assert_eq!(atom.node_type, "word");
    assert_eq!(atom.features.as_deref(), Some("pos=noun number=sg"));
    assert!(parse_atom_syntax("word=type").is_none());

    let atom_op = parse_atom_operator_syntax("  << phrase").unwrap();
    assert_eq!(atom_op.indent, "  ");
    assert_eq!(atom_op.operator, "<<");
    assert_eq!(atom_op.node_type, "phrase");
    assert_eq!(
        parse_atom_operator_syntax("  [[ word").unwrap().operator,
        "[["
    );
    assert_eq!(
        parse_atom_operator_syntax("  ]] phrase").unwrap().operator,
        "]]"
    );
    assert_eq!(
        parse_atom_operator_syntax("  <: word").unwrap().operator,
        "<:"
    );
    assert_eq!(
        parse_atom_operator_syntax("  -parent> phrase")
            .unwrap()
            .operator,
        "-parent>"
    );

    let ident = parse_ident_syntax("word_type=common_noun").unwrap();
    assert_eq!(ident.feature, "word_type");
    assert_eq!(ident.operator, "=");
    assert_eq!(ident.value, "common_noun");
    assert_eq!(parse_ident_syntax("pos#noun").unwrap().operator, "#");

    let comparison = parse_comparison_syntax("chapter<10").unwrap();
    assert_eq!(comparison.feature, "chapter");
    assert_eq!(comparison.operator, "<");
    assert_eq!(comparison.value, "10");
    assert_eq!(parse_comparison_syntax("verse>5").unwrap().operator, ">");

    assert!(is_search_name("word"));
    assert!(is_search_name("half-verse"));
    assert!(is_search_name("word.type"));
    assert!(!is_search_name("word type"));
    assert!(!is_search_name("word=type"));
    assert!(is_search_number("123"));
    assert!(is_search_number("0"));
    assert!(is_search_number("-42"));
    assert!(!is_search_number("3.14"));
    assert!(!is_search_number("abc"));

    let none = parse_none_syntax("pos#").unwrap();
    assert_eq!(none.feature, "pos");
    assert_eq!(none.marker.as_deref(), Some("#"));
    assert_eq!(parse_none_syntax("pos").unwrap().feature, "pos");
    assert_eq!(parse_true_syntax("gloss*").as_deref(), Some("gloss"));

    let relation = parse_relation_syntax("  word < clause").unwrap();
    assert_eq!(relation.indent, "  ");
    assert_eq!(relation.left, "word");
    assert_eq!(relation.operator, "<");
    assert_eq!(relation.right, "clause");
    assert_eq!(parse_relation_syntax("a << b").unwrap().operator, "<<");
    assert_eq!(parse_relation_syntax("a == b").unwrap().operator, "==");
    assert_eq!(parse_relation_syntax("a ## b").unwrap().operator, "##");
    assert_eq!(parse_relation_syntax("a && b").unwrap().operator, "&&");
    assert_eq!(parse_relation_syntax("a || b").unwrap().operator, "||");
    assert_eq!(
        parse_relation_syntax("phrase [[ word").unwrap().operator,
        "[["
    );
    assert_eq!(
        parse_relation_syntax("word ]] phrase").unwrap().operator,
        "]]"
    );
    assert_eq!(parse_relation_syntax("a >> b").unwrap().operator, ">>");
    assert_eq!(parse_relation_syntax("a <: b").unwrap().operator, "<:");
    assert_eq!(parse_relation_syntax("a :> b").unwrap().operator, ":>");
    assert_eq!(parse_relation_syntax("a =: b").unwrap().operator, "=:");
    assert_eq!(parse_relation_syntax("a := b").unwrap().operator, ":=");
    assert_eq!(
        parse_relation_syntax("w -parent> p").unwrap().operator,
        "-parent>"
    );
    assert_eq!(
        parse_relation_syntax("p <parent- w").unwrap().operator,
        "<parent-"
    );
    assert_eq!(
        parse_relation_syntax("a <link> b").unwrap().operator,
        "<link>"
    );
    assert_eq!(
        parse_relation_syntax("w -parent=1> p").unwrap().operator,
        "-parent=1>"
    );
    assert_eq!(
        parse_relation_syntax("a .pos. b").unwrap().operator,
        ".pos."
    );
    assert_eq!(
        parse_relation_syntax("a .pos=gender. b").unwrap().operator,
        ".pos=gender."
    );
    assert_eq!(
        parse_relation_syntax("a .pos#type. b").unwrap().operator,
        ".pos#type."
    );
    assert_eq!(
        parse_relation_syntax("a .chapter<verse. b")
            .unwrap()
            .operator,
        ".chapter<verse."
    );
    assert_eq!(
        parse_relation_syntax("a .verse>chapter. b")
            .unwrap()
            .operator,
        ".verse>chapter."
    );

    let quantifier = parse_quantifier_line_syntax("  /where/").unwrap();
    assert_eq!(quantifier.indent, "  ");
    assert_eq!(quantifier.quantifier, "/where/");
    assert!(parse_quantifier_line_syntax("word").is_none());

    let near = parse_k_nearness_syntax(":10:").unwrap();
    assert_eq!(near.before, ":");
    assert_eq!(near.k, 10);
    assert_eq!(near.after, ":");
    assert_eq!(parse_k_nearness_syntax("=5:").unwrap().before, "=");
    assert_eq!(parse_k_nearness_syntax(":2=").unwrap().after, "=");
    assert_eq!(parse_k_nearness_syntax("<4:").unwrap().before, "<");
    assert_eq!(parse_k_nearness_syntax(":1>").unwrap().after, ">");

    assert_eq!(parse_named_atom_prefix("  w:word").as_deref(), Some("w"));
    assert_eq!(
        parse_named_atom_prefix("first_word:word").as_deref(),
        Some("first_word")
    );
    let regex_feature = parse_regex_feature_syntax("word~^hello$").unwrap();
    assert_eq!(regex_feature.0, "word");
    assert_eq!(regex_feature.1, "^hello$");
    assert_eq!(
        parse_regex_feature_syntax("pos~[nv]oun").unwrap().1,
        "[nv]oun"
    );

    let op_line = parse_operator_line_syntax("  -parent>").unwrap();
    assert_eq!(op_line.0, "  ");
    assert_eq!(op_line.1, "-parent>");
    assert_eq!(
        strip_operator_syntax("  < word pos=noun").as_deref(),
        Some("word pos=noun")
    );
    assert_eq!(
        strip_operator_syntax("<< phrase").as_deref(),
        Some("phrase")
    );
    assert_eq!(search_line_indent("    content"), "    ");
    assert_eq!(search_line_indent("content"), "");
}

#[test]
fn public_precompute_helpers_match_corpus_levels_order_and_rank() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let node_features = NodeFeatures;
    let edge_features = EdgeFeatures;
    let computeds = Computeds;
    assert_eq!(node_features, NodeFeatures::default());
    assert_eq!(edge_features, EdgeFeatures::default());
    assert_eq!(computeds, Computeds::default());

    let otype: &OtypeFeature = corpus.node_feature("otype").unwrap();
    let oslots: &OslotsFeature = corpus.edge_feature("oslots").unwrap();

    let node_types = otype
        .items()
        .into_iter()
        .filter_map(|(node, value)| value.as_str().map(|value| (node, value.to_string())))
        .collect::<BTreeMap<_, _>>();
    let slot_sets = oslots
        .items()
        .into_iter()
        .filter(|(node, _)| *node > corpus.max_slot())
        .collect::<BTreeMap<_, _>>();

    let levels = context_fabric_core::precompute::levels(
        &node_types,
        &slot_sets,
        corpus.max_slot(),
        corpus.slot_type(),
        None,
        None,
    );
    assert_eq!(
        levels,
        corpus
            .levels()
            .into_iter()
            .map(|(node_type, average, first, last)| {
                (node_type.to_string(), average, first, last)
            })
            .collect::<Vec<_>>()
    );

    let order = context_fabric_core::precompute::order(
        &node_types,
        &slot_sets,
        &levels,
        corpus.max_slot(),
        corpus.max_node(),
        corpus.slot_type(),
    );
    assert_eq!(order, corpus.order());
    let order_computed = OrderComputed::new(order.clone());
    let generic_order_computed = Computed::new(order.clone());
    assert_eq!(order_computed.data(), &order);
    assert_eq!(generic_order_computed.into_data(), order);

    let rank = context_fabric_core::precompute::rank(corpus.max_node(), &order);
    assert_eq!(rank, corpus.rank());
    let rank_computed = RankComputed::new(rank.clone());
    assert_eq!(&*rank_computed, &rank);

    let lev_up = context_fabric_core::precompute::lev_up(
        &slot_sets,
        &rank,
        corpus.max_slot(),
        corpus.max_node(),
    );
    assert_eq!(
        context_fabric_core::precompute::levUp(
            &slot_sets,
            &rank,
            corpus.max_slot(),
            corpus.max_node()
        ),
        lev_up
    );
    assert_eq!(lev_up[(1 - 1) as usize], vec![6, 8]);
    assert_eq!(lev_up[(6 - 1) as usize], vec![8]);
    assert_eq!(lev_up[(8 - 1) as usize], Vec::<u32>::new());
    let lev_up_computed = LevUpComputed::new(lev_up.clone());
    assert_eq!(lev_up_computed.data(), &lev_up);

    let lev_down = context_fabric_core::precompute::lev_down(
        &lev_up,
        &rank,
        corpus.max_slot(),
        corpus.max_node(),
    );
    assert_eq!(
        context_fabric_core::precompute::levDown(
            &lev_up,
            &rank,
            corpus.max_slot(),
            corpus.max_node()
        ),
        lev_down
    );
    assert_eq!(
        lev_down[(6 - corpus.max_slot() - 1) as usize],
        Vec::<u32>::new()
    );
    assert_eq!(lev_down[(8 - corpus.max_slot() - 1) as usize], vec![6, 7]);
    let lev_down_computed = LevDownComputed::new(lev_down.clone());
    assert_eq!(lev_down_computed.data(), &lev_down);

    let boundary = context_fabric_core::precompute::boundary(&slot_sets, &rank, corpus.max_slot());
    assert_eq!(boundary, corpus.boundary());
    assert_eq!(boundary.first_slots[0], vec![6, 8]);
    assert_eq!(boundary.last_slots[4], vec![8, 7]);

    let sections = context_fabric_core::precompute::sections(&corpus).unwrap();
    assert_eq!(
        context_fabric_core::precompute::sectionsFromApi(&corpus).unwrap(),
        sections
    );
    assert_eq!(sections.sec1[&8]["1"], 6);
    assert_eq!(sections.sec1[&8]["2"], 7);
    assert!(sections.sec2.is_empty());
    assert!(sections.seq_from_node.is_empty());
    assert!(sections.node_from_seq.is_empty());

    assert!(context_fabric_core::precompute::structure(&corpus).is_none());

    let temp_dir = tempfile::tempdir().unwrap();
    let structure_source = temp_dir.path().join("structured");
    dirCopy(
        &repo_path("libs/core/tests/fixtures/mini_corpus").to_string_lossy(),
        &structure_source.to_string_lossy(),
        false,
    )
    .unwrap();
    let otext_path = structure_source.join("otext.tf");
    let mut otext = fs::read_to_string(&otext_path).unwrap();
    otext = otext.replace(
        "@structureFeatures=",
        "@structureFeatures=sentence_id,phrase_id",
    );
    otext = otext.replace("@structureTypes=", "@structureTypes=sentence,phrase");
    fs::write(&otext_path, otext).unwrap();
    let structured = Corpus::load(&structure_source).unwrap();
    let structure = context_fabric_core::precompute::structure(&structured).unwrap();
    assert_eq!(structure.top, vec![8]);
    assert_eq!(structure.up[&6], 8);
    assert_eq!(structure.up[&7], 8);
    assert_eq!(structure.down[&8], vec![6, 7]);
    assert_eq!(
        structure.heading_from_node[&6],
        vec![
            context_fabric_core::StructureHeading {
                node_type: "sentence".to_string(),
                heading: "S1".to_string()
            },
            context_fabric_core::StructureHeading {
                node_type: "phrase".to_string(),
                heading: "1".to_string()
            },
        ]
    );
    assert_eq!(
        structure.node_from_heading[&vec![
            context_fabric_core::StructureHeading {
                node_type: "sentence".to_string(),
                heading: "S1".to_string()
            },
            context_fabric_core::StructureHeading {
                node_type: "phrase".to_string(),
                heading: "2".to_string()
            },
        ]],
        7
    );
    assert!(structure.multiple.is_empty());
    assert_eq!(structured.top(), Some(vec![8]));
    assert_eq!(structured.structure_up(6), Some(8));
    assert_eq!(structured.structure_down(8), Some(vec![6, 7]));
    assert_eq!(
        structured.heading_from_node(7),
        structure.heading_from_node.get(&7).cloned()
    );
    assert_eq!(
        structured.node_from_heading(&structure.heading_from_node[&7]),
        Some(7)
    );
    assert_eq!(
        structured.structure_info().unwrap().headings,
        vec![
            ("sentence".to_string(), "sentence_id".to_string()),
            ("phrase".to_string(), "phrase_id".to_string()),
        ]
    );
    assert_eq!(
        structured.structure(Some(8)),
        Some(context_fabric_core::StructureTree::Node {
            node: 8,
            children: vec![
                context_fabric_core::StructureTree::Node {
                    node: 6,
                    children: Vec::new()
                },
                context_fabric_core::StructureTree::Node {
                    node: 7,
                    children: Vec::new()
                },
            ]
        })
    );
    assert_eq!(
        structured.structure_pretty(Some(8), false).unwrap(),
        "  sentence:S1\n      phrase:1\n      phrase:2"
    );
    assert_eq!(
        structured.structure_pretty(Some(7), true).unwrap(),
        "  sentence:S1-phrase:2"
    );
    let text_api = Text::new(&structured);
    assert_eq!(text_api.top(), structured.top());
    assert_eq!(text_api.up(7), Some(8));
    assert_eq!(text_api.down(8), Some(vec![6, 7]));
    assert_eq!(text_api.headingFromNode(6), structured.heading_from_node(6));
    assert_eq!(
        text_api.nodeFromHeading(&structure.heading_from_node[&6]),
        Some(6)
    );

    let configured = context_fabric_core::precompute::levels(
        &node_types,
        &slot_sets,
        corpus.max_slot(),
        corpus.slot_type(),
        Some("phrase,sentence,word"),
        None,
    );
    assert_eq!(configured[0].0, "phrase");
    assert_eq!(configured[1].0, "sentence");
    assert_eq!(configured[2].0, "word");

    let character_counts = context_fabric_core::precompute::characters(
        &BTreeMap::from([
            (
                "combo".to_string(),
                vec!["word".to_string(), "translit".to_string()],
            ),
            ("plain".to_string(), vec!["word".to_string()]),
        ]),
        &BTreeMap::from([
            (
                "word".to_string(),
                BTreeMap::from([(1, "aba".to_string()), (2, "β".to_string())]),
            ),
            (
                "translit".to_string(),
                BTreeMap::from([(1, "A".to_string()), (2, "ba".to_string())]),
            ),
        ]),
    );
    assert_eq!(
        character_counts["plain"],
        vec![
            ("a".to_string(), 2),
            ("b".to_string(), 1),
            ("β".to_string(), 1),
        ]
    );
    assert_eq!(
        character_counts["combo"],
        vec![
            ("A".to_string(), 1),
            ("a".to_string(), 3),
            ("b".to_string(), 2),
            ("β".to_string(), 1),
        ]
    );
}

#[test]
fn public_describe_free_functions_match_python_describe_module_behavior() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();

    let overview = describe_corpus_overview(&corpus, "mini");
    assert_eq!(overview, corpus.overview("mini"));
    assert_eq!(overview.name, "mini");
    assert!(
        overview
            .node_types
            .iter()
            .any(|node_type| node_type.node_type == "word" && node_type.count > 0)
    );
    serde_json::to_string(&overview.to_dict()).unwrap();

    let description = describe_corpus(&corpus, "mini");
    assert_eq!(description, corpus.describe_corpus("mini"));
    assert!(!description.node_features.is_empty());
    assert_eq!(
        description.text_representations,
        describe_text_formats(&corpus)
    );

    let all_features = list_features(&corpus, None, None);
    assert!(!all_features.is_empty());
    assert_eq!(all_features, corpus.feature_catalog(None, None));
    assert!(all_features.iter().all(|feature| !feature.name.is_empty()));

    let node_features = list_features(&corpus, Some(FeatureKind::Node), None);
    assert!(
        node_features
            .iter()
            .all(|feature| feature.kind == FeatureKind::Node)
    );
    let edge_features = list_features(&corpus, Some(FeatureKind::Edge), None);
    assert!(
        edge_features
            .iter()
            .all(|feature| feature.kind == FeatureKind::Edge)
    );
    assert!(!list_features(&corpus, None, Some(&["word"])).is_empty());

    let pos_description = describe_feature(&corpus, "pos", 5);
    assert_eq!(pos_description, corpus.describe_feature("pos", 5));
    assert_eq!(pos_description.name, "pos");
    assert_eq!(pos_description.kind, Some(FeatureKind::Node));
    assert!(!pos_description.node_types.is_empty());
    assert!(pos_description.sample_values.len() <= 5);

    let missing_description = describe_feature(&corpus, "nonexistent_feature_xyz", 5);
    assert!(missing_description.error.is_some());

    let batch = describe_features(&corpus, &["word", "parent"], 2);
    assert_eq!(batch, corpus.describe_features(&["word", "parent"], 2));
    assert_eq!(batch.len(), 2);

    assert_eq!(
        get_feature_otypes(&corpus, "word"),
        vec!["word".to_string()]
    );
    assert_eq!(
        get_feature_otypes(&corpus, "nonexistent_xyz"),
        Vec::<String>::new()
    );
    let all_feature_otypes = get_all_feature_otypes(&corpus);
    assert_eq!(all_feature_otypes, corpus.all_node_feature_types());
    assert!(all_feature_otypes.contains_key("word"));
}

#[test]
fn public_text_escape_helpers_match_python_utility_behaviors() {
    assert!(is_int("42"));
    assert!(is_int("-123"));
    assert!(is_int("0"));
    assert!(!is_int("3.14"));
    assert!(!is_int("hello"));
    assert!(!is_int(""));
    assert_eq!(isInt("42"), is_int("42"));
    assert_eq!(isInt("3.14"), is_int("3.14"));

    assert_eq!(math_esc(Some("$100")), "<span>$</span>100");
    assert_eq!(mathEsc(Some("$100")), math_esc(Some("$100")));
    assert_eq!(math_esc(Some("cost: $50")), "cost: <span>$</span>50");
    assert_eq!(
        math_esc(Some("$a + $b")),
        "<span>$</span>a + <span>$</span>b"
    );
    assert_eq!(math_esc(None), "");
    assert_eq!(math_esc(Some("hello world")), "hello world");

    let markdown = md_esc(Some("*bold* and _italic_"), false);
    assert!(markdown.contains("&#42;"));
    assert!(markdown.contains("&#95;"));
    assert_eq!(
        mdEsc(Some("*bold* and _italic_"), false),
        md_esc(Some("*bold* and _italic_"), false)
    );
    assert!(md_esc(Some("[link]"), false).contains("&#91;"));
    assert_eq!(md_esc(None, false), "");
    assert!(md_esc(Some("$x$"), true).contains('$'));
    assert!(!md_esc(Some("$x$"), true).contains("<span>"));
    assert!(md_esc(Some("$x$"), false).contains("<span>$</span>"));

    assert_eq!(html_esc(Some("A & B"), false), "A &amp; B");
    assert_eq!(
        htmlEsc(Some("A & B"), false),
        html_esc(Some("A & B"), false)
    );
    assert_eq!(html_esc(Some("<tag>"), false), "&lt;tag&gt;");
    assert_eq!(html_esc(None, false), "");
    assert!(html_esc(Some("$x$"), true).contains('$'));
    let html = html_esc(Some("<a & b>"), false);
    assert!(html.contains("&lt;"));
    assert!(html.contains("&amp;"));
    assert!(html.contains("&gt;"));

    assert!(xml_esc(Some("it's")).contains("&apos;"));
    assert_eq!(xmlEsc(Some("it's")), xml_esc(Some("it's")));
    assert!(xml_esc(Some("say \"hello\"")).contains("&quot;"));
    let xml = xml_esc(Some("<tag & attr='val'>"));
    assert!(xml.contains("&lt;"));
    assert!(xml.contains("&amp;"));
    assert!(xml.contains("&apos;"));
    assert!(xml.contains("&gt;"));
    assert_eq!(xml_esc(None), "");

    assert!(mdhtml_esc(Some("a | b"), false).contains("&#124;"));
    assert_eq!(
        mdhtmlEsc(Some("a | b"), false),
        mdhtml_esc(Some("a | b"), false)
    );
    let mdhtml = mdhtml_esc(Some("<div>"), false);
    assert!(mdhtml.contains("&lt;"));
    assert!(mdhtml.contains("&gt;"));
    assert_eq!(mdhtml_esc(None, false), "");

    assert_eq!(tsv_esc("\"hello"), "\\\"hello");
    assert_eq!(tsvEsc("\"hello"), tsv_esc("\"hello"));
    assert_eq!(tsv_esc("'hello"), "\\'hello");
    assert_eq!(tsv_esc("say \"hi\""), "say \"hi\"");
    assert_eq!(tsv_esc(""), "");
    assert_eq!(tsv_esc("hello"), "hello");

    assert_eq!(pandas_esc("a\tb"), "a b");
    assert_eq!(pandasEsc("a\tb"), pandas_esc("a\tb"));
    assert!(pandas_esc("say \"hi\"").contains('\u{1}'));
    assert_eq!(pandas_esc(""), "");
}

#[test]
fn public_small_helpers_match_python_utility_behaviors() {
    let before = SystemTime::now();
    let observed = utcnow();
    let after = SystemTime::now();
    assert!(observed >= before);
    assert!(observed <= after);

    unsafe {
        env::set_var("TEST_VAR_CF_RUST", "test_value");
    }
    assert_eq!(var("TEST_VAR_CF_RUST").as_deref(), Some("test_value"));
    unsafe {
        env::remove_var("TEST_VAR_CF_RUST");
    }
    assert_eq!(var("DEFINITELY_NOT_A_REAL_ENV_VAR_XYZ"), None);

    let (on32, warn, msg) = check32();
    assert_eq!(on32, usize::BITS < 64);
    assert_eq!(warn.is_empty(), !on32);
    assert_eq!(msg.is_empty(), on32);
    if on32 {
        assert_eq!(warn, WARN32);
    } else {
        assert_eq!(msg, MSG64);
    }
}

#[test]
fn public_attr_helpers_match_python_utility_behaviors() {
    let mut attr = AttrDict::new(BTreeMap::from([
        ("a".to_string(), serde_json::json!(1)),
        ("b".to_string(), serde_json::json!(2)),
    ]));
    assert_eq!(attr.get("a"), Some(&serde_json::json!(1)));
    assert_eq!(attr.get("missing"), None);

    attr.set("name", serde_json::json!("test"));
    assert_eq!(attr.get("name"), Some(&serde_json::json!("test")));
    attr.update(BTreeMap::from([(
        "value".to_string(),
        serde_json::json!(42),
    )]));
    assert_eq!(attr.get("value"), Some(&serde_json::json!(42)));
    assert_eq!(attr.len(), 4);
    assert!(AttrDict::empty().is_empty());

    let keys = attr.keys().cloned().collect::<Vec<_>>();
    assert!(keys.contains(&"a".to_string()));
    assert!(keys.contains(&"b".to_string()));
    let items = attr
        .items()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(items["name"], serde_json::json!("test"));

    let nested = serde_json::json!({
        "outer": {"inner": 1},
        "items": [{"x": 1}, {"y": 2}],
        "atomic": true,
        "none": null
    });
    assert_eq!(deepdict(&nested), nested);
    assert_eq!(deep_attr_dict(&nested, false), nested);
    assert_eq!(deep_attr_dict(&nested, true), nested);
    assert_eq!(
        AttrDict::new(BTreeMap::from([("nested".to_string(), nested.clone())])).deepdict(),
        serde_json::json!({"nested": nested})
    );

    assert!(is_iterable(&serde_json::json!([1, 2, 3])));
    assert!(is_iterable(&serde_json::json!({"a": 1})));
    assert!(!is_iterable(&serde_json::json!("hello")));
    assert!(!is_iterable(&serde_json::json!(42)));
    assert!(!is_iterable(&serde_json::Value::Null));
}

#[test]
fn public_storage_helpers_match_python_string_pool_behaviors() {
    assert_eq!(MISSING_STR_INDEX, usize::MAX);

    let pool = StringPool::from_dict(
        &BTreeMap::from([
            (1, "hello".to_string()),
            (3, "world".to_string()),
            (5, "hello".to_string()),
        ]),
        6,
    );
    assert_eq!(pool.get(1), Some("hello"));
    assert_eq!(pool.get(2), None);
    assert_eq!(pool.get(3), Some("world"));
    assert_eq!(pool.get(5), Some("hello"));
    assert_eq!(pool.get(0), None);
    assert_eq!(pool.get(-1), None);
    assert_eq!(pool.get(7), None);
    assert_eq!(pool.strings.len(), 2);
    assert_ne!(pool.get_value_index("hello"), pool.get_value_index("world"));
    assert_eq!(pool.get_value_index("missing"), None);
    assert_eq!(
        pool.filter_by_value(&[1, 2, 3, 4, 5, 6], "hello"),
        vec![1, 5]
    );
    assert_eq!(
        pool.filter_by_values(
            &[1, 2, 3, 4, 5],
            &BTreeSet::from(["hello".to_string(), "world".to_string()])
        ),
        vec![1, 3, 5]
    );
    assert!(
        pool.filter_by_values(&[1, 2], &BTreeSet::<String>::new())
            .is_empty()
    );

    let temp_dir = tempfile::tempdir().unwrap();
    let pool_path = temp_dir.path().join("pool.json");
    pool.save(&pool_path).unwrap();
    let loaded_pool = StringPool::load(&pool_path).unwrap();
    assert_eq!(loaded_pool.get(1), Some("hello"));
    assert_eq!(loaded_pool.get(2), None);
    assert_eq!(loaded_pool.get(3), Some("world"));

    let ints = IntFeatureArray::from_dict(&BTreeMap::from([(1, 10), (3, 30), (5, 50)]), 6);
    assert_eq!(ints.get(1), Some(10));
    assert_eq!(ints.get(2), None);
    assert_eq!(ints.get(3), Some(30));
    assert_eq!(ints.get(0), None);
    assert_eq!(ints.get(-1), None);
    assert_eq!(ints.get(7), None);
    assert_eq!(ints.filter_by_value(&[1, 2, 3, 4, 5], 10), vec![1]);
    assert_eq!(
        ints.filter_by_values(&[1, 2, 3, 4, 5], &BTreeSet::from([10, 50])),
        vec![1, 5]
    );
    assert_eq!(ints.filter_less_than(&[1, 3, 5], 50), vec![1, 3]);
    assert_eq!(ints.filter_greater_than(&[1, 3, 5], 10), vec![3, 5]);
    assert_eq!(ints.filter_has_value(&[1, 2, 3, 4, 5, 6]), vec![1, 3, 5]);
    assert_eq!(
        ints.filter_missing_value(&[1, 2, 3, 4, 5, 6]),
        vec![2, 4, 6]
    );

    let ints_path = temp_dir.path().join("ints.json");
    ints.save(&ints_path).unwrap();
    let loaded_ints = IntFeatureArray::load(&ints_path).unwrap();
    assert_eq!(loaded_ints.get(1), Some(10));
    assert_eq!(loaded_ints.get(2), None);
    assert_eq!(loaded_ints.get(5), Some(50));
}

#[test]
fn public_csr_storage_helpers_match_python_storage_behaviors() {
    let mut csr = CSRArray::from_sequences(&[vec![1, 2, 3], Vec::new(), vec![2, 4]]);
    assert_eq!(csr.len(), 3);
    assert_eq!(csr.row(0).unwrap(), &[1, 2, 3]);
    assert_eq!(csr.row(1).unwrap(), &[] as &[u32]);
    assert_eq!(csr.get_as_tuple(0), vec![1, 2, 3]);
    assert_eq!(csr.get_as_tuple(99), Vec::<u32>::new());

    assert_eq!(csr.get_all_targets([1, 3]), BTreeSet::from([1, 2, 3, 4]));
    assert_eq!(csr.get_all_targets([1, 100, -1]), BTreeSet::from([1, 2, 3]));
    assert_eq!(csr.get_all_targets(Vec::<i64>::new()), BTreeSet::new());
    assert_eq!(
        csr.filter_sources_with_targets_in([1, 2, 3], [2, 4]),
        (BTreeSet::from([1, 3]), BTreeSet::from([2, 4]))
    );
    assert_eq!(
        csr.filter_sources_with_targets_in([1, 2], [99]),
        (BTreeSet::new(), BTreeSet::new())
    );
    assert_eq!(
        csr.filter_sources_with_targets_in([1], Vec::<u32>::new()),
        (BTreeSet::new(), BTreeSet::new())
    );

    assert!(!csr.is_cached());
    assert_eq!(csr.memory_usage_bytes(), 0);
    csr.preload_to_ram();
    assert!(csr.is_cached());
    let memory = csr.memory_usage_bytes();
    assert!(memory > 0);
    csr.preload_to_ram();
    assert_eq!(csr.memory_usage_bytes(), memory);
    assert_eq!(csr.row(0).unwrap(), &[1, 2, 3]);
    csr.release_cache();
    assert!(!csr.is_cached());
    assert_eq!(csr.memory_usage_bytes(), 0);
    assert_eq!(csr.row(2).unwrap(), &[2, 4]);

    let temp_dir = tempfile::tempdir().unwrap();
    let csr_path = temp_dir.path().join("csr.json");
    csr.save(&csr_path).unwrap();
    let loaded = CSRArray::load(&csr_path).unwrap();
    assert_eq!(loaded.len(), 3);
    assert_eq!(loaded.row(0).unwrap(), &[1, 2, 3]);
    assert!(!loaded.is_cached());

    let valued = CSRArrayWithValues::from_dict_of_dicts(
        &BTreeMap::from([
            (
                0,
                BTreeMap::from([(10, CsrValue::Int(100)), (20, CsrValue::Int(200))]),
            ),
            (2, BTreeMap::from([(30, CsrValue::Int(300))])),
        ]),
        3,
    );
    let (indices, values) = valued.row(0).unwrap();
    assert_eq!(indices, vec![10, 20]);
    assert_eq!(values, vec![CsrValue::Int(100), CsrValue::Int(200)]);
    assert_eq!(valued.row(1).unwrap(), (Vec::new(), Vec::new()));
    assert_eq!(
        valued.get_as_dict(0),
        BTreeMap::from([(10, CsrValue::Int(100)), (20, CsrValue::Int(200))])
    );

    let valued_path = temp_dir.path().join("valued.json");
    valued.save(&valued_path).unwrap();
    let loaded_valued = CSRArrayWithValues::load(&valued_path).unwrap();
    assert_eq!(
        loaded_valued.get_as_dict(2),
        BTreeMap::from([(30, CsrValue::Int(300))])
    );
    let int_valued = CSRArrayWithValues::from_int_dict_of_dicts(
        &BTreeMap::from([
            (0, BTreeMap::from([(10, 100), (20, 200)])),
            (2, BTreeMap::from([(30, 300)])),
        ]),
        3,
    );
    assert_eq!(
        int_valued.get_as_dict(0),
        BTreeMap::from([(10, CsrValue::Int(100)), (20, CsrValue::Int(200))])
    );
    assert_eq!(int_valued.get_as_dict(1), BTreeMap::new());
    assert_eq!(
        int_valued.get_as_dict(2),
        BTreeMap::from([(30, CsrValue::Int(300))])
    );

    let string_valued = CSRArrayWithValues::from_dict_of_dicts(
        &BTreeMap::from([(
            0,
            BTreeMap::from([
                (10, CsrValue::Str("A0".to_string())),
                (20, CsrValue::Str("A1".to_string())),
            ]),
        )]),
        1,
    );
    let string_path = temp_dir.path().join("string_valued.json");
    string_valued.save(&string_path).unwrap();
    assert_eq!(
        CSRArrayWithValues::load(&string_path)
            .unwrap()
            .get_as_dict(0),
        BTreeMap::from([
            (10, CsrValue::Str("A0".to_string())),
            (20, CsrValue::Str("A1".to_string())),
        ])
    );
    let ergonomic_string_valued = CSRArrayWithValues::from_string_dict_of_dicts(
        &BTreeMap::from([(
            0,
            BTreeMap::from([(10, "A0".to_string()), (20, "A1".to_string())]),
        )]),
        1,
    );
    assert_eq!(
        ergonomic_string_valued.get_as_dict(0),
        string_valued.get_as_dict(0)
    );
}

#[test]
fn public_mmap_manager_matches_python_lazy_storage_behaviors() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cfm_path = temp_dir.path().join(".cfm").join("1");
    let warp_path = cfm_path.join("warp");
    fs::create_dir_all(&warp_path).unwrap();
    fs::write(
        cfm_path.join("meta.json"),
        serde_json::to_vec(&serde_json::json!({
            "max_slot": 5,
            "max_node": 8,
            "slot_type": "word",
            "node_types": ["word", "phrase", "sentence"],
        }))
        .unwrap(),
    )
    .unwrap();
    write_numpy_u8(&warp_path.join("otype.npy"), &[0, 0, 1]);
    fs::write(
        warp_path.join("otype_types.json"),
        serde_json::to_vec(&serde_json::json!(["word", "phrase", "sentence"])).unwrap(),
    )
    .unwrap();

    let mut manager = MmapManager::new(&cfm_path);
    assert!(manager.exists());
    assert_eq!(manager.max_slot().unwrap(), 5);
    assert_eq!(manager.max_node().unwrap(), 8);
    assert_eq!(manager.slot_type().unwrap().as_deref(), Some("word"));
    assert!(
        manager
            .node_types()
            .unwrap()
            .contains(&"phrase".to_string())
    );

    assert_eq!(manager.cached_arrays_count(), 0);
    let array = manager.get_array(&["warp", "otype"]).unwrap();
    assert_eq!(array.values(), &[0, 0, 1]);
    assert_eq!(array.shape(), &[3]);
    assert_eq!(manager.cached_arrays_count(), 1);
    assert_eq!(
        manager.get_json(&["warp", "otype_types"]).unwrap(),
        serde_json::json!(["word", "phrase", "sentence"])
    );

    manager.close();
    assert_eq!(manager.cached_arrays_count(), 0);
    assert_eq!(manager.max_slot().unwrap(), 5);

    let missing = MmapManager::new(cfm_path.parent().unwrap().join("nonexistent"));
    assert!(!missing.exists());
}

#[test]
fn public_misc_helpers_match_python_utility_behaviors() {
    let mut versions = vec!["1.0", "2.0", "1.1", "10.0"];
    versions.sort_by_key(|version| version_sort(version));
    assert_eq!(versions, vec!["1.0", "1.1", "2.0", "10.0"]);

    let mut versions = vec!["1.0a", "1.0b", "1.0", "2.0"];
    versions.sort_by_key(|version| versionSort(version));
    assert_eq!(versions[0], "1.0");
    assert!(versions.contains(&"2.0"));

    let mut versions = vec!["1.2.3", "1.2.10", "1.10.0", "2.0.0"];
    versions.sort_by_key(|version| version_sort(version));
    assert_eq!(versions, vec!["1.2.3", "1.2.10", "1.10.0", "2.0.0"]);

    assert_eq!(nbytes(100.0), "  100B");
    assert_eq!(nbytes(2048.0), "  2.0KB");
    assert_eq!(nbytes((2_u64 * 1024 * 1024) as f64), "  2.0MB");
    assert_eq!(nbytes((2_u64 * 1024 * 1024 * 1024) as f64), "  2.0GB");

    assert_eq!(console_message(&["hello", "world"], true), "hello world\n");
    assert_eq!(console_message(&["\nhello\n"], false), "hello");
    assert_eq!(console_message(&[], true), "\n");

    assert_eq!(itemize(Some("a b c"), None), vec!["a", "b", "c"]);
    assert_eq!(itemize(Some("a,b,c"), Some(",")), vec!["a", "b", "c"]);
    assert_eq!(itemize(Some(""), None), Vec::<String>::new());
    assert_eq!(itemize(None, None), Vec::<String>::new());

    assert_eq!(
        fitemize(Some(FitemizeValue::Str("a, b, c".to_string()))),
        vec!["a", "b", "c"]
    );
    assert_eq!(fitemize(Some(FitemizeValue::Int(42))), vec!["42"]);
    assert_eq!(fitemize(Some(FitemizeValue::Bool(true))), vec!["True"]);
    assert_eq!(
        fitemize(Some(FitemizeValue::Str(String::new()))),
        Vec::<String>::new()
    );
    assert_eq!(fitemize(None), Vec::<String>::new());

    assert_eq!(
        project(vec![vec![1, 2, 3], vec![4, 5, 6]], 1),
        Projection::Values(BTreeSet::from([1, 4]))
    );
    assert_eq!(
        project(vec![vec![1, 2, 3], vec![4, 5, 6]], 2),
        Projection::Tuples(BTreeSet::from([vec![1, 2], vec![4, 5]]))
    );

    let (formats, format_features) = collect_formats(&BTreeMap::from([
        (
            "fmt:text-orig-full".to_string(),
            "{word}{after}".to_string(),
        ),
        (
            "fmt:lex-default".to_string(),
            "{special/normal:.}\\t{missing:}".to_string(),
        ),
        ("sectionTypes".to_string(), "book,chapter".to_string()),
    ]));
    assert_eq!(
        format_features,
        vec![
            "after".to_string(),
            "missing".to_string(),
            "normal".to_string(),
            "special".to_string(),
            "word".to_string()
        ]
    );
    assert_eq!(formats["text-orig-full"].template, "{word}{after}");
    assert_eq!(formats["text-orig-full"].rendered_template, "{}{}");
    assert_eq!(
        formats["lex-default"].features,
        vec![
            (
                vec!["special".to_string(), "normal".to_string()],
                ".".to_string()
            ),
            (vec!["missing".to_string()], String::new()),
        ]
    );
    assert_eq!(
        collectFormats(&BTreeMap::new()),
        collect_formats(&BTreeMap::new())
    );

    assert_eq!(
        set_from_value(
            Some(SetValue::Set(BTreeSet::from([
                "1".to_string(),
                "2".to_string(),
                "3".to_string()
            ]))),
            false
        ),
        BTreeSet::from(["1".to_string(), "2".to_string(), "3".to_string()])
    );
    assert_eq!(
        set_from_value(Some(SetValue::Str("1 2 3".to_string())), true),
        BTreeSet::from(["1".to_string(), "2".to_string(), "3".to_string()])
    );
    assert_eq!(set_from_value(None, false), BTreeSet::<String>::new());
    assert_eq!(
        set_from_str(Some("a b c")),
        BTreeSet::from(["a".to_string(), "b".to_string(), "c".to_string()])
    );
    assert_eq!(set_from_str(None), BTreeSet::<String>::new());
    assert_eq!(
        flatten_to_set(&[
            FlattenFeatureSpec::Name("word".to_string()),
            FlattenFeatureSpec::Values(SetValue::Str("pos number".to_string())),
            FlattenFeatureSpec::Values(SetValue::Items(vec![
                "lex".to_string(),
                "word".to_string()
            ])),
        ]),
        BTreeSet::from([
            "lex".to_string(),
            "number".to_string(),
            "pos".to_string(),
            "word".to_string()
        ])
    );
    assert_eq!(
        flatten_to_set(&[FlattenFeatureSpec::Values(SetValue::Set(BTreeSet::from([
            "a".to_string(),
            "b".to_string()
        ])))]),
        BTreeSet::from(["a".to_string(), "b".to_string()])
    );

    let mut sets = BTreeMap::from([(1, BTreeSet::from([10, 20]))]);
    merge_dict_of_sets(
        &mut sets,
        &BTreeMap::from([(1, BTreeSet::from([20, 30])), (2, BTreeSet::from([40]))]),
    );
    assert_eq!(sets.get(&1), Some(&BTreeSet::from([10, 20, 30])));
    assert_eq!(sets.get(&2), Some(&BTreeSet::from([40])));

    let mut merged = BTreeMap::from([
        ("a".to_string(), serde_json::json!(1)),
        ("b".to_string(), serde_json::json!(2)),
    ]);
    merge_dict(
        &mut merged,
        &BTreeMap::from([
            ("b".to_string(), serde_json::json!(3)),
            ("c".to_string(), serde_json::json!(4)),
        ]),
    );
    assert_eq!(merged["a"], serde_json::json!(1));
    assert_eq!(merged["b"], serde_json::json!(3));
    assert_eq!(merged["c"], serde_json::json!(4));

    let mut nested = BTreeMap::from([("a".to_string(), serde_json::json!({"x": 1, "y": 2}))]);
    merge_dict(
        &mut nested,
        &BTreeMap::from([("a".to_string(), serde_json::json!({"y": 3, "z": 4}))]),
    );
    assert_eq!(nested["a"], serde_json::json!({"x": 1, "y": 3, "z": 4}));

    let formatted = format_meta(&BTreeMap::from([(
        "feat".to_string(),
        BTreeMap::from([
            ("desc".to_string(), "A feature".to_string()),
            ("eg".to_string(), "example".to_string()),
        ]),
    )]));
    assert_eq!(formatted["feat"]["description"], "A feature (example)");
    assert!(!formatted["feat"].contains_key("desc"));
    assert!(!formatted["feat"].contains_key("eg"));

    assert_eq!(make_examples(&[1, 2, 3]), "      3 x: 1, 2, 3");
    assert_eq!(
        make_examples(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
        "     11 x: 1, 2, 3, 4, 5 ... 7, 8, 9, 10, 11"
    );

    assert!(deep_size_json(&serde_json::json!(42)) > 0);
    assert!(deep_size_json(&serde_json::json!("hello")) > 0);
    assert!(
        deep_size_json(&serde_json::json!([1, 2, 3])) > deep_size_json(&serde_json::json!([1]))
    );
    assert!(deep_size_json(&serde_json::json!({"a": [1, 2, {"b": 3}]})) > 0);
    assert!(deep_size_json(&serde_json::json!([])) > 0);
    assert!(deep_size_json(&serde_json::json!({})) > 0);
    assert_eq!(
        deepSize(&serde_json::json!({"a": [1, 2]})),
        deep_size_json(&serde_json::json!({"a": [1, 2]}))
    );
}

#[test]
fn public_logging_helpers_match_python_utility_behaviors() {
    assert_eq!(VERBOSE, "verbose");
    assert_eq!(AUTO, "auto");
    assert_eq!(TERSE, "terse");
    assert_eq!(DEEP, "deep");
    assert_eq!(SILENT_D, AUTO);

    assert_eq!(silentConvert(None), AUTO);
    assert_eq!(silentConvert(Some(SilentInput::Bool(false))), VERBOSE);
    assert_eq!(silentConvert(Some(SilentInput::Bool(true))), DEEP);
    assert_eq!(
        silentConvert(Some(SilentInput::Str(VERBOSE.to_string()))),
        VERBOSE
    );
    assert_eq!(
        silentConvert(Some(SilentInput::Str(AUTO.to_string()))),
        AUTO
    );
    assert_eq!(
        silentConvert(Some(SilentInput::Str(TERSE.to_string()))),
        TERSE
    );
    assert_eq!(
        silentConvert(Some(SilentInput::Str(DEEP.to_string()))),
        DEEP
    );
    assert_eq!(
        silentConvert(Some(SilentInput::Str("invalid".to_string()))),
        AUTO
    );

    let levels = level_map();
    assert_eq!(levels[VERBOSE], LOG_LEVEL_DEBUG);
    assert_eq!(levels[AUTO], LOG_LEVEL_INFO);
    assert_eq!(levels[TERSE], LOG_LEVEL_WARNING);
    assert_eq!(levels[DEEP], LOG_LEVEL_ERROR);
    assert_eq!(
        logging_level(Some(SilentInput::Str(VERBOSE.to_string()))),
        10
    );
    assert_eq!(logging_level(Some(SilentInput::Str(AUTO.to_string()))), 20);
    assert_eq!(logging_level(Some(SilentInput::Str(TERSE.to_string()))), 30);
    assert_eq!(logging_level(Some(SilentInput::Str(DEEP.to_string()))), 40);
    assert_eq!(logging_level(Some(SilentInput::Str("bad".to_string()))), 20);
    assert_eq!(
        configure_logging(Some(SilentInput::Str(VERBOSE.to_string()))),
        LOG_LEVEL_DEBUG
    );
    assert_eq!(active_logging_level(), LOG_LEVEL_DEBUG);
    assert!(should_log(LOG_LEVEL_INFO));
    assert_eq!(
        set_logging_level(Some(SilentInput::Str(DEEP.to_string()))),
        LOG_LEVEL_ERROR
    );
    assert_eq!(active_logging_level(), LOG_LEVEL_ERROR);
    assert!(!should_log(LOG_LEVEL_WARNING));
    assert!(log_message(LOG_LEVEL_ERROR, "logging side-effect test"));
}

#[test]
fn public_cli_argument_reader_matches_python_utility_behaviors() {
    let tasks = BTreeMap::from([
        ("task1".to_string(), "First task".to_string()),
        ("task2".to_string(), "Second task".to_string()),
        ("special".to_string(), "Special task".to_string()),
    ]);
    let params = BTreeMap::from([
        (
            "param1".to_string(),
            ("Parameter one".to_string(), "default1".to_string()),
        ),
        (
            "param2".to_string(),
            ("Parameter two".to_string(), "default2".to_string()),
        ),
    ]);
    let flags = BTreeMap::from([
        (
            "flag1".to_string(),
            CliFlagSpec {
                description: "Binary flag".to_string(),
                default: CliFlagValue::Bool(false),
                n_values: 2,
            },
        ),
        (
            "flag2".to_string(),
            CliFlagSpec {
                description: "Ternary flag".to_string(),
                default: CliFlagValue::Ternary(0),
                n_values: 3,
            },
        ),
    ]);

    let help = read_args(
        "test-cmd",
        "Test description",
        &tasks,
        &params,
        &flags,
        &BTreeSet::new(),
        &[],
    );
    assert!(help.good);
    assert!(help.tasks.is_empty());
    assert!(help.params.is_empty());
    assert!(help.flags.is_empty());
    assert!(help.messages[0].contains("test-cmd"));
    assert!(help.messages[0].contains("Test description"));
    assert!(help.messages[0].contains("task1"));
    assert!(help.messages[0].to_lowercase().contains("all"));
    assert!(help.messages.contains(&"No task specified".to_string()));

    let parsed = read_args(
        "test-cmd",
        "Test description",
        &tasks,
        &params,
        &flags,
        &BTreeSet::new(),
        &[
            "task1".to_string(),
            "task2".to_string(),
            "param1=value1".to_string(),
            "+flag1".to_string(),
            "++flag2".to_string(),
        ],
    );
    assert!(parsed.good);
    assert_eq!(
        parsed.tasks,
        BTreeMap::from([("task1".to_string(), true), ("task2".to_string(), true)])
    );
    assert_eq!(parsed.params["param1"], "value1");
    assert_eq!(parsed.params["param2"], "default2");
    assert_eq!(parsed.flags["flag1"], CliFlagValue::Bool(true));
    assert_eq!(parsed.flags["flag2"], CliFlagValue::Ternary(1));

    let defaults = read_args(
        "test-cmd",
        "Test description",
        &tasks,
        &params,
        &flags,
        &BTreeSet::new(),
        &[
            "task1".to_string(),
            "param1=".to_string(),
            "-flag1".to_string(),
            "+flag2".to_string(),
        ],
    );
    assert!(defaults.good);
    assert_eq!(defaults.params["param1"], "default1");
    assert_eq!(defaults.flags["flag1"], CliFlagValue::Bool(false));
    assert_eq!(defaults.flags["flag2"], CliFlagValue::Ternary(0));

    let expanded = read_args(
        "test-cmd",
        "Test description",
        &tasks,
        &params,
        &flags,
        &BTreeSet::from(["special".to_string()]),
        &["all".to_string()],
    );
    assert!(expanded.good);
    assert_eq!(
        expanded.tasks,
        BTreeMap::from([("task1".to_string(), true), ("task2".to_string(), true)])
    );

    let illegal = read_args(
        "test-cmd",
        "Test description",
        &tasks,
        &params,
        &flags,
        &BTreeSet::new(),
        &["unknown=value".to_string()],
    );
    assert!(!illegal.good);
    assert!(illegal.tasks.is_empty());
    assert!(illegal.params.is_empty());
    assert!(illegal.flags.is_empty());
    assert!(
        illegal
            .messages
            .iter()
            .any(|message| message.contains("Illegal argument `unknown=value`"))
    );

    let help_flag = read_args(
        "test-cmd",
        "Test description",
        &tasks,
        &params,
        &flags,
        &BTreeSet::new(),
        &["--help".to_string()],
    );
    assert!(help_flag.good);
    assert!(help_flag.messages[0].contains("--help"));
}

#[test]
fn public_path_helpers_match_python_file_utility_behaviors() {
    assert_eq!(normpath(Some("/path/to/file")).unwrap(), "/path/to/file");
    assert_eq!(normpath(None), None);
    let normalized_dir = normpath(Some("/path/to/dir/")).unwrap();
    assert!(!normalized_dir.ends_with('/') || normalized_dir == "/");

    assert!(abspath(".").starts_with('/'));
    assert!(!abspath("./test").contains("./"));

    let expanded = expanduser("~/test");
    assert!(!expanded.starts_with('~'));
    assert!(expanded.contains("test"));
    assert!(unexpanduser(&expanded).contains('~'));

    assert_eq!(prefixSlash(Some("path")), Some("/path".to_string()));
    assert_eq!(prefixSlash(Some("/path")), Some("/path".to_string()));
    assert_eq!(prefixSlash(Some("")), Some(String::new()));
    assert_eq!(prefixSlash(None), None);

    assert_eq!(dirNm("/path/to/file.txt"), "/path/to");
    assert_eq!(dirNm("file.txt"), "");
    assert_eq!(fileNm("/path/to/file.txt"), "file.txt");
    assert_eq!(fileNm("file.txt"), "file.txt");
    assert_eq!(extNm("file.txt"), "txt");
    assert_eq!(extNm("file"), "file");
    assert_eq!(extNm("file.tar.gz"), "gz");
    assert_eq!(stripExt("file.txt"), "file");
    assert_eq!(stripExt("/path/to/file.txt"), "/path/to/file");
    assert_eq!(replaceExt("file.txt", "md"), "file.md");
    assert_eq!(
        replaceExt("/path/to/file.txt", "json"),
        "/path/to/file.json"
    );
    assert_eq!(
        splitPath("/path/to/file.txt"),
        ("/path/to".to_string(), "file.txt".to_string())
    );
    assert_eq!(
        splitExt("/path/to/file.txt"),
        ("/path/to/file".to_string(), ".txt".to_string())
    );
    assert_eq!(splitExt("file"), ("file".to_string(), String::new()));
    assert_eq!(LOCATIONS, &["~/text-fabric-data"]);
    let context = setDir();
    assert_eq!(expandDir(&context, "."), context.cur_dir);
    assert_eq!(
        expandDir(&context, "./child"),
        format!("{}/child", context.cur_dir)
    );
    assert_eq!(
        expandDir(&context, "relative"),
        format!("{}/relative", context.cur_dir)
    );

    assert_eq!(backendRep(Some("github"), "norm", None).unwrap(), "github");
    assert_eq!(
        backendRep(Some("github.com"), "norm", None).unwrap(),
        "github"
    );
    assert_eq!(backendRep(Some(""), "norm", None).unwrap(), "github");
    assert_eq!(backendRep(None, "norm", None).unwrap(), "github");
    assert_eq!(backendRep(Some("gitlab"), "norm", None).unwrap(), "gitlab");
    assert_eq!(
        backendRep(Some("gitlab.com"), "norm", None).unwrap(),
        "gitlab"
    );
    assert_eq!(backendRep(Some("github"), "tech", None).unwrap(), "github");
    assert_eq!(backendRep(Some("gitlab"), "tech", None).unwrap(), "gitlab");
    assert_eq!(
        backendRep(Some("custom.server.com"), "tech", None).unwrap(),
        "gitlab"
    );
    assert_eq!(backendRep(Some("github"), "name", None).unwrap(), "GitHub");
    assert_eq!(backendRep(Some("gitlab"), "name", None).unwrap(), "GitLab");
    assert_eq!(
        backendRep(Some("github"), "machine", None).unwrap(),
        "github.com"
    );
    assert_eq!(
        backendRep(Some("gitlab"), "machine", None).unwrap(),
        "gitlab.com"
    );
    assert!(
        backendRep(Some("github"), "url", None)
            .unwrap()
            .contains("github.com")
    );
    assert!(
        backendRep(Some("gitlab"), "url", None)
            .unwrap()
            .contains("gitlab.com")
    );
}

#[test]
fn public_filesystem_helpers_match_python_file_utility_behaviors() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();
    let root_str = root.to_string_lossy();

    let test_file = root.join("test.txt");
    fs::write(&test_file, "test").unwrap();
    assert!(isFile(&test_file.to_string_lossy()));
    assert!(fileExists(&test_file.to_string_lossy()));
    assert!(!isFile(&root_str));
    assert!(isDir(&root_str));
    assert!(dirExists(Some(&root_str)));
    assert!(!dirExists(Some(&test_file.to_string_lossy())));
    assert!(!dirExists(None));
    assert!(dirExists(Some("")));

    let new_file = root.join("subdir").join("new_file.txt");
    fileMake(&new_file.to_string_lossy(), false).unwrap();
    assert!(new_file.exists());
    fs::write(&new_file, "original").unwrap();
    fileMake(&new_file.to_string_lossy(), false).unwrap();
    assert_eq!(fs::read_to_string(&new_file).unwrap(), "original");
    fileMake(&new_file.to_string_lossy(), true).unwrap();
    assert_eq!(fs::read_to_string(&new_file).unwrap(), "");
    fileRemove(&new_file.to_string_lossy()).unwrap();
    assert!(!new_file.exists());
    fileRemove(&new_file.to_string_lossy()).unwrap();

    let src = root.join("src.txt");
    let dst = root.join("dst.txt");
    fs::write(&src, "content").unwrap();
    fileCopy(&src.to_string_lossy(), &dst.to_string_lossy()).unwrap();
    assert_eq!(fs::read_to_string(&dst).unwrap(), "content");
    fs::write(&src, "new").unwrap();
    fileCopy(&src.to_string_lossy(), &dst.to_string_lossy()).unwrap();
    assert_eq!(fs::read_to_string(&dst).unwrap(), "new");
    fileCopy(&src.to_string_lossy(), &src.to_string_lossy()).unwrap();
    assert_eq!(fs::read_to_string(&src).unwrap(), "new");

    let move_src = root.join("move_src.txt");
    let move_dst = root.join("move_dst.txt");
    fs::write(&move_src, "move").unwrap();
    fileMove(&move_src.to_string_lossy(), &move_dst.to_string_lossy()).unwrap();
    assert!(!move_src.exists());
    assert_eq!(fs::read_to_string(&move_dst).unwrap(), "move");

    let nested_dir = root.join("a").join("b").join("c");
    dirMake(&nested_dir.to_string_lossy()).unwrap();
    assert!(nested_dir.is_dir());
    dirMake(&nested_dir.to_string_lossy()).unwrap();

    let remove_dir = root.join("to_remove");
    dirMake(&remove_dir.to_string_lossy()).unwrap();
    fs::write(remove_dir.join("file.txt"), "test").unwrap();
    dirRemove(&remove_dir.to_string_lossy()).unwrap();
    assert!(!remove_dir.exists());

    let copy_src = root.join("src_dir");
    let copy_dst = root.join("dst_dir");
    dirMake(&copy_src.to_string_lossy()).unwrap();
    fs::write(copy_src.join("file.txt"), "content").unwrap();
    assert!(
        dirCopy(
            &copy_src.to_string_lossy(),
            &copy_dst.to_string_lossy(),
            false
        )
        .unwrap()
    );
    assert_eq!(
        fs::read_to_string(copy_dst.join("file.txt")).unwrap(),
        "content"
    );

    let dir_move_src = root.join("move_dir_src");
    let dir_move_dst = root.join("move_dir_dst");
    dirMake(&dir_move_src.to_string_lossy()).unwrap();
    fs::write(dir_move_src.join("file.txt"), "content").unwrap();
    assert!(
        dirMove(
            &dir_move_src.to_string_lossy(),
            &dir_move_dst.to_string_lossy()
        )
        .unwrap()
    );
    assert!(!dir_move_src.exists());
    assert_eq!(
        fs::read_to_string(dir_move_dst.join("file.txt")).unwrap(),
        "content"
    );
    let existing_src = root.join("existing_src");
    let existing_dst = root.join("existing_dst");
    dirMake(&existing_src.to_string_lossy()).unwrap();
    dirMake(&existing_dst.to_string_lossy()).unwrap();
    assert!(
        !dirMove(
            &existing_src.to_string_lossy(),
            &existing_dst.to_string_lossy()
        )
        .unwrap()
    );

    let contents_dir = root.join("contents");
    dirMake(&contents_dir.to_string_lossy()).unwrap();
    fs::write(contents_dir.join("file.txt"), "test").unwrap();
    dirMake(&contents_dir.join("subdir").to_string_lossy()).unwrap();
    let (files, dirs) = dirContents(&contents_dir.to_string_lossy()).unwrap();
    assert!(files.contains(&"file.txt".to_string()));
    assert!(dirs.contains(&"subdir".to_string()));
    assert_eq!(
        scanDir(&contents_dir.to_string_lossy()).unwrap(),
        vec!["file.txt".to_string(), "subdir".to_string()]
    );
    assert_eq!(
        dirContents(&root.join("missing").to_string_lossy()).unwrap(),
        (Vec::<String>::new(), Vec::<String>::new())
    );

    let walk_dir = root.join("walk");
    dirMake(&walk_dir.join("subdir").to_string_lossy()).unwrap();
    dirMake(&walk_dir.join("ignored").to_string_lossy()).unwrap();
    fs::write(walk_dir.join("file1.txt"), "test").unwrap();
    fs::write(walk_dir.join("subdir").join("file2.txt"), "test").unwrap();
    fs::write(walk_dir.join("ignored").join("file3.txt"), "test").unwrap();
    assert_eq!(
        dirAllFiles(&walk_dir.to_string_lossy(), None)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        dirAllFiles(
            &walk_dir.to_string_lossy(),
            Some(&BTreeSet::from(["ignored".to_string()]))
        )
        .unwrap()
        .len(),
        2
    );

    let empty_dir = root.join("empty");
    dirMake(&empty_dir.to_string_lossy()).unwrap();
    assert!(dirEmpty(&empty_dir.to_string_lossy()));
    assert!(!dirEmpty(&walk_dir.to_string_lossy()));
}

#[test]
fn public_structured_file_helpers_match_python_file_utility_behaviors() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original = getCwd().unwrap();
    chDir(temp_dir.path().to_string_lossy().as_ref()).unwrap();
    assert_eq!(
        getCwd().unwrap(),
        normpath(Some(&env::current_dir().unwrap().to_string_lossy())).unwrap()
    );
    chDir(&original).unwrap();

    let json_value = readJson(Some(r#"{"key": "value"}"#), None).unwrap();
    assert_eq!(json_value["key"], serde_json::json!("value"));

    let json_path = temp_dir.path().join("test.json");
    fs::write(&json_path, r#"{"key": "value"}"#).unwrap();
    let json_value = readJson(None, Some(&json_path.to_string_lossy())).unwrap();
    assert_eq!(json_value["key"], serde_json::json!("value"));
    assert_eq!(
        readJson(
            None,
            Some(&temp_dir.path().join("missing.json").to_string_lossy())
        )
        .unwrap(),
        serde_json::json!({})
    );

    let json_out_path = temp_dir.path().join("out.json");
    writeJson(
        &serde_json::json!({"key": "value"}),
        Some(&json_out_path.to_string_lossy()),
    )
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&json_out_path).unwrap())
            .unwrap(),
        serde_json::json!({"key": "value"})
    );
    let json_text = writeJson(&serde_json::json!({"key": "value"}), None)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&json_text).unwrap(),
        serde_json::json!({"key": "value"})
    );

    let yaml_value = readYaml(Some("key: value"), None).unwrap();
    assert_eq!(
        yaml_value["key"],
        serde_yaml::Value::String("value".to_string())
    );

    let yaml_path = temp_dir.path().join("test.yaml");
    fs::write(&yaml_path, "key: value").unwrap();
    let yaml_value = readYaml(None, Some(&yaml_path.to_string_lossy())).unwrap();
    assert_eq!(
        yaml_value["key"],
        serde_yaml::Value::String("value".to_string())
    );
    assert_eq!(
        readYaml(
            None,
            Some(&temp_dir.path().join("missing.yaml").to_string_lossy())
        )
        .unwrap(),
        serde_yaml::Value::Mapping(Default::default())
    );

    let yaml_data =
        serde_yaml::to_value(BTreeMap::from([("key".to_string(), "value".to_string())])).unwrap();
    let yaml_out_path = temp_dir.path().join("out.yaml");
    writeYaml(&yaml_data, Some(&yaml_out_path.to_string_lossy())).unwrap();
    let yaml_roundtrip: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&yaml_out_path).unwrap()).unwrap();
    assert_eq!(
        yaml_roundtrip["key"],
        serde_yaml::Value::String("value".to_string())
    );

    let yaml_text = writeYaml(&yaml_data, None).unwrap().unwrap();
    let yaml_roundtrip: serde_yaml::Value = serde_yaml::from_str(&yaml_text).unwrap();
    assert_eq!(
        yaml_roundtrip["key"],
        serde_yaml::Value::String("value".to_string())
    );
}

#[test]
fn public_name_helpers_match_python_utility_behaviors() {
    assert_eq!(camel(Some("hello_world")), Some("helloWorld".to_string()));
    assert_eq!(
        camel(Some("my_variable_name")),
        Some("myVariableName".to_string())
    );
    assert_eq!(camel(Some("hello")), Some("hello".to_string()));
    assert_eq!(camel(Some("")), Some(String::new()));
    assert_eq!(camel(None), None);

    assert_eq!(clean_name("myVariable"), "myVariable");
    assert_eq!(cleanName("myVariable"), clean_name("myVariable"));
    assert_eq!(clean_name("var_123"), "var_123");
    assert!(clean_name("123abc").starts_with('x'));
    assert!(clean_name("_test").starts_with('x'));
    let cleaned = clean_name("my-var.name");
    assert!(!cleaned.contains('-'));
    assert!(!cleaned.contains('.'));
    assert_eq!(clean_name("type"), "typ");
    assert_eq!(clean_name("database"), "dbase");

    assert!(is_clean(Some("myVar")));
    assert_eq!(isClean(Some("myVar")), is_clean(Some("myVar")));
    assert!(is_clean(Some("var_123")));
    assert!(is_clean(Some("ABC")));
    assert!(!is_clean(Some("123abc")));
    assert!(!is_clean(Some("_test")));
    assert!(!is_clean(None));
    assert!(!is_clean(Some("")));
    assert_eq!(isClean(Some("")), is_clean(Some("")));
}

#[test]
fn public_tf_data_loader_matches_python_loader_behaviors() {
    assert!(DATA_TYPES.contains(&"str"));
    assert!(DATA_TYPES.contains(&"int"));
    assert_eq!(ERROR_CUTOFF, 20);
    assert!(MEM_MSG.starts_with("CF is out of memory!"));
    assert_eq!(FATAL_MSG, "There was a fatal error! The message is:\n");

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("myfeature.tf");
    let data = TfData::new(&path);
    let data_alias = Data::new(&path);
    assert_eq!(data.path, path.to_string_lossy());
    assert_eq!(data_alias.path, data.path);
    assert_eq!(data.dirName(), temp_dir.path().to_string_lossy());
    assert_eq!(data.fileName(), "myfeature");
    assert_eq!(data.extension, ".tf");
    assert_eq!(data.method(), "load");
    assert!(data.dependencies().is_empty());
    assert!(!data.dataLoaded());
    assert!(!data.dataError());

    let mut missing = TfData::new(temp_dir.path().join("missing.tf"));
    assert!(!missing.load(false));
    assert!(missing.dataError());

    fs::write(
        &path,
        "@node\n@valueType=str\n@description=Test feature\n\n1\ta\n2\tb\n",
    )
    .unwrap();
    let mut node_data = TfData::new(&path);
    assert!(node_data.load(false));
    assert!(node_data.dataLoaded());
    assert_eq!(node_data.is_edge, Some(false));
    assert_eq!(node_data.isEdge(), Some(false));
    assert_eq!(node_data.is_config, Some(false));
    assert_eq!(node_data.isConfig(), Some(false));
    assert_eq!(node_data.data_type, "str");
    assert_eq!(node_data.dataType(), "str");
    assert_eq!(
        node_data
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("Test feature")
    );
    assert_eq!(
        node_data
            .metaData()
            .get("description")
            .and_then(Option::as_deref),
        Some("Test feature")
    );
    let Some(TfDataContent::Node(node_feature)) = &node_data.data else {
        panic!("expected node feature data");
    };
    assert_eq!(node_feature.value(1), Some(&FeatureValue::string("a")));
    assert_eq!(node_feature.value(2), Some(&FeatureValue::string("b")));

    node_data.unload();
    assert!(node_data.data.is_none());
    assert!(!node_data.dataLoaded());

    fs::write(&path, "@node\n@valueType=int\n\n100\n200\n").unwrap();
    assert!(node_data.load(false));
    assert_eq!(node_data.data_type, "int");
    node_data
        .metadata
        .insert("valueType".to_string(), Some("not-a-real-type".to_string()));
    node_data.setDataType();
    assert_eq!(node_data.dataType(), "str");
    let Some(TfDataContent::Node(node_feature)) = &node_data.data else {
        panic!("expected integer node feature data");
    };
    assert_eq!(node_feature.value(1), Some(&FeatureValue::Int(100)));
    assert_eq!(node_feature.value(2), Some(&FeatureValue::Int(200)));

    let mut metadata_only = TfData::new(&path);
    assert!(metadata_only.loadMetaOnly());
    assert_eq!(metadata_only.is_edge, Some(false));
    assert_eq!(
        metadata_only
            .metadata
            .get("valueType")
            .and_then(Option::as_deref),
        Some("int")
    );
    assert!(metadata_only.data.is_none());
    assert!(!metadata_only.dataLoaded());

    let edge_path = temp_dir.path().join("edge.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n1\t5\tparent\n",
    )
    .unwrap();
    let mut edge_data = TfData::new(&edge_path);
    assert!(edge_data.load(false));
    assert_eq!(edge_data.is_edge, Some(true));
    assert!(edge_data.edge_values);
    assert!(edge_data.edgeValues());
    let Some(TfDataContent::Edge(edge_feature)) = &edge_data.data else {
        panic!("expected edge feature data");
    };
    assert_eq!(
        edge_feature.edge_value(1, 5),
        Some(&FeatureValue::string("parent"))
    );

    let config_path = temp_dir.path().join("otext.tf");
    fs::write(
        &config_path,
        "@config\n@sectionTypes=book,chapter,verse\n\n",
    )
    .unwrap();
    let mut config_data = TfData::new(&config_path);
    assert!(config_data.load(false));
    assert_eq!(config_data.is_config, Some(true));
    assert_eq!(
        config_data
            .metadata
            .get("sectionTypes")
            .and_then(Option::as_deref),
        Some("book,chapter,verse")
    );

    let save_path = temp_dir.path().join("saved.tf");
    let mut save_data = TfData::new(&save_path);
    save_data.is_edge = Some(false);
    save_data.is_config = Some(false);
    save_data.metadata = BTreeMap::from([("valueType".to_string(), Some("str".to_string()))]);
    save_data.data = Some(TfDataContent::Node(NodeFeature::new(
        "saved".to_string(),
        save_data.metadata.clone(),
        HashMap::from([
            (1, FeatureValue::string("value1")),
            (2, FeatureValue::string("line\tbreak\nslash\\")),
        ]),
    )));
    save_data.data_loaded = true;
    assert!(save_data.save());
    let saved_text = fs::read_to_string(&save_path).unwrap();
    assert!(saved_text.contains("@node"));
    assert!(saved_text.contains("value1"));
    assert!(saved_text.contains("line\\tbreak\\nslash\\\\"));
    let parsed_saved = parse_tf_file(&save_path).unwrap();
    let TfFeature::Node(saved_feature) = parsed_saved else {
        panic!("expected saved node feature");
    };
    assert_eq!(
        saved_feature.value(2),
        Some(&FeatureValue::string("line\tbreak\nslash\\"))
    );

    let unknown_type_path = temp_dir.path().join("unknown.tf");
    let mut unknown_type = TfData::new(&unknown_type_path);
    unknown_type.metadata =
        BTreeMap::from([("valueType".to_string(), Some("unknown".to_string()))]);
    unknown_type.set_data_type();
    assert_eq!(unknown_type.data_type, "str");
}

#[test]
fn tf_data_computed_method_and_feature_accessors_use_canonical_rank_order() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\nsentence\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n4\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("label.tf"),
        "@node\n@valueType=str\n\n1\tsame\n2\tsame\n3\tsame\n4\tsame\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("rel.tf"),
        "@edge\n@valueType=int\n\n4\t1-3\n",
    )
    .unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();
    let mut expected = vec![1, 2, 3, 4];
    corpus.sort_nodes(&mut expected);
    assert_eq!(
        corpus
            .node_feature("label")
            .unwrap()
            .select(&FeatureValue::string("same")),
        expected
    );
    let mut rel_expected = vec![1, 2, 3];
    corpus.sort_nodes(&mut rel_expected);
    assert_eq!(corpus.edge_feature("rel").unwrap().s(4), rel_expected);

    let dependency = TfDataContent::Node(corpus.node_feature("label").unwrap().clone());
    let mut computed = TfData::new_computed(
        dir.path().join("computed_label.tf"),
        "copy_label",
        vec!["label".to_string()],
        vec![dependency],
        std::sync::Arc::new(|dependencies| dependencies[0].clone()),
    );
    assert_eq!(computed.method(), "copy_label");
    assert_eq!(computed.dependencies(), &["label".to_string()]);
    assert_eq!(computed.dependency_data().len(), 1);
    assert!(computed.load(false));
    let Some(TfDataContent::Node(feature)) = computed.data.as_ref() else {
        panic!("expected computed node feature");
    };
    assert_eq!(feature.value(1), Some(&FeatureValue::string("same")));
}

#[test]
fn range_spec_helpers_match_python_utility_behaviors() {
    assert_eq!(set_from_spec("5").unwrap(), BTreeSet::from([5]));
    assert_eq!(setFromSpec("5").unwrap(), set_from_spec("5").unwrap());
    assert_eq!(
        set_from_spec("1-5").unwrap(),
        BTreeSet::from([1, 2, 3, 4, 5])
    );
    assert_eq!(set_from_spec("1,3,5").unwrap(), BTreeSet::from([1, 3, 5]));
    assert_eq!(
        set_from_spec("1-3,5,7-9").unwrap(),
        BTreeSet::from([1, 2, 3, 5, 7, 8, 9])
    );
    assert_eq!(
        set_from_spec("5-1").unwrap(),
        BTreeSet::from([1, 2, 3, 4, 5])
    );
    assert!(set_from_spec("1-2-3").is_err());

    assert_eq!(
        ranges_from_set(&BTreeSet::from([1, 2, 3, 4, 5])),
        vec![(1, 5)]
    );
    assert_eq!(
        rangesFromSet(&BTreeSet::from([1, 2, 3, 4, 5])),
        ranges_from_set(&BTreeSet::from([1, 2, 3, 4, 5]))
    );
    assert_eq!(
        ranges_from_set(&BTreeSet::from([1, 3, 5])),
        vec![(1, 1), (3, 3), (5, 5)]
    );
    assert_eq!(
        ranges_from_set(&BTreeSet::from([1, 2, 3, 5, 7, 8])),
        vec![(1, 3), (5, 5), (7, 8)]
    );
    assert_eq!(ranges_from_set(&BTreeSet::new()), Vec::<(u32, u32)>::new());

    assert_eq!(ranges_from_list(&[1, 2, 3, 5, 6]), vec![(1, 3), (5, 6)]);
    assert_eq!(
        rangesFromList(&[1, 2, 3, 5, 6]),
        ranges_from_list(&[1, 2, 3, 5, 6])
    );
    assert_eq!(ranges_from_list(&[42]), vec![(42, 42)]);
    assert_eq!(ranges_from_list(&[]), Vec::<(u32, u32)>::new());

    assert_eq!(spec_from_ranges(&[(1, 1), (3, 3)]), "1,3");
    assert_eq!(
        specFromRanges(&[(1, 1), (3, 3)]),
        spec_from_ranges(&[(1, 1), (3, 3)])
    );
    assert_eq!(spec_from_ranges(&[(1, 5)]), "1-5");
    assert_eq!(spec_from_ranges(&[(1, 3), (5, 5), (7, 10)]), "1-3,5,7-10");

    assert_eq!(
        spec_from_ranges_logical(&[(1, 1), (3, 3)]),
        vec![Single(1), Single(3)]
    );
    assert_eq!(
        specFromRangesLogical(&[(1, 1), (3, 3)]),
        spec_from_ranges_logical(&[(1, 1), (3, 3)])
    );
    assert_eq!(spec_from_ranges_logical(&[(1, 5)]), vec![Range(1, 5)]);
}

#[test]
fn inverse_mapping_helpers_match_python_utility_behaviors() {
    let index = make_index(&BTreeMap::from([(1, "a"), (2, "a"), (3, "b")]));
    assert_eq!(index["a"], BTreeSet::from([1, 2]));
    assert_eq!(index["b"], BTreeSet::from([3]));
    assert_eq!(
        makeIndex(&BTreeMap::from([(1, "a"), (2, "a"), (3, "b")])),
        index
    );

    let inverse = make_inverse(&BTreeMap::from([
        (1, BTreeSet::from([2, 3])),
        (2, BTreeSet::from([3])),
    ]));
    assert_eq!(inverse[&2], BTreeSet::from([1]));
    assert_eq!(inverse[&3], BTreeSet::from([1, 2]));
    assert_eq!(
        makeInverse(&BTreeMap::from([
            (1, BTreeSet::from([2, 3])),
            (2, BTreeSet::from([3])),
        ])),
        inverse
    );

    let inverse_val =
        make_inverse_val(&BTreeMap::from([(1, BTreeMap::from([(2, "x"), (3, "y")]))]));
    assert_eq!(inverse_val[&2], BTreeMap::from([(1, "x")]));
    assert_eq!(inverse_val[&3], BTreeMap::from([(1, "y")]));
    assert_eq!(
        makeInverseVal(&BTreeMap::from([(1, BTreeMap::from([(2, "x"), (3, "y")]))])),
        inverse_val
    );
}

#[test]
fn parses_implicit_node_feature_values() {
    let path = repo_path("libs/core/tests/fixtures/mini_corpus/word.tf");
    let feature = parse_tf_file(path).expect("word.tf should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(1), Some("hello"));
    assert_eq!(feature.str_value(5), Some("morning"));
}

#[test]
fn parses_explicit_node_feature_rows_and_resumes_implicit_numbering() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("explicit.tf");
    fs::write(
        &path,
        "@node\n@valueType=str\n\n5\tvalue5\n10\tvalue10\nimplicit_after\n",
    )
    .unwrap();

    let feature = parse_tf_file(path).expect("explicit node rows should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(5), Some("value5"));
    assert_eq!(feature.str_value(10), Some("value10"));
    assert_eq!(feature.str_value(11), Some("implicit_after"));
}

#[test]
fn parses_node_and_edge_specs_with_ranges_and_commas() {
    let dir = tempfile::tempdir().unwrap();

    let node_path = dir.path().join("ranges.tf");
    fs::write(
        &node_path,
        "@node\n@valueType=str\n\n1-3,5\tshared\n8\teight\nimplicit_after\n",
    )
    .unwrap();

    let feature = parse_tf_file(&node_path).expect("node ranges should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.s(&FeatureValue::string("shared")), &[1, 2, 3, 5]);
    assert_eq!(feature.str_value(4), None);
    assert_eq!(feature.str_value(8), Some("eight"));
    assert_eq!(feature.str_value(9), Some("implicit_after"));

    let edge_path = dir.path().join("edges.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n1-2,4\t7-8,10\tlinked\n",
    )
    .unwrap();

    let feature = parse_tf_file(&edge_path).expect("edge ranges should parse");
    let TfFeature::Edge(feature) = feature else {
        panic!("expected edge feature");
    };
    assert_eq!(feature.forward(1), vec![7, 8, 10]);
    assert_eq!(feature.forward(2), vec![7, 8, 10]);
    assert_eq!(feature.forward(4), vec![7, 8, 10]);
    assert_eq!(
        feature.edge_value(4, 10),
        Some(&FeatureValue::string("linked"))
    );
    assert_eq!(
        feature.backward(8),
        vec![1, 2, 4],
        "target range should create inverse edges from every expanded source"
    );
}

#[test]
fn parses_descending_node_and_edge_ranges_like_python_loader() {
    let dir = tempfile::tempdir().unwrap();

    let node_path = dir.path().join("descending_ranges.tf");
    fs::write(
        &node_path,
        "@node\n@valueType=str\n\n5-3\treversed\n8\teight\nimplicit_after\n",
    )
    .unwrap();

    let feature = parse_tf_file(&node_path).expect("descending node ranges should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.s(&FeatureValue::string("reversed")), &[3, 4, 5]);
    assert_eq!(feature.str_value(2), None);
    assert_eq!(feature.str_value(8), Some("eight"));
    assert_eq!(feature.str_value(9), Some("implicit_after"));

    let edge_path = dir.path().join("descending_edges.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n4-2\t8-6\treversed\n",
    )
    .unwrap();

    let feature = parse_tf_file(&edge_path).expect("descending edge ranges should parse");
    let TfFeature::Edge(feature) = feature else {
        panic!("expected edge feature");
    };
    assert_eq!(feature.forward(2), vec![6, 7, 8]);
    assert_eq!(feature.forward(3), vec![6, 7, 8]);
    assert_eq!(feature.forward(4), vec![6, 7, 8]);
    assert_eq!(
        feature.edge_value(2, 6),
        Some(&FeatureValue::string("reversed"))
    );
    assert_eq!(
        feature.backward(7),
        vec![2, 3, 4],
        "descending target range should create inverse edges from every normalized source"
    );
}

#[test]
fn parses_text_fabric_string_value_escapes_like_python_loader() {
    let dir = tempfile::tempdir().unwrap();

    let node_path = dir.path().join("escaped.tf");
    fs::write(
        &node_path,
        "@node\n@valueType=str\n\nhas\\ttab\nhas\\nnewline\nhas\\\\backslash\n",
    )
    .unwrap();

    let feature = parse_tf_file(&node_path).expect("escaped node values should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(1), Some("has\ttab"));
    assert_eq!(feature.str_value(2), Some("has\nnewline"));
    assert_eq!(feature.str_value(3), Some("has\\backslash"));

    let edge_path = dir.path().join("escaped_edges.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n1\t2\ta\\tb\n2\t3\ta\\nb\n3\t4\ta\\\\b\n",
    )
    .unwrap();

    let feature = parse_tf_file(&edge_path).expect("escaped edge values should parse");
    let TfFeature::Edge(feature) = feature else {
        panic!("expected edge feature");
    };
    assert_eq!(
        feature.edge_value(1, 2),
        Some(&FeatureValue::string("a\tb"))
    );
    assert_eq!(
        feature.edge_value(2, 3),
        Some(&FeatureValue::string("a\nb"))
    );
    assert_eq!(
        feature.edge_value(3, 4),
        Some(&FeatureValue::string("a\\b"))
    );
}

#[test]
fn text_fabric_value_escape_helpers_match_python_round_trip_behaviors() {
    assert_eq!(value_from_tf(r"a\tb"), "a\tb");
    assert_eq!(value_from_tf(r"a\nb"), "a\nb");
    assert_eq!(value_from_tf(r"a\\b"), r"a\b");
    assert_eq!(value_from_tf("hello world"), "hello world");
    assert_eq!(valueFromTf(r"a\tb"), value_from_tf(r"a\tb"));

    assert_eq!(tf_from_value(&FeatureValue::string("a\tb")), r"a\tb");
    assert_eq!(tf_from_value(&FeatureValue::string("a\nb")), r"a\nb");
    assert_eq!(tf_from_value(&FeatureValue::string(r"a\b")), r"a\\b");
    assert_eq!(tf_from_value(&FeatureValue::Int(42)), "42");
    assert_eq!(
        tfFromValue(&FeatureValue::string("a\tb")),
        tf_from_value(&FeatureValue::string("a\tb"))
    );
}

#[test]
fn parses_explicit_ranges_in_otype() {
    let Some(path) = bhsa_tf().map(|path| path.join("otype.tf")) else {
        return;
    };
    let feature = parse_tf_file(path).expect("BHSA otype.tf should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(1), Some("word"));
    assert_eq!(feature.str_value(426590), Some("word"));
    assert_eq!(feature.str_value(426591), Some("book"));
}

#[test]
fn parses_edge_cases_and_valued_edges() {
    let empty = parse_tf_file(repo_path("libs/core/tests/fixtures/edge_cases/empty.tf")).unwrap();
    let TfFeature::Node(empty) = empty else {
        panic!("expected empty node feature");
    };
    assert!(empty.items().is_empty());

    let single = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/edge_cases/single_node.tf",
    ))
    .unwrap();
    let TfFeature::Node(single) = single else {
        panic!("expected single-node feature");
    };
    assert_eq!(single.str_value(1), Some("only_value"));

    let unicode =
        parse_tf_file(repo_path("libs/core/tests/fixtures/edge_cases/unicode.tf")).unwrap();
    let TfFeature::Node(unicode) = unicode else {
        panic!("expected unicode node feature");
    };
    assert_eq!(unicode.str_value(2), Some("世界"));
    assert_eq!(unicode.str_value(4), Some("שלום"));
    assert_eq!(unicode.str_value(5), Some("🌍"));

    let relation = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/mini_corpus/relation.tf",
    ))
    .unwrap();
    let TfFeature::Edge(relation) = relation else {
        panic!("expected relation edge feature");
    };
    assert!(relation.has_edge_values());
    assert!(relation.hasEdgeValues());
    assert_eq!(relation.edge_value_count(), 5);
    assert_eq!(relation.value_type(), Some("str"));
    assert_eq!(
        relation.description(),
        Some("relation type between nodes (tests string edge values)")
    );
    assert!(relation.meta().contains_key("edgeValues"));
    assert_eq!(relation.metadata_value("edgeValues"), None);
    assert_eq!(relation.forward(1), vec![6]);
    assert_eq!(
        relation.edge_value(1, 6),
        Some(&FeatureValue::string("subject"))
    );
    assert_eq!(
        relation.edge_value(2, 6),
        Some(&FeatureValue::string("predicate"))
    );
    assert_eq!(
        relation.forward_with_values(1),
        vec![(6, Some(FeatureValue::string("subject")))]
    );
    assert_eq!(relation.f_with_values(1), relation.forward_with_values(1));
    assert_eq!(relation.items()[0], (1, vec![6]));
    assert_eq!(
        relation.backward_with_values(6),
        vec![
            (1, Some(FeatureValue::string("subject"))),
            (2, Some(FeatureValue::string("predicate"))),
            (3, Some(FeatureValue::string("object"))),
        ]
    );
    assert_eq!(relation.t_with_values(6), relation.backward_with_values(6));
    assert_eq!(
        relation.both_with_values(6),
        vec![
            (1, Some(FeatureValue::string("subject"))),
            (2, Some(FeatureValue::string("predicate"))),
            (3, Some(FeatureValue::string("object"))),
        ]
    );
    assert_eq!(relation.b_with_values(6), relation.both_with_values(6));

    let distance = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/mini_corpus/distance.tf",
    ))
    .unwrap();
    let TfFeature::Edge(distance) = distance else {
        panic!("expected distance edge feature");
    };
    assert_eq!(distance.edge_value(1, 2), Some(&FeatureValue::Int(0)));
    assert_eq!(distance.edge_value(1, 3), Some(&FeatureValue::Int(5)));
    assert_eq!(distance.forward(2), vec![3]);
    assert_eq!(distance.edge_value(2, 3), None);
    assert_eq!(distance.forward_with_values(2), vec![(3, None)]);
    assert_eq!(distance.f_with_values(2), distance.forward_with_values(2));
    assert_eq!(
        distance.backward_with_values(3),
        vec![(1, Some(FeatureValue::Int(5))), (2, None)]
    );
    assert_eq!(distance.t_with_values(3), distance.backward_with_values(3));
    assert_eq!(
        distance.both_with_values(3),
        vec![
            (1, Some(FeatureValue::Int(5))),
            (2, None),
            (4, Some(FeatureValue::Int(0))),
        ]
    );
    assert_eq!(distance.b_with_values(3), distance.both_with_values(3));
}

#[test]
fn public_io_compiler_wrappers_match_python_compile_entrypoints() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let explicit_cache = temp_dir.path().join("explicit.cfr");

    assert!(compile_corpus(&source, Some(explicit_cache.as_path())).unwrap());
    let explicit = MappedCompiledCorpus::open(&explicit_cache).unwrap();
    assert_eq!(explicit.max_slot().unwrap(), 5);
    assert_eq!(
        explicit.search().search("word", Some(2)).unwrap(),
        vec![vec![1], vec![2]]
    );

    let copied_source = temp_dir.path().join("mini_source");
    dirCopy(
        &source.to_string_lossy(),
        &copied_source.to_string_lossy(),
        false,
    )
    .unwrap();
    let default_cache = default_compiled_output_path(&copied_source);
    assert_eq!(
        default_cache,
        copied_source
            .join(".cfm")
            .join(CFM_VERSION)
            .join("corpus.cfr")
    );

    let compiler = Compiler::new(&copied_source);
    assert_eq!(compiler.source_dir(), copied_source.as_path());
    assert_eq!(compiler.default_output_path(), default_cache);
    assert!(compiler.compile(None).unwrap());
    let default_loaded = MappedCompiledCorpus::open(compiler.default_output_path()).unwrap();
    assert_eq!(default_loaded.slot_type().unwrap(), "word");
    assert_eq!(default_loaded.search().count("phrase", None).unwrap(), 2);

    let precomputed_source = Corpus::load(&copied_source).unwrap();
    let precomputed_cache = temp_dir.path().join("precomputed.cfr");
    assert!(
        compiler
            .compile_precomputed(Some(&precomputed_cache), &precomputed_source)
            .unwrap()
    );
    let precomputed_loaded = MappedCompiledCorpus::open(precomputed_cache).unwrap();
    assert_eq!(
        precomputed_loaded.search().count("phrase", None).unwrap(),
        2
    );
}

#[test]
fn parses_and_loads_config_features() {
    let parsed = parse_tf_file(repo_path("libs/core/tests/fixtures/mini_corpus/otext.tf")).unwrap();
    let TfFeature::Config { name, metadata } = parsed else {
        panic!("expected config feature");
    };
    assert_eq!(name, "otext");
    assert_eq!(
        metadata.get("sectionTypes").and_then(Option::as_deref),
        Some("sentence,phrase")
    );
    assert_eq!(
        metadata.get("sectionFeatures").and_then(Option::as_deref),
        Some("sentence_id,phrase_id")
    );
    assert_eq!(
        metadata
            .get("fmt:text-orig-full")
            .and_then(Option::as_deref),
        Some("{word}")
    );

    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let otext = corpus.config_feature("otext").unwrap();
    assert_eq!(
        otext.get("structureTypes").and_then(Option::as_deref),
        Some("")
    );
}

#[test]
fn parses_tf_metadata_without_loading_data_rows_like_python_meta_only() {
    let word_metadata =
        parse_tf_file_metadata(repo_path("libs/core/tests/fixtures/mini_corpus/word.tf")).unwrap();
    assert_eq!(word_metadata.name, "word");
    assert_eq!(word_metadata.kind, TfFeatureKind::Node);
    assert_eq!(
        word_metadata
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("word text")
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad_rows.tf");
    fs::write(
        &path,
        "@node\n@valueType=int\n@description=metadata only\n\nnot-an-int\n",
    )
    .unwrap();

    let metadata = parse_tf_file_metadata(&path).unwrap();
    assert_eq!(metadata.name, "bad_rows");
    assert_eq!(metadata.kind, TfFeatureKind::Node);
    assert_eq!(
        metadata
            .metadata
            .get("valueType")
            .and_then(Option::as_deref),
        Some("int")
    );
    assert!(parse_tf_file(&path).is_err());
}

#[test]
fn normalizes_desc_and_eg_metadata_like_python_format_meta() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();
    fs::write(
        dir.path().join("gloss.tf"),
        "@node\n@valueType=str\n@desc=A feature\n@eg=example\n\nalpha\nbeta\n",
    )
    .unwrap();

    let metadata = parse_tf_file_metadata(dir.path().join("gloss.tf")).unwrap();
    assert_eq!(
        metadata
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("A feature (example)")
    );
    assert!(!metadata.metadata.contains_key("desc"));
    assert!(!metadata.metadata.contains_key("eg"));

    let parsed = parse_tf_file(dir.path().join("gloss.tf")).unwrap();
    let TfFeature::Node(feature) = parsed else {
        panic!("expected node feature");
    };
    assert_eq!(feature.description(), Some("A feature (example)"));

    let corpus = Corpus::load(dir.path()).unwrap();
    assert_eq!(
        corpus.node_feature("gloss").unwrap().description(),
        Some("A feature (example)")
    );

    let cache_path = dir.path().join("desc-eg.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    assert_eq!(
        mapped.node_feature("gloss").unwrap().unwrap().description(),
        Some("A feature (example)")
    );
}

#[test]
fn reports_invalid_tf_files() {
    assert!(
        parse_tf_file(repo_path(
            "libs/core/tests/fixtures/invalid/missing_header.tf"
        ))
        .is_err()
    );
    assert!(
        parse_tf_file(repo_path(
            "libs/core/tests/fixtures/invalid/corrupt_data.tf"
        ))
        .is_err()
    );
}

#[test]
fn treats_unknown_value_type_as_string_values() {
    let parsed = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/invalid/bad_valueType.tf",
    ))
    .unwrap();
    let TfFeature::Node(feature) = parsed else {
        panic!("expected node feature");
    };
    assert_eq!(feature.value_type(), Some("float"));
    assert_eq!(feature.str_value(1), Some("1.5"));
    assert_eq!(feature.str_value(2), Some("2.5"));
}

#[test]
fn accepts_python_style_value_type_metadata_alias() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("legacy_number.tf"),
        "@node\n@value_type=int\n@description=legacy typed number\n\n7\n11\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("legacy_distance.tf"),
        "@edge\n@value_type=int\n@edgeValues\n@description=legacy typed edge\n\n1\t3\t9\n2\t3\t\n",
    )
    .unwrap();

    let parsed = parse_tf_file(dir.path().join("legacy_number.tf")).unwrap();
    let TfFeature::Node(parsed_feature) = parsed else {
        panic!("expected node feature");
    };
    assert_eq!(parsed_feature.value_type(), Some("int"));
    assert_eq!(parsed_feature.v(1), Some(&FeatureValue::Int(7)));
    assert_eq!(parsed_feature.v(2), Some(&FeatureValue::Int(11)));
    assert_eq!(parsed_feature.metadata_value("valueType"), Some("int"));

    let corpus = Corpus::load(dir.path()).unwrap();
    let feature = corpus.node_feature("legacy_number").unwrap();
    assert_eq!(feature.value_type(), Some("int"));
    assert_eq!(feature.v(1), Some(&FeatureValue::Int(7)));
    assert_eq!(
        corpus
            .is_loaded(Some(&["legacy_number"]))
            .get("legacy_number")
            .unwrap()
            .as_ref()
            .unwrap()
            .value_type,
        "int"
    );
    assert_eq!(
        FeatureInfo::from_corpus(&corpus, "legacy_number", FeatureKind::Node)
            .unwrap()
            .value_type,
        "int"
    );

    let cache_path = dir.path().join("legacy.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_feature = mapped.mixed_node_feature("legacy_number").unwrap().unwrap();
    assert_eq!(mapped_feature.value_type(), Some("int"));
    assert_eq!(mapped_feature.description(), Some("legacy typed number"));
    assert_eq!(mapped_feature.v(1).unwrap(), Some(MappedNodeValue::Int(7)));
    assert_eq!(
        mapped
            .is_loaded(Some(&["legacy_number"]))
            .unwrap()
            .get("legacy_number")
            .unwrap()
            .as_ref()
            .unwrap()
            .value_type,
        "int"
    );
    assert_eq!(
        FeatureInfo::from_mapped(&mapped, "legacy_number", FeatureKind::Node)
            .unwrap()
            .value_type,
        "int"
    );
    assert!(
        mapped
            .feature_catalog(Some(FeatureKind::Node), None)
            .unwrap()
            .iter()
            .any(|entry| entry.name == "legacy_number" && entry.value_type == "int")
    );
    assert_eq!(
        mapped
            .describe_feature("legacy_number", 3)
            .unwrap()
            .value_type,
        "int"
    );
}

#[test]
fn loads_mini_corpus_and_queries_basic_types() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    assert_eq!(corpus.max_slot, 5);
    assert_eq!(corpus.max_slot(), 5);
    assert_eq!(corpus.maxSlot(), 5);
    assert_eq!(corpus.max_node, 8);
    assert_eq!(corpus.max_node(), 8);
    assert_eq!(corpus.maxNode(), 8);
    assert_eq!(corpus.order(), vec![8, 6, 1, 2, 3, 7, 4, 5]);
    assert_eq!(corpus.rank().len(), 8);
    assert_eq!(corpus.slot_type, "word");
    assert_eq!(corpus.slot_type(), "word");
    assert_eq!(corpus.slotType(), "word");
    assert_eq!(corpus.node_type(0), None);
    assert_eq!(corpus.node_type(1), Some("word"));
    assert_eq!(corpus.node_type(6), Some("phrase"));
    assert_eq!(corpus.node_type(8), Some("sentence"));
    assert_eq!(corpus.node_type(999), None);
    assert_eq!(corpus.nodes_of_type("word"), &[1, 2, 3, 4, 5]);
    assert_eq!(
        corpus.node_type_items(),
        vec![
            (1, "word".to_string()),
            (2, "word".to_string()),
            (3, "word".to_string()),
            (4, "word".to_string()),
            (5, "word".to_string()),
            (6, "phrase".to_string()),
            (7, "phrase".to_string()),
            (8, "sentence".to_string()),
        ]
    );
    assert_eq!(corpus.otype_items(), corpus.node_type_items());
    assert_eq!(corpus.otypeItems(), corpus.node_type_items());
    assert_eq!(corpus.node_type_interval("word"), Some((1, 5)));
    assert_eq!(corpus.s_interval("word"), Some((1, 5)));
    assert_eq!(corpus.sInterval("word"), Some((1, 5)));
    assert_eq!(corpus.node_type_interval("phrase"), Some((6, 7)));
    assert_eq!(corpus.node_type_interval("missing"), None);
    assert_eq!(
        corpus.section_features(),
        vec!["sentence_id".to_string(), "phrase_id".to_string()]
    );
    assert_eq!(
        corpus.sectionTypes(),
        vec!["sentence".to_string(), "phrase".to_string()]
    );
    assert_eq!(corpus.sectionTypes(), corpus.section_types());
    assert_eq!(corpus.sectionFeatures(), corpus.section_features());
    assert_eq!(corpus.structure_types(), Vec::<String>::new());
    assert_eq!(corpus.structure_features(), Vec::<String>::new());
    assert_eq!(corpus.structureTypes(), corpus.structure_types());
    assert_eq!(corpus.structureFeatures(), corpus.structure_features());
    assert_eq!(corpus.slots(0), Vec::<u32>::new());
    assert_eq!(corpus.slots(1), vec![1]);
    assert_eq!(corpus.slots(6), vec![1, 2, 3]);
    assert_eq!(corpus.slots(8), vec![1, 2, 3, 4, 5]);
    assert_eq!(corpus.slots(999), Vec::<u32>::new());
    assert_eq!(corpus.text(1, None), "hello");
    assert_eq!(corpus.text(1, Some("text-orig-full")), "hello");
    assert_eq!(corpus.text(1, Some("fmt:text-orig-full")), "hello");
    assert_eq!(corpus.text(6, None), "hellobeautifulworld");
    assert_eq!(corpus.text(8, None), "hellobeautifulworldgoodmorning");
    assert_eq!(corpus.text_nodes(&[3, 1, 2], None), "worldhellobeautiful");
    assert_eq!(corpus.text(1, Some("missing-format")), "");
    assert_eq!(
        corpus.section_tuple(1, &SectionOptions::default()),
        vec![Some(8), Some(6)]
    );
    assert_eq!(
        corpus.sectionTuple(1, &SectionOptions::default()),
        corpus.section_tuple(1, &SectionOptions::default())
    );
    assert_eq!(
        corpus.section_from_node(1, &SectionOptions::default()),
        vec![Some(FeatureValue::string("S1")), Some(FeatureValue::Int(1))]
    );
    assert_eq!(
        corpus.sectionFromNode(1, &SectionOptions::default()),
        corpus.section_from_node(1, &SectionOptions::default())
    );
    assert_eq!(corpus.section_ref(1), "S1 1");
    assert_eq!(
        corpus.section_tuple(
            6,
            &SectionOptions {
                fillup: true,
                ..SectionOptions::default()
            }
        ),
        vec![Some(8), Some(6)]
    );
    assert_eq!(
        corpus.node_from_section(&[FeatureValue::string("S1")]),
        Some(8)
    );
    assert_eq!(
        corpus.nodeFromSection(&[FeatureValue::string("S1")]),
        corpus.node_from_section(&[FeatureValue::string("S1")])
    );
    assert_eq!(
        corpus.node_from_section(&[FeatureValue::string("S1"), FeatureValue::Int(1)]),
        Some(6)
    );
    assert_eq!(
        corpus.node_from_section(&[FeatureValue::string("S1"), FeatureValue::Int(99)]),
        None
    );
    assert_eq!(
        corpus.oslots_items(),
        vec![
            (6, vec![1, 2, 3]),
            (7, vec![4, 5]),
            (8, vec![1, 2, 3, 4, 5]),
        ]
    );
    let score = corpus.node_feature("score").unwrap();
    assert_eq!(score.v(2), Some(&FeatureValue::Int(0)));
    assert_eq!(score.v(3), None);
    assert_eq!(score.s(&FeatureValue::Int(0)), &[2, 5]);

    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let results = MappedSearch::new(&mapped).search("word", None).unwrap();
    assert_eq!(results.len(), 5);
    assert_eq!(results[0], vec![1]);
}

#[test]
fn text_api_supports_multilingual_section_names_and_book_aliases() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nbook\nchapter\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n2\t1\n3\t1\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("book.tf"),
        "@node\n@valueType=str\n@languageCode=en\n@language=English\n@languageEnglish=English\n\n2\tGenesis\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("book@he.tf"),
        "@node\n@valueType=str\n@languageCode=he\n@language=עברית\n@languageEnglish=Hebrew\n\n2\tבראשית\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("chapter.tf"),
        "@node\n@valueType=int\n\n3\t1\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\nhello\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("otext.tf"),
        "@config\n@fmt:text-orig-full={word}\n@sectionTypes=book,chapter\n@sectionFeatures=book,chapter\n\n",
    )
    .unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();
    assert!(corpus.languages().contains_key("en"));
    assert_eq!(
        corpus.languages()["he"]
            .get("languageEnglish")
            .map(String::as_str),
        Some("Hebrew")
    );
    assert_eq!(corpus.name_from_node("en")[&2], "Genesis");
    assert_eq!(corpus.name_from_node("he")[&2], "בראשית");
    assert_eq!(
        corpus.section_from_node_lang(1, &SectionOptions::default(), "he"),
        vec![
            Some(FeatureValue::string("בראשית")),
            Some(FeatureValue::Int(1))
        ]
    );
    assert_eq!(
        corpus.node_from_section_lang(&[FeatureValue::string("בראשית")], "he"),
        Some(2)
    );
    assert_eq!(corpus.bookName(1, "en"), Some("Genesis".to_string()));
    assert_eq!(corpus.bookName(1, "he"), Some("בראשית".to_string()));
    assert_eq!(corpus.bookNode("Genesis", "en"), Some(2));
    assert_eq!(corpus.bookNode("בראשית", "he"), Some(2));

    let text = Text::new(&corpus);
    assert_eq!(text.bookName(1, "he"), Some("בראשית".to_string()));
    assert_eq!(text.bookNode("Genesis", "en"), Some(2));
    assert_eq!(
        text.sectionFromNode(1, &SectionOptions::default(), "en"),
        vec![
            Some(FeatureValue::string("Genesis")),
            Some(FeatureValue::Int(1))
        ]
    );
}

#[test]
fn explores_tf_directory_without_materializing_features() {
    let inventory = explore_features(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();

    assert!(inventory.nodes.contains(&"otype".to_string()));
    assert!(inventory.nodes.contains(&"word".to_string()));
    assert!(inventory.nodes.contains(&"pos".to_string()));
    assert!(inventory.edges.contains(&"oslots".to_string()));
    assert!(inventory.edges.contains(&"parent".to_string()));
    assert!(inventory.configs.contains(&"otext".to_string()));
    assert!(inventory.nodes().contains(&"word".to_string()));
    assert!(inventory.edges().contains(&"parent".to_string()));
    assert!(inventory.configs().contains(&"otext".to_string()));
    assert!(inventory.computeds().contains(&"rank".to_string()));
    let categories = inventory.categories();
    assert!(categories["nodes"].contains(&"word".to_string()));
    assert!(categories["edges"].contains(&"parent".to_string()));
    assert!(categories["configs"].contains(&"otext".to_string()));
    assert!(categories["computeds"].contains(&"rank".to_string()));
    assert!(!inventory.nodes.contains(&"parent".to_string()));
    assert!(!inventory.edges.contains(&"word".to_string()));

    assert!(explore_features(repo_path("libs/core/tests/fixtures/invalid")).is_err());
}

#[test]
fn fabric_facade_explores_loads_compiles_and_opens_mapped_corpora() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let fabric = Fabric::new(&source);

    let inventory = fabric.explore().unwrap();
    assert!(inventory.nodes.contains(&"word".to_string()));
    assert!(inventory.edges.contains(&"parent".to_string()));

    let loaded = fabric.load_all().unwrap();
    assert_eq!(loaded.max_node(), 8);
    assert_eq!(fabric.loadAll().unwrap().max_node(), loaded.max_node());

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
    assert!(selective_from_slice.node_feature("number").unwrap().is_some());
    assert!(selective_from_slice.node_feature("pos").unwrap().is_some());

    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");
    fabric.compile(&cache_path, Vec::<&str>::new()).unwrap();

    let mapped = fabric.open_mapped(&cache_path).unwrap();
    assert_eq!(
        MappedSearch::new(&mapped)
            .search("word word=hello", None)
            .unwrap(),
        vec![vec![1]]
    );
    let mapped_alias = fabric.openMapped(&cache_path).unwrap();
    assert_eq!(
        MappedSearch::new(&mapped_alias)
            .search("word word=hello", None)
            .unwrap(),
        vec![vec![1]]
    );
}

#[test]
fn fabric_load_all_creates_reuses_and_refreshes_mapped_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let corpus_dir = temp_dir.path().join("mini");
    dirCopy(
        &source.to_string_lossy(),
        &corpus_dir.to_string_lossy(),
        false,
    )
    .unwrap();
    let _ = fs::remove_dir_all(corpus_dir.join(".cfr"));
    let fabric = Fabric::new(&corpus_dir);
    let cache_path = fabric.cfr_cache_path();
    assert_eq!(cache_path, corpus_dir.join(".cfr").join("3").join("corpus.cfr"));
    assert!(!cache_path.exists());

    let first = fabric.loadAll().unwrap();
    assert_eq!(first.max_node(), 8);
    assert!(cache_path.exists());
    let first_mtime = fs::metadata(&cache_path).unwrap().modified().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = fabric.loadAll().unwrap();
    assert_eq!(second.max_node(), 8);
    let second_mtime = fs::metadata(&cache_path).unwrap().modified().unwrap();
    assert_eq!(first_mtime, second_mtime);

    std::thread::sleep(std::time::Duration::from_millis(20));
    let word_path = corpus_dir.join("word.tf");
    let word_contents = fs::read_to_string(&word_path).unwrap();
    fs::write(&word_path, word_contents).unwrap();
    let third = fabric.loadAll().unwrap();
    assert_eq!(third.max_node(), 8);
    let third_mtime = fs::metadata(&cache_path).unwrap().modified().unwrap();
    assert!(third_mtime > second_mtime);
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
        inventory.feature_paths("word").unwrap(),
        &[dir.path().join("word.tf"), extra.join("word.tf")]
    );
    assert_eq!(
        inventory
            .feature_metadata("word")
            .unwrap()
            .get("description")
            .and_then(Option::as_deref),
        Some("override words")
    );
    assert_eq!(
        fabric.ignored_feature_paths().unwrap()["word"],
        vec![dir.path().join("word.tf")]
    );
    assert_eq!(
        fabric.features_ignored()["word"],
        vec![dir.path().join("word.tf")]
    );
    assert_eq!(
        fabric.featuresIgnored()["word"],
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
    assert_eq!(corpus.text(2, None).unwrap(), "extra2");
}

#[test]
fn fabric_save_round_trips_loaded_corpus_features_to_tf_files() {
    let source = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fabric = Fabric::new(dir.path());
    let output = dir.path().join("saved");

    assert!(fabric.save(&source, Some(&output), None).unwrap());
    assert!(output.join("otype.tf").exists());
    assert!(output.join("oslots.tf").exists());
    assert!(output.join("relation.tf").exists());
    assert!(output.join("otext.tf").exists());

    let reloaded = Corpus::load(&output).unwrap();
    assert_eq!(
        reloaded.node_feature("word").unwrap().values,
        source.node_feature("word").unwrap().values
    );
    assert_eq!(
        reloaded.edge_feature("oslots").unwrap().values,
        source.edge_feature("oslots").unwrap().values
    );
    assert_eq!(
        reloaded.edge_feature("relation").unwrap().edge_values,
        source.edge_feature("relation").unwrap().edge_values
    );
    assert_eq!(
        reloaded.config_feature("otext").unwrap(),
        source.config_feature("otext").unwrap()
    );
    let cache_path = dir.path().join("reloaded.cfr");
    compile_features(&output, &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    assert_eq!(
        MappedSearch::new(&mapped)
            .search("word word=hello", None)
            .unwrap(),
        vec![vec![1]]
    );
}

#[test]
fn public_fabric_constructor_metadata_matches_python_public_state() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let fabric = Fabric::new(&source);

    assert_eq!(fabric.path(), source.as_path());
    assert_eq!(fabric.banner(), BANNER);
    assert_eq!(fabric.version(), VERSION);
    assert!(fabric.good());
    assert_eq!(fabric.locations(), &[source.clone()]);
    assert_eq!(fabric.modules(), &[String::new()]);
    assert_eq!(fabric.location_rep(), source.display().to_string());
    assert_eq!(fabric.locationRep(), fabric.location_rep());
    assert!(fabric.features_requested().is_empty());
    assert!(fabric.featuresRequested().is_empty());
    assert!(fabric.features_ignored().is_empty());
    assert!(fabric.featuresIgnored().is_empty());

    let from_locations = Fabric::from_locations([source.clone()]);
    assert_eq!(from_locations.path(), source.as_path());
    assert_eq!(from_locations.locations(), &[source.clone()]);
    assert_eq!(from_locations.load_all().unwrap().max_node(), 8);

    let with_modules = Fabric::with_modules(&source, ["", "extra"]);
    assert_eq!(with_modules.path(), source.as_path());
    assert_eq!(
        with_modules.modules(),
        &[String::new(), "extra".to_string()]
    );
    assert_eq!(with_modules.load_all().unwrap().max_node(), 8);
}

#[test]
fn corpus_loading_reports_clean_errors_for_missing_or_empty_locations() {
    let missing_path = tempfile::tempdir().unwrap().path().join("missing-corpus");
    let missing_error = Corpus::load(&missing_path).unwrap_err();
    assert!(
        matches!(missing_error, CfError::Io { .. }),
        "missing corpus path should produce an I/O error, got {missing_error:?}"
    );

    let empty_dir = tempfile::tempdir().unwrap();
    let empty_error = Fabric::new(empty_dir.path()).load_all().unwrap_err();
    assert!(matches!(
        empty_error,
        CfError::MissingFeature(feature) if feature == "otype"
    ));
}

#[test]
fn selective_load_always_includes_warp_features_and_configs() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let mut corpus =
        Corpus::load_features(repo_path("libs/core/tests/fixtures/mini_corpus"), &["word"])
            .unwrap();

    assert!(corpus.node_feature("word").is_some());
    assert!(corpus.node_feature("otype").is_some());
    assert!(corpus.edge_feature("oslots").is_some());
    assert!(corpus.config_feature("otext").is_some());
    assert!(corpus.node_feature("pos").is_none());
    assert!(corpus.edge_feature("parent").is_none());

    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let results = MappedSearch::new(&mapped)
        .search("word word=hello", None)
        .unwrap();
    assert_eq!(results, vec![vec![1]]);

    let fabric = Fabric::new(&source);
    assert!(fabric.ensure_loaded(&mut corpus, "pos parent").unwrap());
    assert!(corpus.node_feature("pos").is_some());
    assert!(corpus.edge_feature("parent").is_some());
    assert!(fabric.ensureLoaded(&mut corpus, "relation").unwrap());
    assert!(corpus.edge_feature("relation").is_some());

    let footprint = corpus.footprint();
    assert_eq!(footprint["maxNode"], 8);
    assert!(footprint["nodes"] >= 3);
    assert!(footprint["edges"] >= 3);
    assert!(footprint["configs"] >= 1);
    assert_eq!(footprint["computed"], corpus.computed_names().len());
}

#[test]
fn oslots_items_returns_empty_when_there_are_no_non_slot_nodes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();

    assert_eq!(corpus.oslots_items(), Vec::<(u32, Vec<u32>)>::new());
}

#[test]
fn node_feature_selection_items_and_frequency_list() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let pos = corpus.node_feature("pos").unwrap();

    assert!(std::ptr::eq(pos, corpus.Fs("pos").unwrap()));
    assert!(corpus.Fs("missing").is_none());
    assert_eq!(pos.value_type(), Some("str"));
    assert_eq!(pos.valueType(), Some("str"));
    assert_eq!(pos.description(), Some("part of speech"));
    assert_eq!(pos.metadata_value("description"), Some("part of speech"));
    assert_eq!(
        pos.meta().get("valueType").and_then(Option::as_deref),
        Some("str")
    );
    assert_eq!(
        pos.data().get(&1),
        Some(&FeatureValue::string("interjection"))
    );
    assert_eq!(pos.str_value(1), Some("interjection"));
    assert_eq!(pos.v(3), Some(&FeatureValue::string("noun")));
    assert_eq!(pos.str_value(999), None);
    assert_eq!(pos.select(&FeatureValue::string("adjective")), vec![2, 4]);
    assert_eq!(pos.s(&FeatureValue::string("noun")), &[3, 5]);

    let items = pos.items();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0], (1, FeatureValue::string("interjection")));

    let frequencies = pos.frequency_list();
    assert_eq!(frequencies[0], (FeatureValue::string("adjective"), 2));
    assert_eq!(frequencies[1], (FeatureValue::string("noun"), 2));
    assert_eq!(pos.freq_list(), frequencies);
    assert_eq!(pos.freqList(), frequencies);

    assert_eq!(
        corpus.node_frequency_list("pos", Some(&["word"])).unwrap(),
        frequencies
    );
    assert_eq!(
        corpus.node_freq_list("pos", Some(&["word"])).unwrap(),
        frequencies
    );
    assert_eq!(
        corpus.node_freqList("pos", Some(&["word"])).unwrap(),
        frequencies
    );
    assert_eq!(
        corpus
            .node_frequency_list("pos", Some(&["phrase"]))
            .unwrap(),
        Vec::<(FeatureValue, usize)>::new()
    );
}

#[test]
fn node_feature_direct_edge_cases_match_python_unit_behaviors() {
    let empty = NodeFeature::new("empty".to_string(), BTreeMap::new(), HashMap::new());
    assert_eq!(empty.v(1), None);
    assert!(empty.items().is_empty());
    assert!(empty.s(&FeatureValue::string("missing")).is_empty());
    assert!(empty.freqList().is_empty());

    let values = HashMap::from([
        (0, FeatureValue::string("zero")),
        (1, FeatureValue::Int(100)),
        (1_000_000_000, FeatureValue::string("large")),
        (4, FeatureValue::string("common")),
        (2, FeatureValue::string("common")),
    ]);
    let feature = NodeFeature::new("direct".to_string(), BTreeMap::new(), values);

    assert_eq!(feature.v(0), Some(&FeatureValue::string("zero")));
    assert_eq!(feature.v(1), Some(&FeatureValue::Int(100)));
    assert_eq!(
        feature.v(1_000_000_000),
        Some(&FeatureValue::string("large"))
    );
    assert_eq!(feature.v(1_000_000_001), None);
    assert_eq!(
        feature.s(&FeatureValue::string("common")),
        &[2, 4],
        "selected nodes should be returned in node order"
    );
    assert_eq!(
        feature.filter_by_value(&[4, 2, 1, 999], &FeatureValue::string("common")),
        vec![4, 2],
        "candidate filtering should preserve caller order"
    );
    assert_eq!(
        feature.filter_by_values(
            &[0, 1, 2, 4, 999],
            &[FeatureValue::string("zero"), FeatureValue::Int(100)]
        ),
        vec![0, 1]
    );
    assert!(
        feature
            .filter_by_values(&[0, 1], &Vec::<FeatureValue>::new())
            .is_empty()
    );
    assert_eq!(feature.filter_has_value(&[0, 3, 999]), vec![0]);
    assert_eq!(feature.filter_missing_value(&[0, 3, 999]), vec![3, 999]);
    assert_eq!(
        feature.filter_by_value(&[1], &FeatureValue::string("100")),
        vec![1],
        "integer values should match compatible numeric string candidates like search constraints"
    );
    assert_eq!(
        feature.filter_less_than(&[0, 1, 2, 4], &FeatureValue::Int(101)),
        vec![1]
    );
    assert_eq!(
        feature.filter_greater_than(&[1], &FeatureValue::string("99")),
        vec![1]
    );
    assert_eq!(
        feature.items(),
        vec![
            (0, FeatureValue::string("zero")),
            (1, FeatureValue::Int(100)),
            (2, FeatureValue::string("common")),
            (4, FeatureValue::string("common")),
            (1_000_000_000, FeatureValue::string("large")),
        ]
    );
    assert_eq!(
        feature.frequency_list()[0],
        (FeatureValue::string("common"), 2)
    );
}

#[test]
fn edge_feature_forward_backward_both_and_count() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let parent = corpus.edge_feature("parent").unwrap();

    assert!(std::ptr::eq(parent, corpus.Es("parent").unwrap()));
    assert!(corpus.Es("missing").is_none());
    assert_eq!(parent.value_type(), Some("int"));
    assert_eq!(parent.valueType(), Some("int"));
    assert_eq!(parent.description(), Some("parent relationship"));
    assert_eq!(
        parent.meta().get("description").and_then(Option::as_deref),
        Some("parent relationship")
    );
    assert_eq!(parent.data().get(&1), Some(&vec![6]));
    assert_eq!(parent.data_inv().get(&6), Some(&vec![1, 2, 3]));
    assert_eq!(parent.dataInv().get(&6), Some(&vec![1, 2, 3]));
    assert_eq!(parent.forward(1), vec![6]);
    assert_eq!(parent.s(1), vec![6]);
    assert_eq!(parent.f(1), vec![6]);
    assert_eq!(parent.get_all_targets([1, 2, 7]), BTreeSet::from([6, 8]));
    assert_eq!(
        parent.get_all_targets([1, 99, 0]),
        BTreeSet::from([6]),
        "batch target lookup should ignore sources without outgoing rows"
    );
    assert_eq!(parent.get_all_targets(Vec::<u32>::new()), BTreeSet::new());
    assert_eq!(
        parent.filter_sources_with_targets_in([1, 2, 3, 4, 5], [6, 7]),
        (BTreeSet::from([1, 2, 3, 4, 5]), BTreeSet::from([6, 7]))
    );
    assert_eq!(
        parent.filter_sources_with_targets_in([1, 2], [99]),
        (BTreeSet::new(), BTreeSet::new())
    );
    assert_eq!(
        parent.filter_sources_with_targets_in([1], Vec::<u32>::new()),
        (BTreeSet::new(), BTreeSet::new())
    );
    assert_eq!(parent.backward(6), vec![1, 2, 3]);
    assert_eq!(parent.t(6), vec![1, 2, 3]);
    assert_eq!(parent.both(6), vec![1, 2, 3, 8]);
    assert_eq!(parent.b(6), vec![1, 2, 3, 8]);
    assert_eq!(parent.forward(999), Vec::<u32>::new());
    let parent_items = parent.items();
    assert_eq!(parent_items.len(), 7);
    let mut parent_sources = vec![1, 2, 3, 4, 5, 6, 7];
    corpus.sort_nodes(&mut parent_sources);
    assert_eq!(parent_items[0].0, parent_sources[0]);
    assert_eq!(parent_items[6].0, parent_sources[6]);
    assert_eq!(parent.edge_count(), 7);
    assert_eq!(parent.edge_value_count(), 0);
    assert!(!parent.has_edge_values());
    assert!(!parent.hasEdgeValues());
    let relation = corpus.edge_feature("relation").unwrap();
    assert_eq!(relation.edge_value_count(), 5);
    assert!(relation.has_edge_values());
    assert!(relation.hasEdgeValues());
    assert_eq!(
        relation.forward_with_values(1),
        vec![(6, Some(FeatureValue::string("subject")))]
    );
    assert_eq!(
        relation.data_with_values().get(&1),
        Some(&vec![(6, Some(FeatureValue::string("subject")))])
    );
    assert_eq!(
        relation.dataInvWithValues().get(&6),
        Some(&vec![
            (1, Some(FeatureValue::string("subject"))),
            (2, Some(FeatureValue::string("predicate"))),
            (3, Some(FeatureValue::string("object"))),
        ])
    );
    assert_eq!(
        relation.itemsWithValues()[0],
        (1, vec![(6, Some(FeatureValue::string("subject")))])
    );
    let oslots = corpus.edge_feature("oslots").unwrap();
    assert_eq!(oslots.s(8), vec![1, 2, 3, 4, 5]);
    assert_eq!(oslots.s(1), vec![1]);
    let mut embedders_for_slot_1 = vec![6, 8];
    corpus.sort_nodes(&mut embedders_for_slot_1);
    assert_eq!(oslots.t(1), embedders_for_slot_1);
    assert_eq!(
        oslots.items().into_iter().find(|(source, _)| *source == 8),
        Some((8, vec![1, 2, 3, 4, 5]))
    );

    assert_eq!(
        corpus.edge_frequency_list("parent", None, None).unwrap(),
        EdgeFrequency::Count(7)
    );
    assert_eq!(parent.frequency_list(), EdgeFrequency::Count(7));
    assert_eq!(parent.freq_list(), EdgeFrequency::Count(7));
    assert_eq!(parent.freqList(), EdgeFrequency::Count(7));
    assert_eq!(
        corpus
            .edge_frequency_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap(),
        EdgeFrequency::Count(5)
    );
    assert_eq!(
        corpus
            .edge_freq_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap(),
        EdgeFrequency::Count(5)
    );
    assert_eq!(
        corpus
            .edge_freqList("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap(),
        EdgeFrequency::Count(5)
    );
    assert_eq!(
        corpus
            .edge_frequency_list("parent", Some(&["phrase"]), Some(&["sentence"]))
            .unwrap(),
        EdgeFrequency::Count(2)
    );

    assert_eq!(
        corpus.edge_frequency_list("distance", None, None).unwrap(),
        EdgeFrequency::Values(vec![
            (Some(FeatureValue::Int(0)), 3),
            (None, 3),
            (Some(FeatureValue::Int(5)), 1),
            (Some(FeatureValue::Int(10)), 1),
        ])
    );
    assert_eq!(
        corpus.edge_feature("distance").unwrap().frequency_list(),
        EdgeFrequency::Values(vec![
            (Some(FeatureValue::Int(0)), 3),
            (None, 3),
            (Some(FeatureValue::Int(5)), 1),
            (Some(FeatureValue::Int(10)), 1),
        ])
    );
}

#[test]
fn edge_feature_handles_empty_and_self_referential_edges() {
    let empty = EdgeFeature::new(
        "empty".to_string(),
        BTreeMap::new(),
        HashMap::from([(1, Vec::<u32>::new())]),
    );

    assert_eq!(empty.forward(1), Vec::<u32>::new());
    assert_eq!(empty.f(1), Vec::<u32>::new());
    assert_eq!(empty.backward(1), Vec::<u32>::new());
    assert_eq!(empty.t(1), Vec::<u32>::new());
    assert_eq!(empty.both(1), Vec::<u32>::new());
    assert_eq!(empty.items(), vec![(1, Vec::<u32>::new())]);
    assert_eq!(empty.edge_count(), 0);
    assert_eq!(empty.frequency_list(), EdgeFrequency::Count(0));

    let self_edge = EdgeFeature::new(
        "self".to_string(),
        BTreeMap::new(),
        HashMap::from([(1, vec![1])]),
    );

    assert_eq!(self_edge.forward(1), vec![1]);
    assert_eq!(self_edge.backward(1), vec![1]);
    assert_eq!(self_edge.both(1), vec![1]);
    assert_eq!(self_edge.data_inv().get(&1), Some(&vec![1]));
    assert_eq!(self_edge.items(), vec![(1, vec![1])]);
    assert_eq!(self_edge.edge_count(), 1);
    assert_eq!(self_edge.frequency_list(), EdgeFrequency::Count(1));
}

#[test]
fn edge_feature_direct_edges_are_set_like_as_in_python_units() {
    let feature = EdgeFeature::new(
        "set_like".to_string(),
        BTreeMap::new(),
        HashMap::from([(1, vec![3, 2, 2, 3]), (4, vec![5])]),
    );

    assert_eq!(feature.forward(1), vec![2, 3]);
    assert_eq!(feature.f(999), Vec::<u32>::new());
    assert_eq!(feature.backward(2), vec![1]);
    assert_eq!(feature.t(999), Vec::<u32>::new());
    assert_eq!(feature.both(1), vec![2, 3]);
    assert_eq!(feature.both(999), Vec::<u32>::new());
    assert_eq!(feature.items(), vec![(1, vec![2, 3]), (4, vec![5])]);
    assert_eq!(feature.edge_count(), 3);
    assert_eq!(feature.frequency_list(), EdgeFrequency::Count(3));
}

#[test]
fn valued_edge_both_prefers_outgoing_value_for_bidirectional_pair() {
    let values = HashMap::from([(1, vec![2]), (2, vec![1])]);
    let edge_values = HashMap::from([
        ((1, 2), FeatureValue::string("out")),
        ((2, 1), FeatureValue::string("in")),
    ]);
    let feature = EdgeFeature::new_with_values(
        "relation".to_string(),
        BTreeMap::from([
            ("edgeValues".to_string(), None),
            ("valueType".to_string(), Some("str".to_string())),
        ]),
        values,
        edge_values,
    );

    assert_eq!(
        feature.both_with_values(1),
        vec![(2, Some(FeatureValue::string("out")))]
    );
}

#[test]
fn corpus_node_ordering_and_locality_navigation() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let locality = Locality::new(&corpus);
    let node_api = Nodes::new(&corpus);

    let mut nodes = vec![3, 1, 2];
    corpus.sort_nodes(&mut nodes);
    assert_eq!(nodes, vec![1, 2, 3]);
    assert_eq!(corpus.sorted_nodes([3, 1, 2]), vec![1, 2, 3]);
    assert_eq!(
        corpus.sortNodes(BTreeMap::from([(5, ()), (3, ()), (1, ())]).into_keys()),
        vec![1, 3, 5]
    );
    assert_eq!(corpus.sortNodes(Vec::<u32>::new()), Vec::<u32>::new());
    assert_eq!(corpus.sortNodes([5]), vec![5]);
    assert_eq!(corpus.sortNodes([8, 6, 1, 7]), vec![8, 6, 1, 7]);
    assert_eq!(node_api.sortNodes([8, 6, 1, 7]), vec![8, 6, 1, 7]);
    assert_eq!(corpus.all_nodes(), vec![8, 6, 1, 2, 3, 7, 4, 5]);
    let otype_rank = corpus.otype_rank();
    assert_eq!(otype_rank.get("word"), Some(&0));
    assert!(otype_rank["word"] < otype_rank["phrase"]);
    assert!(otype_rank["phrase"] < otype_rank["sentence"]);
    assert_eq!(corpus.otypeRank(), otype_rank);
    assert_eq!(node_api.otypeRank(), otype_rank);
    assert!(corpus.sort_key(8) < corpus.sort_key(6));
    assert!(corpus.sort_key(6) < corpus.sort_key(1));
    assert!(corpus.sort_key(1) < corpus.sort_key(2));
    assert_eq!(corpus.sortKey(1), corpus.sort_key(1));
    assert_eq!(node_api.sortKey(1), corpus.sort_key(1));
    assert_eq!(
        node_api.sortKeyTuple(&[8, 6, 1]),
        corpus.sortKeyTuple(&[8, 6, 1])
    );

    assert_eq!(corpus.up(1, Some("phrase")), vec![6]);
    assert_eq!(locality.up(1, Some("phrase")), corpus.up(1, Some("phrase")));
    assert_eq!(corpus.u(1, Some("phrase")), corpus.up(1, Some("phrase")));
    assert_eq!(locality.u(1, Some("phrase")), corpus.u(1, Some("phrase")));
    assert_eq!(corpus.up(1, Some("sentence")), vec![8]);
    assert_eq!(
        corpus.up_types(1, Some(&["phrase", "sentence"])),
        vec![6, 8]
    );
    assert_eq!(
        locality.up_types(1, Some(&["phrase", "sentence"])),
        corpus.up_types(1, Some(&["phrase", "sentence"]))
    );
    assert_eq!(
        corpus.u_types(1, Some(&["phrase", "sentence"])),
        corpus.up_types(1, Some(&["phrase", "sentence"]))
    );
    assert_eq!(corpus.u(8, None), corpus.up(8, None));
    assert_eq!(corpus.up(8, None), Vec::<u32>::new());

    // L.i returns intersectors in canonical ascending rank order (matches TF
    // `tf/core/locality.py` `i`).
    assert_eq!(corpus.intersecting(6, None), vec![8, 1, 2, 3]);
    assert_eq!(locality.intersecting(6, None), corpus.intersecting(6, None));
    assert_eq!(corpus.i(6, None), corpus.intersecting(6, None));
    assert_eq!(locality.i(6, None), corpus.i(6, None));
    assert_eq!(corpus.i(6, Some("sentence")), vec![8]);
    assert_eq!(corpus.i(6, Some("word")), vec![1, 2, 3]);
    assert_eq!(corpus.intersecting(1, None), Vec::<u32>::new());
    assert_eq!(
        corpus.intersecting_types(8, Some(&["phrase", "word"])),
        vec![6, 1, 2, 3, 7, 4, 5]
    );

    assert_eq!(corpus.down(6, Some("word")), vec![1, 2, 3]);
    assert_eq!(locality.down(6, Some("word")), corpus.down(6, Some("word")));
    assert_eq!(corpus.d(6, Some("word")), corpus.down(6, Some("word")));
    assert_eq!(locality.d(6, Some("word")), corpus.d(6, Some("word")));
    assert_eq!(corpus.down(7, Some("word")), vec![4, 5]);
    assert_eq!(
        corpus.down_types(8, Some(&["phrase", "word"])),
        vec![6, 1, 2, 3, 7, 4, 5]
    );
    assert_eq!(
        corpus.d_types(8, Some(&["phrase", "word"])),
        corpus.down_types(8, Some(&["phrase", "word"]))
    );
    assert_eq!(corpus.down(1, None), Vec::<u32>::new());
    assert_eq!(corpus.d(1, None), corpus.down(1, None));

    assert_eq!(corpus.next(3, None), vec![4, 7]);
    assert_eq!(locality.next(3, None), corpus.next(3, None));
    assert_eq!(corpus.n(3, None), corpus.next(3, None));
    assert_eq!(locality.n(3, None), corpus.n(3, None));
    assert_eq!(corpus.previous(4, None), vec![6, 3]);
    assert_eq!(locality.previous(4, None), corpus.previous(4, None));
    assert_eq!(corpus.p(4, None), corpus.previous(4, None));
    assert_eq!(locality.p(4, None), corpus.p(4, None));
    assert_eq!(corpus.next(5, None), Vec::<u32>::new());
    assert_eq!(corpus.previous(1, None), Vec::<u32>::new());
    assert_eq!(corpus.next(1, Some("word")), vec![2]);
    assert_eq!(corpus.n(1, Some("word")), corpus.next(1, Some("word")));
    assert_eq!(corpus.previous(5, Some("word")), vec![4]);
    assert_eq!(corpus.p(5, Some("word")), corpus.previous(5, Some("word")));
    assert_eq!(corpus.next(6, Some("phrase")), vec![7]);
    assert_eq!(corpus.previous(7, Some("phrase")), vec![6]);
    assert_eq!(corpus.next_types(3, Some(&["phrase", "word"])), vec![4, 7]);
    assert_eq!(
        locality.next_types(3, Some(&["phrase", "word"])),
        corpus.next_types(3, Some(&["phrase", "word"]))
    );
    assert_eq!(
        corpus.previous_types(4, Some(&["phrase", "word"])),
        vec![6, 3]
    );
    assert_eq!(
        locality.previous_types(4, Some(&["phrase", "word"])),
        corpus.previous_types(4, Some(&["phrase", "word"]))
    );
}

#[test]
fn walks_nodes_and_boundary_events_in_canonical_order() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();

    assert_eq!(corpus.walk(None), vec![8, 6, 1, 2, 3, 7, 4, 5]);
    assert_eq!(corpus.allNodes(), corpus.all_nodes());
    assert_eq!(corpus.walk(Some(&[3, 1, 8])), vec![8, 1, 3]);

    let events = corpus.walk_events(None);
    assert_eq!(corpus.walkEvents(None), events);
    assert_eq!(
        events,
        vec![
            WalkEvent::Start(8),
            WalkEvent::Start(6),
            WalkEvent::Slot(1),
            WalkEvent::Slot(2),
            WalkEvent::Slot(3),
            WalkEvent::End(6),
            WalkEvent::Start(7),
            WalkEvent::Slot(4),
            WalkEvent::Slot(5),
            WalkEvent::End(7),
            WalkEvent::End(8),
        ]
    );

    assert_eq!(
        corpus.walk_events(Some(&[8, 1, 5])),
        vec![
            WalkEvent::Start(8),
            WalkEvent::Slot(1),
            WalkEvent::Slot(5),
            WalkEvent::End(8),
        ]
    );
    assert_eq!(
        corpus.walkEvents(Some(&[8, 1, 5])),
        corpus.walk_events(Some(&[8, 1, 5]))
    );
}

#[test]
fn sorts_chunks_with_python_navigation_keys() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let chunks = vec![
        Chunk {
            node: 1,
            start: 1,
            end: 1,
        },
        Chunk {
            node: 6,
            start: 1,
            end: 3,
        },
        Chunk {
            node: 8,
            start: 1,
            end: 5,
        },
        Chunk {
            node: 7,
            start: 4,
            end: 5,
        },
        Chunk {
            node: 4,
            start: 4,
            end: 4,
        },
    ];

    let mut by_position = chunks.clone();
    by_position.sort_by_key(|chunk| corpus.sort_key_chunk(*chunk));
    let mut by_position_alias = chunks.clone();
    by_position_alias.sort_by_key(|chunk| corpus.sortKeyChunk(*chunk));
    assert_eq!(by_position_alias, by_position);
    assert_eq!(
        by_position
            .iter()
            .map(|chunk| chunk.node)
            .collect::<Vec<_>>(),
        vec![8, 6, 1, 7, 4]
    );

    let mut by_length = chunks.clone();
    by_length.sort_by_key(|chunk| corpus.sort_key_chunk_length(*chunk));
    let mut by_length_alias = chunks;
    by_length_alias.sort_by_key(|chunk| corpus.sortKeyChunkLength(*chunk));
    assert_eq!(by_length_alias, by_length);
    assert_eq!(
        by_length.iter().map(|chunk| chunk.node).collect::<Vec<_>>(),
        vec![8, 6, 7, 1, 4]
    );
}

#[test]
fn mapped_compiled_sorts_chunks_with_materialized_navigation_keys() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let chunks = vec![
        Chunk {
            node: 1,
            start: 1,
            end: 1,
        },
        Chunk {
            node: 6,
            start: 1,
            end: 3,
        },
        Chunk {
            node: 8,
            start: 1,
            end: 5,
        },
        Chunk {
            node: 7,
            start: 4,
            end: 5,
        },
        Chunk {
            node: 4,
            start: 4,
            end: 4,
        },
    ];

    let mut mapped_by_position = chunks.clone();
    mapped_by_position.sort_by_key(|chunk| mapped.sort_key_chunk(*chunk).unwrap());
    let mut mapped_by_position_alias = chunks.clone();
    mapped_by_position_alias.sort_by_key(|chunk| mapped.sortKeyChunk(*chunk).unwrap());
    assert_eq!(mapped_by_position_alias, mapped_by_position);
    let mut parsed_by_position = chunks.clone();
    parsed_by_position.sort_by_key(|chunk| parsed.sort_key_chunk(*chunk));
    assert_eq!(mapped_by_position, parsed_by_position);

    let mut mapped_by_length = chunks.clone();
    mapped_by_length.sort_by_key(|chunk| mapped.sort_key_chunk_length(*chunk).unwrap());
    let mut mapped_by_length_alias = chunks.clone();
    mapped_by_length_alias.sort_by_key(|chunk| mapped.sortKeyChunkLength(*chunk).unwrap());
    assert_eq!(mapped_by_length_alias, mapped_by_length);
    let mut parsed_by_length = chunks;
    parsed_by_length.sort_by_key(|chunk| parsed.sort_key_chunk_length(*chunk));
    assert_eq!(mapped_by_length, parsed_by_length);
}

#[test]
fn exposes_feature_lists_and_computed_data() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();

    let overview = corpus.overview("mini");
    assert_eq!(overview.name, "mini");
    assert_eq!(overview.section_types, vec!["sentence", "phrase"]);
    assert_eq!(overview.node_types.len(), 3);
    assert_eq!(overview.node_types[0].node_type, "sentence");
    assert_eq!(overview.node_types[0].count, 1);
    assert!(!overview.node_types[0].is_slot_type);
    assert_eq!(overview.node_types[2].node_type, "word");
    assert_eq!(overview.node_types[2].count, 5);
    assert!(overview.node_types[2].is_slot_type);
    assert_eq!(corpus.section_types(), vec!["sentence", "phrase"]);
    let overview_json = overview.to_dict();
    assert_eq!(overview_json["name"], serde_json::json!("mini"));
    assert_eq!(
        overview_json["node_types"][0]["type"],
        serde_json::json!("sentence")
    );
    assert_eq!(
        overview_json["node_types"][2]["is_slot_type"],
        serde_json::json!(true)
    );
    assert_eq!(
        overview_json["sections"]["levels"],
        serde_json::json!(["sentence", "phrase"])
    );
    serde_json::to_string(&overview_json).unwrap();

    let description = corpus.describe_corpus("mini");
    assert_eq!(description.name, "mini");
    assert_eq!(description.node_types, overview.node_types);
    assert_eq!(description.section_types, vec!["sentence", "phrase"]);
    assert_eq!(
        description.text_representations.description,
        "No text format metadata available or no orig/trans pairs defined"
    );
    assert!(
        description
            .node_features
            .iter()
            .any(|entry| entry.name == "word" && entry.kind == FeatureKind::Node)
    );
    assert!(
        description
            .edge_features
            .iter()
            .any(|entry| entry.name == "parent" && entry.kind == FeatureKind::Edge)
    );
    assert!(
        !description
            .node_features
            .iter()
            .any(|entry| entry.kind == FeatureKind::Edge)
    );
    assert!(
        !description
            .edge_features
            .iter()
            .any(|entry| entry.kind == FeatureKind::Node)
    );
    let description_json = description.to_dict();
    assert_eq!(description_json["name"], serde_json::json!("mini"));
    assert_eq!(
        description_json["text_representations"]["description"],
        serde_json::json!("No text format metadata available or no orig/trans pairs defined")
    );
    assert!(
        description_json["features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == serde_json::json!("word")
                && entry["kind"] == serde_json::json!("node"))
    );
    assert!(
        description_json["edge_features"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == serde_json::json!("parent")
                && entry["kind"] == serde_json::json!("edge"))
    );
    serde_json::to_string(&description_json).unwrap();

    let node_features = corpus.node_feature_names();
    assert!(node_features.contains(&"otype"));
    assert!(node_features.contains(&"word"));

    let edge_features = corpus.edge_feature_names();
    assert!(edge_features.contains(&"oslots"));
    assert!(edge_features.contains(&"parent"));

    assert!(corpus.Fall(true).contains(&"otype".to_string()));
    assert!(!corpus.Fall(false).contains(&"otype".to_string()));
    assert!(corpus.Fall(false).contains(&"word".to_string()));
    assert!(corpus.Eall(true).contains(&"oslots".to_string()));
    assert!(!corpus.Eall(false).contains(&"oslots".to_string()));
    assert!(corpus.Eall(false).contains(&"parent".to_string()));
    assert_eq!(
        corpus.Call(),
        vec![
            "levels".to_string(),
            "order".to_string(),
            "rank".to_string(),
            "boundary".to_string()
        ]
    );

    let loaded = corpus.is_loaded(Some(&[
        "word",
        "parent",
        "otext",
        "levels",
        "missing_feature",
    ]));
    assert_eq!(
        corpus.isLoaded(Some(&[
            "word",
            "parent",
            "otext",
            "levels",
            "missing_feature",
        ])),
        loaded
    );
    let word_info = loaded.get("word").unwrap().as_ref().unwrap();
    assert_eq!(word_info.kind, LoadedFeatureKind::Node);
    assert_eq!(word_info.value_type, "str");
    assert_eq!(
        word_info
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("word text")
    );
    assert_eq!(word_info.edge_values, None);

    let parent_info = loaded.get("parent").unwrap().as_ref().unwrap();
    assert_eq!(parent_info.kind, LoadedFeatureKind::Edge);
    assert_eq!(parent_info.edge_values, Some(false));

    let otext_info = loaded.get("otext").unwrap().as_ref().unwrap();
    assert_eq!(otext_info.kind, LoadedFeatureKind::Config);
    assert_eq!(otext_info.edge_values, None);

    let levels_info = loaded.get("levels").unwrap().as_ref().unwrap();
    assert_eq!(levels_info.kind, LoadedFeatureKind::Computed);
    assert_eq!(levels_info.value_type, "");
    assert!(loaded.get("missing_feature").unwrap().is_none());

    let all_loaded = corpus.is_loaded(None);
    assert_eq!(corpus.isLoaded(None), all_loaded);
    assert_eq!(
        all_loaded.get("oslots").unwrap().as_ref().unwrap().kind,
        LoadedFeatureKind::Edge
    );
    assert_eq!(all_loaded.get("missing_feature"), None);

    assert_eq!(corpus.node_feature_types("word"), vec!["word"]);
    assert_eq!(corpus.node_feature_types("phrase_id"), vec!["phrase"]);
    assert_eq!(corpus.node_feature_types("missing"), Vec::<&str>::new());
    let all_node_feature_types = corpus.all_node_feature_types();
    assert_eq!(
        all_node_feature_types.get("word"),
        Some(&vec!["word".to_string()])
    );
    assert_eq!(
        all_node_feature_types.get("phrase_id"),
        Some(&vec!["phrase".to_string()])
    );
    assert_eq!(all_node_feature_types.get("missing"), None);
    assert_eq!(
        corpus.edge_feature_source_types("parent"),
        vec!["phrase", "word"]
    );
    assert_eq!(
        corpus.edge_feature_target_types("parent"),
        vec!["sentence", "phrase"]
    );
    assert_eq!(
        corpus.edge_feature_source_types("oslots"),
        vec!["sentence", "phrase"]
    );
    assert_eq!(corpus.edge_feature_target_types("oslots"), vec!["word"]);
    assert_eq!(
        corpus.edge_feature_source_types("missing"),
        Vec::<&str>::new()
    );
    assert_eq!(
        corpus.edge_feature_target_types("missing"),
        Vec::<&str>::new()
    );
    let all_edge_source_types = corpus.all_edge_feature_source_types();
    assert_eq!(
        all_edge_source_types.get("parent"),
        Some(&vec!["phrase".to_string(), "word".to_string()])
    );
    assert_eq!(
        all_edge_source_types.get("oslots"),
        Some(&vec!["sentence".to_string(), "phrase".to_string()])
    );
    let all_edge_target_types = corpus.all_edge_feature_target_types();
    assert_eq!(
        all_edge_target_types.get("parent"),
        Some(&vec!["sentence".to_string(), "phrase".to_string()])
    );
    assert_eq!(
        all_edge_target_types.get("oslots"),
        Some(&vec!["word".to_string()])
    );

    let node_catalog = corpus.feature_catalog(Some(FeatureKind::Node), None);
    assert!(node_catalog.iter().any(|entry| {
        entry.name == "word"
            && entry.kind == FeatureKind::Node
            && entry.value_type == "str"
            && entry.description == "word text"
    }));
    assert!(
        !node_catalog
            .iter()
            .any(|entry| entry.kind == FeatureKind::Edge)
    );

    let word_catalog = corpus.feature_catalog(None, Some(&["word"]));
    assert!(
        word_catalog
            .iter()
            .any(|entry| entry.name == "word" && entry.kind == FeatureKind::Node)
    );
    assert!(
        word_catalog
            .iter()
            .any(|entry| entry.name == "parent" && entry.kind == FeatureKind::Edge)
    );
    assert!(!word_catalog.iter().any(|entry| entry.name == "phrase_id"));

    let word_description = corpus.describe_feature("pos", 1);
    assert_eq!(word_description.name, "pos");
    assert_eq!(word_description.kind, Some(FeatureKind::Node));
    assert_eq!(word_description.value_type, "str");
    assert_eq!(word_description.description, "part of speech");
    assert_eq!(word_description.node_types, vec!["word"]);
    assert_eq!(word_description.unique_values, 3);
    assert_eq!(word_description.sample_values.len(), 1);
    assert_eq!(
        word_description.sample_values[0].value,
        Some(FeatureValue::string("adjective"))
    );
    assert_eq!(word_description.sample_values[0].count, 2);
    assert_eq!(word_description.has_values, None);
    assert_eq!(word_description.error, None);
    let word_description_json = word_description.to_dict();
    assert_eq!(word_description_json["kind"], serde_json::json!("node"));
    assert_eq!(
        word_description_json["node_types"],
        serde_json::json!(["word"])
    );
    assert_eq!(
        word_description_json["sample_values"][0]["value"],
        serde_json::json!("adjective")
    );
    serde_json::to_string(&word_description_json).unwrap();

    let parent_description = corpus.describe_feature("parent", 5);
    assert_eq!(parent_description.kind, Some(FeatureKind::Edge));
    assert_eq!(parent_description.has_values, Some(false));
    assert_eq!(parent_description.unique_values, 0);
    assert!(parent_description.sample_values.is_empty());

    let distance_description = corpus.describe_feature("distance", 2);
    assert_eq!(distance_description.kind, Some(FeatureKind::Edge));
    assert_eq!(distance_description.has_values, Some(true));
    assert_eq!(distance_description.unique_values, 4);
    assert_eq!(distance_description.sample_values.len(), 2);
    assert_eq!(
        distance_description.sample_values[0].value,
        Some(FeatureValue::Int(0))
    );
    assert_eq!(distance_description.sample_values[0].count, 3);

    let missing_description = corpus.describe_feature("missing", 5);
    assert_eq!(missing_description.kind, None);
    assert_eq!(
        missing_description.error,
        Some("Feature 'missing' not found".to_string())
    );
    let missing_description_json = missing_description.to_dict();
    assert_eq!(
        missing_description_json["kind"],
        serde_json::json!("unknown")
    );
    assert_eq!(
        missing_description_json["error"],
        serde_json::json!("Feature 'missing' not found")
    );
    assert!(missing_description_json.get("node_types").is_none());
    assert!(missing_description_json.get("sample_values").is_none());
    serde_json::to_string(&missing_description_json).unwrap();

    let descriptions = corpus.describe_features(&["word", "distance", "missing"], 1);
    assert_eq!(descriptions.len(), 3);
    assert_eq!(descriptions["word"].kind, Some(FeatureKind::Node));
    assert_eq!(descriptions["word"].sample_values.len(), 1);
    assert_eq!(descriptions["distance"].kind, Some(FeatureKind::Edge));
    assert_eq!(descriptions["distance"].sample_values.len(), 1);
    assert_eq!(
        descriptions["missing"].error,
        Some("Feature 'missing' not found".to_string())
    );

    let text_representations = corpus.text_representations();
    assert_eq!(
        text_representations.description,
        "No text format metadata available or no orig/trans pairs defined"
    );
    assert!(text_representations.formats.is_empty());

    let computed = corpus.computed_names();
    assert!(computed.contains(&"levels"));
    assert!(computed.contains(&"order"));
    assert!(computed.contains(&"rank"));
    assert!(computed.contains(&"boundary"));

    let levels = corpus.levels();
    let level_types: Vec<_> = levels
        .iter()
        .map(|(node_type, _, _, _)| *node_type)
        .collect();
    assert_eq!(level_types, vec!["sentence", "phrase", "word"]);
    assert_eq!(levels[0], ("sentence", 5.0, 8, 8));
    assert_eq!(levels[1], ("phrase", 2.5, 6, 7));
    assert_eq!(levels[2], ("word", 1.0, 1, 5));
    assert_eq!(
        corpus.Cs("levels"),
        Some(ComputedFeatureData::Levels(vec![
            ("sentence".to_string(), 5.0, 8, 8),
            ("phrase".to_string(), 2.5, 6, 7),
            ("word".to_string(), 1.0, 1, 5),
        ]))
    );

    let order = corpus.order();
    assert_eq!(order, vec![8, 6, 1, 2, 3, 7, 4, 5]);
    assert_eq!(
        corpus.computed_feature("order"),
        Some(ComputedFeatureData::Order(order.clone()))
    );

    let rank = corpus.rank();
    assert_eq!(rank.len(), 8);
    for (index, node) in order.iter().enumerate() {
        assert_eq!(rank[(*node - 1) as usize], index as u32);
    }
    assert_eq!(
        corpus.Cs("rank"),
        Some(ComputedFeatureData::Rank(rank.clone()))
    );

    let boundary = corpus.boundary();
    assert_eq!(boundary.first_slots.len(), 5);
    assert_eq!(boundary.last_slots.len(), 5);
    assert_eq!(boundary.first_slots[0], vec![6, 8]);
    assert_eq!(boundary.first_slots[3], vec![7]);
    assert_eq!(boundary.last_slots[2], vec![6]);
    assert_eq!(boundary.last_slots[4], vec![8, 7]);
    assert_eq!(corpus.boundary(), boundary);
    assert_eq!(
        corpus.Cs("boundary"),
        Some(ComputedFeatureData::Boundary(boundary))
    );
    assert_eq!(corpus.Cs("missing"), None);

    assert_eq!(corpus.sort_key_tuple(&[1, 2, 3]), vec![2, 3, 4]);
    assert_eq!(
        corpus.sortKeyTuple(&[1, 2, 3]),
        corpus.sort_key_tuple(&[1, 2, 3])
    );
}

#[test]
fn describes_paired_text_representation_formats() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();
    fs::write(
        dir.path().join("orig.tf"),
        "@node\n@valueType=str\n\nאב\nאג\nאב\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("trans.tf"),
        "@node\n@valueType=str\n\nab\nag\nab\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("otext.tf"),
        "@config\n@fmt:text-orig-full={orig}\n@fmt:text-trans-full={trans}\n\n",
    )
    .unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();
    let representations = corpus.text_representations();

    assert_eq!(
        corpus.split_format("word#{orig}{trans}"),
        ("word".to_string(), "{orig}{trans}".to_string())
    );
    assert_eq!(
        corpus.splitFormat("unknown#{orig}"),
        ("word".to_string(), "unknown#{orig}".to_string())
    );
    assert_eq!(
        corpus.split_format("{orig}"),
        ("word".to_string(), "{orig}".to_string())
    );
    assert_eq!(
        corpus.split_default_format("word-default"),
        Some("word".to_string())
    );
    assert_eq!(corpus.splitDefaultFormat("unknown-default"), None);
    assert_eq!(corpus.split_default_format("word-text"), None);

    assert_eq!(
        representations.description,
        "Shows how text values are encoded in this corpus. Samples provide exhaustive character coverage for understanding the relationship between original script and transliterated forms."
    );
    assert_eq!(representations.formats.len(), 1);
    let format = &representations.formats[0];
    assert_eq!(format.name, "text-full");
    assert_eq!(format.original_spec, "{orig}");
    assert_eq!(format.transliteration_spec, "{trans}");
    assert_eq!(format.unique_characters, 3);
    assert_eq!(format.total_samples, 2);
    assert_eq!(
        format.samples,
        vec![
            context_fabric_core::TextFormatSample {
                original: "אב".to_string(),
                transliterated: "ab".to_string(),
            },
            context_fabric_core::TextFormatSample {
                original: "אג".to_string(),
                transliterated: "ag".to_string(),
            },
        ]
    );
}

#[test]
fn text_formats_support_python_fallback_defaults_and_layout_escapes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\nhello\nworld\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("otext.tf"),
        "@config\n@fmt:text-orig-full={word}\n@fmt:fallback={missing/word:.}\\t{missing:.}\\n{missing:}\n\n",
    )
    .unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();
    assert_eq!(corpus.text(1, Some("fallback")), "hello\t.\n");
    assert_eq!(corpus.text(2, Some("fallback")), "world\t.\n");
    assert_eq!(
        corpus.text(3, Some("fallback")),
        "hello\t.\nworld\t.\n",
        "non-slot text should apply the same format to each contained slot"
    );

    let cache_path = dir.path().join("text-defaults.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_text = MappedText::new(&mapped).unwrap();
    assert_eq!(
        mapped_text.text(1, Some("fallback")).unwrap(),
        corpus.text(1, Some("fallback"))
    );
    assert_eq!(
        mapped_text.text(3, Some("fallback")).unwrap(),
        corpus.text(3, Some("fallback"))
    );
}

#[test]
fn text_respects_node_default_target_formats_and_descend_option() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\nhello\nworld\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("label.tf"),
        "@node\n@valueType=str\n\n3\tP1\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("otext.tf"),
        "@config\n@fmt:text-orig-full={word} \n@fmt:phrase-default=phrase#{label}\n@fmt:phrase-label=phrase#{label}\n\n",
    )
    .unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();
    assert_eq!(corpus.text(3, None), "P1");
    assert_eq!(corpus.text(3, Some("phrase-label")), "P1");
    assert_eq!(
        corpus.text_with_options(3, &TextOptions::new(Some("phrase-label"), Some(true))),
        "P1"
    );
    assert_eq!(
        corpus.text_with_options(3, &TextOptions::new(Some("text-orig-full"), Some(false))),
        " "
    );
    assert_eq!(
        corpus.text_with_options(3, &TextOptions::new(Some("text-orig-full"), Some(true))),
        "hello world "
    );
    assert_eq!(
        Text::new(&corpus).textWithOptions(3, &TextOptions::new(None::<String>, None)),
        "P1"
    );

    let cache_path = dir.path().join("targeted-text.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_text = MappedText::new(&mapped).unwrap();
    assert_eq!(mapped_text.text(3, None).unwrap(), corpus.text(3, None));
    assert_eq!(
        mapped_text
            .text_with_options(3, &TextOptions::new(Some("text-orig-full"), Some(true)))
            .unwrap(),
        corpus.text_with_options(3, &TextOptions::new(Some("text-orig-full"), Some(true)))
    );
}

#[test]
fn wraps_nodes_and_search_results_for_serializable_result_shapes() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let options = NodeInfoOptions {
        include_slots: true,
        include_features: vec!["word".to_string(), "pos".to_string()],
        ..NodeInfoOptions::default()
    };

    let slot = NodeInfo::from_corpus(&corpus, 1, &options);
    assert_eq!(slot.node, 1);
    assert_eq!(slot.node_type, "word");
    assert_eq!(slot.text, "hello");
    assert_eq!(slot.section_ref, "S1 1");
    assert_eq!(slot.slots, None);
    let slot_features = slot.features.as_ref().unwrap();
    assert_eq!(
        slot_features.get("word"),
        Some(&FeatureValue::string("hello"))
    );
    assert_eq!(
        slot_features.get("pos"),
        Some(&FeatureValue::string("interjection"))
    );
    let slot_json = slot.to_dict();
    assert_eq!(slot_json["node"], serde_json::json!(1));
    assert_eq!(slot_json["otype"], serde_json::json!("word"));
    assert!(slot_json.get("node_type").is_none());
    assert!(slot_json.get("slots").is_none());
    assert_eq!(slot_json["features"]["word"], serde_json::json!("hello"));
    serde_json::to_string(&slot_json).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&slot.to_json()).unwrap(),
        slot_json
    );
    assert_eq!(slot.toJson(), slot.to_json());

    let phrase = NodeInfo::from_corpus(&corpus, 6, &options);
    assert_eq!(phrase.node_type, "phrase");
    assert_eq!(phrase.text, "hellobeautifulworld");
    assert_eq!(phrase.slots, Some(vec![1, 2, 3]));
    assert_eq!(phrase.features, None);

    let node_list = NodeList::from_nodes(
        &corpus,
        &[8, 6, 1],
        Some(2),
        Some("subset".to_string()),
        &NodeInfoOptions::default(),
    );
    assert_eq!(node_list.total_count, 3);
    assert_eq!(node_list.query.as_deref(), Some("subset"));
    assert_eq!(
        node_list.text,
        "hellobeautifulworldgoodmorninghellobeautifulworld"
    );
    assert_eq!(
        node_list
            .nodes
            .iter()
            .map(|node| node.node)
            .collect::<Vec<_>>(),
        vec![8, 6]
    );
    let node_list_json = node_list.to_dict();
    assert_eq!(node_list_json["total_count"], serde_json::json!(3));
    assert_eq!(node_list_json["query"], serde_json::json!("subset"));
    assert_eq!(node_list_json["nodes"][0]["node"], serde_json::json!(8));
    assert!(node_list_json.get("text").is_none());
    serde_json::to_string(&node_list_json).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&node_list.to_json()).unwrap(),
        node_list_json
    );
    assert_eq!(node_list.toJson(), node_list.to_json());

    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let raw_results = MappedSearch::new(&mapped)
        .search("word word=hello", None)
        .unwrap();
    let result = SearchResult::from_search(
        &corpus,
        &raw_results,
        "word word=hello",
        Some(1),
        &NodeInfoOptions::default(),
    );
    assert_eq!(result.total_count, 1);
    assert_eq!(result.template, "word word=hello");
    assert_eq!(result.plan, None);
    assert_eq!(result.results[0][0].node, 1);
    assert_eq!(result.results[0][0].node_type, "word");
    let result_json = result.to_dict();
    assert_eq!(result_json["total_count"], serde_json::json!(1));
    assert_eq!(
        result_json["template"],
        serde_json::json!("word word=hello")
    );
    assert_eq!(result_json["results"][0][0]["node"], serde_json::json!(1));
    assert_eq!(
        result_json["results"][0][0]["otype"],
        serde_json::json!("word")
    );
    serde_json::to_string(&result_json).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result.to_json()).unwrap(),
        result_json
    );
    assert_eq!(result.toJson(), result.to_json());

    let word_info = FeatureInfo::from_corpus(&corpus, "word", FeatureKind::Node).unwrap();
    assert_eq!(word_info.name, "word");
    assert_eq!(word_info.kind, FeatureKind::Node);
    assert_eq!(word_info.value_type, "str");
    assert_eq!(word_info.description, "word text");
    let word_json = word_info.to_dict();
    assert_eq!(word_json["name"], serde_json::json!("word"));
    assert_eq!(word_json["kind"], serde_json::json!("node"));
    assert_eq!(word_json["value_type"], serde_json::json!("str"));
    assert_eq!(word_info.has_values, None);
    assert!(word_json.get("has_values").is_none());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&word_info.to_json()).unwrap(),
        word_json
    );
    assert_eq!(word_info.toJson(), word_info.to_json());

    let distance_info = FeatureInfo::from_corpus(&corpus, "distance", FeatureKind::Edge).unwrap();
    assert_eq!(distance_info.name, "distance");
    assert_eq!(distance_info.kind, FeatureKind::Edge);
    assert_eq!(distance_info.value_type, "int");
    assert_eq!(
        distance_info.description,
        "distance between nodes (tests None vs 0 values)"
    );
    assert_eq!(distance_info.has_values, Some(true));

    assert_eq!(
        FeatureInfo::from_corpus(&corpus, "word", FeatureKind::Edge),
        None
    );

    let corpus_info = CorpusInfo::from_corpus(&corpus, "mini", "fixtures/mini_corpus");
    assert_eq!(corpus_info.name, "mini");
    assert_eq!(corpus_info.path, "fixtures/mini_corpus");
    assert_eq!(corpus_info.slot_type, "word");
    assert_eq!(corpus_info.max_slot, 5);
    assert_eq!(corpus_info.max_node, 8);
    let corpus_json = corpus_info.to_dict();
    assert_eq!(corpus_json["name"], serde_json::json!("mini"));
    assert_eq!(corpus_json["max_node"], serde_json::json!(8));
    assert_eq!(corpus_json["slot_type"], serde_json::json!("word"));
    serde_json::to_string(&corpus_json).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&corpus_info.to_json()).unwrap(),
        corpus_json
    );
    assert_eq!(corpus_info.toJson(), corpus_info.to_json());
    assert_eq!(corpus_info.section_types, vec!["sentence", "phrase"]);
    assert!(corpus_info.node_features.contains(&"word".to_string()));
    assert!(corpus_info.edge_features.contains(&"parent".to_string()));
    assert_eq!(corpus_info.node_types[0].node_type, "sentence");
    assert_eq!(
        corpus_json["node_types"][0]["type"],
        serde_json::json!("sentence")
    );
    assert_eq!(
        corpus_json["node_types"][0]["avg_slots"],
        serde_json::json!(5.0)
    );
    assert!(corpus_json["node_types"][0].get("node_type").is_none());
    assert!(corpus_json["node_types"][0].get("average_slots").is_none());
    assert_eq!(corpus_info.node_types[0].count, 1);
    assert_eq!(corpus_info.node_types[0].average_slots, 5.0);
    assert_eq!(corpus_info.node_types[0].min_node, 8);
    assert_eq!(corpus_info.node_types[0].max_node, 8);
    assert_eq!(corpus_info.node_types[2].node_type, "word");
    assert_eq!(corpus_info.node_types[2].count, 5);
    assert_eq!(corpus_info.node_types[2].average_slots, 1.0);
    assert_eq!(corpus_info.node_types[2].min_node, 1);
    assert_eq!(corpus_info.node_types[2].max_node, 5);
}

#[test]
fn node_info_omits_large_non_slot_text_by_default() {
    let temp_dir = tempfile::tempdir().unwrap();
    let corpus_dir = temp_dir.path().join("large_text");
    fs::create_dir(&corpus_dir).unwrap();

    let words = (1..=101)
        .map(|index| format!("w{index}"))
        .collect::<Vec<_>>();
    let mut otype = String::from("@node\n@valueType=str\n\n");
    for _ in 1..=101 {
        otype.push_str("word\n");
    }
    otype.push_str("phrase\n");
    fs::write(corpus_dir.join("otype.tf"), otype).unwrap();
    fs::write(
        corpus_dir.join("oslots.tf"),
        "@edge\n@valueType=int\n\n102\t1-101\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("word.tf"),
        format!(
            "@node\n@valueType=str\n@description=word text\n\n{}\n",
            words.join("\n")
        ),
    )
    .unwrap();
    fs::write(
        corpus_dir.join("otext.tf"),
        "@config\n@fmt:text-orig-full={word}\n\n",
    )
    .unwrap();

    let corpus = Corpus::load(&corpus_dir).unwrap();
    let omitted = NodeInfo::from_corpus(&corpus, 102, &NodeInfoOptions::default());
    assert_eq!(omitted.text, "[101 slots - text omitted]");

    let full_options = NodeInfoOptions {
        max_text_slots: None,
        ..NodeInfoOptions::default()
    };
    let rendered = NodeInfo::from_corpus(&corpus, 102, &full_options);
    assert_eq!(rendered.text, words.concat());

    let cache_path = temp_dir.path().join("large_text.cfr");
    compile_features(&corpus_dir, &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_text = MappedText::new(&mapped).unwrap();
    let mapped_sections = MappedSections::new(&mapped).unwrap();
    assert_eq!(
        NodeInfo::from_mapped(
            &mapped_text,
            &mapped_sections,
            102,
            &NodeInfoOptions::default(),
        )
        .unwrap()
        .text,
        "[101 slots - text omitted]"
    );
    assert_eq!(
        NodeInfo::from_mapped(&mapped_text, &mapped_sections, 102, &full_options)
            .unwrap()
            .text,
        words.concat()
    );
}

#[test]
fn supports_feature_constraints_regex_limits_and_containment() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    assert_eq!(search.glean(&[]).unwrap(), "");
    assert_eq!(search.glean(&[1]).unwrap(), "hello");
    assert_eq!(search.glean(&[1, 2, 3]).unwrap(), "hellobeautifulworld");

    let hello = search.search("word word=hello", None).unwrap();
    assert_eq!(hello, vec![vec![1]]);
    assert_eq!(
        search
            .search("word\nword=hello\npos=interjection", None)
            .unwrap(),
        hello,
        "feature continuation lines should extend the previous atom like Python search"
    );

    let hello_with_comment = search
        .search(
            "
% leading comment
word word=hello
",
            None,
        )
        .unwrap();
    assert_eq!(hello_with_comment, hello);

    let nouns_and_adjectives = search.search("word pos=noun|adjective", None).unwrap();
    assert_eq!(
        nouns_and_adjectives,
        vec![vec![2], vec![3], vec![4], vec![5]]
    );

    let not_nouns = search.search("word pos#noun", None).unwrap();
    assert_eq!(not_nouns, vec![vec![1], vec![2], vec![4]]);

    let missing_pos = search.search("phrase pos#", None).unwrap();
    assert_eq!(missing_pos, vec![vec![6], vec![7]]);

    let has_pos = search.search("word pos*", None).unwrap();
    assert_eq!(has_pos.len(), 5);
    assert_eq!(search.search("word pos", None).unwrap(), has_pos);
    assert!(search.search("phrase pos", None).unwrap().is_empty());

    let low_numbers = search.search("word number<3", None).unwrap();
    assert_eq!(low_numbers, vec![vec![1], vec![2], vec![4], vec![5]]);
    assert_eq!(
        search
            .search("word\nnumber<3\npos=adjective", None)
            .unwrap(),
        vec![vec![2], vec![4]]
    );

    let high_numbers = search.search("word number>2", None).unwrap();
    assert_eq!(high_numbers, vec![vec![3]]);

    let h_words = search.search("word word~^h", None).unwrap();
    assert_eq!(h_words, vec![vec![1]]);
    assert_eq!(
        search.search("w:word word=hello\nw", None).unwrap(),
        vec![vec![1, 1]]
    );

    let limited = search.search("word", Some(2)).unwrap();
    assert_eq!(limited, vec![vec![1], vec![2]]);

    let phrase_words = search.search("phrase\n  word", None).unwrap();
    assert_eq!(phrase_words.len(), 5);
    assert!(phrase_words.contains(&vec![6, 1]));
    assert!(phrase_words.contains(&vec![7, 5]));
    // A lonely operator as a first child is rejected by the mapped engine,
    // mirroring TF ("Lonely relation: not allowed as first child"). The
    // operator-prefixed atom form is the correct way to express containment.
    assert!(search.search("phrase\n  [[\n  word", None).is_err());
    assert_eq!(search.search("phrase\n  [[ word", None).unwrap(), phrase_words);
    assert!(
        search
            .search(
                "
word
  ]]
  phrase
",
                None
            )
            .is_err()
    );

    let all_nodes = search.search(".", None).unwrap();
    assert_eq!(
        all_nodes,
        vec![
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![6],
            vec![7],
            vec![8]
        ]
    );
    assert_eq!(
        search.search(". pos=noun", None).unwrap(),
        vec![vec![3], vec![5]]
    );
    let generic_embedders = search
        .search(
            "
p:.
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(generic_embedders.len(), 10);
    assert!(generic_embedders.contains(&vec![6, 1]));
    assert!(generic_embedders.contains(&vec![7, 5]));
    assert!(generic_embedders.contains(&vec![8, 5]));
}

#[test]
fn supports_feature_relation_operators_between_named_nodes() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let same_pos = search
        .search(
            "
w1:word word=world
w2:word word=morning
w1 .pos. w2
",
            None,
        )
        .unwrap();
    assert_eq!(same_pos, vec![vec![3, 5]]);

    let different_pos = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .pos#pos. w2
",
            None,
        )
        .unwrap();
    assert_eq!(different_pos, vec![vec![1, 3]]);

    let regex_normalized_words = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .word~.+~word. w2
",
            None,
        )
        .unwrap();
    assert_eq!(regex_normalized_words, vec![vec![1, 3]]);

    let regex_normalized_words_mismatch = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .word~^h~word. w2
",
            None,
        )
        .unwrap();
    assert!(regex_normalized_words_mismatch.is_empty());

    let number_less_than = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 .number<number. w2
",
            None,
        )
        .unwrap();
    assert_eq!(number_less_than, vec![vec![1, 3]]);

    let number_greater_than = search
        .search(
            "
w1:word word=world
w2:word word=hello
w1 .number>number. w2
",
            None,
        )
        .unwrap();
    assert_eq!(number_greater_than, vec![vec![3, 1]]);

    let cross_feature = search
        .search(
            "
w1:word word=hello
w2:word word=beautiful
w1 .number=score. w2
",
            None,
        )
        .unwrap();
    assert_eq!(cross_feature, Vec::<Vec<u32>>::new());
}

#[test]
fn supports_materialized_edge_relations_with_value_constraints() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let subject = search
        .search(
            "
w:word
p:phrase
w -relation=subject> p
",
            None,
        )
        .unwrap();
    assert_eq!(subject, vec![vec![1, 6], vec![4, 7]]);

    let predicate_backward = search
        .search(
            "
p:phrase
w:word
p <relation=predicate- w
",
            None,
        )
        .unwrap();
    assert_eq!(predicate_backward, vec![vec![6, 2], vec![7, 5]]);

    let zero_distance = search
        .search(
            "
w1:word
w2:word
w1 -distance=0> w2
",
            None,
        )
        .unwrap();
    assert_eq!(zero_distance, vec![vec![1, 2], vec![3, 4]]);

    let missing_distance_value = search
        .search(
            "
w1:word
w2:word
w1 -distance=5> w2
",
            None,
        )
        .unwrap();
    assert_eq!(missing_distance_value, vec![vec![1, 3]]);
}

#[test]
fn search_supports_python_style_escaped_spaces_in_values() {
    let temp_dir = tempfile::tempdir().unwrap();
    let corpus_dir = temp_dir.path();
    fs::write(
        corpus_dir.join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1,2\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("word.tf"),
        "@node\n@valueType=str\n\ngood morning\nplain\n",
    )
    .unwrap();
    fs::write(
        corpus_dir.join("relation.tf"),
        "@edge\n@edgeValues\n@valueType=str\n\n1\t3\tmain phrase\n",
    )
    .unwrap();

    let cache_path = corpus_dir.join("escaped.cfr");
    compile_features(
        corpus_dir,
        &cache_path,
        &["otype", "oslots", "word", "relation"],
    )
    .unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_search = MappedSearch::new(&mapped);
    assert_eq!(
        mapped_search
            .search("word word=good\\ morning", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        mapped_search
            .search("word\nword=good\\ morning", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        mapped_search
            .search("word word~good\\ morning", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        mapped_search
            .search(
                "
w:word
p:phrase
w -relation=main\\ phrase> p
",
                None,
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
}

#[test]
fn supports_custom_search_sets() {
    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let mut sets = HashMap::new();
    sets.insert("mywords", vec![1, 3]);
    assert_eq!(
        search.search_with_sets("mywords", &sets, None).unwrap(),
        vec![vec![1], vec![3]]
    );

    let mut sets = HashMap::new();
    sets.insert("w", corpus.nodes_of_type("word").to_vec());
    assert_eq!(
        search.search_with_sets("w pos=noun", &sets, None).unwrap(),
        vec![vec![3], vec![5]]
    );

    let mut sets = HashMap::new();
    sets.insert("empty", Vec::new());
    assert!(
        search
            .search_with_sets("empty", &sets, None)
            .unwrap()
            .is_empty()
    );

    let mut sets = HashMap::new();
    sets.insert("single", vec![1]);
    assert_eq!(
        search.search_with_sets("single", &sets, None).unwrap(),
        vec![vec![1]]
    );

    let mut sets = HashMap::new();
    sets.insert("mywords", vec![2, 1]);
    sets.insert("myphrases", vec![6]);
    let custom_relation = search
        .search_with_sets(
            "
w:mywords
p:myphrases
w ]] p
",
            &sets,
            None,
        )
        .unwrap();
    assert_eq!(custom_relation, vec![vec![1, 6], vec![2, 6]]);
}

#[test]
fn supports_studied_search_fetch_and_count_workflow() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let study = search.study("word").unwrap();
    assert_eq!(study.template(), "word");
    assert_eq!(study.plan().template, "word");
    assert_eq!(study.plan().atom_count, 1);
    assert_eq!(study.plan().relation_count, 0);
    assert_eq!(study.plan().result_count, 5);
    assert!(!study.plan().has_quantifiers);
    assert_eq!(
        study.show_plan(false),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert_eq!(study.showPlan(false), study.show_plan(false));
    assert!(study.show_plan(true).contains("  1: [1]"));
    assert_eq!(
        search.show_plan("word", false).unwrap(),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert_eq!(
        search.showPlan("word", false).unwrap(),
        search.show_plan("word", false).unwrap()
    );
    let relation_legend = search.relations_legend();
    assert!(relation_legend.contains("[[ embeds"));
    assert!(relation_legend.contains("-edge=value> forward valued edge"));
    assert!(relation_legend.contains(".feature~regex~feature."));
    assert_eq!(search.relationsLegend(), relation_legend);
    assert_eq!(study.total_count(), 5);
    assert_eq!(study.count(None), 5);
    assert_eq!(study.count(Some(2)), 2);
    assert_eq!(study.fetch(Some(2)), vec![vec![1], vec![2]]);
    assert_eq!(
        search.fetch("word", Some(2)).unwrap(),
        vec![vec![1], vec![2]]
    );
    assert_eq!(search.count("word", None).unwrap(), 5);
    assert_eq!(search.count("word", Some(2)).unwrap(), 2);
    assert_eq!(study.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(study.fetch_first_nodes(Some(0)), Vec::<u32>::new());
    assert_eq!(study.fetch_first_nodes(Some(2)), vec![1, 2]);
    assert_eq!(study.count_first_nodes(Some(3)), 3);
    assert_eq!(
        search.search_first_nodes("word", Some(2)).unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        search.searchFirstNodes("word", Some(2)).unwrap(),
        search.search_first_nodes("word", Some(2)).unwrap()
    );
    assert!(!study.is_empty());

    let empty = search.study("word word=missing").unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.count(None), 0);

    let mut sets = HashMap::new();
    sets.insert("mywords", vec![2, 1]);
    let custom = search.study_with_sets("mywords", &sets).unwrap();
    assert_eq!(custom.fetch(None), vec![vec![1], vec![2]]);
    assert_eq!(
        search.fetch_with_sets("mywords", &sets, Some(1)).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(search.count_with_sets("mywords", &sets, None).unwrap(), 2);
    assert_eq!(
        search.count_with_sets("mywords", &sets, Some(1)).unwrap(),
        1
    );
    assert_eq!(
        search
            .search_first_nodes_with_sets("mywords", &sets, None)
            .unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        search
            .searchFirstNodesWithSets("mywords", &sets, None)
            .unwrap(),
        search
            .search_first_nodes_with_sets("mywords", &sets, None)
            .unwrap()
    );
    assert_eq!(
        search
            .search_prefixes_with_sets("mywords", &sets, 1, Some(1))
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .searchPrefixesWithSets("mywords", &sets, 1, Some(1))
            .unwrap(),
        search
            .search_prefixes_with_sets("mywords", &sets, 1, Some(1))
            .unwrap()
    );
    assert_eq!(
        search.show_plan_with_sets("mywords", &sets, false).unwrap(),
        "template: mywords\natoms: 1\nrelations: 0\nresults: 2\nquantifiers: false"
    );
    assert_eq!(
        search.showPlanWithSets("mywords", &sets, false).unwrap(),
        search.show_plan_with_sets("mywords", &sets, false).unwrap()
    );

    let related = search
        .study(
            "
w:word
p:phrase
w ]] p
",
        )
        .unwrap();
    assert_eq!(related.plan().atom_count, 2);
    assert_eq!(related.plan().relation_count, 1);
    assert_eq!(related.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(
        related.fetch_prefixes(1, None),
        vec![vec![1], vec![2], vec![3], vec![4], vec![5]]
    );
    assert_eq!(
        related.fetch_prefixes(2, Some(2)),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search
            .search_prefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap(),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search
            .searchPrefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap(),
        search
            .search_prefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap()
    );
    assert_eq!(related.fetch_prefixes(0, None), Vec::<Vec<u32>>::new());
    assert_eq!(related.fetch_prefixes(1, Some(0)), Vec::<Vec<u32>>::new());
    assert_eq!(related.count_prefixes(2, None), 5);

    let phrase_children = search
        .study(
            "
p:phrase
w:word
w ]] p
",
        )
        .unwrap();
    assert_eq!(phrase_children.fetch_first_nodes(None), vec![6, 7]);
    assert_eq!(phrase_children.fetch_prefixes(1, Some(1)), vec![vec![6]]);

    let quantified = search
        .study(
            "
phrase
/with/
  word pos=noun
/-/
",
        )
        .unwrap();
    assert_eq!(quantified.plan().atom_count, 2);
    assert_eq!(quantified.plan().relation_count, 0);
    assert!(quantified.plan().has_quantifiers);
}

#[test]
fn supports_escaped_query_value_literals() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nword\nword\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n7\t1-6\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\na|b\na=b\na\\b\na#b\na<b\na>b\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("relation.tf"),
        "@edge\n@valueType=str\n@edgeValues\n\n1\t7\ta|b\n2\t7\ta=b\n3\t7\ta\\b\n4\t7\ta#b\n5\t7\ta<b\n6\t7\ta>b\n",
    )
    .unwrap();

    let cache_path = dir.path().join("escaped.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);
    assert_eq!(
        search.search(r"word word=a\|b", None).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.search(r"word word=a\=b", None).unwrap(),
        vec![vec![2]]
    );
    assert_eq!(
        search.search(r"word word=a\\b", None).unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search.search(r"word word=a\#b", None).unwrap(),
        vec![vec![4]]
    );
    assert_eq!(
        search.search(r"word word=a\<b", None).unwrap(),
        vec![vec![5]]
    );
    assert_eq!(
        search.search(r"word word=a\>b", None).unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search.search(r"word word#a\#b", None).unwrap(),
        vec![vec![1], vec![2], vec![3], vec![5], vec![6]]
    );
    assert_eq!(
        search.search(r"word word=missing|a\|b", None).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\|b> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
p:phrase
w:word
p <relation=a\=b- w
",
                None
            )
            .unwrap(),
        vec![vec![7, 2]]
    );
    assert_eq!(
        search
            .search(
                r"
p:phrase
w:word
p <relation=a\\b> w
",
                None
            )
            .unwrap(),
        vec![vec![7, 3]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\#b> p
",
                None
            )
            .unwrap(),
        vec![vec![4, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\<b> p
",
                None
            )
            .unwrap(),
        vec![vec![5, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=a\>b> p
",
                None
            )
            .unwrap(),
        vec![vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation=missing|a\|b> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation#a\#b> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7], vec![2, 7], vec![3, 7], vec![5, 7], vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                r"
w:word
p:phrase
w -relation~^a[<>]b$> p
",
                None
            )
            .unwrap(),
        vec![vec![5, 7], vec![6, 7]]
    );
}

#[test]
fn supports_negative_integer_query_values_like_python_syntax() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("score.tf"),
        "@node\n@valueType=int\n\n-1\n2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("distance.tf"),
        "@edge\n@valueType=int\n@edgeValues\n\n1\t2\t-5\n2\t1\t3\n",
    )
    .unwrap();

    let cache_path = dir.path().join("negative.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);
    assert_eq!(search.search("word score=-1", None).unwrap(), vec![vec![1]]);
    assert_eq!(search.search("word score<0", None).unwrap(), vec![vec![1]]);
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 -distance=-5> w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 2]]
    );
}

#[test]
fn supports_named_atom_constraints_and_basic_relations() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let before = search
        .search(
            "
w1:word
w2:word
w1 < w2
w1 word=hello
w2 word=world
",
            None,
        )
        .unwrap();
    assert_eq!(before, vec![vec![1, 3, 1, 3]]);

    let after = search
        .search(
            "
w1:word
w2:word
w1 > w2
w1 word=world
w2 word=hello
",
            None,
        )
        .unwrap();
    assert_eq!(after, vec![vec![3, 1, 3, 1]]);

    let different_nouns = search
        .search(
            "
w1:word
w2:word
w1 # w2
w1 pos=noun
w2 pos=noun
",
            None,
        )
        .unwrap();
    assert_eq!(different_nouns, vec![vec![3, 5, 3, 5], vec![5, 3, 5, 3]]);

    let same_nodes = search
        .search(
            "
p1:phrase
p2:phrase
p1 = p2
",
            None,
        )
        .unwrap();
    assert_eq!(same_nodes, vec![vec![6, 6], vec![7, 7]]);

    let same_slots = search
        .search(
            "
s:sentence
s2:sentence
s == s2
",
            None,
        )
        .unwrap();
    assert_eq!(same_slots, vec![vec![8, 8]]);

    let different_slots = search
        .search(
            "
p1:phrase
p2:phrase
p1 ## p2
",
            None,
        )
        .unwrap();
    assert!(different_slots.contains(&vec![6, 7]));
    assert!(different_slots.contains(&vec![7, 6]));
    assert!(!different_slots.contains(&vec![6, 6]));

    let overlapping = search
        .search(
            "
s:sentence
p:phrase
s && p
",
            None,
        )
        .unwrap();
    assert_eq!(overlapping, vec![vec![8, 6], vec![8, 7]]);

    let disjoint = search
        .search(
            "
p1:phrase
p2:phrase
p1 || p2
",
            None,
        )
        .unwrap();
    assert_eq!(disjoint, vec![vec![6, 7], vec![7, 6]]);

    let slot_before = search
        .search(
            "
p1:phrase
p2:phrase
p1 << p2
",
            None,
        )
        .unwrap();
    assert_eq!(slot_before, vec![vec![6, 7]]);

    let slot_after = search
        .search(
            "
p1:phrase
p2:phrase
p1 >> p2
",
            None,
        )
        .unwrap();
    assert_eq!(slot_after, vec![vec![7, 6]]);

    let adjacent_after = search
        .search(
            "
p1:phrase
p2:phrase
p1 :> p2
",
            None,
        )
        .unwrap();
    assert_eq!(adjacent_after, vec![vec![7, 6]]);

    let same_first_slot = search
        .search(
            "
s:sentence
p:phrase
s =: p
",
            None,
        )
        .unwrap();
    assert_eq!(same_first_slot, vec![vec![8, 6]]);

    let same_last_slot = search
        .search(
            "
s:sentence
p:phrase
s := p
",
            None,
        )
        .unwrap();
    assert_eq!(same_last_slot, vec![vec![8, 7]]);

    let same_boundary = search
        .search(
            "
p1:phrase
p2:phrase
p1 :: p2
",
            None,
        )
        .unwrap();
    assert_eq!(same_boundary, vec![vec![6, 6], vec![7, 7]]);

    let near_first_slot = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 =2: w2
",
            None,
        )
        .unwrap();
    assert_eq!(near_first_slot, vec![vec![1, 3]]);

    let not_near_first_slot = search
        .search(
            "
w1:word word=hello
w2:word word=world
w1 =1: w2
",
            None,
        )
        .unwrap();
    assert!(not_near_first_slot.is_empty());

    let near_last_slot = search
        .search(
            "
s:sentence
p:phrase phrase_id=1
s :2= p
",
            None,
        )
        .unwrap();
    assert_eq!(near_last_slot, vec![vec![8, 6]]);

    let near_boundary = search
        .search(
            "
s:sentence
p:phrase phrase_id=1
s :2: p
",
            None,
        )
        .unwrap();
    assert_eq!(near_boundary, vec![vec![8, 6]]);

    let near_before = search
        .search(
            "
p1:phrase phrase_id=1
p2:phrase phrase_id=2
p1 <0: p2
",
            None,
        )
        .unwrap();
    assert_eq!(near_before, vec![vec![6, 7]]);

    let near_after = search
        .search(
            "
p1:phrase phrase_id=2
p2:phrase phrase_id=1
p1 :0> p2
",
            None,
        )
        .unwrap();
    assert_eq!(near_after, vec![vec![7, 6]]);

    let phrase_contains_word = search
        .search(
            "
p:phrase
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(phrase_contains_word.len(), 5);
    assert!(phrase_contains_word.contains(&vec![6, 1]));
    assert!(phrase_contains_word.contains(&vec![7, 5]));

    let explicit_phrase_contains_word = search
        .search(
            "
p:phrase
  [[ w:word
",
            None,
        )
        .unwrap();
    assert_eq!(explicit_phrase_contains_word, phrase_contains_word);

    let word_in_phrase = search
        .search(
            "
w:word
p:phrase
w ]] p
",
            None,
        )
        .unwrap();
    assert_eq!(word_in_phrase.len(), 5);
    assert!(word_in_phrase.contains(&vec![1, 6]));
    assert!(word_in_phrase.contains(&vec![5, 7]));

    // Under TF two-edge semantics the operator-prefixed `]] p:phrase` keeps the
    // indentation embedding (phrase embedded in the single-slot word, which is
    // impossible) alongside the operator edge, so it yields no results. The flat
    // relation form `w ]] p` is the correct way to express "word in phrase".
    let explicit_word_in_phrase = search
        .search(
            "
w:word
  ]] p:phrase
",
            None,
        )
        .unwrap();
    assert!(explicit_word_in_phrase.is_empty());

    let adjacent_words = search
        .search(
            "
w1:word
w2:word
w1 <: w2
",
            None,
        )
        .unwrap();
    assert_eq!(
        adjacent_words,
        vec![vec![1, 2], vec![2, 3], vec![3, 4], vec![4, 5]]
    );
    // A lonely `<:` operator as a first child is rejected, mirroring TF
    // ("Lonely relation: not allowed as first child").
    assert!(
        search
            .search(
                "
w1:word
  <:
  w2:word
",
                None
            )
            .is_err()
    );

    let parent_forward = search
        .search(
            "
w:word
p:phrase
w -parent> p
",
            None,
        )
        .unwrap();
    assert_eq!(parent_forward.len(), 5);
    assert!(parent_forward.contains(&vec![1, 6]));
    assert!(parent_forward.contains(&vec![5, 7]));
    // A lonely edge operator as a first child is rejected, mirroring TF
    // ("Lonely relation: not allowed as first child").
    assert!(
        search
            .search(
                "
w:word
  -parent>
  p:phrase
",
                None
            )
            .is_err()
    );

    let parent_backward = search
        .search(
            "
p:phrase
w:word
p <parent- w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_backward.len(), 5);
    assert!(parent_backward.contains(&vec![6, 1]));
    assert!(parent_backward.contains(&vec![7, 5]));

    let parent_either = search
        .search(
            "
p:phrase
w:word
p <parent> w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_either.len(), 5);
    assert!(parent_either.contains(&vec![6, 1]));
    assert!(parent_either.contains(&vec![7, 5]));

    let valued_either = search
        .search(
            "
p:phrase
w:word
p <relation=subject> w
",
            None,
        )
        .unwrap();
    assert_eq!(valued_either, vec![vec![6, 1], vec![7, 4]]);
}

#[test]
fn supports_with_and_without_quantified_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    let with_interjection = search
        .search(
            "
phrase
/with/
  % comment inside quantified block
  word pos=interjection
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(with_interjection, vec![vec![6]]);

    let without_interjection = search
        .search(
            "
phrase
/without/
  word pos=interjection
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(without_interjection, vec![vec![7]]);

    let with_two_top_level_children = search
        .search(
            "
sentence
/with/
  phrase phrase_id=1
  phrase phrase_id=2
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(with_two_top_level_children, vec![vec![8]]);

    let where_have = search
        .search(
            "
sentence
/where/
  word word=hello
/have/
  word word=world
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(where_have, vec![vec![8]]);

    let where_have_is_universal_not_existential = search
        .search(
            "
phrase
/where/
  w:word pos=adjective
/have/
  w number=1
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(where_have_is_universal_not_existential, vec![vec![7]]);

    let with_or_alternatives = search
        .search(
            "
phrase
/with/
  word word=hello
/or/
  word word=good
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(with_or_alternatives, vec![vec![6], vec![7]]);

    let parent_ref_embeds = search
        .search(
            "
p:phrase
/with/
  .. [[ w
  w:word word=hello
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_embeds, vec![vec![6]]);

    let parent_ref_embedded_in = search
        .search(
            "
p:phrase
/with/
  w:word word=hello
  w ]] ..
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_embedded_in, vec![vec![6]]);

    let parent_ref_atom_constraint = search
        .search(
            "
phrase
/with/
  .. phrase_id=1
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_atom_constraint, vec![vec![6]]);

    let parent_ref_relation_before = search
        .search(
            "
p:phrase
/with/
  .. < w
  w:word word=hello
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_relation_before, vec![vec![6]]);

    let parent_ref_relation_not_equal = search
        .search(
            "
p:phrase
/with/
  w:word word=hello
  .. # w
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_relation_not_equal, vec![vec![6]]);

    let parent_ref_feature_relation = search
        .search(
            "
p:phrase
/with/
  w:word word=hello
  .. .phrase_id=number. w
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(parent_ref_feature_relation, vec![vec![6]]);
}

// Mini corpus (see layout note above): words 1..5 are slots; phrase 6 = slots
// 1-3, phrase 7 = slots 4-5, sentence 8 = slots 1-5. Edge `parent`: 1..3 -> 6,
// 4..5 -> 7, 6/7 -> 8. Edge `relation` (valued): 1->6 subject, 2->6 predicate,
// 3->6 object, 4->7 subject, 5->7 predicate.
#[test]
fn mapped_edge_relations_drive_from_edge_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // Forward edge `-parent>`: word -> phrase via the `parent` edge.
    let mut forward = search
        .search("w:word\np:phrase\nw -parent> p", None)
        .unwrap();
    forward.sort_unstable();
    assert_eq!(
        forward,
        vec![vec![1, 6], vec![2, 6], vec![3, 6], vec![4, 7], vec![5, 7]]
    );

    // Backward edge `<parent-`: phrase <- word (same pairs, reversed columns).
    let mut backward = search
        .search("p:phrase\nw:word\np <parent- w", None)
        .unwrap();
    backward.sort_unstable();
    assert_eq!(
        backward,
        vec![vec![6, 1], vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );

    // Valued forward edge `-relation=subject>`: only the `subject` edges.
    let mut valued = search
        .search("w:word\np:phrase\nw -relation=subject> p", None)
        .unwrap();
    valued.sort_unstable();
    assert_eq!(valued, vec![vec![1, 6], vec![4, 7]]);
}

#[test]
fn mapped_embedding_drives_both_directions() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `[[` container relation: phrase embeds word. Exercises reverse-embedding
    // driving (container found from a bound contained word) and the slot index.
    let mut embeds = search.search("p:phrase\nw:word\np [[ w", None).unwrap();
    embeds.sort_unstable();
    assert_eq!(
        embeds,
        vec![vec![6, 1], vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );

    // `]]` embedded-in: word embedded in phrase (reversed columns).
    let mut embedded_in = search.search("w:word\np:phrase\nw ]] p", None).unwrap();
    embedded_in.sort_unstable();
    assert_eq!(
        embedded_in,
        vec![vec![1, 6], vec![2, 6], vec![3, 6], vec![4, 7], vec![5, 7]]
    );

    // A slot embeds nothing: a generic container `.` of a word must never match the
    // word itself, only the genuine non-slot embedders.
    let mut generic = search.search("p:.\nw:word\np [[ w", None).unwrap();
    generic.sort_unstable();
    assert_eq!(
        generic,
        vec![
            vec![6, 1],
            vec![6, 2],
            vec![6, 3],
            vec![7, 4],
            vec![7, 5],
            vec![8, 1],
            vec![8, 2],
            vec![8, 3],
            vec![8, 4],
            vec![8, 5],
        ]
    );
}

#[test]
fn mapped_quantifier_attaches_to_preceding_atom_not_trailing_base() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // The quantifier modifies `sentence` (the atom before `/with/`), NOT the
    // trailing base atom `phrase`. Sentence 8 contains the interjection word 1, so
    // it qualifies, and the base then pairs it with each child phrase.
    let mut result = search
        .search(
            "
sentence
/with/
  word pos=interjection
/-/
  phrase
",
            None,
        )
        .unwrap();
    result.sort_unstable();
    assert_eq!(result, vec![vec![8, 6], vec![8, 7]]);
}

#[test]
fn mapped_operator_prefixed_parent_reference_emits_relation() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `&& ..` is an operator-prefixed reference to the quantifier's parent: the
    // sibling phrase must overlap the sentence. Sentence 8 has overlapping phrases,
    // so it is retained (this used to bind nothing and return empty).
    let result = search
        .search(
            "
sentence
/with/
phrase
&& ..
/-/
",
            None,
        )
        .unwrap();
    assert_eq!(result, vec![vec![8]]);
}

#[test]
fn mapped_different_slots_with_shared_start() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&dir);
    let search = MappedSearch::new(&mapped);

    // `=:` shares the first slot (driveable window) and `##` requires different
    // slot sets: sentence 8 (slots 1-5) and phrase 6 (slots 1-3) share slot 1.
    let result = search
        .search("s:sentence\n=: p:phrase\ns ## p", None)
        .unwrap();
    assert_eq!(result, vec![vec![8, 6]]);
}

#[test]
fn compiled_cache_can_be_inspected_without_materializing_corpus() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let metadata = inspect_compiled(&cache_path).unwrap();
    assert!(metadata.byte_len > 0);
    assert_eq!(metadata.node_features.len(), 7);
    assert_eq!(metadata.edge_features.len(), 4);
    assert_eq!(metadata.config_features.len(), 1);
    assert_eq!(metadata.order_len, 8);
    assert_eq!(metadata.rank_len, 8);

    let otype = metadata.node_feature("otype").unwrap();
    assert_eq!(otype.encoding, NodeFeatureEncoding::StringPool);
    assert_eq!(otype.row_count, 8);
    assert_eq!(otype.string_pool_count, Some(3));
    assert!(otype.payload_start < otype.payload_end);
    assert_eq!(
        metadata
            .node_feature("word")
            .unwrap()
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("word text")
    );

    let number = metadata.node_feature("number").unwrap();
    assert_eq!(number.encoding, NodeFeatureEncoding::Mixed);
    assert_eq!(number.row_count, 5);

    let oslots = metadata.edge_feature("oslots").unwrap();
    assert_eq!(oslots.row_count, 3);
    assert!(oslots.payload_start < oslots.payload_end);
    assert_eq!(
        metadata
            .edge_feature("distance")
            .unwrap()
            .metadata
            .get("valueType")
            .and_then(Option::as_deref),
        Some("int")
    );

    let otext = metadata.config_feature("otext").unwrap();
    assert_eq!(otext.metadata_count, 5);
    assert!(otext.payload_start < otext.payload_end);
}

#[test]
fn mapped_compiled_string_pool_features_read_values_without_materializing() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(
        &source,
        &cache_path,
        &[
            "otype",
            "oslots",
            "word",
            "pos",
            "number",
            "score",
            "phrase_id",
            "parent",
            "relation",
            "distance",
        ],
    )
    .unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let parsed = Corpus::load(&source).unwrap();
    assert_eq!(mapped.metadata().order_len, 8);
    assert_eq!(
        mapped.all_node_features(true),
        vec!["number", "otype", "phrase_id", "pos", "score", "word"]
    );
    assert_eq!(
        mapped.all_node_features(false),
        vec!["number", "phrase_id", "pos", "score", "word"]
    );
    assert_eq!(mapped.Fall(true), mapped.all_node_features(true));
    assert_eq!(mapped.Fall(false), mapped.all_node_features(false));
    assert_eq!(
        mapped.all_edge_features(true),
        vec!["distance", "oslots", "parent", "relation"]
    );
    assert_eq!(
        mapped.all_edge_features(false),
        vec!["distance", "parent", "relation"]
    );
    assert_eq!(mapped.Eall(true), mapped.all_edge_features(true));
    assert_eq!(mapped.Eall(false), mapped.all_edge_features(false));
    assert_eq!(
        mapped.all_computed_features(),
        parsed.all_computed_features()
    );
    assert_eq!(mapped.Call(), mapped.all_computed_features());
    assert_eq!(mapped.order().unwrap(), parsed.order());
    assert_eq!(mapped.rank().unwrap(), parsed.rank());
    assert_eq!(mapped.Cs("levels").unwrap(), parsed.Cs("levels"));
    assert_eq!(mapped.Cs("order").unwrap(), parsed.Cs("order"));
    assert_eq!(mapped.Cs("rank").unwrap(), parsed.Cs("rank"));
    assert_eq!(mapped.Cs("boundary").unwrap(), parsed.Cs("boundary"));
    assert_eq!(mapped.Cs("missing").unwrap(), None);
    assert_eq!(mapped.otype_rank().unwrap(), parsed.otype_rank());
    assert_eq!(mapped.otypeRank().unwrap(), mapped.otype_rank().unwrap());
    let mut mapped_nodes = vec![3, 1, 2];
    mapped.sort_nodes(&mut mapped_nodes).unwrap();
    assert_eq!(mapped_nodes, vec![1, 2, 3]);
    assert_eq!(mapped.sorted_nodes([3, 1, 2]).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        mapped
            .sortNodes(BTreeMap::from([(5, ()), (3, ()), (1, ())]).into_keys())
            .unwrap(),
        vec![1, 3, 5]
    );
    assert_eq!(
        mapped.sortNodes(Vec::<u32>::new()).unwrap(),
        Vec::<u32>::new()
    );
    assert_eq!(mapped.sortNodes([5]).unwrap(), vec![5]);
    assert_eq!(mapped.sortNodes([8, 6, 1, 7]).unwrap(), vec![8, 6, 1, 7]);
    assert_eq!(
        mapped.sort_key_tuple(&[1, 2, 3]).unwrap(),
        parsed
            .sort_key_tuple(&[1, 2, 3])
            .into_iter()
            .map(|rank| Some(rank as u32))
            .collect::<Vec<_>>()
    );
    assert_eq!(mapped.sortKey(1).unwrap(), mapped.sort_key(1).unwrap());
    assert_eq!(
        mapped.sortKeyTuple(&[1, 2, 3]).unwrap(),
        mapped.sort_key_tuple(&[1, 2, 3]).unwrap()
    );
    assert_eq!(mapped.sort_key(999).unwrap(), None);

    let word = mapped.string_pool_node_feature("word").unwrap().unwrap();
    let dynamic_word = mapped.node_feature("word").unwrap().unwrap();
    assert!(matches!(dynamic_word, MappedNodeFeatureView::StringPool(_)));
    assert_eq!(dynamic_word.row_count(), 5);
    assert_eq!(dynamic_word.value_type(), Some("str"));
    assert_eq!(dynamic_word.valueType(), Some("str"));
    assert_eq!(dynamic_word.description(), Some("word text"));
    assert_eq!(
        dynamic_word.metadata_value("description"),
        Some("word text")
    );
    assert_eq!(
        dynamic_word
            .meta()
            .get("valueType")
            .and_then(Option::as_deref),
        Some("str")
    );
    assert_eq!(
        dynamic_word.v(1).unwrap(),
        Some(MappedNodeValue::Str("hello"))
    );
    assert_eq!(
        dynamic_word.s(MappedNodeValue::Str("hello")).unwrap(),
        vec![1]
    );
    assert_eq!(
        dynamic_word
            .nodes_with_value(MappedNodeValue::Str("hello"))
            .unwrap(),
        vec![1]
    );
    assert_eq!(
        dynamic_word.select(MappedNodeValue::Str("hello")).unwrap(),
        vec![1]
    );
    assert_eq!(
        dynamic_word
            .value_interval(MappedNodeValue::Str("hello"))
            .unwrap(),
        Some((1, 1))
    );
    assert_eq!(
        dynamic_word
            .sInterval(MappedNodeValue::Str("hello"))
            .unwrap(),
        dynamic_word
            .value_interval(MappedNodeValue::Str("hello"))
            .unwrap()
    );
    assert_eq!(
        dynamic_word.items().unwrap()[0],
        (1, MappedNodeValue::Str("hello"))
    );
    assert_eq!(
        dynamic_word.s(MappedNodeValue::Int(1)).unwrap(),
        Vec::<u32>::new()
    );
    let alias_word = mapped.Fs("word").unwrap().unwrap();
    assert!(matches!(alias_word, MappedNodeFeatureView::StringPool(_)));
    assert_eq!(word.row_count(), 5);
    assert_eq!(word.value_type(), Some("str"));
    assert_eq!(word.valueType(), Some("str"));
    assert_eq!(word.description(), Some("word text"));
    assert_eq!(word.metadata_value("description"), Some("word text"));
    assert_eq!(
        word.meta().get("valueType").and_then(Option::as_deref),
        Some("str")
    );
    assert_eq!(word.str_value(1).unwrap(), Some("hello"));
    assert_eq!(word.v(1).unwrap(), Some(MappedNodeValue::Str("hello")));
    assert_eq!(word.str_value(5).unwrap(), Some("morning"));
    assert_eq!(word.str_value(8).unwrap(), None);
    assert_eq!(word.s("hello").unwrap(), vec![1]);
    assert_eq!(word.nodes_with_value("hello").unwrap(), vec![1]);
    assert_eq!(word.select("hello").unwrap(), vec![1]);
    assert_eq!(
        word.filter_by_value(&[5, 1, 2, 99], "hello").unwrap(),
        vec![1]
    );
    assert_eq!(
        word.filter_by_values(&[1, 2, 3, 4, 5], &["hello", "world"])
            .unwrap(),
        vec![1, 3]
    );
    assert!(
        word.filter_by_values(&[1, 2], &Vec::<&str>::new())
            .unwrap()
            .is_empty()
    );
    assert_eq!(word.filter_has_value(&[1, 8, 99]).unwrap(), vec![1]);
    assert_eq!(word.filter_missing_value(&[1, 8, 99]).unwrap(), vec![8, 99]);
    let word_rows: Vec<_> = word
        .rows()
        .collect::<context_fabric_core::Result<_>>()
        .unwrap();
    assert_eq!(word_rows[0], (1, "hello"));
    assert_eq!(word_rows[4], (5, "morning"));
    let word_items = word.items().unwrap();
    assert_eq!(word_items[0], (1, MappedNodeValue::Str("hello")));
    assert_eq!(word_items[4], (5, MappedNodeValue::Str("morning")));

    let otype = mapped.string_pool_node_feature("otype").unwrap().unwrap();
    assert_eq!(otype.str_value(1).unwrap(), Some("word"));
    assert_eq!(otype.str_value(6).unwrap(), Some("phrase"));
    assert_eq!(otype.str_value(8).unwrap(), Some("sentence"));
    assert_eq!(otype.s("word").unwrap(), vec![1, 2, 3, 4, 5]);
    assert_eq!(otype.value_interval("word").unwrap(), Some((1, 5)));
    assert_eq!(otype.sInterval("word").unwrap(), Some((1, 5)));
    assert_eq!(otype.value_interval("phrase").unwrap(), Some((6, 7)));
    assert_eq!(otype.value_interval("missing").unwrap(), None);

    let number = mapped.mixed_node_feature("number").unwrap().unwrap();
    let dynamic_number = mapped.node_feature("number").unwrap().unwrap();
    assert!(matches!(dynamic_number, MappedNodeFeatureView::Mixed(_)));
    assert_eq!(dynamic_number.value_type(), Some("int"));
    assert_eq!(dynamic_number.valueType(), Some("int"));
    assert_eq!(dynamic_number.description(), Some("word count in phrase"));
    assert_eq!(
        dynamic_number.metadata_value("description"),
        Some("word count in phrase")
    );
    assert_eq!(dynamic_number.v(3).unwrap(), Some(MappedNodeValue::Int(3)));
    assert_eq!(
        dynamic_number.s(MappedNodeValue::Int(2)).unwrap(),
        vec![2, 5]
    );
    assert_eq!(
        dynamic_number
            .filter_by_value(&[5, 4, 3, 2, 1], MappedNodeValue::Int(2))
            .unwrap(),
        vec![5, 2]
    );
    assert_eq!(
        dynamic_number
            .filter_by_values(
                &[1, 2, 3, 4, 5],
                &[MappedNodeValue::Int(1), MappedNodeValue::Int(3)]
            )
            .unwrap(),
        vec![1, 3, 4]
    );
    assert_eq!(
        dynamic_number
            .filter_less_than(&[1, 2, 3, 4, 5], MappedNodeValue::Int(3))
            .unwrap(),
        vec![1, 2, 4, 5]
    );
    assert_eq!(
        dynamic_number
            .filter_greater_than(&[1, 2, 3, 4, 5], MappedNodeValue::Int(2))
            .unwrap(),
        vec![3]
    );
    assert_eq!(
        dynamic_number.filter_has_value(&[1, 8, 99]).unwrap(),
        vec![1]
    );
    assert_eq!(
        dynamic_number.filter_missing_value(&[1, 8, 99]).unwrap(),
        vec![8, 99]
    );
    assert_eq!(
        dynamic_number
            .value_interval(MappedNodeValue::Int(2))
            .unwrap(),
        Some((2, 5))
    );
    assert_eq!(number.v(3).unwrap(), Some(MappedNodeValue::Int(3)));
    assert_eq!(number.value_type(), Some("int"));
    assert_eq!(number.valueType(), Some("int"));
    assert_eq!(number.description(), Some("word count in phrase"));
    assert_eq!(
        number.metadata_value("description"),
        Some("word count in phrase")
    );
    assert_eq!(number.s(MappedNodeValue::Int(2)).unwrap(), vec![2, 5]);
    assert_eq!(
        number.nodes_with_value(MappedNodeValue::Int(2)).unwrap(),
        vec![2, 5]
    );
    assert_eq!(number.select(MappedNodeValue::Int(2)).unwrap(), vec![2, 5]);
    assert_eq!(
        number
            .filter_by_value(&[1, 2, 3, 4, 5], MappedNodeValue::Int(2))
            .unwrap(),
        vec![2, 5]
    );
    assert_eq!(number.items().unwrap()[2], (3, MappedNodeValue::Int(3)));
    assert_eq!(
        number.value_interval(MappedNodeValue::Int(2)).unwrap(),
        Some((2, 5))
    );
    assert_eq!(
        number.sInterval(MappedNodeValue::Int(2)).unwrap(),
        number.value_interval(MappedNodeValue::Int(2)).unwrap()
    );
    assert_eq!(
        number.value_interval(MappedNodeValue::Int(999)).unwrap(),
        None
    );

    let score = mapped.mixed_node_feature("score").unwrap().unwrap();
    assert_eq!(score.v(2).unwrap(), Some(MappedNodeValue::Int(0)));
    assert_eq!(score.v(3).unwrap(), None);
    assert_eq!(score.s(MappedNodeValue::Int(0)).unwrap(), vec![2, 5]);

    assert!(
        mapped
            .string_pool_node_feature("missing")
            .unwrap()
            .is_none()
    );
    assert!(mapped.node_feature("missing").unwrap().is_none());
    assert!(mapped.Fs("missing").unwrap().is_none());
}

#[test]
fn mapped_compiled_config_features_read_metadata_without_materializing() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let otext = mapped.config_feature("otext").unwrap().unwrap();
    assert_eq!(otext.metadata_count(), 5);
    assert_eq!(
        otext.get("sectionTypes").unwrap().flatten(),
        Some("sentence,phrase")
    );
    assert_eq!(
        otext.metadata_value("sectionTypes").unwrap(),
        Some("sentence,phrase")
    );
    assert_eq!(
        otext.get("fmt:text-orig-full").unwrap().flatten(),
        Some("{word}")
    );
    assert_eq!(otext.get("missing").unwrap(), None);

    let metadata = otext.metadata().unwrap();
    assert_eq!(
        metadata.get("sectionFeatures").and_then(Option::as_deref),
        Some("sentence_id,phrase_id")
    );
    assert_eq!(otext.meta().unwrap(), metadata);
}

#[test]
fn mapped_text_renders_otext_formats_without_materializing_corpus() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let text = MappedText::new(&mapped).unwrap();
    let materialized_text = Text::new(&parsed);

    assert_eq!(
        text.split_format("phrase#{word}").unwrap(),
        ("phrase".to_string(), "{word}".to_string())
    );
    assert_eq!(
        text.splitFormat("unknown#{word}").unwrap(),
        ("word".to_string(), "unknown#{word}".to_string())
    );
    assert_eq!(
        text.split_format("{word}").unwrap(),
        ("word".to_string(), "{word}".to_string())
    );
    assert_eq!(
        text.split_default_format("phrase-default").unwrap(),
        Some("phrase".to_string())
    );
    assert_eq!(text.splitDefaultFormat("unknown-default").unwrap(), None);
    assert_eq!(text.split_default_format("phrase-text").unwrap(), None);

    assert_eq!(text.text(1, None).unwrap(), parsed.text(1, None));
    assert_eq!(materialized_text.text(1, None), parsed.text(1, None));
    assert_eq!(
        text.text(1, Some("fmt:text-orig-full")).unwrap(),
        parsed.text(1, Some("text-orig-full"))
    );
    assert_eq!(text.text(6, None).unwrap(), parsed.text(6, None));
    assert_eq!(text.text(8, None).unwrap(), parsed.text(8, None));
    assert_eq!(
        text.text_nodes(&[3, 1, 2], None).unwrap(),
        parsed.text_nodes(&[3, 1, 2], None)
    );
    assert_eq!(
        materialized_text.text_nodes(&[3, 1, 2], None),
        parsed.text_nodes(&[3, 1, 2], None)
    );
    assert_eq!(
        materialized_text.textNodes(&[3, 1, 2], None),
        materialized_text.text_nodes(&[3, 1, 2], None)
    );
    assert_eq!(
        text.textNodes(&[3, 1, 2], None).unwrap(),
        text.text_nodes(&[3, 1, 2], None).unwrap()
    );
    assert_eq!(text.text(1, Some("missing-format")).unwrap(), "");
    assert_eq!(
        text.text_representations().unwrap(),
        parsed.text_representations()
    );
}

#[test]
fn mapped_text_describes_paired_text_representation_formats_without_materializing() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();
    fs::write(
        dir.path().join("orig.tf"),
        "@node\n@valueType=str\n\nאב\nאג\nאב\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("trans.tf"),
        "@node\n@valueType=str\n\nab\nag\nab\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("otext.tf"),
        "@config\n@fmt:text-orig-full={orig}\n@fmt:text-trans-full={trans}\n\n",
    )
    .unwrap();

    let cache_path = dir.path().join("paired.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();

    let parsed = Corpus::load(dir.path()).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let text = MappedText::new(&mapped).unwrap();

    assert_eq!(
        text.text_representations().unwrap(),
        parsed.text_representations()
    );
}

#[test]
fn mapped_sections_resolve_section_references_without_materializing_corpus() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let sections = MappedSections::new(&mapped).unwrap();

    assert_eq!(
        sections.section_types(),
        &["sentence".to_string(), "phrase".to_string()]
    );
    assert_eq!(sections.sectionTypes(), sections.section_types());
    assert_eq!(
        sections.section_features(),
        &["sentence_id".to_string(), "phrase_id".to_string()]
    );
    assert_eq!(sections.sectionFeatures(), sections.section_features());
    assert_eq!(sections.structure_types(), Vec::<String>::new().as_slice());
    assert_eq!(
        sections.structure_features(),
        Vec::<String>::new().as_slice()
    );
    assert_eq!(sections.structureTypes(), sections.structure_types());
    assert_eq!(sections.structureFeatures(), sections.structure_features());
    assert_eq!(
        sections
            .section_tuple(1, &SectionOptions::default())
            .unwrap(),
        parsed.section_tuple(1, &SectionOptions::default())
    );
    assert_eq!(
        sections
            .sectionTuple(1, &SectionOptions::default())
            .unwrap(),
        parsed.sectionTuple(1, &SectionOptions::default())
    );
    assert_eq!(
        sections
            .section_from_node(1, &SectionOptions::default())
            .unwrap(),
        parsed.section_from_node(1, &SectionOptions::default())
    );
    assert_eq!(
        sections
            .sectionFromNode(1, &SectionOptions::default())
            .unwrap(),
        parsed.sectionFromNode(1, &SectionOptions::default())
    );
    assert_eq!(sections.section_ref(1).unwrap(), parsed.section_ref(1));
    assert_eq!(
        sections
            .node_from_section(&[FeatureValue::string("S1")])
            .unwrap(),
        Some(8)
    );
    assert_eq!(
        sections
            .nodeFromSection(&[FeatureValue::string("S1")])
            .unwrap(),
        parsed.nodeFromSection(&[FeatureValue::string("S1")])
    );
    assert_eq!(
        sections
            .node_from_section(&[FeatureValue::string("S1"), FeatureValue::Int(1)])
            .unwrap(),
        Some(6)
    );
    assert_eq!(
        sections
            .node_from_section(&[FeatureValue::string("S1"), FeatureValue::Int(99)])
            .unwrap(),
        None
    );
}

#[test]
fn mapped_sections_match_materialized_containment_and_locality_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let sections = MappedSections::new(&mapped).unwrap();

    assert_eq!(sections.contains(8, 1).unwrap(), parsed.contains(8, 1));
    assert_eq!(sections.contains(8, 6).unwrap(), parsed.contains(8, 6));
    assert_eq!(sections.contains(6, 8).unwrap(), parsed.contains(6, 8));
    assert_eq!(sections.contains(999, 1).unwrap(), parsed.contains(999, 1));

    assert_eq!(
        sections.up(1, Some("phrase")).unwrap(),
        parsed.up(1, Some("phrase"))
    );
    assert_eq!(
        sections.u(1, Some("phrase")).unwrap(),
        parsed.u(1, Some("phrase"))
    );
    assert_eq!(
        sections.up(1, Some("sentence")).unwrap(),
        parsed.up(1, Some("sentence"))
    );
    assert_eq!(sections.u(8, None).unwrap(), parsed.u(8, None));
    assert_eq!(sections.up(8, None).unwrap(), parsed.up(8, None));
    assert_eq!(
        sections.up_types(1, Some(&["phrase", "sentence"])).unwrap(),
        vec![6, 8]
    );
    assert_eq!(
        sections.u_types(1, Some(&["phrase", "sentence"])).unwrap(),
        sections.up_types(1, Some(&["phrase", "sentence"])).unwrap()
    );

    assert_eq!(
        sections.intersecting(6, None).unwrap(),
        parsed.intersecting(6, None)
    );
    assert_eq!(sections.i(6, None).unwrap(), parsed.i(6, None));
    assert_eq!(
        sections.i(6, Some("sentence")).unwrap(),
        parsed.i(6, Some("sentence"))
    );
    assert_eq!(
        sections.i(6, Some("word")).unwrap(),
        parsed.i(6, Some("word"))
    );
    assert_eq!(
        sections.intersecting(1, None).unwrap(),
        parsed.intersecting(1, None)
    );
    assert_eq!(
        sections
            .intersecting_types(8, Some(&["phrase", "word"]))
            .unwrap(),
        parsed.intersecting_types(8, Some(&["phrase", "word"]))
    );

    assert_eq!(
        sections.down(6, Some("word")).unwrap(),
        parsed.down(6, Some("word"))
    );
    assert_eq!(
        sections.d(6, Some("word")).unwrap(),
        parsed.d(6, Some("word"))
    );
    assert_eq!(
        sections.down(7, Some("word")).unwrap(),
        parsed.down(7, Some("word"))
    );
    assert_eq!(sections.down(1, None).unwrap(), parsed.down(1, None));
    assert_eq!(sections.d(1, None).unwrap(), parsed.d(1, None));
    assert_eq!(
        sections.down_types(8, Some(&["phrase"])).unwrap(),
        parsed.down_types(8, Some(&["phrase"]))
    );
    assert_eq!(
        sections.down_types(8, Some(&["word"])).unwrap(),
        parsed.down_types(8, Some(&["word"]))
    );
    assert_eq!(
        sections.d_types(8, Some(&["phrase", "word"])).unwrap(),
        sections.down_types(8, Some(&["phrase", "word"])).unwrap()
    );

    assert_eq!(sections.next(3, None).unwrap(), parsed.next(3, None));
    assert_eq!(sections.n(3, None).unwrap(), parsed.n(3, None));
    assert_eq!(
        sections.previous(4, None).unwrap(),
        parsed.previous(4, None)
    );
    assert_eq!(sections.p(4, None).unwrap(), parsed.p(4, None));
    assert_eq!(sections.next(5, None).unwrap(), parsed.next(5, None));
    assert_eq!(
        sections.previous(1, None).unwrap(),
        parsed.previous(1, None)
    );
    assert_eq!(
        sections.next(1, Some("word")).unwrap(),
        parsed.next(1, Some("word"))
    );
    assert_eq!(
        sections.n(1, Some("word")).unwrap(),
        parsed.n(1, Some("word"))
    );
    assert_eq!(
        sections.previous(5, Some("word")).unwrap(),
        parsed.previous(5, Some("word"))
    );
    assert_eq!(
        sections.p(5, Some("word")).unwrap(),
        parsed.p(5, Some("word"))
    );
    assert_eq!(
        sections.next(6, Some("phrase")).unwrap(),
        parsed.next(6, Some("phrase"))
    );
    assert_eq!(
        sections.previous(7, Some("phrase")).unwrap(),
        parsed.previous(7, Some("phrase"))
    );
    assert_eq!(
        sections.next_types(3, Some(&["phrase", "word"])).unwrap(),
        parsed.next_types(3, Some(&["phrase", "word"]))
    );
    assert_eq!(
        sections
            .previous_types(4, Some(&["phrase", "word"]))
            .unwrap(),
        parsed.previous_types(4, Some(&["phrase", "word"]))
    );

    assert_eq!(sections.all_nodes().unwrap(), parsed.all_nodes());
    assert_eq!(sections.allNodes().unwrap(), sections.all_nodes().unwrap());
    assert_eq!(
        sections.node_type_items().unwrap(),
        parsed.node_type_items()
    );
    assert_eq!(sections.otype_items().unwrap(), parsed.otype_items());
    assert_eq!(
        sections.otypeItems().unwrap(),
        sections.node_type_items().unwrap()
    );
    assert_eq!(sections.walk(None).unwrap(), parsed.walk(None));
    assert_eq!(
        sections.walk(Some(&[3, 1, 8])).unwrap(),
        parsed.walk(Some(&[3, 1, 8]))
    );
    assert_eq!(
        sections.walk_events(None).unwrap(),
        parsed.walk_events(None)
    );
    assert_eq!(
        sections.walkEvents(None).unwrap(),
        sections.walk_events(None).unwrap()
    );
    assert_eq!(
        sections.walk_events(Some(&[8, 1, 5])).unwrap(),
        parsed.walk_events(Some(&[8, 1, 5]))
    );
    assert_eq!(
        sections.walkEvents(Some(&[8, 1, 5])).unwrap(),
        sections.walk_events(Some(&[8, 1, 5])).unwrap()
    );
    let mapped_boundary = sections.boundary().unwrap();
    assert_eq!(mapped_boundary, parsed.boundary());
    assert_eq!(mapped_boundary.first_slots[0], vec![6, 8]);
    assert_eq!(mapped_boundary.last_slots[4], vec![8, 7]);
}

#[test]
fn mapped_compile_reload_cycle_preserves_features_locality_and_search() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &["otype", "oslots", "word", "pos"]).unwrap();

    let fresh = MappedCompiledCorpus::open(&cache_path).unwrap();
    let cached = MappedCompiledCorpus::open(&cache_path).unwrap();

    let fresh_otype = fresh.string_pool_node_feature("otype").unwrap().unwrap();
    let cached_otype = cached.string_pool_node_feature("otype").unwrap().unwrap();
    for node in 1..=fresh.max_node() {
        assert_eq!(
            fresh_otype.str_value(node).unwrap(),
            cached_otype.str_value(node).unwrap(),
            "otype differs after cache reopen for node {node}"
        );
    }

    let fresh_word = fresh.string_pool_node_feature("word").unwrap().unwrap();
    let cached_word = cached.string_pool_node_feature("word").unwrap().unwrap();
    for node in fresh_otype.s("word").unwrap() {
        assert_eq!(
            fresh_word.str_value(node).unwrap(),
            cached_word.str_value(node).unwrap(),
            "word differs after cache reopen for node {node}"
        );
    }

    let fresh_sections = MappedSections::new(&fresh).unwrap();
    let cached_sections = MappedSections::new(&cached).unwrap();
    for sentence in fresh_otype.s("sentence").unwrap() {
        assert_eq!(
            fresh_sections.d(sentence, None).unwrap(),
            cached_sections.d(sentence, None).unwrap(),
            "descendants differ after cache reopen for node {sentence}"
        );
        assert_eq!(
            fresh_sections.d(sentence, Some("word")).unwrap(),
            cached_sections.d(sentence, Some("word")).unwrap()
        );
    }

    assert_eq!(
        MappedSearch::new(&fresh).search("word", None).unwrap(),
        MappedSearch::new(&cached).search("word", None).unwrap()
    );
}

#[test]
fn mapped_compile_reload_cycle_preserves_section_metadata_and_lookup() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let fresh = MappedCompiledCorpus::open(&cache_path).unwrap();
    let cached = MappedCompiledCorpus::open(&cache_path).unwrap();
    let fresh_sections = MappedSections::new(&fresh).unwrap();
    let cached_sections = MappedSections::new(&cached).unwrap();

    assert_eq!(
        fresh_sections.section_types(),
        cached_sections.section_types()
    );
    assert_eq!(
        fresh_sections.section_features(),
        cached_sections.section_features()
    );
    assert_eq!(
        fresh_sections
            .node_from_section(&[FeatureValue::string("S1")])
            .unwrap(),
        cached_sections
            .node_from_section(&[FeatureValue::string("S1")])
            .unwrap()
    );
    assert_eq!(
        fresh_sections
            .node_from_section(&[FeatureValue::string("S1"), FeatureValue::Int(1)])
            .unwrap(),
        cached_sections
            .node_from_section(&[FeatureValue::string("S1"), FeatureValue::Int(1)])
            .unwrap()
    );
}

#[test]
fn mapped_result_wrappers_match_materialized_result_shapes() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let text = MappedText::new(&mapped).unwrap();
    let sections = MappedSections::new(&mapped).unwrap();
    let options = NodeInfoOptions {
        include_slots: true,
        include_features: vec!["word".to_string(), "pos".to_string()],
        ..NodeInfoOptions::default()
    };

    assert_eq!(
        NodeInfo::from_mapped(&text, &sections, 1, &options).unwrap(),
        NodeInfo::from_corpus(&parsed, 1, &options)
    );
    assert_eq!(
        NodeInfo::from_mapped(&text, &sections, 6, &options).unwrap(),
        NodeInfo::from_corpus(&parsed, 6, &options)
    );

    assert_eq!(
        NodeList::from_mapped_nodes(
            &text,
            &sections,
            &[8, 6, 1],
            Some(2),
            Some("subset".to_string()),
            &NodeInfoOptions::default(),
        )
        .unwrap(),
        NodeList::from_nodes(
            &parsed,
            &[8, 6, 1],
            Some(2),
            Some("subset".to_string()),
            &NodeInfoOptions::default(),
        )
    );

    let raw_results = MappedSearch::new(&mapped)
        .search("word word=hello", None)
        .unwrap();
    assert_eq!(
        SearchResult::from_mapped_search(
            &text,
            &sections,
            &raw_results,
            "word word=hello",
            Some(1),
            &NodeInfoOptions::default(),
        )
        .unwrap(),
        SearchResult::from_search(
            &parsed,
            &raw_results,
            "word word=hello",
            Some(1),
            &NodeInfoOptions::default(),
        )
    );
}

#[test]
fn mapped_corpus_info_matches_materialized_summary_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(
        CorpusInfo::from_mapped(&mapped, "mini", "fixtures/mini_corpus").unwrap(),
        CorpusInfo::from_corpus(&parsed, "mini", "fixtures/mini_corpus")
    );
}

#[test]
fn mapped_corpus_description_matches_materialized_description_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let text = MappedText::new(&mapped).unwrap();

    assert_eq!(
        context_fabric_core::CorpusDescription::from_mapped(&mapped, &text, "mini").unwrap(),
        parsed.describe_corpus("mini")
    );
}

#[test]
fn mapped_feature_info_matches_materialized_metadata_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(mapped.max_node(), parsed.max_node());
    assert_eq!(mapped.maxNode(), parsed.maxNode());
    assert_eq!(mapped.max_slot().unwrap(), parsed.max_slot());
    assert_eq!(mapped.maxSlot().unwrap(), parsed.maxSlot());
    assert_eq!(mapped.slot_type().unwrap(), parsed.slot_type());
    assert_eq!(mapped.slotType().unwrap(), parsed.slotType());
    assert_eq!(
        FeatureInfo::from_mapped(&mapped, "word", FeatureKind::Node),
        FeatureInfo::from_corpus(&parsed, "word", FeatureKind::Node)
    );
    assert_eq!(
        FeatureInfo::from_mapped(&mapped, "distance", FeatureKind::Edge),
        FeatureInfo::from_corpus(&parsed, "distance", FeatureKind::Edge)
    );
    assert_eq!(
        FeatureInfo::from_mapped(&mapped, "word", FeatureKind::Edge),
        None
    );
}

#[test]
fn mapped_loaded_feature_introspection_matches_materialized_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(mapped.Fall(true), parsed.Fall(true));
    assert_eq!(mapped.Fall(false), parsed.Fall(false));
    assert_eq!(mapped.Eall(true), parsed.Eall(true));
    assert_eq!(mapped.Eall(false), parsed.Eall(false));
    assert_eq!(mapped.Call(), parsed.Call());
    assert_eq!(
        mapped
            .is_loaded(Some(&[
                "word",
                "parent",
                "distance",
                "otext",
                "levels",
                "missing_feature",
            ]))
            .unwrap(),
        parsed.is_loaded(Some(&[
            "word",
            "parent",
            "distance",
            "otext",
            "levels",
            "missing_feature",
        ]))
    );
    assert_eq!(
        mapped
            .isLoaded(Some(&[
                "word",
                "parent",
                "distance",
                "otext",
                "levels",
                "missing_feature",
            ]))
            .unwrap(),
        mapped
            .is_loaded(Some(&[
                "word",
                "parent",
                "distance",
                "otext",
                "levels",
                "missing_feature",
            ]))
            .unwrap()
    );
    assert_eq!(mapped.is_loaded(None).unwrap(), parsed.is_loaded(None));
    assert_eq!(
        mapped.isLoaded(None).unwrap(),
        mapped.is_loaded(None).unwrap()
    );
}

#[test]
fn mapped_feature_type_discovery_and_catalog_match_materialized_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(
        mapped.node_feature_types("word").unwrap(),
        parsed
            .node_feature_types("word")
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        mapped.node_feature_types("phrase_id").unwrap(),
        parsed
            .node_feature_types("phrase_id")
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        mapped.edge_feature_source_types("parent").unwrap(),
        parsed
            .edge_feature_source_types("parent")
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        mapped.edge_feature_target_types("parent").unwrap(),
        parsed
            .edge_feature_target_types("parent")
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        mapped.edge_feature_target_types("oslots").unwrap(),
        parsed
            .edge_feature_target_types("oslots")
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        mapped.all_node_feature_types().unwrap(),
        parsed.all_node_feature_types()
    );
    assert_eq!(
        mapped.all_edge_feature_source_types().unwrap(),
        parsed.all_edge_feature_source_types()
    );
    assert_eq!(
        mapped.all_edge_feature_target_types().unwrap(),
        parsed.all_edge_feature_target_types()
    );
    assert_eq!(
        mapped.feature_catalog(None, Some(&["word"])).unwrap(),
        parsed.feature_catalog(None, Some(&["word"]))
    );
    assert_eq!(
        mapped
            .feature_catalog(Some(FeatureKind::Edge), Some(&["phrase"]))
            .unwrap(),
        parsed.feature_catalog(Some(FeatureKind::Edge), Some(&["phrase"]))
    );
}

#[test]
fn mapped_feature_descriptions_match_materialized_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(
        mapped.describe_feature("pos", 1).unwrap(),
        parsed.describe_feature("pos", 1)
    );
    assert_eq!(
        mapped.describe_feature("parent", 5).unwrap(),
        parsed.describe_feature("parent", 5)
    );
    assert_eq!(
        mapped.describe_feature("distance", 2).unwrap(),
        parsed.describe_feature("distance", 2)
    );
    assert_eq!(
        mapped.describe_feature("missing", 5).unwrap(),
        parsed.describe_feature("missing", 5)
    );
    assert_eq!(
        mapped
            .describe_features(&["word", "distance", "missing"], 1)
            .unwrap(),
        parsed.describe_features(&["word", "distance", "missing"], 1)
    );
}

#[test]
fn mapped_frequency_lists_match_materialized_filters_without_materializing_cache() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(
        mapped.node_frequency_list("pos", Some(&["word"])).unwrap(),
        parsed.node_frequency_list("pos", Some(&["word"])).unwrap()
    );
    assert_eq!(
        mapped.node_freq_list("pos", Some(&["word"])).unwrap(),
        parsed.node_freq_list("pos", Some(&["word"])).unwrap()
    );
    assert_eq!(
        mapped.node_freqList("pos", Some(&["word"])).unwrap(),
        parsed.node_freqList("pos", Some(&["word"])).unwrap()
    );
    assert_eq!(
        mapped
            .node_frequency_list("pos", Some(&["phrase"]))
            .unwrap(),
        parsed
            .node_frequency_list("pos", Some(&["phrase"]))
            .unwrap()
    );
    assert!(mapped.node_frequency_list("missing", None).is_err());

    assert_eq!(
        mapped.edge_frequency_list("parent", None, None).unwrap(),
        parsed.edge_frequency_list("parent", None, None).unwrap()
    );
    assert_eq!(
        mapped
            .edge_frequency_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap(),
        parsed
            .edge_frequency_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap()
    );
    assert_eq!(
        mapped
            .edge_freq_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap(),
        parsed
            .edge_freq_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap()
    );
    assert_eq!(
        mapped
            .edge_freqList("parent", Some(&["phrase"]), Some(&["sentence"]))
            .unwrap(),
        parsed
            .edge_freqList("parent", Some(&["phrase"]), Some(&["sentence"]))
            .unwrap()
    );
    assert_eq!(
        mapped.edge_frequency_list("distance", None, None).unwrap(),
        parsed.edge_frequency_list("distance", None, None).unwrap()
    );
    assert!(mapped.edge_frequency_list("missing", None, None).is_err());
}

#[test]
fn mapped_compiled_edge_features_read_targets_without_materializing() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &["otype", "oslots", "word", "pos"]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let oslots = mapped.edge_feature("oslots").unwrap().unwrap();
    assert!(mapped.Es("missing").unwrap().is_none());
    assert_eq!(
        mapped.Es("oslots").unwrap().unwrap().row_count(),
        oslots.row_count()
    );
    assert_eq!(oslots.row_count(), 3);
    assert_eq!(oslots.row_offset_count(), 3);
    assert_eq!(oslots.value_type(), Some("int"));
    assert_eq!(oslots.valueType(), Some("int"));
    assert_eq!(oslots.description(), Some("slot containment"));
    assert_eq!(
        oslots.metadata_value("description"),
        Some("slot containment")
    );
    assert_eq!(
        oslots.meta().get("valueType").and_then(Option::as_deref),
        Some("int")
    );
    assert_eq!(oslots.edge_value_count(), 0);
    assert!(!oslots.has_edge_values());
    assert!(!oslots.hasEdgeValues());

    let phrase_slots: Vec<_> = oslots
        .targets(6)
        .unwrap()
        .unwrap()
        .collect::<context_fabric_core::Result<_>>()
        .unwrap();
    assert_eq!(phrase_slots, vec![1, 2, 3]);
    assert_eq!(oslots.s(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(oslots.f(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(oslots.forward(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        oslots.get_all_targets([6, 7]).unwrap(),
        BTreeSet::from([1, 2, 3, 4, 5])
    );
    assert_eq!(
        oslots.get_all_targets([6, 99]).unwrap(),
        BTreeSet::from([1, 2, 3])
    );
    assert_eq!(
        oslots
            .filter_sources_with_targets_in([6, 7, 8], [1, 5])
            .unwrap(),
        (BTreeSet::from([6, 7, 8]), BTreeSet::from([1, 5]))
    );
    assert_eq!(
        oslots.filter_sources_with_targets_in([6, 7], [99]).unwrap(),
        (BTreeSet::new(), BTreeSet::new())
    );
    assert_eq!(
        oslots.forward_with_values(6).unwrap(),
        vec![(1, None), (2, None), (3, None)]
    );
    assert_eq!(
        oslots.f_with_values(6).unwrap(),
        oslots.forward_with_values(6).unwrap()
    );

    let sentence_slots: Vec<_> = oslots
        .targets(8)
        .unwrap()
        .unwrap()
        .collect::<context_fabric_core::Result<_>>()
        .unwrap();
    assert_eq!(sentence_slots, vec![1, 2, 3, 4, 5]);
    assert_eq!(oslots.s(8).unwrap(), vec![1, 2, 3, 4, 5]);
    let mut embedders_for_slot_1 = vec![6, 8];
    parsed.sort_nodes(&mut embedders_for_slot_1);
    assert_eq!(oslots.t(1).unwrap(), embedders_for_slot_1);
    assert_eq!(oslots.backward(1).unwrap(), embedders_for_slot_1);
    assert_eq!(
        oslots.backward_with_values(1).unwrap(),
        embedders_for_slot_1
            .iter()
            .copied()
            .map(|node| (node, None))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        oslots.t_with_values(1).unwrap(),
        oslots.backward_with_values(1).unwrap()
    );
    assert_eq!(oslots.both(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(oslots.b(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        oslots.both_with_values(6).unwrap(),
        vec![(1, None), (2, None), (3, None)]
    );
    assert_eq!(
        oslots.b_with_values(6).unwrap(),
        oslots.both_with_values(6).unwrap()
    );
    assert_eq!(
        oslots.items().unwrap(),
        parsed.edge_feature("oslots").unwrap().items()
    );
    assert_eq!(
        oslots
            .items()
            .unwrap()
            .into_iter()
            .find(|(source, _)| *source == 8),
        Some((8, vec![1, 2, 3, 4, 5]))
    );
    assert_eq!(oslots.edge_count().unwrap(), 10);

    assert!(oslots.targets(1).unwrap().is_none());
    assert!(mapped.edge_feature("missing").unwrap().is_none());
}

#[test]
fn mapped_compiled_edge_features_read_values_without_materializing() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(
        &source,
        &cache_path,
        &["otype", "oslots", "word", "relation", "distance"],
    )
    .unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let relation = mapped.edge_feature("relation").unwrap().unwrap();
    assert_eq!(relation.value_type(), Some("str"));
    assert_eq!(relation.valueType(), Some("str"));
    assert_eq!(
        relation.description(),
        Some("relation type between nodes (tests string edge values)")
    );
    assert_eq!(
        relation.metadata_value("description"),
        Some("relation type between nodes (tests string edge values)")
    );
    assert!(relation.meta().contains_key("edgeValues"));
    assert_eq!(relation.edge_value_count(), 5);
    assert!(relation.has_edge_values());
    assert!(relation.hasEdgeValues());
    assert_eq!(
        relation.edge_value(1, 6).unwrap(),
        Some(MappedNodeValue::Str("subject"))
    );
    assert_eq!(
        relation.edge_value(2, 6).unwrap(),
        Some(MappedNodeValue::Str("predicate"))
    );
    assert_eq!(relation.forward(1).unwrap(), vec![6]);
    assert_eq!(
        relation.forward_with_values(1).unwrap(),
        vec![(6, Some(MappedNodeValue::Str("subject")))]
    );
    assert_eq!(
        relation.f_with_values(1).unwrap(),
        relation.forward_with_values(1).unwrap()
    );
    assert_eq!(relation.backward(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        relation.backward_with_values(6).unwrap(),
        vec![
            (1, Some(MappedNodeValue::Str("subject"))),
            (2, Some(MappedNodeValue::Str("predicate"))),
            (3, Some(MappedNodeValue::Str("object"))),
        ]
    );
    assert_eq!(
        relation.t_with_values(6).unwrap(),
        relation.backward_with_values(6).unwrap()
    );
    assert_eq!(relation.both(1).unwrap(), vec![6]);
    assert_eq!(relation.b(1).unwrap(), vec![6]);
    assert_eq!(
        relation.both_with_values(1).unwrap(),
        vec![(6, Some(MappedNodeValue::Str("subject")))]
    );
    assert_eq!(
        relation.b_with_values(1).unwrap(),
        relation.both_with_values(1).unwrap()
    );
    assert_eq!(relation.edge_count().unwrap(), 5);

    let distance = mapped.edge_feature("distance").unwrap().unwrap();
    assert_eq!(distance.value_type(), Some("int"));
    assert_eq!(distance.valueType(), Some("int"));
    assert_eq!(distance.edge_value_count(), 5);
    assert!(distance.has_edge_values());
    assert!(distance.hasEdgeValues());
    assert_eq!(
        distance.edge_value(1, 2).unwrap(),
        Some(MappedNodeValue::Int(0))
    );
    assert_eq!(distance.edge_value(2, 3).unwrap(), None);
    assert_eq!(
        distance.forward_with_values(1).unwrap(),
        vec![
            (2, Some(MappedNodeValue::Int(0))),
            (3, Some(MappedNodeValue::Int(5)))
        ]
    );
    assert_eq!(
        distance.f_with_values(1).unwrap(),
        distance.forward_with_values(1).unwrap()
    );
    assert_eq!(
        distance.backward_with_values(2).unwrap(),
        vec![(1, Some(MappedNodeValue::Int(0)))]
    );
    assert_eq!(
        distance.t_with_values(2).unwrap(),
        distance.backward_with_values(2).unwrap()
    );
    assert_eq!(
        distance.backward_with_values(3).unwrap(),
        vec![(1, Some(MappedNodeValue::Int(5))), (2, None)]
    );
    assert_eq!(
        distance.t_with_values(3).unwrap(),
        distance.backward_with_values(3).unwrap()
    );
}

#[test]
fn mapped_compile_reload_cycle_preserves_string_edge_values() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &["otype", "oslots", "relation"]).unwrap();

    let fresh = MappedCompiledCorpus::open(&cache_path).unwrap();
    let cached = MappedCompiledCorpus::open(&cache_path).unwrap();
    let fresh_relation = fresh.edge_feature("relation").unwrap().unwrap();
    let cached_relation = cached.edge_feature("relation").unwrap().unwrap();

    assert_eq!(
        cached_relation.edge_value(1, 6).unwrap(),
        Some(MappedNodeValue::Str("subject"))
    );
    assert_eq!(
        cached_relation.edge_value(2, 6).unwrap(),
        Some(MappedNodeValue::Str("predicate"))
    );
    assert_eq!(
        cached_relation.edge_value(3, 6).unwrap(),
        Some(MappedNodeValue::Str("object"))
    );

    for source_node in 1..=5 {
        assert_eq!(
            fresh_relation.forward_with_values(source_node).unwrap(),
            cached_relation.forward_with_values(source_node).unwrap(),
            "string edge values differ after cache reopen for source node {source_node}"
        );
    }
}

#[test]
fn mapped_search_runs_simple_string_pool_queries() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(
        &source,
        &cache_path,
        &[
            "otype",
            "oslots",
            "word",
            "pos",
            "number",
            "phrase_id",
            "parent",
            "relation",
            "distance",
        ],
    )
    .unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);

    assert_eq!(search.glean(&[]).unwrap(), "");
    assert_eq!(search.glean(&[1]).unwrap(), "hello");
    assert_eq!(search.glean(&[1, 2, 3]).unwrap(), "hellobeautifulworld");
    assert_eq!(search.search("word", None).unwrap().len(), 5);
    assert_eq!(
        search.search("word word=hello", None).unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .search("word\nword=hello\npos=interjection", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search
            .search(
                "
% leading comment
word word=hello
",
                None
            )
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.search("word pos=noun", Some(1)).unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search.search("word pos=noun|adjective", None).unwrap(),
        vec![vec![2], vec![3], vec![4], vec![5]]
    );
    assert_eq!(
        search.search("word pos#noun", None).unwrap(),
        vec![vec![1], vec![2], vec![4]]
    );
    assert_eq!(
        search.search("phrase pos#", None).unwrap(),
        vec![vec![6], vec![7]]
    );
    let has_pos = search.search("word pos*", None).unwrap();
    assert_eq!(has_pos.len(), 5);
    assert_eq!(search.search("word pos", None).unwrap(), has_pos);
    assert!(search.search("phrase pos", None).unwrap().is_empty());
    assert_eq!(
        search.search("word number<3", None).unwrap(),
        vec![vec![1], vec![2], vec![4], vec![5]]
    );
    assert_eq!(
        search
            .search("word\nnumber<3\npos=adjective", None)
            .unwrap(),
        vec![vec![2], vec![4]]
    );
    assert_eq!(search.search("word number>2", None).unwrap(), vec![vec![3]]);
    assert_eq!(search.search("word word~^h", None).unwrap(), vec![vec![1]]);
    assert_eq!(
        search
            .search("word word=hello pos=interjection", None)
            .unwrap(),
        vec![vec![1]]
    );
    assert_eq!(
        search.search("w:word word=morning", None).unwrap(),
        vec![vec![5]]
    );
    assert_eq!(
        search.search("w:word word=hello\nw", None).unwrap(),
        vec![vec![1, 1]]
    );
    assert_eq!(
        search
            .search(
                "
w1 < w2
w1:word
w2:word
w1 word=hello
w2 word=world
",
                None
            )
            .unwrap(),
        vec![vec![1, 3, 1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 # w2
w1 pos=noun
w2 pos=noun
",
                None
            )
            .unwrap(),
        vec![vec![3, 5, 3, 5], vec![5, 3, 5, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=1
w .number=phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 6]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=2
w .number#phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    let study = search.study("word").unwrap();
    assert_eq!(study.template(), "word");
    assert_eq!(study.plan().atom_count, 1);
    assert_eq!(study.plan().relation_count, 0);
    assert_eq!(study.plan().result_count, 5);
    assert_eq!(
        study.show_plan(false),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert_eq!(study.showPlan(false), study.show_plan(false));
    assert!(study.show_plan(true).contains("  1: [1]"));
    assert_eq!(
        search.show_plan("word", false).unwrap(),
        "template: word\natoms: 1\nrelations: 0\nresults: 5\nquantifiers: false"
    );
    assert_eq!(
        search.showPlan("word", false).unwrap(),
        search.show_plan("word", false).unwrap()
    );
    let relation_legend = search.relations_legend();
    assert!(relation_legend.contains("[[ embeds"));
    assert!(relation_legend.contains("-edge=value> forward valued edge"));
    assert!(relation_legend.contains(".feature~regex~feature."));
    assert_eq!(search.relationsLegend(), relation_legend);
    assert_eq!(study.total_count(), 5);
    assert_eq!(study.count(Some(2)), 2);
    assert_eq!(study.fetch(Some(2)), vec![vec![1], vec![2]]);
    assert_eq!(
        search.fetch("word", Some(2)).unwrap(),
        vec![vec![1], vec![2]]
    );
    assert_eq!(search.count("word", None).unwrap(), 5);
    assert_eq!(search.count("word", Some(2)).unwrap(), 2);
    assert_eq!(study.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(study.fetch_first_nodes(Some(0)), Vec::<u32>::new());
    assert_eq!(study.fetch_first_nodes(Some(2)), vec![1, 2]);
    assert_eq!(study.count_first_nodes(Some(3)), 3);
    assert_eq!(
        search.search_first_nodes("word", Some(2)).unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        search.searchFirstNodes("word", Some(2)).unwrap(),
        search.search_first_nodes("word", Some(2)).unwrap()
    );
    let mut sets = HashMap::new();
    sets.insert("mywords", vec![1, 3]);
    assert_eq!(
        search.search_with_sets("mywords", &sets, None).unwrap(),
        vec![vec![1], vec![3]]
    );
    let mut sets = HashMap::new();
    sets.insert("w", vec![1, 2, 3, 4, 5]);
    assert_eq!(
        search.search_with_sets("w pos=noun", &sets, None).unwrap(),
        vec![vec![3], vec![5]]
    );
    let custom_study = search.study_with_sets("w pos=noun", &sets).unwrap();
    assert_eq!(custom_study.total_count(), 2);
    assert_eq!(custom_study.fetch(None), vec![vec![3], vec![5]]);
    assert_eq!(
        search
            .fetch_with_sets("w pos=noun", &sets, Some(1))
            .unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search.count_with_sets("w pos=noun", &sets, None).unwrap(),
        2
    );
    assert_eq!(
        search
            .count_with_sets("w pos=noun", &sets, Some(1))
            .unwrap(),
        1
    );
    assert_eq!(
        search
            .search_first_nodes_with_sets("w pos=noun", &sets, None)
            .unwrap(),
        vec![3, 5]
    );
    assert_eq!(
        search
            .searchFirstNodesWithSets("w pos=noun", &sets, None)
            .unwrap(),
        search
            .search_first_nodes_with_sets("w pos=noun", &sets, None)
            .unwrap()
    );
    assert_eq!(
        search
            .search_prefixes_with_sets("w pos=noun", &sets, 1, Some(1))
            .unwrap(),
        vec![vec![3]]
    );
    assert_eq!(
        search
            .searchPrefixesWithSets("w pos=noun", &sets, 1, Some(1))
            .unwrap(),
        search
            .search_prefixes_with_sets("w pos=noun", &sets, 1, Some(1))
            .unwrap()
    );
    assert_eq!(
        search
            .show_plan_with_sets("w pos=noun", &sets, false)
            .unwrap(),
        "template: w pos=noun\natoms: 1\nrelations: 0\nresults: 2\nquantifiers: false"
    );
    assert_eq!(
        search.showPlanWithSets("w pos=noun", &sets, false).unwrap(),
        search
            .show_plan_with_sets("w pos=noun", &sets, false)
            .unwrap()
    );
    let mapped_related = search
        .study(
            "
w:word
p:phrase
w ]] p
",
        )
        .unwrap();
    assert_eq!(mapped_related.plan().atom_count, 2);
    assert_eq!(mapped_related.plan().relation_count, 1);
    assert_eq!(mapped_related.fetch_first_nodes(None), vec![1, 2, 3, 4, 5]);
    assert_eq!(
        mapped_related.fetch_prefixes(1, None),
        vec![vec![1], vec![2], vec![3], vec![4], vec![5]]
    );
    assert_eq!(
        mapped_related.fetch_prefixes(2, Some(2)),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search
            .search_prefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap(),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search
            .searchPrefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap(),
        search
            .search_prefixes(
                "
w:word
p:phrase
w ]] p
",
                2,
                Some(2)
            )
            .unwrap()
    );
    assert_eq!(
        mapped_related.fetch_prefixes(0, None),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(
        mapped_related.fetch_prefixes(1, Some(0)),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(mapped_related.count_prefixes(2, None), 5);
    let mapped_phrase_children = search
        .study(
            "
p:phrase
w:word
w ]] p
",
        )
        .unwrap();
    assert_eq!(mapped_phrase_children.fetch_first_nodes(None), vec![6, 7]);
    assert_eq!(
        mapped_phrase_children.fetch_prefixes(1, Some(1)),
        vec![vec![6]]
    );
    let mut sets = HashMap::new();
    sets.insert("empty", Vec::new());
    assert!(
        search
            .search_with_sets("empty", &sets, None)
            .unwrap()
            .is_empty()
    );
    let mut sets = HashMap::new();
    sets.insert("single", vec![1]);
    assert_eq!(
        search.search_with_sets("single", &sets, None).unwrap(),
        vec![vec![1]]
    );
    let mut sets = HashMap::new();
    sets.insert("mywords", vec![2, 1]);
    sets.insert("myphrases", vec![6]);
    assert_eq!(
        search
            .search_with_sets(
                "
w:mywords
p:myphrases
w ]] p
",
                &sets,
                None
            )
            .unwrap(),
        vec![vec![1, 6], vec![2, 6]]
    );
    assert_eq!(
        search.search(".", None).unwrap(),
        vec![
            vec![1],
            vec![2],
            vec![3],
            vec![4],
            vec![5],
            vec![6],
            vec![7],
            vec![8]
        ]
    );
    assert_eq!(
        search.search(". pos=noun", None).unwrap(),
        vec![vec![3], vec![5]]
    );
    let generic_embedders = search
        .search(
            "
p:.
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(generic_embedders.len(), 10);
    assert!(generic_embedders.contains(&vec![6, 1]));
    assert!(generic_embedders.contains(&vec![7, 5]));
    assert!(generic_embedders.contains(&vec![8, 5]));
    assert_eq!(
        search
            .search(
                "
w1:word word=world
w2:word word=morning
w1 .pos. w2
",
                None
            )
            .unwrap(),
        vec![vec![3, 5]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=1
w .number=phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 6]]
    );
    assert_eq!(
        search
            .search(
                "
w:word word=hello
p:phrase phrase_id=2
w .number#phrase_id. p
",
                None
            )
            .unwrap(),
        vec![vec![1, 7]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .pos#pos. w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .word~.+~word. w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .word~^h~word. w2
",
                None
            )
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 .number<number. w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w:word
p:phrase
w -relation=subject> p
",
                None
            )
            .unwrap(),
        vec![vec![1, 6], vec![4, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
w:word
p <relation=predicate- w
",
                None
            )
            .unwrap(),
        vec![vec![6, 2], vec![7, 5]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 -distance=0> w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 2], vec![3, 4]]
    );

    let phrase_words = search.search("phrase\n  word", None).unwrap();
    assert_eq!(phrase_words.len(), 5);
    assert!(phrase_words.contains(&vec![6, 1]));
    assert!(phrase_words.contains(&vec![7, 5]));
    // A lonely operator as a first child is rejected, mirroring TF
    // ("Lonely relation: not allowed as first child").
    assert!(search.search("phrase\n  [[\n  word", None).is_err());
    assert!(search.search("word\n  ]]\n  phrase", None).is_err());
    // The operator-prefixed atom form embeds correctly (phrase contains word).
    assert_eq!(search.search("phrase\n  [[ word", None).unwrap(), phrase_words);
    // "word embedded in phrase" expressed as an explicit relation; the indented
    // `]] phrase` form would contradict the indentation embedding under TF's
    // two-edge semantics and is intentionally not equivalent.
    let mut word_in_phrase = search.search("w:word\np:phrase\nw ]] p", None).unwrap();
    word_in_phrase.sort_unstable();
    let mut expected_word_in_phrase: Vec<Vec<u32>> =
        phrase_words.iter().map(|row| vec![row[1], row[0]]).collect();
    expected_word_in_phrase.sort_unstable();
    assert_eq!(word_in_phrase, expected_word_in_phrase);
    assert_eq!(
        search.search("sentence\n  phrase", None).unwrap(),
        vec![vec![8, 6], vec![8, 7]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
  phrase phrase_id=1
  phrase phrase_id=2
",
                None
            )
            .unwrap(),
        vec![vec![8, 6, 7]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
  phrase phrase_id=1
    word word=world
  phrase phrase_id=2
    word word=morning
",
                None
            )
            .unwrap(),
        vec![vec![8, 6, 3, 7, 5]]
    );

    assert_eq!(
        search.search("phrase\n  word pos=noun", Some(2)).unwrap(),
        vec![vec![6, 3], vec![7, 5]]
    );
    assert_eq!(
        search
            .search("phrase\n  word pos=noun|adjective", None)
            .unwrap(),
        vec![vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );

    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 < w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=world
w2:word word=hello
w1 > w2
",
                None
            )
            .unwrap(),
        vec![vec![3, 1]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 < w2
",
                Some(3)
            )
            .unwrap(),
        vec![vec![1, 2], vec![1, 3], vec![1, 4]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word
w2:word
w1 <: w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 2], vec![2, 3], vec![3, 4], vec![4, 5]]
    );
    // Lonely `<:` as a first child is rejected, mirroring TF ("Lonely relation").
    assert!(search.search("w1:word\n  <:\n  w2:word", None).is_err());
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 == p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 6], vec![7, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 ## p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7], vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase
s && p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6], vec![8, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 || p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7], vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 << p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 >> p2
",
                None
            )
            .unwrap(),
        vec![vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 :> p2
",
                None
            )
            .unwrap(),
        vec![vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase
s =: p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase
s := p
",
                None
            )
            .unwrap(),
        vec![vec![8, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 :: p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 6], vec![7, 7]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 =2: w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w1 =1: w2
",
                None
            )
            .unwrap(),
        Vec::<Vec<u32>>::new()
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase phrase_id=1
s :2= p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6]]
    );
    assert_eq!(
        search
            .search(
                "
s:sentence
p:phrase phrase_id=1
s :2: p
",
                None
            )
            .unwrap(),
        vec![vec![8, 6]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase phrase_id=1
p2:phrase phrase_id=2
p1 <0: p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 7]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase phrase_id=2
p2:phrase phrase_id=1
p1 :0> p2
",
                None
            )
            .unwrap(),
        vec![vec![7, 6]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
w2:word word=world
w3:word word=good
w1 < w2
w2 < w3
",
                None
            )
            .unwrap(),
        vec![vec![1, 3, 4]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word word=hello
p:phrase
w2:word word=world
p [[ w1
p [[ w2
",
                None
            )
            .unwrap(),
        vec![vec![1, 6, 3]]
    );
    assert_eq!(
        search
            .search(
                "
w1:word pos=noun
w2:word pos=noun
w1 # w2
",
                None
            )
            .unwrap(),
        vec![vec![3, 5], vec![5, 3]]
    );
    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 = p2
",
                None
            )
            .unwrap(),
        vec![vec![6, 6], vec![7, 7]]
    );
    let phrase_contains_word = search
        .search(
            "
p:phrase
w:word
p [[ w
",
            None,
        )
        .unwrap();
    assert_eq!(phrase_contains_word.len(), 5);
    assert!(phrase_contains_word.contains(&vec![6, 1]));
    assert!(phrase_contains_word.contains(&vec![7, 5]));

    assert_eq!(
        search
            .search(
                "
p:phrase
  [[ w:word
",
                None
            )
            .unwrap(),
        phrase_contains_word
    );

    let word_in_phrase = search
        .search(
            "
w:word
p:phrase
w ]] p
",
            None,
        )
        .unwrap();
    assert_eq!(word_in_phrase.len(), 5);
    assert!(word_in_phrase.contains(&vec![1, 6]));
    assert!(word_in_phrase.contains(&vec![5, 7]));
    // The indented `]] phrase` form requires the phrase to be embedded in the
    // single-slot word via the indentation edge (impossible under TF two-edge
    // semantics), so it yields no results. The flat relation form is the correct
    // way to express "word embedded in phrase".
    assert!(
        search
            .search("w:word\n  ]] p:phrase", None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
w:word pos=noun|adjective
p [[ w
",
                None
            )
            .unwrap(),
        vec![vec![6, 2], vec![6, 3], vec![7, 4], vec![7, 5]]
    );
    let parent_forward = search
        .search(
            "
w:word
p:phrase
w -parent> p
",
            None,
        )
        .unwrap();
    assert_eq!(parent_forward.len(), 5);
    assert!(parent_forward.contains(&vec![1, 6]));
    assert!(parent_forward.contains(&vec![5, 7]));
    // Lonely edge operator as a first child is rejected (TF "Lonely relation").
    assert!(search.search("w:word\n  -parent>\n  p:phrase", None).is_err());

    let parent_backward = search
        .search(
            "
p:phrase
w:word
p <parent- w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_backward.len(), 5);
    assert!(parent_backward.contains(&vec![6, 1]));
    assert!(parent_backward.contains(&vec![7, 5]));

    let parent_either = search
        .search(
            "
p:phrase
w:word
p <parent> w
",
            None,
        )
        .unwrap();
    assert_eq!(parent_either.len(), 5);
    assert!(parent_either.contains(&vec![6, 1]));
    assert!(parent_either.contains(&vec![7, 5]));

    assert_eq!(
        search
            .search(
                "
p:phrase
w:word
p <relation=subject> w
",
                None
            )
            .unwrap(),
        vec![vec![6, 1], vec![7, 4]]
    );

    assert_eq!(
        search
            .search(
                "
phrase
/with/
  % comment inside quantified block
  word pos=interjection
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  .. < w
  w:word word=hello
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  w:word word=hello
  .. # w
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  w:word word=hello
  .. .phrase_id=number. w
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/without/
  word pos=interjection
/-/
",
                None
            )
            .unwrap(),
        vec![vec![7]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
/with/
  phrase phrase_id=1
  phrase phrase_id=2
/-/
",
                None
            )
            .unwrap(),
        vec![vec![8]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
/where/
  word word=hello
/have/
  word word=world
/-/
",
                None
            )
            .unwrap(),
        vec![vec![8]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/where/
  w:word pos=adjective
/have/
  w number=1
/-/
",
                None
            )
            .unwrap(),
        vec![vec![7]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/with/
  word word=hello
/or/
  word word=good
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6], vec![7]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/with/
  w1:word word=hello
  w2:word word=beautiful
  w1 <: w2
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  .. [[ w
  w:word word=hello
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
p:phrase
/with/
  w:word word=hello
  w ]] ..
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
phrase
/with/
  .. phrase_id=1
/-/
",
                None
            )
            .unwrap(),
        vec![vec![6]]
    );
    assert_eq!(
        search
            .search(
                "
sentence
/with/
  phrase
  /with/
    word word=hello
  /-/
/-/
",
                None
            )
            .unwrap(),
        vec![vec![8]]
    );
    // The quantifier attaches to the last base atom (`p`, the phrase), per TF.
    // Phrase 6 contains a word=hello, so all words embedded in phrase 6 match.
    let mut multi_atom_quant = search
        .search(
            "
w:word
p:phrase
w ]] p
/with/
  word word=hello
/-/
",
            None,
        )
        .unwrap();
    multi_atom_quant.sort_unstable();
    assert_eq!(multi_atom_quant, vec![vec![1, 6], vec![2, 6], vec![3, 6]]);
}

#[test]
fn mapped_adjacent_before_uses_slot_boundaries_not_node_ids() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\n1-4\tword\n5\tphrase\n6\tsentence\n7\tphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n5\t1-2\n6\t1-4\n7\t3-4\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("word.tf"),
        "@node\n@valueType=str\n\n1\ta\n2\tb\n3\tc\n4\td\n",
    )
    .unwrap();

    let cache_path = dir.path().join("synthetic.cfr");
    compile_features(dir.path(), &cache_path, &["otype", "oslots", "word"]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);

    assert_eq!(
        search
            .search(
                "
p1:phrase
p2:phrase
p1 <: p2
",
                None
            )
            .unwrap(),
        vec![vec![5, 7]]
    );
}

#[test]
fn golden_master_public_api_probes_match_python_when_available() {
    let script = repo_path("libs/core/tests/golden/compare.py");
    let output = std::process::Command::new("python3")
        .arg(script)
        .arg("--mode")
        .arg("materialized")
        .arg("--mode")
        .arg("mapped")
        .current_dir(repo_path("libs/core"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "golden comparison failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn committed_golden_masters_are_regenerable_from_python_when_available() {
    let python = repo_path("libs/core/.venv/bin/python");
    if !python.exists() {
        eprintln!(
            "skipping Python golden authenticity test: missing {}",
            python.to_string_lossy()
        );
        return;
    }
    let corpora = ["bhsa", "n1904", "banks"]
        .into_iter()
        .filter(|name| optional_corpus_tf(name).is_some())
        .collect::<Vec<_>>();
    if corpora.is_empty() {
        return;
    }

    let temp_dir = tempfile::tempdir().unwrap();
    let script = repo_path("libs/core/tests/golden/gen_golden.py");
    let output = std::process::Command::new(&python)
        .arg(script)
        .arg("--output-dir")
        .arg(temp_dir.path())
        .args(&corpora)
        .current_dir(repo_path(""))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Python golden regeneration failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    for corpus in corpora {
        let expected = fs::read_to_string(repo_path(&format!(
            "libs/core/tests/golden/golden/{corpus}.json"
        )))
        .unwrap();
        let regenerated = fs::read_to_string(temp_dir.path().join(format!("{corpus}.json")))
            .unwrap_or_else(|error| panic!("missing regenerated golden for {corpus}: {error}"));
        assert_eq!(regenerated, expected, "golden mismatch for {corpus}");
    }
}

#[test]
fn loads_bhsa_and_runs_lexical_queries() {
    let Some(path) = bhsa_tf() else {
        return;
    };
    let corpus = Corpus::load_features(&path, &["otype", "oslots", "sp"]).unwrap();
    assert_eq!(corpus.nodes_of_type("word").len(), 426_590);
    assert_eq!(corpus.nodes_of_type("book").len(), 39);
    assert_eq!(corpus.node_type(1), Some("word"));
    assert_eq!(corpus.node_type(426_591), Some("book"));
    assert_eq!(corpus.slots(1), vec![1]);
    assert_eq!(corpus.node_type_interval("word"), Some((1, 426_590)));
    assert_eq!(corpus.sInterval("word"), Some((1, 426_590)));
    assert_eq!(corpus.node_type_interval("book"), Some((426_591, 426_629)));

    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("bhsa-lexical.cfr");
    compile_features(path, &cache_path, &["otype", "oslots", "sp"]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let verbs = MappedSearch::new(&mapped)
        .search("word sp=verb", Some(10))
        .unwrap();
    assert_eq!(verbs.len(), 10);
    for row in verbs {
        let node = row[0];
        assert_eq!(
            corpus.node_feature("sp").unwrap().value(node),
            Some(&FeatureValue::string("verb"))
        );
    }
}

#[test]
fn renders_bhsa_text_from_otext_format_features() {
    let Some(path) = bhsa_tf() else {
        return;
    };
    let corpus = Corpus::load_features(
        path,
        &[
            "otype",
            "oslots",
            "qere_utf8",
            "g_word_utf8",
            "qere_trailer_utf8",
            "trailer_utf8",
            "qere",
            "g_word",
            "qere_trailer",
            "trailer",
        ],
    )
    .unwrap();

    assert!(!corpus.text(1, None).is_empty());
    assert_eq!(
        corpus.text(1, Some("text-orig-full")),
        corpus.text(1, Some("fmt:text-orig-full"))
    );
    assert!(!corpus.text(1, Some("text-trans-full")).is_empty());
    assert_ne!(
        corpus.text(1, Some("text-orig-full")),
        corpus.text(1, Some("text-trans-full"))
    );

    let representations = corpus.text_representations();
    assert!(
        representations
            .formats
            .iter()
            .any(|format| format.name == "text-full"
                && format.original_spec
                    == "{qere_utf8/g_word_utf8}{qere_trailer_utf8/trailer_utf8}"
                && format.transliteration_spec == "{qere/g_word}{qere_trailer/trailer}"
                && format.total_samples > 0
                && format.unique_characters > 0)
    );
}

#[test]
fn runs_representative_bhsa_curated_query_shapes() {
    let Some(path) = bhsa_tf() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("bhsa-curated.cfr");
    compile_features(
        path,
        &cache_path,
        &[
            "otype", "oslots", "sp", "vt", "vs", "gn", "nu", "language", "function", "typ", "kind",
        ],
    )
    .unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);

    let lexical = search.search("word sp=verb vt=perf", Some(5)).unwrap();
    assert_eq!(lexical.len(), 5);

    let structural = search
        .search(
            "
phrase typ=VP
  word sp=verb
",
            Some(5),
        )
        .unwrap();
    assert_eq!(structural.len(), 5);

    let quantified = search
        .search(
            "
phrase typ=NP
/with/
  word sp=art
/-/
",
            Some(5),
        )
        .unwrap();
    assert_eq!(quantified.len(), 5);
}

#[test]
fn bhsa_phase1_spot_checks() {
    if std::env::var_os("CF_SPOTCHECK").is_none() {
        eprintln!("skipping bhsa_phase1_spot_checks (set CF_SPOTCHECK=1 to run)");
        return;
    }
    let Some(source) = optional_corpus_tf("bhsa") else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("bhsa-spotcheck.cfr");
    compile_features(&source, &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped);

    let q31 = "sentence\n  := word rank_lex<100";
    let n31 = search.count(q31, None).unwrap();
    eprintln!("SPOT q31 (expect 28095): {n31}");

    let q33 = "word vbe#H=";
    let r33 = search.count(q33, None);
    eprintln!("SPOT q33 (expect parses ok): {r33:?}");

    let q25 = "verse verse=1 chapter=1 book=Genesis\n  sentence\n    word nu=sg\n    <: word nu=pl";
    let n25 = search.count(q25, None).unwrap();
    eprintln!("SPOT q25 (expect 1): {n25}");

    let q11 = "chapter book=Genesis chapter=1\n  w1:word lex=>RY/\n  :30> w2:word lex=>RY/";
    let n11 = search.count(q11, None).unwrap();
    eprintln!("SPOT q11 (expect 50): {n11}");

    assert_eq!(n31, 28095, "q31 sentence := word rank_lex<100");
    assert!(r33.is_ok(), "q33 word vbe#H= must parse");
    assert_eq!(n25, 1, "q25 nu=sg <: nu=pl in Genesis 1:1");
    assert_eq!(n11, 50, "q11 lex=>RY/ :30> lex=>RY/ in Genesis 1");
}
