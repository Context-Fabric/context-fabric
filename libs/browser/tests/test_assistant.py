"""Tests for the tools a language model gets (searching in plain language),
on the miniature BHSA of `test_lexicon.py` and a real search pool."""

import pytest
from cfabric_browser.assistant.tools import ToolError, Tools
from cfabric_browser.corpora import CorpusService, resolve_corpus
from cfabric_browser.workers import SearchPool
from test_lexicon import write_tf


@pytest.fixture(scope="module")
def tools(tmp_path_factory):
    root = tmp_path_factory.mktemp("assistant")
    write_tf(root / "Hebrew" / "bible" / "tf" / "1.0")
    roots = (str(root),)
    spec = resolve_corpus("Hebrew/bible", roots)
    service = CorpusService([spec], roots=roots)
    pool = SearchPool(
        [(spec.name, str(spec.path))],
        sort_key=lambda c: service.api(c).N.sortKeyTuple,
        workers=1,
        timeout=20,
        max_results=1000,
    )
    pool.start(wait=True)
    yield Tools(service, pool)
    pool.shutdown()


def test_list_and_describe(tools):
    [corpus] = tools.list_corpora()["corpora"]
    assert corpus["name"] == "bible" and corpus["translation"] is False
    assert corpus["sectionTypes"] == ["book", "chapter", "verse"]
    described = tools.describe_corpus("bible")
    assert {t["type"] for t in described["node_types"]} >= {"word", "lex", "verse"}
    features = tools.list_features("bible", ["lex"])["features"]
    assert "gloss" in {f["name"] for f in features}
    gloss = tools.describe_feature("bible", ["gloss"])
    assert "create" in str(gloss)
    assert "quantifiers" in str(tools.search_syntax_guide()).lower()


def test_unknown_corpus(tools):
    with pytest.raises(ToolError, match="open: bible"):
        tools.describe_corpus("bhsa")


def test_check_template(tools):
    assert tools.check_template("bible", "lex lex=BR>[") == {"ok": True, "problems": []}
    checked = tools.check_template("bible", "wurd")
    assert not checked["ok"] and checked["problems"][0]["nodeType"] == "wurd"


def test_try_search(tools):
    found = tools.try_search("bible", "word")
    assert found["ok"] and found["state"] == "done"
    assert found["count"] == 8 and not found["atLeast"]
    assert len(found["examples"]) == 5
    first = found["examples"][0]
    assert first["ref"] == "Genesis 1:1"
    assert first["nodes"] == [{"node": 1, "type": "word", "text": "בְּ"}]
    # a lexeme and its words: tuples of two nodes
    pairs = tools.try_search("bible", "lex lex=BR>[\n  word", examples=2)
    assert pairs["count"] == 3
    assert [n["type"] for n in pairs["examples"][0]["nodes"]] == ["lex", "word"]


def test_try_search_with_problems(tools):
    found = tools.try_search("bible", "wurd")
    assert not found["ok"] and found["problems"]
    assert "count" not in found


def test_corpus_guide_missing(tools):
    guide = tools.corpus_guide("bible")
    assert guide["guide"] is None and "describe_corpus" in guide["note"]


def test_hints_for_empty_results(tools):
    found = tools.try_search("bible", "lex lex=BR>")
    assert found["ok"] and found["count"] == 0
    assert found["hints"] == ["lex never has the value BR>"]
    assert "hints" not in tools.try_search("bible", "lex lex=BR>[")
