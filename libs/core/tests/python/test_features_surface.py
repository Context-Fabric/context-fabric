"""Surface tests for the F/E/C/N feature bindings owned by Phase 3 Agent 3B.

Covers:
  * node/edge ``freqList`` node-type filters (compared against text-fabric),
  * ``F.otype.all`` (level-ordered node-type names),
  * ``C.levUp`` / ``C.levDown`` lazy views,
  * ``C.sections`` and ``C.characters`` shapes,
  * ``N.sortKeyChunk`` / ``N.sortKeyChunkLength`` canonical chunk ordering.

The text-fabric comparisons are skipped automatically if ``tf`` is not
importable.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from cfabric import Fabric

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"
TF_FEATURES = (
    "otype oslots word pos number phrase_id sentence_id score distance parent relation"
)


@pytest.fixture
def api():
    return Fabric(locations=str(FIXTURE), silent="deep").loadAll(silent="deep")


@pytest.fixture
def tf_api():
    tf_fabric = pytest.importorskip("tf.fabric")
    TF = tf_fabric.Fabric(locations=[str(FIXTURE)], silent="deep")
    loaded = TF.load(TF_FEATURES, silent="deep")
    if loaded is False:
        pytest.skip("text-fabric could not load the fixture")
    return loaded


# --------------------------------------------------------------------------- #
# freqList node filters
# --------------------------------------------------------------------------- #
def test_node_freqlist_unfiltered(api):
    assert api.F.pos.freqList() == (
        ("adjective", 2),
        ("noun", 2),
        ("interjection", 1),
    )


def test_node_freqlist_filter_matches(api):
    # pos lives only on word nodes, so a {"word"} filter is a no-op.
    assert api.F.pos.freqList(nodeTypes={"word"}) == api.F.pos.freqList()
    # a single string is accepted as a one-element filter.
    assert api.F.pos.freqList("word") == api.F.pos.freqList()


def test_node_freqlist_filter_excludes(api):
    # pos never occurs on phrase nodes -> empty.
    assert api.F.pos.freqList(nodeTypes={"phrase"}) == ()


def test_node_freqlist_parity_with_tf(api, tf_api):
    assert api.F.pos.freqList() == tf_api.F.pos.freqList()
    assert api.F.pos.freqList(nodeTypes={"word"}) == tf_api.F.pos.freqList(
        nodeTypes={"word"}
    )
    assert api.F.pos.freqList(nodeTypes={"phrase"}) == tf_api.F.pos.freqList(
        nodeTypes={"phrase"}
    )
    assert api.F.number.freqList() == tf_api.F.number.freqList()


# --------------------------------------------------------------------------- #
# freqList edge filters
# --------------------------------------------------------------------------- #
def test_edge_freqlist_valued_unfiltered(api):
    assert api.E.relation.freqList() == (
        ("predicate", 2),
        ("subject", 2),
        ("object", 1),
    )


def test_edge_freqlist_valued_filters(api):
    # relation goes word -> phrase.
    assert api.E.relation.freqList(nodeTypesFrom={"word"}) == api.E.relation.freqList()
    assert api.E.relation.freqList(nodeTypesTo={"phrase"}) == api.E.relation.freqList()
    # no relation targets a word -> empty.
    assert api.E.relation.freqList(nodeTypesTo={"word"}) == ()


def test_edge_freqlist_unvalued_count(api):
    # parent has no edge values -> freqList returns an int count.
    assert api.E.parent.freqList() == 7
    # parent edges land on phrase (6,7) and sentence (8); none target a word.
    assert api.E.parent.freqList(nodeTypesTo={"word"}) == 0


def test_edge_freqlist_parity_with_tf(api, tf_api):
    assert api.E.relation.freqList() == tf_api.E.relation.freqList()
    assert api.E.relation.freqList(nodeTypesFrom={"word"}) == tf_api.E.relation.freqList(
        nodeTypesFrom={"word"}
    )
    assert api.E.relation.freqList(nodeTypesTo={"word"}) == tf_api.E.relation.freqList(
        nodeTypesTo={"word"}
    )
    assert api.E.parent.freqList() == tf_api.E.parent.freqList()
    assert api.E.parent.freqList(nodeTypesTo={"word"}) == tf_api.E.parent.freqList(
        nodeTypesTo={"word"}
    )


# --------------------------------------------------------------------------- #
# F.otype.all
# --------------------------------------------------------------------------- #
def test_otype_all(api):
    assert api.F.otype.all == ("sentence", "phrase", "word")


def test_otype_all_parity_with_tf(api, tf_api):
    assert tuple(api.F.otype.all) == tuple(tf_api.F.otype.all)


def test_all_only_on_otype(api):
    with pytest.raises(AttributeError):
        _ = api.F.pos.all


# --------------------------------------------------------------------------- #
# C.levUp / C.levDown lazy views
# --------------------------------------------------------------------------- #
def test_levup_lazy_view(api):
    levup = api.C.levUp.data
    # indexed by node id directly (divergence from TF's data[n-1]).
    assert levup[6] == (8,)  # phrase 6 embedded in sentence 8
    assert levup[1] == (6, 8)  # word 1 embedded in phrase 6 and sentence 8
    assert levup.row(1) == levup[1]
    assert 6 in levup


def test_levup_parity_with_tf(api, tf_api):
    cf = api.C.levUp.data
    tf = tf_api.C.levUp.data
    for node in range(1, api.F.otype.maxNode + 1):
        assert tuple(cf[node]) == tuple(tf[node - 1]), node


def test_levdown_lazy_view(api):
    levdown = api.C.levDown.data
    # levDown only contains non-slot embeddees (matching TF), so sentence 8
    # contains its phrases but not its words.
    assert set(levdown[8]) == {6, 7}
    # slots have no embeddees
    assert levdown[1] == ()


def test_levdown_parity_with_tf(api, tf_api):
    cf = api.C.levDown.data
    tf = tf_api.C.levDown.data
    max_slot = api.F.otype.maxSlot
    for node in range(max_slot + 1, api.F.otype.maxNode + 1):
        assert tuple(cf[node]) == tuple(tf[node - max_slot - 1]), node


# --------------------------------------------------------------------------- #
# C.sections
# --------------------------------------------------------------------------- #
def test_sections_shape(api):
    data = api.C.sections.data
    assert set(data.keys()) == {"sec1", "sec2", "seqFromNode", "nodeFromSeq"}
    assert isinstance(data["sec1"], dict)
    assert isinstance(data["seqFromNode"], dict)


def _stringify_headings(mapping):
    """Normalize TF sec1/sec2 heading keys to strings.

    DIVERGENCE: the Rust core's ``SectionsData`` stores section-heading keys as
    strings (``BTreeMap<u32, BTreeMap<String, u32>>``), whereas TF preserves the
    heading feature's native type (e.g. int chapter/verse numbers). We compare
    with stringified TF keys to validate structure rather than key type.
    """
    return {
        node: {str(h): target for h, target in inner.items()}
        for node, inner in mapping.items()
    }


def _stringify_sec2(mapping):
    return {
        node: {
            str(h1): {str(h2): target for h2, target in inner2.items()}
            for h1, inner2 in inner.items()
        }
        for node, inner in mapping.items()
    }


def test_sections_parity_with_tf(api, tf_api):
    cf = api.C.sections.data
    tf = tf_api.C.sections.data
    # node ids and seq tuples are ints in both -> exact match.
    assert cf["seqFromNode"] == tf["seqFromNode"]
    assert cf["nodeFromSeq"] == tf["nodeFromSeq"]
    # sec1/sec2 heading keys are stringified by the core (documented divergence).
    assert cf["sec1"] == _stringify_headings(tf["sec1"])
    assert cf["sec2"] == _stringify_sec2(tf["sec2"])


# --------------------------------------------------------------------------- #
# C.characters
# --------------------------------------------------------------------------- #
def test_characters_shape(api):
    data = api.C.characters.data
    assert set(data.keys()) == {"text-orig-full"}
    rows = data["text-orig-full"]
    # rows are (char, count), sorted by char.
    chars = [c for c, _ in rows]
    assert chars == sorted(chars)


def test_characters_parity_with_tf(api, tf_api):
    cf = api.C.characters.data
    tf = tf_api.C.characters.data
    assert set(cf.keys()) == set(tf.keys())
    for fmt in cf:
        # TF stores lists; CF stores tuples of tuples -> normalize.
        assert [tuple(x) for x in cf[fmt]] == [tuple(x) for x in tf[fmt]]


# --------------------------------------------------------------------------- #
# N.sortKeyChunk / N.sortKeyChunkLength
# --------------------------------------------------------------------------- #
CHUNKS = [
    (8, (1, 5)),
    (6, (1, 3)),
    (7, (4, 5)),
    (1, (1, 1)),
    (3, (3, 3)),
]


def test_sort_key_chunk_orders(api):
    ordered = sorted(CHUNKS, key=api.N.sortKeyChunk)
    # canonical position order: begin asc, bigger type first, longer first.
    assert ordered[0] == (8, (1, 5))  # starts at slot 1, largest type
    # every key is self-consistent (reflexive equality)
    keys = [api.N.sortKeyChunk(c) for c in CHUNKS]
    assert not (keys[0] < keys[0])


def test_sort_key_chunk_parity_with_tf(api, tf_api):
    cf_order = sorted(CHUNKS, key=api.N.sortKeyChunk)
    tf_order = sorted(CHUNKS, key=tf_api.N.sortKeyChunk)
    assert cf_order == tf_order


def test_sort_key_chunk_length_parity_with_tf(api, tf_api):
    cf_order = sorted(CHUNKS, key=api.N.sortKeyChunkLength)
    tf_order = sorted(CHUNKS, key=tf_api.N.sortKeyChunkLength)
    assert cf_order == tf_order
