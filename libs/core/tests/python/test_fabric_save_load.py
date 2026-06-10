"""Round-trip tests for ``Fabric.save`` / ``Fabric.load`` against text-fabric.

These exercise Phase 3 (Agent 3D) of the cf-rust remediation plan:

* ``Fabric.load("a b c")`` whitespace-splits feature strings (TF ``fitemize``).
* ``Fabric.load(..., add=True)`` extends the visible feature set.
* ``Fabric.save(...)`` authors TF-compatible ``.tf`` files that text-fabric can
  re-read with matching values, and that CF can re-read after TF authored them.
"""

from __future__ import annotations

import shutil
from pathlib import Path

import pytest

from cfabric import Fabric

# text-fabric is installed in the shared venv; skip cleanly if it is not.
tf_fabric = pytest.importorskip("tf.fabric")
TFFabric = tf_fabric.Fabric

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"


@pytest.fixture
def corpus_dir(tmp_path: Path) -> Path:
    """A writable copy of the mini_corpus (otype/oslots/otext + features)."""
    dst = tmp_path / "corpus"
    shutil.copytree(FIXTURE, dst)
    return dst


# --------------------------------------------------------------------------- #
# Fabric.load whitespace splitting + add semantics
# --------------------------------------------------------------------------- #


def test_load_whitespace_split(corpus_dir: Path) -> None:
    """A single space-separated string is split into individual feature names."""
    corpus = Fabric(locations=str(corpus_dir), silent="deep").load("pos word")
    # Both requested features are exposed...
    assert corpus.Fs("pos") is not None
    assert corpus.Fs("word") is not None
    # ...but a feature that was NOT requested is hidden, proving the string was
    # split (otherwise the lone token "pos word" would expose nothing at all).
    assert corpus.Fs("number") is None


def test_load_add_extends_visible_features(corpus_dir: Path) -> None:
    """``add=True`` accumulates requested features across calls."""
    fabric = Fabric(locations=str(corpus_dir), silent="deep")
    first = fabric.load("pos")
    assert first.Fs("pos") is not None
    assert first.Fs("number") is None

    second = fabric.load("number", add=True)
    # The second corpus exposes both the previously and newly requested features.
    assert second.Fs("pos") is not None
    assert second.Fs("number") is not None


# --------------------------------------------------------------------------- #
# CF saves -> TF reads
# --------------------------------------------------------------------------- #


def test_cf_save_then_tf_load(corpus_dir: Path) -> None:
    color = {1: "red", 2: "green", 3: "blue"}
    # valued edge feature: {source: {target: value}}
    link = {1: {2: "near"}, 3: {4: "far", 5: "mid"}}

    fabric = Fabric(locations=str(corpus_dir), silent="deep")
    ok = fabric.save(
        nodeFeatures={"color": color},
        edgeFeatures={"link": link},
        metaData={
            "": {"author": "cf-test"},
            "color": {"valueType": "str", "description": "a colour"},
            "link": {"valueType": "str", "edgeValues": True},
        },
    )
    assert ok is True

    # text-fabric must be able to reload the CF-authored features.
    TF = TFFabric(locations=str(corpus_dir), silent="deep")
    api = TF.load("color link", silent="deep")
    assert api is not False

    F = api.F
    E = api.E

    # Node feature values round-trip.
    assert F.color.v(1) == "red"
    assert F.color.v(2) == "green"
    assert F.color.v(3) == "blue"

    # Generic + feature-specific metadata round-trip.
    assert F.color.meta["author"] == "cf-test"
    assert F.color.meta["description"] == "a colour"
    assert F.color.meta["valueType"] == "str"

    # Valued edges round-trip (f returns (target, value) tuples when valued).
    assert dict(E.link.f(1)) == {2: "near"}
    assert dict(E.link.f(3)) == {4: "far", 5: "mid"}
    assert E.link.meta["author"] == "cf-test"


def test_cf_save_unvalued_edge_then_tf_load(corpus_dir: Path) -> None:
    """Edge features given as target *sets* round-trip without values."""
    link = {1: {2, 3}, 4: {5}}

    fabric = Fabric(locations=str(corpus_dir), silent="deep")
    fabric.save(
        edgeFeatures={"plain": link},
        metaData={"plain": {"valueType": "str"}},
    )

    TF = TFFabric(locations=str(corpus_dir), silent="deep")
    api = TF.load("plain", silent="deep")
    assert api is not False

    # Unvalued edges: f returns a plain tuple of target nodes.
    assert set(api.E.plain.f(1)) == {2, 3}
    assert set(api.E.plain.f(4)) == {5}


# --------------------------------------------------------------------------- #
# TF saves -> CF reads
# --------------------------------------------------------------------------- #


def test_tf_save_then_cf_load(corpus_dir: Path) -> None:
    color = {1: "alpha", 2: "beta", 3: "gamma"}
    link = {1: {2: "near"}, 3: {4: "far", 5: "mid"}}

    TF = TFFabric(locations=str(corpus_dir), silent="deep")
    ok = TF.save(
        nodeFeatures={"shade": color},
        edgeFeatures={"bond": link},
        metaData={
            "": {"author": "tf-test"},
            "shade": {"valueType": "str"},
            "bond": {"valueType": "str", "edgeValues": True},
        },
    )
    assert ok is True

    # CF must reload the TF-authored features identically.
    corpus = Fabric(locations=str(corpus_dir), silent="deep").loadAll(silent="deep")

    shade = corpus.Fs("shade")
    assert shade is not None
    assert shade.v(1) == "alpha"
    assert shade.v(2) == "beta"
    assert shade.v(3) == "gamma"
    assert shade.meta["author"] == "tf-test"

    bond = corpus.Es("bond")
    assert bond is not None
    # CF's E.f returns (target, value) tuples for valued edges.
    assert dict(bond.f(1)) == {2: "near"}
    assert dict(bond.f(3)) == {4: "far", 5: "mid"}
