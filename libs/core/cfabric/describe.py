"""Corpus description utilities for Context Fabric.

This module provides centralized utilities for describing corpora,
features, and text representations. It generates exhaustive samples
for text format character coverage.

Usage:
    >>> from cfabric.describe import describe_corpus, describe_feature
    >>> result = describe_corpus(api, "BHSA")
    >>> feature_info = describe_feature(api, "sp")
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from cfabric.core.api import Api

__all__ = [
    "describe_corpus",
    "describe_corpus_overview",
    "describe_feature",
    "describe_features",
    "describe_text_formats",
    "list_features",
    "get_feature_otypes",
    "get_all_feature_otypes",
    "CorpusDescription",
    "CorpusOverview",
    "FeatureDescription",
    "FeatureCatalogEntry",
    "TextRepresentationInfo",
    "TextFormatInfo",
    "TextFormatSample",
]


# =============================================================================
# Dataclasses
# =============================================================================


@dataclass
class TextFormatSample:
    """A single text sample showing original and transliterated forms."""

    original: str
    transliterated: str

    def to_dict(self) -> dict[str, str]:
        return {"original": self.original, "transliterated": self.transliterated}


@dataclass
class TextFormatInfo:
    """Information about a text format pair (orig/trans)."""

    name: str
    original_spec: str
    transliteration_spec: str
    samples: list[TextFormatSample] = field(default_factory=list)
    unique_characters: int = 0
    total_samples: int = 0

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "original_script": self.original_spec,
            "transliteration": self.transliteration_spec,
            "samples": [s.to_dict() for s in self.samples],
            "unique_characters": self.unique_characters,
            "total_samples": self.total_samples,
        }


@dataclass
class TextRepresentationInfo:
    """Complete text representation info for a corpus."""

    description: str
    formats: list[TextFormatInfo] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "description": self.description,
            "formats": [f.to_dict() for f in self.formats],
        }


@dataclass
class FeatureDescription:
    """Detailed description of a feature.

    Attributes:
        name: Feature name
        kind: 'node' or 'edge'
        value_type: 'str', 'int', or '' for edges without values
        description: Feature description from metadata
        node_types: List of node types this feature applies to
        unique_values: Number of unique values
        sample_values: Top values by frequency
        has_values: For edge features, whether edges have values
        error: Error message if feature not found
    """

    name: str
    kind: str
    value_type: str = ""
    description: str = ""
    node_types: list[str] = field(default_factory=list)
    unique_values: int = 0
    sample_values: list[dict[str, Any]] = field(default_factory=list)
    has_values: bool | None = None
    error: str | None = None

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "name": self.name,
            "kind": self.kind,
            "value_type": self.value_type,
            "description": self.description,
        }
        if self.error:
            result["error"] = self.error
            return result
        result["node_types"] = self.node_types
        result["unique_values"] = self.unique_values
        result["sample_values"] = self.sample_values
        if self.has_values is not None:
            result["has_values"] = self.has_values
        return result

    @classmethod
    def from_api(
        cls,
        api: Api,
        feature: str,
        sample_limit: int = 20,
    ) -> FeatureDescription:
        """Create FeatureDescription from API.

        Parameters
        ----------
        api : Api
            Context Fabric API instance
        feature : str
            Feature name
        sample_limit : int
            Maximum sample values to return
        """
        import numpy as np

        CF = api.CF
        fObj = CF.features.get(feature)
        if not fObj:
            return cls(name=feature, kind="unknown", error=f"Feature '{feature}' not found")

        meta = fObj.metaData or {}
        value_type = meta.get("valueType", meta.get("value_type", "str"))
        description = meta.get("description", "")

        def _convert(v: Any) -> str | int | float:
            if isinstance(v, np.integer):
                return int(v)
            elif isinstance(v, np.floating):
                return float(v)
            return v

        # Get node types this feature applies to
        node_types = get_feature_otypes(api, feature)

        # Try as node feature
        fobj = api.Fs(feature, warn=False)
        if fobj:
            freq_list = fobj.freqList()
            return cls(
                name=feature,
                kind="node",
                value_type=value_type,
                description=description,
                node_types=node_types,
                unique_values=len(freq_list),
                sample_values=[
                    {"value": _convert(v), "count": int(c)} for v, c in freq_list[:sample_limit]
                ],
            )

        # Try as edge feature
        eobj = api.Es(feature, warn=False)
        if eobj:
            has_values = eobj.doValues
            result = cls(
                name=feature,
                kind="edge",
                value_type=value_type,
                description=description,
                node_types=node_types,
                has_values=has_values,
            )
            if has_values:
                freq_list = eobj.freqList()
                result.unique_values = len(freq_list)
                result.sample_values = [
                    {"value": _convert(v), "count": int(c)} for v, c in freq_list[:sample_limit]
                ]
            return result

        return cls(name=feature, kind="unknown", error=f"Feature '{feature}' not accessible")


@dataclass
class FeatureCatalogEntry:
    """Lightweight feature entry for catalog listing."""

    name: str
    kind: str  # "node" or "edge"
    value_type: str
    description: str = ""

    def to_dict(self) -> dict[str, str]:
        return {
            "name": self.name,
            "kind": self.kind,
            "value_type": self.value_type,
            "description": self.description,
        }


@dataclass
class CorpusOverview:
    """Slim corpus overview (node types and sections only).

    Use this for lightweight discovery. For full details including
    text representations and feature lists, use CorpusDescription.
    """

    name: str
    node_types: list[dict[str, Any]] = field(default_factory=list)
    sections: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "node_types": self.node_types,
            "sections": self.sections,
        }


@dataclass
class CorpusDescription:
    """Complete corpus description.

    Attributes:
        name: Corpus name
        node_types: List of node types with counts
        sections: Section hierarchy information
        text_representations: Text format information with samples
        features: List of node feature metadata
        edge_features: List of edge feature metadata
    """

    name: str
    node_types: list[dict[str, Any]] = field(default_factory=list)
    sections: dict[str, Any] = field(default_factory=dict)
    text_representations: TextRepresentationInfo = field(
        default_factory=lambda: TextRepresentationInfo(description="")
    )
    features: list[dict[str, str]] = field(default_factory=list)
    edge_features: list[dict[str, str]] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "node_types": self.node_types,
            "sections": self.sections,
            "text_representations": self.text_representations.to_dict(),
            "features": self.features,
            "edge_features": self.edge_features,
        }


# =============================================================================
# Internal Helpers - Text Representations
# =============================================================================


def _parse_otext_format_pairs(api: Api) -> list[tuple[str, str, str, str, str]]:
    """Parse otext metadata to find orig/trans format pairs.

    Returns list of (base_name, orig_name, trans_name, orig_spec, trans_spec)
    """
    CF = api.CF
    otext_feature = CF.features.get("otext")
    if not otext_feature:
        return []

    otext_meta = otext_feature.metaData or {}

    # Collect all orig and trans formats
    orig_formats: dict[str, tuple[str, str]] = {}  # base -> (full_name, spec)
    trans_formats: dict[str, tuple[str, str]] = {}

    for key, spec in otext_meta.items():
        if not key.startswith("fmt:"):
            continue
        full_name = key[4:]  # Remove "fmt:"

        if "-orig-" in full_name:
            base_name = full_name.replace("-orig-", "-")
            orig_formats[base_name] = (full_name, spec)
        elif "-trans-" in full_name:
            base_name = full_name.replace("-trans-", "-")
            trans_formats[base_name] = (full_name, spec)

    # Match pairs
    pairs = []
    for base_name, (orig_name, orig_spec) in orig_formats.items():
        if base_name in trans_formats:
            trans_name, trans_spec = trans_formats[base_name]
            pairs.append((base_name, orig_name, trans_name, orig_spec, trans_spec))

    return pairs


def _get_exhaustive_text_samples(
    api: Api,
    orig_format_name: str,
    trans_format_name: str,
    orig_format_spec: str = "",
) -> tuple[list[TextFormatSample], set[str]]:
    """Get text samples with EXHAUSTIVE character coverage.

    Simple O(n) algorithm: single pass through nodes, greedily collecting
    samples that introduce new characters. Guarantees complete coverage
    since we visit all nodes.

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    orig_format_name : str
        Original script format name (e.g., "lex-orig-plain")
    trans_format_name : str
        Transliteration format name (e.g., "lex-trans-plain")
    orig_format_spec : str
        Unused, kept for API compatibility

    Returns
    -------
    tuple[list[TextFormatSample], set[str]]
        List of samples and set of all covered characters
    """
    T = api.T
    F = api.F
    max_slot = F.otype.maxSlot

    samples: list[TextFormatSample] = []
    covered_chars: set[str] = set()
    seen_values: set[str] = set()

    # Single pass through all nodes - O(n)
    # Greedily collect samples that introduce new characters
    for node in range(1, max_slot + 1):
        try:
            orig_text = T.text(node, fmt=orig_format_name).strip()
        except Exception:
            continue

        if not orig_text:
            continue

        # Check if this introduces new characters
        new_chars = set(orig_text) - covered_chars
        if not new_chars:
            continue

        # Skip duplicate values
        if orig_text in seen_values:
            continue

        try:
            trans_text = T.text(node, fmt=trans_format_name).strip()
        except Exception:
            continue

        if not trans_text:
            continue

        # Add sample
        samples.append(
            TextFormatSample(
                original=orig_text,
                transliterated=trans_text,
            )
        )
        covered_chars.update(orig_text)
        seen_values.add(orig_text)

    return samples, covered_chars


def _build_text_representations(api: Api) -> TextRepresentationInfo:
    """Build text representations from otext metadata.

    Parses otext format specifications to find orig/trans pairs and
    generates diverse samples with exhaustive character coverage.

    Parameters
    ----------
    api : Api
        Context Fabric API instance

    Returns
    -------
    TextRepresentationInfo
        Text representation information with samples
    """
    pairs = _parse_otext_format_pairs(api)

    if not pairs:
        return TextRepresentationInfo(
            description="No text format metadata available or no orig/trans pairs defined",
            formats=[],
        )

    formats = []
    for base_name, orig_name, trans_name, orig_spec, trans_spec in pairs:
        # Pass orig_spec for fast StringPool-based character discovery
        samples, covered_chars = _get_exhaustive_text_samples(
            api, orig_name, trans_name, orig_format_spec=orig_spec
        )

        if samples:
            formats.append(
                TextFormatInfo(
                    name=base_name,
                    original_spec=orig_spec,
                    transliteration_spec=trans_spec,
                    samples=samples,
                    unique_characters=len(covered_chars),
                    total_samples=len(samples),
                )
            )

    return TextRepresentationInfo(
        description=(
            "Shows how text values are encoded in this corpus. "
            "Samples provide exhaustive character coverage for understanding "
            "the relationship between original script and transliterated forms."
        ),
        formats=formats,
    )


# =============================================================================
# Internal Helpers - Feature-Otype Mapping
# =============================================================================


def get_feature_otypes(api: Api, feature: str, samples_per_type: int = 100) -> list[str]:
    """Determine which node types a feature applies to.

    Uses C.levels.data to efficiently sample each node type range
    and check for non-null values.

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    feature : str
        Feature name
    samples_per_type : int
        Number of samples to check per node type

    Returns
    -------
    list[str]
        List of node types that have this feature
    """
    f = api.Fs(feature, warn=False)
    if not f:
        return []

    otypes = []
    # C.levels.data gives (type, avg_slots, min_node, max_node) for each type
    for ntype, _, min_node, max_node in api.C.levels.data:
        min_node, max_node = int(min_node), int(max_node)

        # Sample nodes from this type range
        step = max(1, (max_node - min_node) // samples_per_type)
        for node in range(min_node, max_node + 1, step):
            if f.v(node) is not None:
                otypes.append(ntype)
                break

    return otypes


def get_all_feature_otypes(api: Api, samples_per_type: int = 100) -> dict[str, list[str]]:
    """Pre-compute otype mappings for all features.

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    samples_per_type : int
        Number of samples to check per node type

    Returns
    -------
    dict[str, list[str]]
        Feature name to list of applicable node types
    """
    result = {}
    for feature in api.Fall(warp=False):
        result[feature] = get_feature_otypes(api, feature, samples_per_type)
    return result


# =============================================================================
# Public API
# =============================================================================


def describe_corpus_overview(api: Api, name: str = "") -> CorpusOverview:
    """Get slim corpus overview (node types and sections only).

    Use this for lightweight discovery. For full details including
    text representations, use describe_corpus().

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    name : str
        Corpus name for identification

    Returns
    -------
    CorpusOverview
        Slim overview with node types and sections
    """
    F = api.F

    # Node types (from C.levels.data)
    node_types = []
    slot_type = F.otype.slotType
    for ntype, _, min_node, max_node in api.C.levels.data:
        node_types.append(
            {
                "type": ntype,
                "count": int(max_node) - int(min_node) + 1,
                "is_slot_type": ntype == slot_type,
            }
        )

    # Sections
    try:
        sections = {"levels": list(api.T.sectionTypes)}
    except Exception:
        sections = {"levels": []}

    return CorpusOverview(
        name=name,
        node_types=node_types,
        sections=sections,
    )


def list_features(
    api: Api,
    kind: str = "all",
    node_types: list[str] | None = None,
) -> list[FeatureCatalogEntry]:
    """List features with optional filtering.

    Returns lightweight catalog for discovery. Use node_types to filter
    by object type. For full details with samples, use describe_feature().

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    kind : str
        Filter by "all", "node", or "edge"
    node_types : list[str] | None
        Filter to features for these types (e.g., ["word"])

    Returns
    -------
    list[FeatureCatalogEntry]
        List of features with name, kind, value_type, description
    """
    CF = api.CF

    # Build feature→node_types mapping if filtering
    feature_node_types: dict[str, set[str]] = {}
    if node_types:
        for fname in api.Fall(warp=False):
            fobj = api.Fs(fname, warn=False)
            if fobj:
                types_with_feature: set[str] = set()
                for ntype, _, min_node, max_node in api.C.levels.data:
                    for node in range(int(min_node), min(int(min_node) + 10, int(max_node) + 1)):
                        if fobj.v(node) is not None:
                            types_with_feature.add(ntype)
                            break
                feature_node_types[fname] = types_with_feature

        for fname in api.Eall(warp=False):
            eobj = api.Es(fname, warn=False)
            if eobj:
                types_with_feature = set()
                for ntype, _, min_node, max_node in api.C.levels.data:
                    for node in range(int(min_node), min(int(min_node) + 10, int(max_node) + 1)):
                        if eobj.f(node):
                            types_with_feature.add(ntype)
                            break
                feature_node_types[fname] = types_with_feature

    features: list[FeatureCatalogEntry] = []

    # Node features
    if kind in ("all", "node"):
        for fname in api.Fall(warp=False):
            if node_types:
                feature_types = feature_node_types.get(fname, set())
                if not any(nt in feature_types for nt in node_types):
                    continue

            fObj = CF.features.get(fname)
            if fObj:
                meta = fObj.metaData or {}
                features.append(
                    FeatureCatalogEntry(
                        name=fname,
                        kind="node",
                        value_type=meta.get("valueType", meta.get("value_type", "str")),
                        description=meta.get("description", ""),
                    )
                )

    # Edge features
    if kind in ("all", "edge"):
        for fname in api.Eall(warp=False):
            if node_types:
                feature_types = feature_node_types.get(fname, set())
                if not any(nt in feature_types for nt in node_types):
                    continue

            fObj = CF.features.get(fname)
            if fObj:
                meta = fObj.metaData or {}
                features.append(
                    FeatureCatalogEntry(
                        name=fname,
                        kind="edge",
                        value_type=meta.get("valueType", meta.get("value_type", "str")),
                        description=meta.get("description", ""),
                    )
                )

    return features


def describe_corpus(api: Api, name: str = "") -> CorpusDescription:
    """Get complete corpus description.

    Returns node types, section structure, text representations with
    exhaustive character coverage, and feature catalogs.

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    name : str
        Corpus name for identification

    Returns
    -------
    CorpusDescription
        Complete description including node types, sections,
        text representations, and feature lists
    """
    CF = api.CF
    F = api.F

    # Node types (from C.levels.data)
    node_types = []
    slot_type = F.otype.slotType
    for ntype, _, min_node, max_node in api.C.levels.data:
        node_types.append(
            {
                "type": ntype,
                "count": int(max_node) - int(min_node) + 1,
                "is_slot_type": ntype == slot_type,
            }
        )

    # Sections
    try:
        sections = {"levels": list(api.T.sectionTypes)}
    except Exception:
        sections = {"levels": []}

    # Text representations with exhaustive character coverage
    text_representations = _build_text_representations(api)

    # Features - names and types only
    features = []
    for fname in api.Fall(warp=False):
        fObj = CF.features.get(fname)
        if fObj:
            meta = fObj.metaData or {}
            features.append(
                {
                    "name": fname,
                    "value_type": meta.get("valueType", meta.get("value_type", "str")),
                }
            )

    # Edge features - names and types only
    edge_features = []
    for fname in api.Eall(warp=False):
        fObj = CF.features.get(fname)
        if fObj:
            meta = fObj.metaData or {}
            edge_features.append(
                {
                    "name": fname,
                    "value_type": meta.get("valueType", meta.get("value_type", "str")),
                }
            )

    return CorpusDescription(
        name=name,
        node_types=node_types,
        sections=sections,
        text_representations=text_representations,
        features=features,
        edge_features=edge_features,
    )


def describe_feature(api: Api, feature: str, sample_limit: int = 20) -> FeatureDescription:
    """Get detailed description of a single feature.

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    feature : str
        Feature name
    sample_limit : int
        Maximum sample values to return

    Returns
    -------
    FeatureDescription
        Feature details including samples and node types
    """
    return FeatureDescription.from_api(api, feature, sample_limit)


def describe_features(
    api: Api,
    features: list[str],
    sample_limit: int = 20,
) -> dict[str, FeatureDescription]:
    """Get detailed descriptions for multiple features.

    Parameters
    ----------
    api : Api
        Context Fabric API instance
    features : list[str]
        Feature names
    sample_limit : int
        Maximum sample values per feature

    Returns
    -------
    dict[str, FeatureDescription]
        Feature descriptions keyed by name
    """
    return {f: describe_feature(api, f, sample_limit) for f in features}


def describe_text_formats(api: Api) -> TextRepresentationInfo:
    """Get text format descriptions with exhaustive character coverage.

    Parameters
    ----------
    api : Api
        Context Fabric API instance

    Returns
    -------
    TextRepresentationInfo
        Text format information with samples
    """
    return _build_text_representations(api)
