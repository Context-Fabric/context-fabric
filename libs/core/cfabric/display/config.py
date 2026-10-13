"""Display configuration of a corpus.

How a corpus is displayed (which features label which node types, how deep
displays unfold, writing direction, section separators, ...) is configured per
corpus. Text-Fabric keeps this in the `config.yaml` of the corpus' *app*.
We read those files as they are, so every existing TF corpus displays
correctly, and compute the same defaults as Text-Fabric for whatever the file
leaves open.

The defaults follow `tf.advanced.settings` (Text-Fabric, MIT licensed,
by Dirk Roorda).
"""

from __future__ import annotations

import logging
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING, Any

import yaml

if TYPE_CHECKING:
    from cfabric.core.api import Api

logger = logging.getLogger(__name__)

APP_DIR = "app"
APP_CONFIG = "config.yaml"
APP_SEARCH_DEPTH = 4

VAR_PATTERN = re.compile(r"\{([^}]+)\}")

WRITING = {
    "akk": ("akkadian", "ltr"),
    "hbo": ("hebrew", "rtl"),
    "syc": ("syriac", "rtl"),
    "uga": ("ugaritic", "ltr"),
    "ara": ("arabic", "rtl"),
    "grc": ("greek", "ltr"),
    "cld": ("aramaic", "ltr"),
    "": ("", "ltr"),
}

ORIG = "orig"
DEFAULT_FORMAT = "text-orig-full"

LEVEL_FLOW = {4: "hor", 3: "hor", 2: "hor", 1: "hor", 0: "ver"}
FLOW_DEFAULTS = {
    "ver": {"wrap": False, "stretch": False},
    "hor": {"wrap": True, "stretch": True},
}


def parse_features(spec: Any) -> tuple[tuple[str, ...], dict[str, str]]:
    """Parse a feature list like `"pdp vs lex:gloss"`.

    Returns the bare feature names and a mapping from feature name to the
    node type it must be looked up on (for `type:feature` entries).
    """
    if not spec:
        return ((), {})
    items = spec.split() if isinstance(spec, str) else list(spec)
    names: list[str] = []
    indirect: dict[str, str] = {}
    for item in items:
        if not item:
            continue
        parts = str(item).split(":", 1)
        name = parts[-1]
        names.append(name)
        if len(parts) > 1:
            indirect[name] = parts[0]
    return (tuple(names), indirect)


@dataclass
class TypeDisplay:
    """How nodes of one type are displayed."""

    type: str
    level: int
    flow: str
    wrap: bool
    stretch: bool
    template: str | bool
    """Plain display: `True` means the node's text (or section heading),
    a string is a template with `{feature}` placeholders, `""` means nothing."""
    label: str | bool
    """Pretty display: same conventions as `template`."""
    features: tuple[str, ...] = ()
    features_indirect: dict[str, str] = field(default_factory=dict)
    features_bare: tuple[str, ...] = ()
    features_bare_indirect: dict[str, str] = field(default_factory=dict)
    style: str | None = None
    hidden: bool = False
    children: tuple[str, ...] = ()
    is_big: bool = False
    is_lex: bool = False
    line_number: str | None = None
    graphics: bool = False

    def to_dict(self) -> dict[str, Any]:
        return {
            "type": self.type,
            "level": self.level,
            "flow": self.flow,
            "wrap": self.wrap,
            "stretch": self.stretch,
            "template": self.template,
            "label": self.label,
            "features": list(self.features),
            "featuresIndirect": dict(self.features_indirect),
            "featuresBare": list(self.features_bare),
            "featuresBareIndirect": dict(self.features_bare_indirect),
            "style": self.style,
            "hidden": self.hidden,
            "children": list(self.children),
            "isBig": self.is_big,
            "isLex": self.is_lex,
            "lineNumber": self.line_number,
            "graphics": self.graphics,
        }


@dataclass
class DisplayConfig:
    """Everything needed to display nodes of a corpus."""

    slot_type: str
    node_types: tuple[str, ...]
    types: dict[str, TypeDisplay]
    section_types: tuple[str, ...]
    structure_types: tuple[str, ...]
    text_formats: tuple[str, ...]
    default_format: str
    format_styles: dict[str, str | None]
    section_sep1: str = " "
    section_sep2: str = ":"
    none_values: frozenset[Any] = frozenset({None})
    excluded_features: frozenset[str] = frozenset()
    language: str = ""
    language_code: str = ""
    direction: str = "ltr"
    base_types: frozenset[str] = frozenset()
    condense_type: str | None = None
    verse_types: frozenset[str] = frozenset()
    lex_types: frozenset[str] = frozenset()
    lex_map: dict[str, str] = field(default_factory=dict)
    exclusions: dict[str, dict[str, Any]] = field(default_factory=dict)
    descendant_types: dict[str, frozenset[str]] = field(default_factory=dict)
    show_verse_in_tuple: bool = False
    browse_nav_level: int = 1
    example_section: str | None = None
    provenance: dict[str, Any] = field(default_factory=dict)
    source: Path | None = None
    raw: dict[str, Any] = field(default_factory=dict, repr=False)

    @property
    def hidden_types(self) -> frozenset[str]:
        """Types that are hidden by default."""
        return frozenset(t for (t, d) in self.types.items() if d.hidden)

    @property
    def big_types(self) -> frozenset[str]:
        return frozenset(t for (t, d) in self.types.items() if d.is_big)

    def ltr(self, fmt: str | None) -> str:
        """Writing direction for a text format: `rtl` or `ltr`.

        Only formats in the original script (`...-orig-...`) of a right-to-left
        corpus are right-to-left; transliterations are left-to-right.
        """
        fmt = fmt or self.default_format
        if self.direction == "rtl" and (f"{ORIG}-" in fmt or f"-{ORIG}" in fmt):
            return "rtl"
        return "ltr"

    def to_dict(self) -> dict[str, Any]:
        """JSON-ready representation, for clients that render displays."""
        return {
            "slotType": self.slot_type,
            "nodeTypes": list(self.node_types),
            "types": {t: d.to_dict() for (t, d) in self.types.items()},
            "sectionTypes": list(self.section_types),
            "structureTypes": list(self.structure_types),
            "textFormats": list(self.text_formats),
            "defaultFormat": self.default_format,
            "formatStyles": dict(self.format_styles),
            "sectionSep1": self.section_sep1,
            "sectionSep2": self.section_sep2,
            "noneValues": sorted(str(v) for v in self.none_values if v is not None),
            "excludedFeatures": sorted(self.excluded_features),
            "language": self.language,
            "languageCode": self.language_code,
            "direction": self.direction,
            "baseTypes": sorted(self.base_types),
            "condenseType": self.condense_type,
            "verseTypes": sorted(self.verse_types),
            "lexTypes": sorted(self.lex_types),
            "hiddenTypes": sorted(self.hidden_types),
            "bigTypes": sorted(self.big_types),
            "browseNavLevel": self.browse_nav_level,
            "exampleSection": self.example_section,
            "provenance": _jsonable(self.provenance),
        }

    @classmethod
    def from_api(
        cls,
        api: Api,
        config: str | Path | dict[str, Any] | None = None,
        corpus_path: str | Path | None = None,
    ) -> DisplayConfig:
        """Build the display configuration of a loaded corpus.

        Parameters
        ----------
        api: Api
            The loaded corpus.
        config: str | Path | dict | None
            A TF app `config.yaml` (path) or its parsed contents.
            If `None`, it is looked up near `corpus_path` with `find_app_config`.
        corpus_path: str | Path | None
            Directory of the corpus data, used to find the app config.
        """
        source: Path | None = None
        if config is None and corpus_path is not None:
            source = find_app_config(corpus_path)
            config = source
        if isinstance(config, (str, Path)):
            source = Path(config)
            config = read_app_config(source)
        return _build(api, config or {}, source)


def find_app_config(corpus_path: str | Path) -> Path | None:
    """Find the TF app `config.yaml` that belongs to a corpus.

    Text-Fabric corpora keep their data in `<repo>/tf/<version>` and their app
    in `<repo>/app`. We look for `app/config.yaml` in the corpus directory and
    up to a few levels above it.
    """
    path = Path(corpus_path).expanduser().resolve()
    for folder in [path, *path.parents][: APP_SEARCH_DEPTH + 1]:
        candidate = folder / APP_DIR / APP_CONFIG
        if candidate.is_file():
            return candidate
    return None


def read_app_config(path: str | Path) -> dict[str, Any]:
    """Read a TF app `config.yaml`."""
    with open(path, encoding="utf-8") as fh:
        data = yaml.safe_load(fh) or {}
    if isinstance(data, dict):
        return data
    raise ValueError(f"App config {path} is not a mapping")


def _jsonable(value: Any) -> Any:
    if isinstance(value, dict):
        return {str(k): _jsonable(v) for (k, v) in value.items()}
    if isinstance(value, (list, tuple, set, frozenset)):
        return [_jsonable(v) for v in value]
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    return str(value)


def _transitive_closure(
    relation: dict[str, set[str]], reflexive_exceptions: set[str]
) -> dict[str, frozenset[str]]:
    descendants = {parent: set(children) for (parent, children) in relation.items()}
    changed = True
    while changed:
        changed = False
        for parent, children in relation.items():
            for child in children:
                for grandChild in descendants.get(child, ()):
                    if grandChild not in descendants[parent]:
                        descendants[parent].add(grandChild)
                        changed = True
    for parent in relation:
        if parent not in reflexive_exceptions:
            descendants[parent].add(parent)
    return {k: frozenset(v) for (k, v) in descendants.items()}


def _build(api: Api, cfg: dict[str, Any], source: Path | None) -> DisplayConfig:
    F = api.F
    T = api.T
    otypeRank = api.N.otypeRank

    slotType = F.otype.slotType
    nTypes = tuple(F.otype.all)
    sectionTypes = tuple(T.sectionTypes or ())
    structureTypes = tuple(getattr(T, "structureTypes", ()) or ())
    sectionalTypes = set(sectionTypes) | set(structureTypes)

    dataDisplay = cfg.get("dataDisplay", {}) or {}
    typeInfo = cfg.get("typeDisplay", {}) or {}

    unknown = sorted(t for t in typeInfo if t not in nTypes)
    if unknown:
        logger.warning("App config mentions unknown node types: %s", ", ".join(unknown))
        typeInfo = {t: v for (t, v) in typeInfo.items() if t in nTypes}

    # writing system

    writing = cfg.get("writing", "") or ""
    (language, direction) = WRITING.get(writing, ("", "ltr"))

    # text formats

    formats = tuple(sorted(T.formats)) if T.formats else ()
    givenFormats = dataDisplay.get("textFormats", {}) or {}
    formatStyles: dict[str, str | None] = {
        fmt: (givenFormats.get(fmt) or {}).get("style") for fmt in formats
    }
    defaultFormat = dataDisplay.get("textFormat") or (
        DEFAULT_FORMAT if DEFAULT_FORMAT in formats or not formats else formats[0]
    )

    # per type settings

    verseTypes = {sectionTypes[-1]} if sectionTypes else set()
    verseRank = otypeRank[sectionTypes[-1]] if sectionTypes else None
    lexMap: dict[str, str] = {}
    baseTypes: set[str] = set()
    condenseType: str | None = None
    children: dict[str, set[str]] = {}
    exclusions: dict[str, dict[str, Any]] = {}
    givenLevels: dict[str, dict[str, Any]] = {}

    for nType, info in typeInfo.items():
        info = info or {}
        if info.get("verselike", False):
            verseTypes.add(nType)
        lexOcc = info.get("lexOcc")
        if lexOcc is not None:
            lexMap[lexOcc] = nType
        if "base" in info:
            baseTypes.add(nType)
        if info.get("condense", False):
            condenseType = nType
        if "children" in info:
            kids = info["children"] or ()
            children[nType] = {kids} if isinstance(kids, str) else set(kids)
        if any(k in info for k in ("level", "flow", "wrap", "stretch")):
            givenLevels[nType] = {
                k: info[k] for k in ("level", "flow", "wrap", "stretch") if k in info
            }
        if "exclude" in info:
            exclusions[nType] = info["exclude"] or {}

    lexTypes = set(lexMap.values())
    nTypesNoLex = [t for t in nTypes if t not in lexTypes]

    # levels, as in TF: 4 = sections, 3 = verses, 0 = slots and lexemes,
    # the remaining types are divided over 2 (bigger) and 1 (smaller)

    levelTypes: list[set[str]] = [set(), set(), set(), set(), set()]
    levelTypes[4] = sectionalTypes - verseTypes
    levelTypes[3] = set(verseTypes)
    levelTypes[0] = {slotType} | lexTypes
    remainingSet = set(nTypes) - levelTypes[4] - levelTypes[3] - levelTypes[0]
    remaining = tuple(t for t in nTypes if t in remainingSet)
    if len(remaining) == 1:
        levelTypes[1] = {remaining[0]}
    elif len(remaining) > 1:
        mid = len(remaining) // 2
        levelTypes[2] = set(remaining[0:mid])
        levelTypes[1] = set(remaining[mid:])

    # which types may occur directly inside which types

    childType: dict[str, set[str]] = {
        t: {nTypesNoLex[i + 1]}
        for (i, t) in enumerate(nTypesNoLex)
        if i < len(nTypesNoLex) - 1
    }
    for seq in (structureTypes, sectionTypes):
        for i, t in enumerate(seq[:-1]):
            childType.setdefault(t, set()).add(seq[i + 1])
    for t, kids in children.items():
        childType[t] = kids

    lowestSectional = set(verseTypes)
    if sectionTypes:
        lowestSectional.add(sectionTypes[-1])
    if structureTypes:
        lowestSectional.add(structureTypes[-1])

    biggestOther = slotType
    for t in remaining:
        if verseRank is None or otypeRank[t] < verseRank:
            biggestOther = t
            break
    smallestOther = remaining[-1] if remaining else None

    for t in lexTypes:
        childType.pop(t, None)
    for t in lowestSectional:
        childType.setdefault(t, set()).add(biggestOther)
    if smallestOther is not None and smallestOther != slotType:
        childType.setdefault(smallestOther, set()).add(slotType)

    if condenseType is None:
        if sectionTypes:
            condenseType = sectionTypes[-1]
        elif remaining:
            condenseType = (
                remaining[len(remaining) // 2] if len(remaining) > 1 else remaining[0]
            )
        else:
            condenseType = slotType

    descendantTypes = _transitive_closure(childType, {slotType})

    types: dict[str, TypeDisplay] = {}
    for i, levelSet in enumerate(levelTypes):
        for nType in levelSet:
            info = typeInfo.get(nType, {}) or {}
            given = givenLevels.get(nType, {})
            level = given.get("level", i)
            isVerse = nType in verseTypes
            flow = given.get("flow", "hor" if isVerse else LEVEL_FLOW.get(level, "hor"))
            flowDefaults = FLOW_DEFAULTS.get(flow, FLOW_DEFAULTS["hor"])
            defaultTpl: str | bool = (
                True
                if nType == slotType or nType in sectionalTypes - verseTypes
                else ""
            )
            (features, featuresIndirect) = parse_features(info.get("features"))
            (featuresBare, featuresBareIndirect) = parse_features(
                info.get("featuresBare")
            )
            style = info.get("style")
            types[nType] = TypeDisplay(
                type=nType,
                level=level,
                flow=flow,
                wrap=given.get("wrap", flowDefaults["wrap"]),
                stretch=given.get("stretch", flowDefaults["stretch"]),
                template=info.get("template", defaultTpl),
                label=info.get("label", defaultTpl),
                features=features,
                features_indirect=featuresIndirect,
                features_bare=featuresBare,
                features_bare_indirect=featuresBareIndirect,
                style=style if style else None,
                hidden=bool(info.get("hidden", False)),
                children=tuple(t for t in nTypes if t in childType.get(nType, ())),
                is_big=bool(info.get("isBig", False)),
                is_lex=nType in lexTypes,
                line_number=info.get("lineNumber"),
                graphics=bool(info.get("graphics", False)),
            )
    types = {t: types[t] for t in nTypes if t in types}

    noneValues = dataDisplay.get("noneValues")
    noneSet = frozenset({None} | set(noneValues or ()))

    browseNavLevel = dataDisplay.get("browseNavLevel")
    if browseNavLevel is None:
        browseNavLevel = len(sectionTypes) - 1 if sectionTypes else 1

    return DisplayConfig(
        slot_type=slotType,
        node_types=nTypes,
        types=types,
        section_types=sectionTypes,
        structure_types=structureTypes,
        text_formats=formats,
        default_format=defaultFormat,
        format_styles=formatStyles,
        section_sep1=dataDisplay.get("sectionSep1", " "),
        section_sep2=dataDisplay.get("sectionSep2", ":"),
        none_values=noneSet,
        excluded_features=frozenset(dataDisplay.get("excludedFeatures") or ()),
        language=language,
        language_code=writing,
        direction=direction,
        base_types=frozenset(baseTypes or {slotType}),
        condense_type=condenseType,
        verse_types=frozenset(verseTypes),
        lex_types=frozenset(lexTypes),
        lex_map=lexMap,
        exclusions=exclusions,
        descendant_types=descendantTypes,
        show_verse_in_tuple=bool(dataDisplay.get("showVerseInTuple", False)),
        browse_nav_level=browseNavLevel,
        example_section=dataDisplay.get("exampleSection"),
        provenance=dict(cfg.get("provenanceSpec", {}) or {}),
        source=source,
        raw=cfg,
    )
