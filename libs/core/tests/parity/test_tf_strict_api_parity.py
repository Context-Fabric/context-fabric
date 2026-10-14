"""Type-strict parity of the core API with Text-Fabric on real corpora.

``test_tf_oracle_parity.py`` and the RC baseline oracle compare values after
normalizing containers, so a tuple where Text-Fabric returns a list (or an int
where it returns a float) passes there. This module compares return values
*and* shapes: container types all the way down, int vs float, key types,
ordering, ``None`` vs empty, and which calls raise.

Every check runs the same call on a Text-Fabric 13.0.19 API and a Context-Fabric
API loaded from the same ``.tf`` files, on sample nodes of every node type
(first, second, last but one, last, and evenly spaced ones in between).

Accepted differences (not failures):

* Text-Fabric stores slot and level rows as ``array('I')`` (``E.oslots.s``,
  ``C.levDown.data``); Context-Fabric returns tuples with the same items.
* Methods Context-Fabric offers beyond Text-Fabric (Text-Fabric raises
  ``AttributeError``): ``F.otype.freqList`` and ``E.oslots.f/t/b``.

Node 0 is not probed: Text-Fabric has no node 0 and answers with incidental
``IndexError``s or wrapped-around values.

Gated by ``CONTEXT_FABRIC_RUN_PARITY=1``. ``CFABRIC_PARITY_CORPORA`` points to a
directory holding ``<corpus>/*.tf`` (default: the repository's
``libs/benchmarks/.corpora/<corpus>/tf``); ``CFABRIC_PARITY_PER_TYPE`` sets the
number of sample nodes per node type (default 25).
"""

from __future__ import annotations

import array
import contextlib
import io
import os
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pytest

from cfabric import Fabric

pytestmark = pytest.mark.skipif(
    os.environ.get("CONTEXT_FABRIC_RUN_PARITY") != "1",
    reason="set CONTEXT_FABRIC_RUN_PARITY=1 to run Text-Fabric oracle parity tests",
)

ROOT = Path(__file__).resolve().parents[4]
PER_TYPE = int(os.environ.get("CFABRIC_PARITY_PER_TYPE", "25"))

EXTENSIONS = {"F.otype.freqList", "E.oslots.f", "E.oslots.t", "E.oslots.b"}
"""Checks where Context-Fabric answers and Text-Fabric raises AttributeError."""

FAILURES_SHOWN = 3


def _corpus_path(name: str) -> Path:
    override = os.environ.get("CFABRIC_PARITY_CORPORA")
    path = Path(override) / name if override else ROOT / "libs/benchmarks/.corpora" / name / "tf"
    if not (path / "otype.tf").exists():
        pytest.skip(f"missing corpus at {path}")
    return path


def canonical(value: Any) -> Any:
    """A comparable, type-tagged form of an API return value."""
    if isinstance(value, array.array):
        value = tuple(value)
    if value is None or isinstance(value, (bool, int, float, str)):
        return (type(value).__name__, value)
    if isinstance(value, (tuple, list)):
        return (type(value).__name__, tuple(canonical(item) for item in value))
    if isinstance(value, dict):
        items = sorted(((canonical(k), canonical(v)) for k, v in value.items()), key=repr)
        return ("dict", tuple(items))
    if isinstance(value, (set, frozenset)):
        return (type(value).__name__, tuple(sorted((canonical(v) for v in value), key=repr)))
    return ("other:" + type(value).__name__, repr(value))


def outcome(call: Callable[[], Any]) -> Any:
    """The canonical return value of ``call``, or the exception type it raised."""
    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
        try:
            return canonical(call())
        except Exception as error:  # noqa: BLE001 - raising is part of the behavior
            return ("raise", type(error).__name__)


@dataclass
class Parity:
    """Runs each check on both APIs and collects the differences."""

    tf: Any
    cf: Any
    checks: int = 0
    failures: dict[str, list[tuple[Any, Any, Any]]] = field(default_factory=dict)

    def check(self, label: str, call: Callable[[Any], Any], arg: Any = None) -> None:
        self.checks += 1
        expected = outcome(lambda: call(self.tf))
        actual = outcome(lambda: call(self.cf))
        if expected == actual:
            return
        if label in EXTENSIONS and expected == ("raise", "AttributeError"):
            return
        self.failures.setdefault(label, []).append((arg, expected, actual))

    def report(self) -> str:
        lines = [f"{sum(map(len, self.failures.values()))} of {self.checks} checks differ"]
        for label, rows in self.failures.items():
            lines.append(f"{label}: {len(rows)} differ")
            for arg, expected, actual in rows[:FAILURES_SHOWN]:
                lines.append(f"  arg={arg!r}\n    TF={str(expected)[:240]}\n    CF={str(actual)[:240]}")
        return "\n".join(lines)


def sample_nodes(api: Any, per_type: int) -> list[int]:
    nodes: set[int] = set()
    for _node_type, _size, first, last in api.C.levels.data:
        count = last - first + 1
        if count <= per_type:
            nodes.update(range(first, last + 1))
            continue
        nodes.update(first + i * count // per_type for i in range(per_type))
        nodes.update({first, first + 1, last - 1, last})
    return sorted(nodes)


def check_metadata(p: Parity) -> None:
    for attr in ("slotType", "maxSlot", "maxNode", "all"):
        p.check(f"F.otype.{attr}", lambda a, attr=attr: getattr(a.F.otype, attr))
    p.check("C.levels.data", lambda a: a.C.levels.data)
    p.check("C.order.data", lambda a: a.C.order.data)
    p.check("C.rank.data", lambda a: a.C.rank.data)
    p.check("C.boundary.data", lambda a: a.C.boundary.data)
    p.check("C.sections.data", lambda a: a.C.sections.data)
    p.check("C.structure.data", lambda a: a.C.structure.data)
    p.check("N.otypeRank", lambda a: a.N.otypeRank)
    p.check("Fall", lambda a: a.Fall())
    p.check("Eall", lambda a: a.Eall())
    p.check("Call", lambda a: a.Call())
    for attr in (
        "sectionTypes",
        "sectionFeats",
        "structureTypes",
        "structureFeats",
        "headings",
        "formats",
        "languages",
        "defaultFormat",
    ):
        p.check(f"T.{attr}", lambda a, attr=attr: getattr(a.T, attr))
    p.check("T.top", lambda a: a.T.top())
    p.check("T.structure()", lambda a: a.T.structure())
    p.check("T.structurePretty()", lambda a: a.T.structurePretty())
    for row in p.tf.C.levels.data:
        p.check("F.otype.s", lambda a, t=row[0]: a.F.otype.s(t), row[0])
        p.check("F.otype.sInterval", lambda a, t=row[0]: a.F.otype.sInterval(t), row[0])


def check_nodes(p: Parity, nodes: Iterable[int]) -> None:
    node_types = [row[0] for row in p.tf.C.levels.data]
    formats = sorted(p.tf.T.formats)
    languages = sorted(p.tf.T.languages)
    max_slot = p.tf.F.otype.maxSlot
    for n in nodes:
        p.check("F.otype.v", lambda a: a.F.otype.v(n), n)
        p.check("E.oslots.s", lambda a: a.E.oslots.s(n), n)
        p.check("C.levUp.data", lambda a: a.C.levUp.data[n - 1], n)
        if n > max_slot:
            p.check("C.levDown.data", lambda a: a.C.levDown.data[n - max_slot - 1], n)
        p.check("N.sortKey", lambda a: a.N.sortKey(n), n)
        for name in "udnpi":
            p.check(f"L.{name}", lambda a, name=name: getattr(a.L, name)(n), n)
            for node_type in node_types:
                p.check(
                    f"L.{name}(otype)",
                    lambda a, name=name, t=node_type: getattr(a.L, name)(n, otype=t),
                    (n, node_type),
                )
        p.check("T.text", lambda a: a.T.text(n), n)
        for fmt in formats:
            for descend in (None, True, False):
                p.check(
                    f"T.text[{fmt}, descend={descend}]",
                    lambda a, fmt=fmt, descend=descend: a.T.text(n, fmt=fmt, descend=descend),
                    n,
                )
        for kwargs in ({}, {"lastSlot": True}, {"fillup": True}, {"level": 1}):
            p.check(f"T.sectionFromNode{kwargs}", lambda a, k=kwargs: a.T.sectionFromNode(n, **k), n)
        for kwargs in ({}, {"lastSlot": True}, {"fillup": True}):
            p.check(f"T.sectionTuple{kwargs}", lambda a, k=kwargs: a.T.sectionTuple(n, **k), n)
        for lang in languages:
            p.check("T.sectionFromNode(lang)", lambda a, lang=lang: a.T.sectionFromNode(n, lang=lang), (n, lang))
        for name in ("headingFromNode", "up", "down", "structure", "structurePretty"):
            p.check(f"T.{name}", lambda a, name=name: getattr(a.T, name)(n), n)


def check_section_lookups(p: Parity, nodes: Iterable[int]) -> None:
    languages = sorted(p.tf.T.languages)
    for n in nodes:
        section = p.tf.T.sectionFromNode(n)
        for depth in range(1, len(section) + 1):
            p.check("T.nodeFromSection", lambda a, s=section[:depth]: a.T.nodeFromSection(s), section[:depth])
            for lang in languages:
                with contextlib.suppress(Exception):
                    named = p.tf.T.sectionFromNode(n, lang=lang)[:depth]
                    p.check(
                        "T.nodeFromSection(lang)",
                        lambda a, s=named, lang=lang: a.T.nodeFromSection(s, lang=lang),
                        (named, lang),
                    )
        heading = p.tf.T.headingFromNode(n) if p.tf.T.headings else None
        if heading:
            p.check("T.nodeFromHeading", lambda a, h=heading: a.T.nodeFromHeading(h), heading)
    p.check("T.nodeFromSection(unknown)", lambda a: a.T.nodeFromSection(("No such section",)))


def check_features(p: Parity, nodes: list[int]) -> None:
    slot_type = p.tf.F.otype.slotType
    for name in sorted(set(p.tf.Fall()) | {"otype"}):
        for n in nodes:
            p.check(f"F.{name}.v", lambda a, name=name: a.Fs(name).v(n), n)
        p.check(f"F.{name}.freqList", lambda a, name=name: a.Fs(name).freqList())
        if name == "otype":
            continue
        p.check(
            f"F.{name}.freqList(nodeTypes)",
            lambda a, name=name: a.Fs(name).freqList(nodeTypes={slot_type}),
        )
        for value, _count in p.tf.Fs(name).freqList()[:3]:
            p.check(f"F.{name}.s", lambda a, name=name, v=value: a.Fs(name).s(v), value)
    for name in sorted(p.tf.Eall()):
        for n in nodes:
            for method in "ftb":
                p.check(f"E.{name}.{method}", lambda a, name=name, m=method: getattr(a.Es(name), m)(n), n)
        if name != "oslots":
            with contextlib.suppress(TypeError):  # TF cannot sort None among values
                p.tf.Es(name).freqList()
                p.check(f"E.{name}.freqList", lambda a, name=name: a.Es(name).freqList())


def check_ordering(p: Parity, nodes: list[int]) -> None:
    p.check("N.sortNodes", lambda a: a.N.sortNodes(list(reversed(nodes))))
    p.check("N.sortNodes(set)", lambda a: a.N.sortNodes(set(nodes)))
    p.check("N.walk(nodes)", lambda a: list(a.N.walk(nodes=nodes)))
    p.check("N.walk(nodes, events)", lambda a: list(a.N.walk(nodes=nodes[:200], events=True)))
    p.check("N.walk", lambda a: list(a.N.walk()))


@pytest.mark.parametrize("corpus", ["banks", "n1904", "bhsa"])
def test_core_api_matches_text_fabric_strictly(corpus: str) -> None:
    tf_fabric = pytest.importorskip("tf.fabric").Fabric
    path = _corpus_path(corpus)
    tf_api = tf_fabric(locations=str(path), silent="deep").loadAll(silent="deep")
    cf_api = Fabric(locations=str(path), silent="deep").loadAll(silent="deep")
    p = Parity(tf_api, cf_api)
    nodes = sample_nodes(tf_api, PER_TYPE)

    check_metadata(p)
    check_nodes(p, nodes)
    check_section_lookups(p, nodes)
    check_features(p, nodes)
    check_ordering(p, nodes)

    assert not p.failures, p.report()
