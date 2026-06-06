use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::compiled::{MappedCompiledCorpus, compile_features, load_compiled};
use crate::config::{BANNER, VERSION};
use crate::corpus::Corpus;
use crate::error::Result;
use crate::explore::{FeatureInventory, explore_feature_paths};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fabric {
    path: PathBuf,
    locations: Vec<PathBuf>,
    modules: Vec<String>,
    banner: String,
    version: String,
    good: bool,
    features_requested: Vec<String>,
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

    pub fn features_ignored(&self) -> BTreeMap<String, Vec<PathBuf>> {
        self.ignored_feature_paths().unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn featuresIgnored(&self) -> BTreeMap<String, Vec<PathBuf>> {
        self.features_ignored()
    }

    pub fn explore(&self) -> Result<FeatureInventory> {
        explore_feature_paths(self.module_paths())
    }

    pub fn feature_inventory(&self) -> Result<FeatureInventory> {
        self.explore()
    }

    pub fn feature_catalog(&self) -> Result<FeatureInventory> {
        self.explore()
    }

    pub fn ignored_feature_paths(&self) -> Result<BTreeMap<String, Vec<PathBuf>>> {
        Ok(self
            .explore()?
            .paths
            .into_iter()
            .filter_map(|(name, paths)| {
                (paths.len() > 1).then(|| {
                    let ignored = paths[..paths.len() - 1].to_vec();
                    (name, ignored)
                })
            })
            .collect())
    }

    pub fn load_all(&self) -> Result<Corpus> {
        Corpus::load_paths(self.module_paths())
    }

    #[allow(non_snake_case)]
    pub fn loadAll(&self) -> Result<Corpus> {
        self.load_all()
    }

    pub fn load(&self, features: impl FeatureSpec) -> Result<Corpus> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        Corpus::load_features_from_paths(self.module_paths(), &feature_refs)
    }

    pub fn load_add(&self, corpus: &mut Corpus, features: impl FeatureSpec) -> Result<bool> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        corpus.add_features_from_paths(self.module_paths(), &feature_refs)
    }

    #[allow(non_snake_case)]
    pub fn loadAdd(&self, corpus: &mut Corpus, features: impl FeatureSpec) -> Result<bool> {
        self.load_add(corpus, features)
    }

    pub fn ensure_loaded(&self, corpus: &mut Corpus, features: impl FeatureSpec) -> Result<bool> {
        self.load_add(corpus, features)
    }

    #[allow(non_snake_case)]
    pub fn ensureLoaded(&self, corpus: &mut Corpus, features: impl FeatureSpec) -> Result<bool> {
        self.ensure_loaded(corpus, features)
    }

    pub fn save(
        &self,
        corpus: &Corpus,
        location: Option<&Path>,
        module: Option<&str>,
    ) -> Result<bool> {
        let base = location
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.path.clone());
        let output = module
            .filter(|module| !module.is_empty())
            .map(|module| base.join(module))
            .unwrap_or(base);
        corpus.save(output)
    }

    pub fn compile(&self, output_path: impl AsRef<Path>, features: impl FeatureSpec) -> Result<()> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        compile_features(
            self.module_paths().last().unwrap_or(&self.path),
            output_path,
            &feature_refs,
        )
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

    fn module_paths(&self) -> Vec<PathBuf> {
        self.locations
            .iter()
            .flat_map(|location| {
                self.modules.iter().map(move |module| {
                    if module.is_empty() {
                        location.clone()
                    } else {
                        location.join(module)
                    }
                })
            })
            .filter(|path| path.exists())
            .collect()
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
