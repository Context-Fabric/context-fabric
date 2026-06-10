from __future__ import annotations

from cfabric import BANNER, NAME, VERSION, Fabric
from cfabric.describe import describe_corpus_overview, describe_feature, list_features
from cfabric.results import CorpusInfo, FeatureInfo, NodeInfo, NodeList, SearchResult


def test_top_level_exports(mini_corpus_path):
    assert VERSION == "0.6.0rc1"
    assert NAME == "Context-Fabric"
    assert "0.6.0rc1" in BANNER

    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    assert fabric.good is True
    assert fabric.locations == (str(mini_corpus_path),)


def test_feature_access_and_lists(loaded_api):
    assert loaded_api.F.word.v(1) == "hello"
    assert loaded_api.F.word.s("hello") == (1,)
    assert loaded_api.F.word.items()[0] == (1, "hello")
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
    assert loaded_api.N.sortNodes({5, 3, 1}) == (1, 3, 5)
    assert loaded_api.L.u(1, otype={"phrase", "sentence"}) == (6, 8)

    assert loaded_api.T.text(1) == "hello"
    assert loaded_api.T.sectionTypes == ("sentence", "phrase")

    loaded_api.S.study("word")
    assert loaded_api.S.exe is None
    search = loaded_api.S
    search.study("word")
    assert search.exe.good is True
    assert search.search("word", limit=2) == ((1,), (2,))


def test_text_fabric_compatibility_surface(mini_corpus_path, loaded_api):
    fabric = Fabric(locations=str(mini_corpus_path), silent="deep")
    inventory = fabric.explore(show=False)
    assert "word" in inventory["nodes"]
    assert fabric.features["nodes"]
    assert fabric.isLoaded("word")
    assert fabric.save()

    api = fabric.load(features=("otype", "word", "oslots"), add=True, silent="deep")
    assert api.isLoaded(("otype", "word"))
    assert api.ensureLoaded(("otype", "word"))
    assert "node features" in api.footprint()
    scope = {}
    api.makeAvailableIn(scope)
    assert {"F", "E", "L", "N", "T", "S", "C"} <= set(scope)

    assert api.Cs("order").data
    assert loaded_api.F.word.meta == loaded_api.F.word.metaData
    assert loaded_api.F.word.data[1] == "hello"
    assert loaded_api.F.word.freqList(node_types=("word",))
    assert loaded_api.E.oslots.data[8] == (1, 2, 3, 4, 5)
    assert loaded_api.E.oslots.dataInv[1] == (6, 8)
    assert loaded_api.E.distance.data[1][3] == 5
    assert loaded_api.E.distance.dataInv[2][1] == 0
    assert loaded_api.E.distance.freqList(node_types_from=("word",), node_types_to=("word",))

    assert loaded_api.N.walk(events=True) == loaded_api.N.walk()
    assert loaded_api.T.sectionFeats == loaded_api.T.sectionFeatures
    assert loaded_api.T.sectionTuple(1) == loaded_api.T.sectionFromNode(1)
    assert loaded_api.T.formats == {"text-orig-full": "word"}
    assert isinstance(loaded_api.T.structureTypes, tuple)
    assert isinstance(loaded_api.T.structureFeats, tuple)
    assert loaded_api.T.languages == {
        "": {"language": "default", "languageEnglish": "default"}
    }
    assert loaded_api.T.structureInfo() == "No structural elements configured"
    assert loaded_api.T.up(1) == loaded_api.L.u(1)
    assert loaded_api.T.down(8) == loaded_api.L.d(8)

    search = loaded_api.S
    search.study("word")
    assert search.fetch(limit=1) == ((1,),)
    assert search.count(limit=2) == 2
    assert "word" in search.showPlan(False)
    assert search.glean(((1,),)) == ((1,),)
    assert search.tweakPerformance()
    assert isinstance(search.relationsLegend(), str)
    assert search.perfParams == {}
    assert search.api is True


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
    assert mapped.Fall(warp=False) == ("word",)
    assert mapped.S.search("word", limit=1) == ((1,),)


def test_result_dataclasses(loaded_api):
    node = NodeInfo.from_api(loaded_api, 1)
    assert node.to_dict()["text"] == "hello"

    nodes = NodeList.from_nodes(loaded_api, (1, 2), query="word")
    assert nodes.total_count == 2

    search = SearchResult.from_search(loaded_api, ((1,),), template="word")
    assert search.total_count == 1

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
    assert any(entry.name == "word" and entry.description == "word text" for entry in entries)

    feature = describe_feature(loaded_api, "word", sample_limit=2).to_dict()
    assert feature["name"] == "word"
    assert feature["kind"] == "node"
    assert feature["value_type"] == "str"
    assert feature["description"] == "word text"
    assert feature["sample_values"]
