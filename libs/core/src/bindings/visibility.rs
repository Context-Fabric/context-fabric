//! Which features a `Corpus` (TF `api`) exposes under `F` / `E` / `Fs` / `Es`.
//!
//! The memory-mapped corpus always carries every feature, but Text-Fabric only
//! exposes what was requested: `TF.load("sp lex")` makes `F.sp`, `F.lex`, the
//! warp features (`otype`, `oslots`), and the features the `otext` config needs
//! for the T-API (section, structure, and format features) available, and
//! `F.<other>` raises `AttributeError`. `TF.load(..., add=True)` and
//! `api.ensureLoaded(...)` widen the set in place.

use std::collections::BTreeSet;
use std::sync::RwLock;

use crate::config::{OSLOTS, OTYPE};
use crate::error::Result;
use crate::mapped::{FormatTemplate, MappedCompiledCorpus};

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

/// The features TF loads implicitly for the T-API (`TF.textFeatures` in
/// `tf/core/fabric.py`): section features (and the language variants of the
/// first one) when sections are configured, structure features when structure
/// is configured, and every feature referenced by a `fmt:` text format.
pub(crate) fn text_features(corpus: &MappedCompiledCorpus) -> Result<BTreeSet<String>> {
    let otext = corpus.otext()?;
    let mut features = BTreeSet::new();
    if otext.has_sections() {
        let variant_prefix = format!("{}@", otext.section_features[0]);
        features.extend(
            corpus
                .metadata()
                .node_features
                .iter()
                .filter(|feature| feature.name.starts_with(&variant_prefix))
                .map(|feature| feature.name.clone()),
        );
        features.extend(otext.section_features.iter().cloned());
    }
    if otext.has_structure() {
        features.extend(otext.structure_features.iter().cloned());
    }
    for template in otext.formats.values() {
        let template = FormatTemplate::parse(template);
        features.extend(template.features().into_iter().map(str::to_string));
    }
    Ok(features)
}
