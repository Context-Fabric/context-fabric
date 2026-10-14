//! Which features a `Corpus` (TF `api`) exposes under `F` / `E` / `Fs` / `Es`.
//!
//! The memory-mapped corpus always carries every feature, but Text-Fabric only
//! exposes what was requested: `TF.load("sp lex")` makes `F.sp`, `F.lex`, the
//! warp features (`otype`, `oslots`), and the features the `otext` config needs
//! for the T-API (section, structure, and format features) available, and
//! `F.<other>` raises `AttributeError`. `TF.load(..., add=True)` and
//! `api.ensureLoaded(...)` widen the set in place.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::RwLock;

use crate::config::{OSLOTS, OTEXT, OTYPE};
use crate::error::Result;
use crate::mapped::MappedCompiledCorpus;
use crate::mapped::text::collect_formats;

/// The set of loaded (exposed) feature names, or "everything" for `loadAll`.
pub(crate) struct FeatureVisibility {
    loaded: RwLock<Option<BTreeSet<String>>>,
}

impl FeatureVisibility {
    /// Expose every feature in the corpus (`TF.loadAll()`, `openMapped`).
    pub(crate) fn all() -> Self {
        Self {
            loaded: RwLock::new(None),
        }
    }

    /// Expose `requested` plus the warp and T-API features, like `TF.load`.
    pub(crate) fn requested(corpus: &MappedCompiledCorpus, requested: &[String]) -> Result<Self> {
        let mut loaded: BTreeSet<String> = requested.iter().cloned().collect();
        loaded.insert(OTYPE.to_string());
        loaded.insert(OSLOTS.to_string());
        loaded.extend(text_features(corpus)?);
        Ok(Self {
            loaded: RwLock::new(Some(loaded)),
        })
    }

    /// Whether `name` is exposed.
    pub(crate) fn exposes(&self, name: &str) -> bool {
        self.loaded
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .as_ref()
            .is_none_or(|loaded| loaded.contains(name))
    }

    /// Expose additional features (`load(..., add=True)`, `ensureLoaded`).
    pub(crate) fn extend<I: IntoIterator<Item = String>>(&self, names: I) {
        if let Some(loaded) = self
            .loaded
            .write()
            .unwrap_or_else(|err| err.into_inner())
            .as_mut()
        {
            loaded.extend(names);
        }
    }
}

/// Split an `otext` comma-separated list, like TF's `itemize(value, ",")`.
fn itemize(value: Option<&str>) -> Vec<String> {
    value
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The features TF loads implicitly for the T-API (`TF.textFeatures` in
/// `tf/core/fabric.py`): section features (and the language variants of the
/// first one) when sections are configured, structure features when structure
/// is configured, and every feature referenced by a `fmt:` text format.
pub(crate) fn text_features(corpus: &MappedCompiledCorpus) -> Result<BTreeSet<String>> {
    let mut features = BTreeSet::new();
    let Some(otext) = corpus.config_feature(OTEXT)? else {
        return Ok(features);
    };
    let config: BTreeMap<String, String> = otext
        .metadata()?
        .into_iter()
        .map(|(key, value)| (key, value.unwrap_or_default()))
        .collect();
    let section_types = itemize(config.get("sectionTypes").map(String::as_str));
    let section_feats = itemize(config.get("sectionFeatures").map(String::as_str));
    if (1..=3).contains(&section_types.len()) && (1..=3).contains(&section_feats.len()) {
        let base = section_feats[0].clone();
        let prefix = format!("{base}@");
        features.extend(
            corpus
                .metadata()
                .node_features
                .iter()
                .filter(|feature| feature.name.starts_with(&prefix))
                .map(|feature| feature.name.clone()),
        );
        features.extend(section_feats);
    }
    let structure_types = itemize(config.get("structureTypes").map(String::as_str));
    let structure_feats = itemize(config.get("structureFeatures").map(String::as_str));
    if !structure_types.is_empty() && !structure_feats.is_empty() {
        features.extend(structure_feats);
    }
    features.extend(collect_formats(&config).1);
    Ok(features)
}
