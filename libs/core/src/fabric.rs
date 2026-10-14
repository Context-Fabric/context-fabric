use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::cache::{CacheLocations, ensure_compiled};
use crate::compiled::{MappedCompiledCorpus, compile_features, compile_loaded_corpus};
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

    pub fn explore(&self) -> Result<FeatureInventory> {
        explore_feature_paths(self.module_paths())
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

    /// Preferred location of the compiled cache `load` builds (inside the
    /// last module directory). See [`crate::cache`] for the fallback used
    /// when that directory is read-only.
    pub fn cfr_cache_path(&self) -> PathBuf {
        CacheLocations::for_module_paths(&self.cache_module_paths()).in_corpus
    }

    pub fn load_mapped(&self, features: impl FeatureSpec) -> Result<MappedCompiledCorpus> {
        let _ = features.feature_names();
        let module_paths = self.cache_module_paths();
        let cache_path = ensure_compiled(&module_paths, |output| {
            if module_paths.len() > 1 {
                let corpus = Corpus::load_paths(module_paths.clone())?.with_rank_arrays();
                compile_loaded_corpus(&corpus, output)
            } else {
                compile_features(&module_paths[0], output, &[])
            }
        })?;
        MappedCompiledCorpus::open(cache_path)
    }

    pub fn load_all(&self) -> Result<MappedCompiledCorpus> {
        self.load_mapped(Vec::<String>::new())
    }

    pub fn load(&self, features: impl FeatureSpec) -> Result<MappedCompiledCorpus> {
        self.load_mapped(features)
    }

    pub fn compile(&self, output_path: impl AsRef<Path>, features: impl FeatureSpec) -> Result<()> {
        let features = features.feature_names();
        let feature_refs = features.iter().map(String::as_str).collect::<Vec<_>>();
        let module_paths = self.module_paths();
        // Multi-location compile: merge every location's warp + data features into
        // one in-memory corpus before compiling (TF semantics: later locations add
        // features and override same-named earlier ones). Compiling only the last
        // location — as the single-location path does — drops `otype`/`oslots`
        // when they live in an earlier location, raising "missing required
        // feature: otype". Mirrors `load_mapped`'s multi-path branch.
        if module_paths.len() > 1 {
            let corpus = Corpus::load_paths(module_paths.clone())?.with_rank_arrays();
            compile_loaded_corpus(&corpus, output_path.as_ref())
        } else {
            compile_features(
                module_paths.last().unwrap_or(&self.path),
                output_path,
                &feature_refs,
            )
        }
    }

    pub fn open_mapped(&self, cache_path: impl AsRef<Path>) -> Result<MappedCompiledCorpus> {
        MappedCompiledCorpus::open(cache_path)
    }

    /// Existing module directories, falling back to the base path so a missing
    /// corpus surfaces as a "not found" error for that path.
    fn cache_module_paths(&self) -> Vec<PathBuf> {
        let module_paths = self.module_paths();
        if module_paths.is_empty() {
            vec![self.path.clone()]
        } else {
            module_paths
        }
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
