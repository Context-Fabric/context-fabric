"""Public API surface parity with Text-Fabric 13 (``tf.fabric.Fabric``).

The test walks Text-Fabric's object graph from a loaded ``Fabric`` (``TF``,
``TF.api``, ``F``/``E``/``C`` and every feature object, ``L``, ``N``, ``T``,
``S``, ``TF.tmObj``, ...) and checks that the matching Context-Fabric object
has every public member, with a compatible call signature (parameter names,
order, defaults, ``**kwargs``) and, for attributes, the same container type.
Nothing is hand-listed except the reasoned exclusions below, so new
Text-Fabric members surface as failures.

A second, table-driven test compares the container type of the values that
core calls return.

Skipped when ``tf`` (Text-Fabric) is not installed.
"""

from __future__ import annotations

import inspect
import shutil
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import pytest
from cfabric import Fabric

tf_fabric = pytest.importorskip("tf.fabric")

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"

CF_CLASS_FOR_TF_CLASS = {
    "Fabric": "Fabric",
    "FabricCore": "Fabric",
    "Api": "Corpus",
    "Data": "FeatureFile",
    "Timestamp": "Timestamp",
    "NodeFeatures": "NodeFeatures",
    "EdgeFeatures": "EdgeFeatures",
    "NodeFeature": "NodeFeature",
    "OtypeFeature": "NodeFeature",
    "EdgeFeature": "EdgeFeature",
    "OslotsFeature": "EdgeFeature",
    "Computeds": "Computeds",
    "Computed": "Computed",
    "Locality": "Locality",
    "Nodes": "Nodes",
    "Text": "Text",
    "Search": "Search",
}
"""Text-Fabric class name -> Context-Fabric class name of the counterpart."""

INTERNAL = "Text-Fabric load/display internals, not part of the core API"
VOLUMES = (
    "volumes, collections, and MQL export are out of scope (README: Compatibility)"
)
NO_STRUCTURE_INDEX = (
    "raw structure indexes; use headingFromNode / nodeFromHeading / top / up / down"
)
NO_UNLOADING = "feature data is memory-mapped; there is no per-feature (un)loading"
NO_SECTION_INDEX = (
    "raw section-name indexes; use sectionFromNode / nodeFromSection / T.<sec0>Name"
)

EXCLUDED: dict[tuple[str, str], str] = {
    ("Fabric", "cformats"): INTERNAL,
    ("Fabric", "formatFeats"): INTERNAL,
    ("Fabric", "dep1Feats"): INTERNAL,
    ("Fabric", "precomputeList"): INTERNAL,
    ("Fabric", "textFeatures"): INTERNAL,
    ("Fabric", "sectionFeatsWithLanguage"): "available as T.sectionFeatsWithLanguage",
    ("Fabric", "sectionsOK"): INTERNAL,
    ("Fabric", "structureOK"): INTERNAL,
    ("Fabric", "warpDir"): INTERNAL,
    ("Fabric", "writeDir"): INTERNAL,
    ("Fabric", "curDir"): INTERNAL,
    ("Fabric", "homeDir"): INTERNAL,
    ("Fabric", "parentDir"): INTERNAL,
    ("Fabric", "collect"): VOLUMES,
    ("Fabric", "collection"): VOLUMES,
    ("Fabric", "collectionBase"): VOLUMES,
    ("Fabric", "extract"): VOLUMES,
    ("Fabric", "getVolumes"): VOLUMES,
    ("Fabric", "volume"): VOLUMES,
    ("Fabric", "volumeBase"): VOLUMES,
    ("Fabric", "exportMQL"): VOLUMES,
    ("Timestamp", "oneLevelRep"): INTERNAL,
    ("Timestamp", "levelRep"): INTERNAL,
    ("Timestamp", "level"): INTERNAL,
    ("Timestamp", "log"): INTERNAL,
    ("Timestamp", "timestamp"): INTERNAL,
    ("Timestamp", "verbose"): INTERNAL,
    ("Timestamp", "silent"): "read it with isSilent()",
    ("Timestamp", "wasSilent"): INTERNAL,
    ("Data", "data"): "feature data is memory-mapped and read through F / E / C",
    ("Data", "dataError"): INTERNAL,
    ("Data", "dependencies"): INTERNAL,
    ("Data", "isDict"): INTERNAL,
    ("Data", "dataFile"): INTERNAL,
    ("Data", "binDir"): INTERNAL,
    ("Data", "binPath"): INTERNAL,
    ("Data", "tmObj"): INTERNAL,
    ("Data", "version"): INTERNAL,
    ("Data", "load"): NO_UNLOADING,
    ("Data", "unload"): NO_UNLOADING,
    ("Data", "save"): "write features with TF.save",
    ("Data", "cleanDataBin"): "use TF.clearCache",
    ("Text", "hdFromNd"): NO_STRUCTURE_INDEX,
    ("Text", "ndFromHd"): NO_STRUCTURE_INDEX,
    ("Text", "hdMult"): NO_STRUCTURE_INDEX,
    ("Text", "hdTop"): NO_STRUCTURE_INDEX,
    ("Text", "hdUp"): NO_STRUCTURE_INDEX,
    ("Text", "hdDown"): NO_STRUCTURE_INDEX,
    ("Text", "nameFromNode"): NO_SECTION_INDEX,
    ("Text", "nodeFromName"): NO_SECTION_INDEX,
    ("Text", "sectionFeatureTypes"): NO_SECTION_INDEX,
}
"""``(Text-Fabric class, member) -> reason`` for members deliberately absent."""

SHAPE_DIVERGENCES: dict[tuple[str, str], str] = {
    ("Computed", "data"): (
        "C.levUp / C.levDown data is a lazy LevView indexed by node id "
        "(README: Compatibility), not a flat tuple"
    ),
    ("OtypeFeature", "data"): "dict {node: type}; TF packs non-slot types in a tuple",
    ("OslotsFeature", "data"): "dict {node: slots}; Text-Fabric packs slots in a tuple",
    ("Text", "sectionFeatures"): (
        "tuple of section feature names; Text-Fabric holds the features' data dicts"
    ),
    ("Data", "dataLoaded"): "a bool; Text-Fabric stores the load time",
}
"""``(Text-Fabric class, attribute) -> reason`` for attributes whose value has a
different container type than in Text-Fabric."""

BUILTIN_TYPES = (list, tuple, dict, set, frozenset, str, int, float, bool, type(None))


@dataclass(frozen=True)
class Mismatch:
    path: str
    problem: str

    def __str__(self) -> str:
        return f"{self.path}: {self.problem}"


def _copy_fixture(tmp_path: Path) -> Path:
    corpus = tmp_path / "mini"
    shutil.copytree(FIXTURE, corpus, ignore=shutil.ignore_patterns(".cf*", ".tf"))
    return corpus


@pytest.fixture(scope="module")
def fabrics(tmp_path_factory: pytest.TempPathFactory) -> tuple[Any, Any]:
    """A Text-Fabric and a Context-Fabric ``Fabric`` with all features loaded."""
    corpus = _copy_fixture(tmp_path_factory.mktemp("surface"))
    tf = tf_fabric.Fabric(locations=str(corpus), silent="deep")
    assert tf.loadAll(silent="deep")
    cf = Fabric(locations=str(corpus), silent="deep")
    cf.loadAll(silent="deep")
    return tf, cf


def _is_tf_object(value: Any) -> bool:
    return type(value).__module__.startswith("tf.")


def _public_members(obj: Any) -> list[str]:
    return sorted(name for name in dir(obj) if not name.startswith("_"))


def _signature_problem(tf_member: Any, cf_member: Any) -> str | None:
    """Why `cf_member` cannot be called the way `tf_member` can, if it cannot."""
    try:
        tf_sig = inspect.signature(tf_member)
    except (TypeError, ValueError):
        return None
    try:
        cf_sig = inspect.signature(cf_member)
    except (TypeError, ValueError):
        return f"no introspectable signature (Text-Fabric: {tf_sig})"
    tf_params = [p for p in tf_sig.parameters.values() if not p.name.startswith("_")]
    cf_params = list(cf_sig.parameters.values())
    cf_kinds = {p.kind for p in cf_params}
    positional = [p for p in tf_params if p.kind is not p.VAR_KEYWORD]
    for index, tf_param in enumerate(positional):
        if index >= len(cf_params):
            return f"missing parameter {tf_param.name!r}: TF{tf_sig} CF{cf_sig}"
        cf_param = cf_params[index]
        if (cf_param.name, cf_param.kind) != (tf_param.name, tf_param.kind):
            return f"parameter {index} differs: TF{tf_sig} CF{cf_sig}"
        if not _same_default(tf_param.default, cf_param.default):
            return f"default of {tf_param.name!r} differs: TF{tf_sig} CF{cf_sig}"
    for cf_param in cf_params[len(positional) :]:
        if cf_param.kind not in (cf_param.VAR_KEYWORD, cf_param.VAR_POSITIONAL) and (
            cf_param.default is cf_param.empty
        ):
            return f"extra required parameter {cf_param.name!r}: TF{tf_sig} CF{cf_sig}"
    if any(p.kind is p.VAR_KEYWORD for p in tf_params) and (
        inspect.Parameter.VAR_KEYWORD not in cf_kinds
    ):
        return f"does not accept **kwargs: TF{tf_sig} CF{cf_sig}"
    return None


def _same_default(tf_default: Any, cf_default: Any) -> bool:
    if tf_default is inspect.Parameter.empty or cf_default is inspect.Parameter.empty:
        return tf_default is cf_default
    # A mutable `{}` / `[]` default in Text-Fabric is equivalent to `None` here.
    if cf_default is None and tf_default in ({}, [], ()):
        return True
    return type(tf_default) is type(cf_default) and tf_default == cf_default


def _walk(tf_obj: Any, cf_obj: Any, path: str, seen: set[int]) -> Iterator[Mismatch]:
    seen.add(id(tf_obj))
    tf_class = type(tf_obj).__name__
    expected = CF_CLASS_FOR_TF_CLASS.get(tf_class)
    if expected is None:
        yield Mismatch(path, f"unmapped Text-Fabric class {tf_class}")
        return
    if type(cf_obj).__name__ != expected:
        yield Mismatch(path, f"is a {type(cf_obj).__name__}, expected {expected}")
        return
    for name in _public_members(tf_obj):
        if (expected, name) in EXCLUDED or (tf_class, name) in EXCLUDED:
            continue
        member_path = f"{path}.{name}"
        tf_member = getattr(tf_obj, name)
        try:
            cf_member = getattr(cf_obj, name)
        except AttributeError:
            yield Mismatch(member_path, "missing")
            continue
        if _is_tf_object(tf_member):
            if id(tf_member) in seen:
                expected_member = CF_CLASS_FOR_TF_CLASS.get(type(tf_member).__name__)
                if type(cf_member).__name__ != expected_member:
                    yield Mismatch(
                        member_path,
                        f"is a {type(cf_member).__name__}, expected {expected_member}",
                    )
                continue
            yield from _walk(tf_member, cf_member, member_path, seen)
        elif callable(tf_member):
            if not callable(cf_member):
                yield Mismatch(member_path, "is not callable")
                continue
            problem = _signature_problem(tf_member, cf_member)
            if problem:
                yield Mismatch(member_path, problem)
        elif isinstance(tf_member, BUILTIN_TYPES) and tf_member is not None:
            if (tf_class, name) in SHAPE_DIVERGENCES:
                continue
            if type(cf_member) is not type(tf_member):
                yield Mismatch(
                    member_path,
                    f"is a {type(cf_member).__name__}, Text-Fabric has "
                    f"{type(tf_member).__name__}",
                )


def test_public_surface_matches_text_fabric(fabrics):
    tf, cf = fabrics
    mismatches = list(_walk(tf, cf, "TF", set()))
    # TF.features maps names to `Data` objects; compare one entry of each kind.
    for name in ("word", "distance", "otext"):
        mismatches.extend(
            _walk(tf.features[name], cf.features[name], f"TF.features[{name!r}]", set())
        )
    assert not mismatches, "\n".join(map(str, mismatches))


def test_section_lookups_follow_first_section_type(fabrics):
    """TF attaches ``T.<sec0>Name`` / ``T.<sec0>Node`` named after the first
    section type (``sentence`` in the fixture, ``book`` in BHSA)."""
    _, cf = fabrics
    api = cf.api
    assert api.T.sectionTypes[0] == "sentence"
    assert api.T.sentenceName(1) == "S1"
    assert api.T.sentenceNode("S1") == 8
    assert not hasattr(api.T, "bookName")


RETURN_TYPE_CALLS = (
    "api.Fall()",
    "api.Eall()",
    "api.Call()",
    "api.Cs('levels')",
    "api.isLoaded('word', pretty=False)",
    "api.isLoaded('word')",
    "api.ensureLoaded('word')",
    "api.makeAvailableIn({})",
    "api.footprint()",
    "api.F.word.v(1)",
    "api.F.word.s('hello')",
    "api.F.word.freqList()",
    "api.F.otype.v(1)",
    "api.F.otype.s('phrase')",
    "api.F.otype.sInterval('phrase')",
    "api.E.oslots.s(8)",
    "api.E.distance.f(1)",
    "api.E.distance.t(2)",
    "api.E.distance.b(2)",
    "api.E.distance.freqList()",
    "api.E.parent.f(1)",
    "api.L.u(1)",
    "api.L.d(8)",
    "api.L.n(1)",
    "api.L.p(2)",
    "api.L.i(6)",
    "api.N.sortNodes({3, 1})",
    "api.N.sortKeyTuple((1, 2))",
    "api.N.makeSortKeyChunk()",
    "api.T.text(1)",
    "api.T.text([1, 2])",
    "api.T.sectionFromNode(1)",
    "api.T.sectionTuple(1)",
    "api.T.nodeFromSection(('S1', 1))",
    "api.T.splitFormat('phrase#{word}')",
    "api.T.splitDefaultFormat('phrase-default')",
    "api.T.structureInfo()",
    "api.T.sentenceName(1)",
    "api.S.search('word', shallow=True)",
    "api.S.search('word', shallow=2)",
    "api.S.relationsLegend()",
    "api.S.glean((1, 2))",
    "TF.explore(show=False)",
    "TF.explore()",
    "TF.featuresIgnored",
    "TF.featuresRequested",
    "TF.ignored",
)
"""Calls whose result container type must match Text-Fabric."""

KNOWN_RETURN_DIVERGENCES = {
    "api.F.word.s('hello')": "tuple instead of a lazy iterable (same members)",
    "api.F.otype.s('phrase')": "tuple instead of range (same members)",
    "api.E.parent.f(1)": "tuple; Text-Fabric returns a frozenset or tuple per feature",
    "api.E.oslots.s(8)": "tuple instead of array.array (same members)",
    "api.L.i(6)": "tuple, as Text-Fabric documents for all L functions (its L.i returns a list)",
    "api.E.distance.freqList()": "Text-Fabric raises TypeError sorting None edge values",
    "api.S.glean((1, 2))": "Text-Fabric raises IndexError on datasets with fewer than 3 section levels",
}
"""Calls whose Context-Fabric result container intentionally differs, or that
fail in Text-Fabric on the fixture."""


@pytest.mark.parametrize("call", RETURN_TYPE_CALLS)
def test_return_container_types_match_text_fabric(fabrics, call, capsys):
    tf, cf = fabrics
    if call in KNOWN_RETURN_DIVERGENCES:
        pytest.skip(KNOWN_RETURN_DIVERGENCES[call])
    tf_result = eval(call, {"TF": tf, "api": tf.api})
    cf_result = eval(call, {"TF": cf, "api": cf.api})
    capsys.readouterr()
    assert type(cf_result).__name__ == type(tf_result).__name__ or (
        type(tf_result) is type(cf_result)
    ), (
        f"{call}: Context-Fabric {type(cf_result).__name__}, TF {type(tf_result).__name__}"
    )


def test_fabric_constructor_matches_text_fabric():
    """Matches ``FabricCore``; ``tf.fabric.Fabric`` only adds the out-of-scope
    ``volume`` / ``collection`` parameters."""
    from tf.core.fabric import FabricCore

    assert _signature_problem(FabricCore, Fabric) is None


def test_text_fabric_import_path():
    """``from tf.fabric import Fabric`` becomes ``from cfabric.fabric import Fabric``."""
    from cfabric.fabric import Fabric as FabricFromModule

    assert FabricFromModule is Fabric


def test_is_loaded_report_matches_text_fabric(fabrics):
    tf, cf = fabrics
    names = "word distance oslots otext __levels__ nope"
    assert cf.api.isLoaded(names, pretty=False) == tf.api.isLoaded(names, pretty=False)
