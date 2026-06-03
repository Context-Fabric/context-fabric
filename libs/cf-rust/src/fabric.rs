use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::compiled::{MappedCompiledCorpus, compile_features, load_compiled};
use crate::config::{BANNER, VERSION};
use crate::corpus::Corpus;
use crate::error::Result;
use crate::explore::{FeatureInventory, explore_features};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fabric {
    path: PathBuf,
    locations: Vec<PathBuf>,
    modules: Vec<String>,
    banner: String,
    version: String,
    good: bool,
    features_requested: Vec<String>,
    features_ignored: BTreeMap<String, Vec<PathBuf>>,
}

impl Fabric {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self {
            locations: vec![path.clone()],
            path,
            modules: vec![String::new()],
            banner: BANNER.to_string(),
            version: VERSION.to_string(),
            good: true,
            features_requested: Vec::new(),
            features_ignored: BTreeMap::new(),
        }
    }

    pub fn from_locations<I, P>(locations: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut locations = locations
            .into_iter()
            .map(|location| location.as_ref().to_path_buf())
            .collect::<Vec<_>>();
        if locations.is_empty() {
            locations.push(PathBuf::new());
        }
        let path = locations[0].clone();

        Self {
            path,
            locations,
            modules: vec![String::new()],
            banner: BANNER.to_string(),
            version: VERSION.to_string(),
            good: true,
            features_requested: Vec::new(),
            features_ignored: BTreeMap::new(),
        }
    }

    pub fn with_modules<I, S>(path: impl AsRef<Path>, modules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut fabric = Self::new(path);
        fabric.modules = modules
            .into_iter()
            .map(|module| module.as_ref().to_string())
            .collect();
        if fabric.modules.is_empty() {
            fabric.modules.push(String::new());
        }
        fabric
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn locations(&self) -> &[PathBuf] {
        &self.locations
    }

    pub fn modules(&self) -> &[String] {
        &self.modules
    }

    pub fn banner(&self) -> &str {
        &self.banner
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn good(&self) -> bool {
        self.good
    }

    pub fn location_rep(&self) -> String {
        self.locations
            .iter()
            .map(|location| location.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(non_snake_case)]
    pub fn locationRep(&self) -> String {
        self.location_rep()
    }

    pub fn features_requested(&self) -> &[String] {
        &self.features_requested
    }

    #[allow(non_snake_case)]
    pub fn featuresRequested(&self) -> &[String] {
        self.features_requested()
    }

    pub fn features_ignored(&self) -> &BTreeMap<String, Vec<PathBuf>> {
        &self.features_ignored
    }

    #[allow(non_snake_case)]
    pub fn featuresIgnored(&self) -> &BTreeMap<String, Vec<PathBuf>> {
        self.features_ignored()
    }

    pub fn explore(&self) -> Result<FeatureInventory> {
        explore_features(&self.path)
    }

    pub fn load_all(&self) -> Result<Corpus> {
        Corpus::load(&self.path)
    }

    #[allow(non_snake_case)]
    pub fn loadAll(&self) -> Result<Corpus> {
        self.load_all()
    }

    pub fn load(&self, features: impl FeatureSpec) -> Result<Corpus> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        Corpus::load_features(&self.path, &feature_refs)
    }

    pub fn load_add(&self, corpus: &mut Corpus, features: impl FeatureSpec) -> Result<bool> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        corpus.add_features_from(&self.path, &feature_refs)
    }

    #[allow(non_snake_case)]
    pub fn loadAdd(&self, corpus: &mut Corpus, features: impl FeatureSpec) -> Result<bool> {
        self.load_add(corpus, features)
    }

    pub fn compile(&self, output_path: impl AsRef<Path>, features: impl FeatureSpec) -> Result<()> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        compile_features(&self.path, output_path, &feature_refs)
    }

    pub fn load_compiled(&self, cache_path: impl AsRef<Path>) -> Result<Corpus> {
        load_compiled(cache_path)
    }

    #[allow(non_snake_case)]
    pub fn loadCompiled(&self, cache_path: impl AsRef<Path>) -> Result<Corpus> {
        self.load_compiled(cache_path)
    }

    pub fn open_mapped(&self, cache_path: impl AsRef<Path>) -> Result<MappedCompiledCorpus> {
        MappedCompiledCorpus::open(cache_path)
    }

    #[allow(non_snake_case)]
    pub fn openMapped(&self, cache_path: impl AsRef<Path>) -> Result<MappedCompiledCorpus> {
        self.open_mapped(cache_path)
    }
}

pub trait FeatureSpec {
    fn feature_names(self) -> Vec<String>;
}

impl FeatureSpec for &str {
    fn feature_names(self) -> Vec<String> {
        self.split(|ch: char| ch.is_whitespace() || ch == ',')
            .filter(|feature| !feature.is_empty())
            .map(str::to_string)
            .collect()
    }
}

impl FeatureSpec for String {
    fn feature_names(self) -> Vec<String> {
        self.as_str().feature_names()
    }
}

impl<const N: usize> FeatureSpec for [&str; N] {
    fn feature_names(self) -> Vec<String> {
        self.into_iter().map(str::to_string).collect()
    }
}

impl FeatureSpec for &[&str] {
    fn feature_names(self) -> Vec<String> {
        self.iter().map(|feature| (*feature).to_string()).collect()
    }
}

impl FeatureSpec for Vec<&str> {
    fn feature_names(self) -> Vec<String> {
        self.into_iter().map(str::to_string).collect()
    }
}

impl FeatureSpec for Vec<String> {
    fn feature_names(self) -> Vec<String> {
        self
    }
}
