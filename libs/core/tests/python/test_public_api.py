from __future__ import annotations

import importlib
import importlib.metadata

import cfabric
import pytest
from cfabric import BANNER, NAME, VERSION, Fabric
from cfabric.describe import describe_corpus_overview, describe_feature, list_features
from cfabric.results import CorpusInfo, FeatureInfo, NodeInfo


def test_top_level_exports(mini_corpus_path):
    assert VERSION == importlib.metadata.version("context-fabric")
    assert cfabric.__version__ == VERSION
    assert NAME == "Context-Fabric"
    assert BANNER == f"This is Context-Fabric {VERSION}"

    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    assert fabric.good is True
    assert fabric.locations == [str(mini_corpus_path)]


def test_feature_access_and_lists(loaded_api):
    assert loaded_api.F.word.v(1) == "hello"
    assert loaded_api.F.word.s("hello") == (1,)
    assert next(iter(loaded_api.F.word.items())) == (1, "hello")
    assert loaded_api.F.word.valueType == "str"
    assert loaded_api.F.word.description == "word text"
    assert loaded_api.F.word.metaData["description"] == "word text"
    assert loaded_api.Fs("word").v(2) == "beautiful"

    assert loaded_api.E.oslots.s(8) == (1, 2, 3, 4, 5)
    assert loaded_api.E.distance.valueType == "int"
    assert loaded_api.E.distance.description.startswith("distance between nodes")
    assert loaded_api.E.distance.metaData["edgeValues"] is None
    assert loaded_api.E.distance.f(1) == ((2, 0), (3, 5))
    assert loaded_api.E.distance.b(2) == ((1, 0), (3, None))
    assert loaded_api.E.distance.doValues is True
    assert loaded_api.Es("oslots").s(6) == (1, 2, 3)

    assert "word" in loaded_api.Fall(warp=False)
    assert "oslots" not in loaded_api.Eall(warp=False)
    assert "levels" in loaded_api.Call()


def test_computed_navigation_text_and_search(loaded_api):
    assert loaded_api.F.otype.slotType == "word"
    assert loaded_api.F.otype.maxSlot == 5
    assert loaded_api.F.otype.maxNode == 8

    assert loaded_api.C.order.data
    assert loaded_api.N.sortNodes({5, 3, 1}) == [1, 3, 5]
    assert loaded_api.L.u(1, otype={"phrase", "sentence"}) == (6, 8)

    assert loaded_api.T.text(1) == "hello"
    assert loaded_api.T.sectionTypes == ["sentence", "phrase"]

    # As in TF, `api.S` and `api.T` (like every member) are one persistent
    # object each, so the executor of the last `study` stays on it.
    search = loaded_api.S
    assert search is loaded_api.S
    assert loaded_api.T is loaded_api.T
    assert search.exe is None
    loaded_api.S.study("word")
    assert search.exe.good is True
    assert loaded_api.S.fetch(limit=1) == ((1,),)
    assert search.search("word", limit=2) == ((1,), (2,))


def test_text_fabric_compatibility_surface(mini_corpus_path, loaded_api, capsys):
    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    assert fabric.explore(show=False) is None
    inventory = fabric.explore()
    assert "word" in inventory["nodes"]
    assert "__levels__" in inventory["computeds"]
    assert "word" in fabric.featureSets["nodes"]
    assert fabric.features["word"].metaData["description"] == "word text"
    assert fabric.features["distance"].isEdge is True
    assert fabric.isLoaded("word", pretty=False) == {"word": None}
    assert fabric.save()

    api = fabric.load(("word",), silent="deep")
    assert fabric.api is api
    assert api.TF is fabric
    assert api.Fall() == ["otype", "phrase_id", "sentence_id", "word"]
    assert api.isLoaded("word", pretty=False)["word"]["kind"] == "node"
    assert api.isLoaded("pos", pretty=False) == {"pos": None}
    assert api.isLoaded("word") is None
    assert api.ensureLoaded("pos") == {"pos"}
    assert api.F.pos.v(1) == "interjection"
    assert api.footprint() is None
    assert "TOTAL" in capsys.readouterr().out
    scope = {}
    docs = api.makeAvailableIn(scope)
    assert {"F", "E", "L", "N", "T", "S", "C", "Fs", "Feature", "TF"} <= set(scope)
    assert ("Fabric", "loading", ("TF",)) in docs

    assert api.Cs("order").data
    assert loaded_api.F.word.meta == loaded_api.F.word.metaData
    assert loaded_api.F.word.data[1] == "hello"
    assert loaded_api.F.word.freqList(nodeTypes=("word",))
    assert loaded_api.E.oslots.data[8 - loaded_api.F.otype.maxSlot - 1] == (1, 2, 3, 4, 5)
    assert loaded_api.E.oslots.dataInv[1] == {6, 8}
    assert loaded_api.E.distance.data[1][3] == 5
    assert loaded_api.E.distance.dataInv[2][1] == 0
    assert loaded_api.E.distance.freqList(
        nodeTypesFrom=("word",), nodeTypesTo=("word",)
    )

    events = list(loaded_api.N.walk(events=True))
    assert all(isinstance(item, tuple) and len(item) == 2 for item in events)
    # The non-end events (slots and container starts) replay walk() order.
    assert list(loaded_api.N.walk()) == [
        node for (node, kind) in events if kind is not True
    ]
    assert loaded_api.T.sectionFeats == loaded_api.T.sectionFeatures
    # sectionTuple returns section NODE ids; sectionFromNode returns their values.
    assert loaded_api.T.sectionTuple(1) == (8, 6)
    assert loaded_api.T.sectionFromNode(1) == ("S1", 1)
    assert loaded_api.T.formats == {"text-orig-full": "word"}
    assert loaded_api.T.structureTypes == []
    assert loaded_api.T.structureFeats == []
    assert loaded_api.T.languages == {
        "": {"language": "default", "languageEnglish": "default"}
    }
    capsys.readouterr()
    assert loaded_api.T.structureInfo() is None
    assert "No structural elements configured" in capsys.readouterr().out
    # No structure configured: TF's structure navigation answers None.
    assert loaded_api.T.up(1) is None
    assert loaded_api.T.down(8) is None

    search = loaded_api.S
    search.study("word")
    assert search.fetch(limit=1) == ((1,),)
    assert search.count(limit=2) == 2
    assert search.showPlan(False) is None
    assert "word" in capsys.readouterr().out
    assert search.glean((1,)) == "hello"
    assert search.glean((1, 2)) == "hello beautiful"
    assert search.tweakPerformance() is None
    assert search.relationsLegend() is None
    assert capsys.readouterr().out
    assert search.perfParams == {}
    assert search.api is loaded_api


def test_compiled_cache_round_trip(mini_corpus_path, tmp_path):
    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    cache_path = tmp_path / "mini.cfr"

    assert fabric.compile(str(cache_path), features=("otype", "oslots", "word"))
    assert not hasattr(fabric, "loadCompiled")
    assert not hasattr(fabric, "load_compiled")
    api = fabric.openMapped(str(cache_path))

    assert api.F.word.v(1) == "hello"
    assert api.E.oslots.s(8) == (1, 2, 3, 4, 5)
    assert api.S.search("word", limit=1) == ((1,),)

    mapped = fabric.openMapped(str(cache_path))
    assert mapped.F.otype.maxNode == 8
    assert mapped.Fall(warp=False) == ["word"]
    assert mapped.S.search("word", limit=1) == ((1,),)


def test_result_dataclasses(loaded_api):
    node = NodeInfo.from_api(loaded_api, 1)
    assert node.to_dict()["text"] == "hello"

    feature = FeatureInfo.from_api(loaded_api, "word", "node")
    assert feature and feature.name == "word"
    assert feature.value_type == "str"
    assert feature.description == "word text"

    corpus = CorpusInfo.from_api(loaded_api, "mini", "fixture")
    assert corpus.slot_type == "word"


def test_describe_helpers(loaded_api):
    overview = describe_corpus_overview(loaded_api, "mini").to_dict()
    assert overview["name"] == "mini"
    assert any(row["type"] == "word" for row in overview["node_types"])

    entries = list_features(loaded_api, kind="node")
    assert any(
        entry.name == "word" and entry.description == "word text" for entry in entries
    )

    feature = describe_feature(loaded_api, "word", sample_limit=2).to_dict()
    assert feature["name"] == "word"
    assert feature["kind"] == "node"
    assert feature["value_type"] == "str"
    assert feature["description"] == "word text"
    assert feature["sample_values"]


def test_public_extension_members_have_docstrings():
    """Every public class and member of `cfabric._core` documents itself."""
    from cfabric import _core

    undocumented = []
    for class_name in _core.__all__:
        cls = getattr(_core, class_name)
        if not isinstance(cls, type):
            continue
        if not (cls.__doc__ or "").strip():
            undocumented.append(class_name)
        for name, member in vars(cls).items():
            if (
                not name.startswith("_")
                and not (getattr(member, "__doc__", "") or "").strip()
            ):
                undocumented.append(f"{class_name}.{name}")
    assert not undocumented, undocumented


def test_fabric_forwards_api_shortcuts_after_loading(mini_corpus_path, capsys):
    """TF attaches ensureLoaded / makeAvailableIn / footprint to the Fabric."""
    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    with pytest.raises(AttributeError):
        fabric.ensureLoaded("pos")
    api = fabric.load("word")
    assert fabric.ensureLoaded("pos") == {"pos"}
    assert "pos" in api.Fall()
    assert "pos" in fabric.featuresRequested
    scope = {}
    fabric.makeAvailableIn(scope)
    assert scope["F"] is api.F
    assert fabric.footprint() is None
    assert "TOTAL" in capsys.readouterr().out


def test_fabric_and_api_are_garbage_collected(mini_corpus_path):
    """The Fabric <-> api reference cycle (TF.api / api.TF) is collectable."""
    import gc
    import weakref

    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    api = fabric.loadAll()
    api.S.study("word")
    api_ref = weakref.ref(api)
    del fabric, api
    gc.collect()
    assert api_ref() is None


def test_api_members_report_a_collected_api(mini_corpus_path):
    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    text = fabric.loadAll().T
    assert text.api is fabric.api
    del fabric
    import gc

    gc.collect()
    assert text.text(1) == "hello"
    with pytest.raises(ReferenceError):
        _ = text.api


@pytest.mark.parametrize("module_name", ["cfabric", "cfabric.results", "cfabric.describe"])
def test_all_exports_resolve(module_name):
    module = importlib.import_module(module_name)
    missing = [name for name in getattr(module, "__all__", ()) if not hasattr(module, name)]
    assert missing == []
