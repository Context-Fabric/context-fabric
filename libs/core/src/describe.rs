use std::collections::BTreeMap;

use crate::corpus::{
    Corpus, CorpusDescription, CorpusOverview, FeatureCatalogEntry, FeatureDescription,
    FeatureKind, TextRepresentationInfo,
};

pub fn describe_corpus_overview(corpus: &Corpus, name: impl Into<String>) -> CorpusOverview {
    corpus.overview(name)
}

pub fn describe_corpus(corpus: &Corpus, name: impl Into<String>) -> CorpusDescription {
    corpus.describe_corpus(name)
}

pub fn list_features(
    corpus: &Corpus,
    kind: Option<FeatureKind>,
    node_types: Option<&[&str]>,
) -> Vec<FeatureCatalogEntry> {
    corpus.feature_catalog(kind, node_types)
}

pub fn describe_feature(
    corpus: &Corpus,
    feature_name: &str,
    sample_limit: usize,
) -> FeatureDescription {
    corpus.describe_feature(feature_name, sample_limit)
}

pub fn describe_features(
    corpus: &Corpus,
    feature_names: &[&str],
    sample_limit: usize,
) -> BTreeMap<String, FeatureDescription> {
    corpus.describe_features(feature_names, sample_limit)
}

pub fn describe_text_formats(corpus: &Corpus) -> TextRepresentationInfo {
    corpus.text_representations()
}

pub fn get_feature_otypes(corpus: &Corpus, feature_name: &str) -> Vec<String> {
    corpus
        .node_feature_types(feature_name)
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub fn get_all_feature_otypes(corpus: &Corpus) -> BTreeMap<String, Vec<String>> {
    corpus.all_node_feature_types()
}
