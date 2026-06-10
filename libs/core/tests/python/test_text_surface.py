"""Tests for the text/section surface in ``bindings/accessors.rs``.

Covers the TF-parity behaviour of:

* ``T.sectionTuple`` returning section *node ids* (not values),
  honoring ``lastSlot``/``fillup`` (tf/core/text.py:461).
* ``T.text`` accepting a single int or an iterable of ints,
  concatenating per-node renders with no separator (tf/core/text.py:972).
* ``S.glean`` rendering result tuples (tf/search/search.py:475-542).
* ``S.showPlan`` emitting the engine's real plan summary.
* ``N.walk(events=True)`` yielding ``(node, kind)`` pairs (tf/core/nodes.py:278).

The mini-corpus tests always run; the BHSA tests run only when the prebuilt
mapped cache ``/tmp/cf_bhsa.cfr`` and the BHSA ``tf`` directory are present.
"""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from cfabric import Fabric

BHSA_CACHE = Path("/tmp/cf_bhsa.cfr")
BHSA_TF = Path("/Users/cody/github/context-fabric/libs/benchmarks/.corpora/bhsa/tf")


# --------------------------------------------------------------------------- #
# Mini-corpus surface (always available via the shared conftest fixtures)
# --------------------------------------------------------------------------- #


def test_section_tuple_returns_node_ids(loaded_api):
    # node 1 is a slot contained by sentence 8 and phrase 6;
    # sectionTypes = (sentence, phrase) -> (sentence_node, phrase_node).
    assert loaded_api.T.sectionTuple(1) == (8, 6)
    # sectionFromNode resolves those nodes to their feature values.
    assert loaded_api.T.sectionFromNode(1) == ("S1", 1)
    # A section node itself appears as the corresponding element.
    assert loaded_api.T.sectionTuple(8) == (8,)
    assert loaded_api.T.sectionTuple(6) == (8, 6)


def test_section_tuple_fillup_and_last_slot(loaded_api):
    # fillup keeps the full section depth even for a top-level section node.
    assert loaded_api.T.sectionTuple(8, fillup=True) == (8, 6)
    # phrase 7 covers slots 4-5; first slot (4) and last slot (5) both sit in
    # sentence 8 and phrase 7, so lastSlot has no effect here but is honored.
    assert loaded_api.T.sectionTuple(7) == (8, 7)
    assert loaded_api.T.sectionTuple(7, lastSlot=True) == (8, 7)
    # level truncates the tuple.
    assert loaded_api.T.sectionTuple(1, level=1) == (8,)


def test_text_single_int(loaded_api):
    assert loaded_api.T.text(1) == "hello"
    assert loaded_api.T.text(6) == "hellobeautifulworld"
    assert loaded_api.T.text(8) == "hellobeautifulworldgoodmorning"


def test_text_iterable_of_ints(loaded_api):
    # Iterable input renders each node and concatenates with no separator.
    assert loaded_api.T.text([1, 2, 3]) == "hellobeautifulworld"
    assert loaded_api.T.text((1, 5)) == "hellomorning"
    # An iterable of containers descends each and concatenates.
    assert loaded_api.T.text([6, 7]) == "hellobeautifulworldgoodmorning"
    # Empty iterable -> empty string.
    assert loaded_api.T.text([]) == ""


def test_glean_renders_slots(loaded_api):
    # Single slot -> its text; multiple slots -> space-joined fields.
    assert loaded_api.S.glean((1,)) == "hello"
    assert loaded_api.S.glean((1, 2)) == "hello beautiful"
    assert loaded_api.S.glean(()) == ""
    # Upper-level section nodes (sentence/phrase here) render as empty fields.
    assert loaded_api.S.glean((8,)) == ""
    assert loaded_api.S.glean((6,)) == ""
    assert loaded_api.S.glean((1, 6)) == "hello "


def test_show_plan_is_a_real_plan(loaded_api):
    search = loaded_api.S
    search.study("word")
    plan = search.showPlan(False)
    assert plan is not None
    # The engine plan summary names the template and atom/relation counts.
    assert "word" in plan
    assert "atoms" in plan


def test_walk_events_pairs(loaded_api):
    events = loaded_api.N.walk(events=True)
    assert all(isinstance(item, tuple) and len(item) == 2 for item in events)
    kinds = {kind for (_node, kind) in events}
    # slot events carry None, container start/end carry False/True.
    assert None in kinds
    assert any(kind is True for kind in kinds)
    # Non-end events replay plain walk() order.
    plain = loaded_api.N.walk()
    assert plain == tuple(node for (node, kind) in events if kind is not True)
    # Restricting to a node subset keeps end events only for in-set nodes.
    subset_events = loaded_api.N.walk(nodes=[1, 2, 3, 6], events=True)
    end_nodes = {node for (node, kind) in subset_events if kind is True}
    assert end_nodes <= {1, 2, 3, 6}


# --------------------------------------------------------------------------- #
# BHSA surface (skipped unless the prebuilt mapped cache exists)
# --------------------------------------------------------------------------- #

bhsa_available = BHSA_CACHE.exists() and BHSA_TF.exists()
bhsa_reason = "BHSA mapped cache /tmp/cf_bhsa.cfr or tf dir not available"


@pytest.fixture(scope="module")
def bhsa_api():
    if not bhsa_available:
        pytest.skip(bhsa_reason)
    fabric = Fabric(locations=str(BHSA_TF), silent="deep")
    return fabric.openMapped(str(BHSA_CACHE))


@pytest.mark.skipif(not bhsa_available, reason=bhsa_reason)
def test_bhsa_section_tuple_and_from_node(bhsa_api):
    T = bhsa_api.T
    assert T.sectionTypes == ("book", "chapter", "verse")
    verse = T.nodeFromSection(("Genesis", 1, 1))
    assert verse is not None
    # sectionTuple returns node ids; the verse element equals the verse node.
    stuple = T.sectionTuple(verse)
    assert len(stuple) == 3
    assert stuple[2] == verse
    assert all(isinstance(n, int) for n in stuple)
    # sectionFromNode resolves them back to the human-readable heading.
    assert T.sectionFromNode(verse) == ("Genesis", 1, 1)


@pytest.mark.skipif(not bhsa_available, reason=bhsa_reason)
def test_bhsa_text_iterable(bhsa_api):
    T = bhsa_api.T
    verse = T.nodeFromSection(("Genesis", 1, 1))
    single = T.text(verse)
    assert single
    # The text of a list of the verse's word slots concatenates to the verse.
    slots = bhsa_api.L.d(verse, otype="word")
    assert T.text(list(slots)) == single


@pytest.mark.skipif(not bhsa_available, reason=bhsa_reason)
def test_bhsa_glean_verse_and_words(bhsa_api):
    T = bhsa_api.T
    verse = T.nodeFromSection(("Genesis", 1, 1))
    # A level-2 (verse) section node renders as "book ch:vs".
    assert T.sectionTypes[2] == "verse"
    assert bhsa_api.S.glean((verse,)) == "Genesis 1:1"
    # A word-bearing non-section node renders as "otype[text...]".
    clause = bhsa_api.L.u(verse, otype="clause")
    if clause:
        rendered = bhsa_api.S.glean((clause[0],))
        assert rendered.startswith("clause[")
        assert rendered.endswith("]")
