//! Fixtures shared by the integration tests: the bundled mini corpus, small
//! generated corpora, and the optional real corpora.
#![allow(dead_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use context_fabric_core::{MappedCompiledCorpus, compile_features};

pub fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// The bundled mini corpus: words 1..5 are slots; phrase 6 = slots 1-3,
/// phrase 7 = slots 4-5, sentence 8 = slots 1-5.
pub fn mini_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini_corpus")
}

/// Copies the `.tf` files of the mini corpus (no caches) into `dest`.
pub fn copy_mini_corpus(dest: &Path) -> PathBuf {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(mini_corpus_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "tf") {
            fs::copy(&path, dest.join(path.file_name().unwrap())).unwrap();
        }
    }
    dest.to_path_buf()
}

/// Compiles the mini corpus into `dir` and opens it. The corpus maps a file in
/// `dir`, which must outlive it.
pub fn mapped_mini_corpus(dir: &tempfile::TempDir) -> MappedCompiledCorpus {
    let cache = dir.path().join("mini.cfr");
    compile_features(mini_corpus_dir(), &cache, &[]).unwrap();
    MappedCompiledCorpus::open(&cache).unwrap()
}

/// The mini corpus with structure configured on sentences and phrases.
pub fn structured_mini_corpus(temp_dir: &tempfile::TempDir) -> PathBuf {
    let source = copy_mini_corpus(&temp_dir.path().join("structured"));
    let otext_path = source.join("otext.tf");
    let otext = fs::read_to_string(&otext_path)
        .unwrap()
        .replace(
            "@structureFeatures=",
            "@structureFeatures=sentence_id,phrase_id",
        )
        .replace("@structureTypes=", "@structureTypes=sentence,phrase");
    fs::write(&otext_path, otext).unwrap();
    source
}

/// Writes `files` (name, contents) into `dir`.
pub fn write_files(dir: &Path, files: &[(&str, &str)]) {
    fs::create_dir_all(dir).unwrap();
    for (name, contents) in files {
        fs::write(dir.join(name), contents).unwrap();
    }
}

/// A book > chapter > verse > word corpus that exercises every level of the
/// `CFRSECT1` sections data: words 1..6, verses 7 (1-3) and 8 (4-6), chapter 9
/// and book 10 (1-6).
pub fn write_three_level_corpus(dir: &Path) {
    write_files(
        dir,
        &[
            (
                "otype.tf",
                "@node\n@valueType=str\n\n\
                 word\nword\nword\nword\nword\nword\n\
                 7\tverse\n8\tverse\n9\tchapter\n10\tbook\n",
            ),
            (
                "oslots.tf",
                "@edge\n@valueType=int\n\n7\t1-3\n8\t4-6\n9\t1-6\n10\t1-6\n",
            ),
            ("word.tf", "@node\n@valueType=str\n\na\nb\nc\nd\ne\nf\n"),
            ("bookname.tf", "@node\n@valueType=str\n\n10\tGenesis\n"),
            ("chapternum.tf", "@node\n@valueType=int\n\n9\t1\n"),
            ("versenum.tf", "@node\n@valueType=int\n\n7\t1\n8\t2\n"),
            (
                "otext.tf",
                "@config\n@fmt:text-orig-full={word}\n\
                 @sectionTypes=book,chapter,verse\n\
                 @sectionFeatures=bookname,chapternum,versenum\n\
                 @structureTypes=\n@structureFeatures=\n",
            ),
        ],
    );
}

/// Compiles the three-level corpus into `dir` and opens it.
pub fn mapped_three_level_corpus(dir: &tempfile::TempDir) -> MappedCompiledCorpus {
    let source = dir.path().join("three_level");
    write_three_level_corpus(&source);
    let cache = dir.path().join("three_level.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    MappedCompiledCorpus::open(&cache).unwrap()
}

/// `<corpora>/<name>/tf` of a real corpus, or `None` (skipping the caller)
/// when absent. `CFABRIC_CORPORA` overrides the default corpora directory;
/// `CF_REQUIRE_CORPORA` turns an absent corpus into a failure.
pub fn optional_corpus_tf(name: &str) -> Option<PathBuf> {
    let corpora = env::var_os("CFABRIC_CORPORA")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path("libs/benchmarks/.corpora"));
    let path = corpora.join(name).join("tf");
    if path.join("otype.tf").exists() {
        Some(path)
    } else if env::var_os("CF_REQUIRE_CORPORA").is_some() {
        panic!("required corpus '{name}' is absent at {}", path.display());
    } else {
        eprintln!(
            "skipping corpus-dependent test: '{name}' is absent at {}",
            path.display()
        );
        None
    }
}

pub fn bhsa_tf() -> Option<PathBuf> {
    optional_corpus_tf("bhsa")
}
