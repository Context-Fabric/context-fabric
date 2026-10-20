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


def test_propose_search(tools):
    shown = tools.propose_search(
        "bible", "lex lex=BR>[", "the lexeme create", "lex is the lexeme", "claude"
    )
    assert shown["ok"] and shown["count"] == 1
    [newest, *_] = tools.proposals.snapshot()["proposals"]
    assert newest["id"] == shown["proposal"] and newest["state"] == "new"
    assert newest["request"] == "the lexeme create" and newest["model"] == "claude"
    # a template with problems is not shown
    assert not tools.propose_search("bible", "wurd", "x", "")["ok"]
    assert tools.proposals.snapshot()["proposals"][0]["id"] == shown["proposal"]
    assert tools.proposals.set_state(shown["proposal"], "dismissed")["state"] == (
        "dismissed"
    )


def test_proposal_routes(tmp_path):
    from cfabric_browser.app import create_app
    from fastapi.testclient import TestClient

    write_tf(tmp_path / "Hebrew" / "bible" / "tf" / "1.0")
    roots = (str(tmp_path),)
    spec = resolve_corpus("Hebrew/bible", roots)
    service = CorpusService([spec], roots=roots)
    pool = SearchPool(
        [(spec.name, str(spec.path))],
        sort_key=lambda c: service.api(c).N.sortKeyTuple,
        workers=1,
        timeout=20,
    )
    pool.start(wait=True)
    app = create_app(service, pool, manage_pool=False, secret="s3cret")
    body = {"corpus": "bible", "template": "word", "request": "all words"}
    try:
        with TestClient(app) as client:
            assert client.get("/api/proposals").json()["proposals"] == []
            # without the secret, or with another, nothing is proposed
            assert client.post("/api/proposals", json=body).status_code == 403
            bad = {"X-Cfabric-Secret": "guess"}
            assert (
                client.post("/api/proposals", json=body, headers=bad).status_code == 403
            )
            good = {"X-Cfabric-Secret": "s3cret"}
            shown = client.post("/api/proposals", json=body, headers=good).json()
            assert shown["ok"] and shown["count"] == 8
            listed = client.get("/api/proposals").json()["proposals"]
            assert [p["id"] for p in listed] == [shown["proposal"]]
            ran = client.post(
                f"/api/proposals/{shown['proposal']}", json={"state": "run"}
            )
            assert ran.json()["state"] == "run"
            assert (
                client.post("/api/proposals/nope", json={"state": "run"}).status_code
                == 404
            )
            unknown = {**body, "corpus": "bhsa"}
            assert (
                client.post("/api/proposals", json=unknown, headers=good).status_code
                == 404
            )
    finally:
        pool.shutdown()


def test_running_file(tmp_path):
    from cfabric_browser.assistant import running

    path = tmp_path / "running.json"
    assert running.read(path) == []
    first = running.announce("http://127.0.0.1:8107/", "a", path)
    running.announce("http://127.0.0.1:8108/", "b", path)
    assert [b["url"] for b in running.read(path)] == [
        "http://127.0.0.1:8107/",
        "http://127.0.0.1:8108/",
    ]
    assert first["secret"] == "a" and first["pid"]
    running.withdraw("http://127.0.0.1:8107/", path)
    assert [b["secret"] for b in running.read(path)] == ["b"]
    path.write_text("not json")
    assert running.read(path) == []
