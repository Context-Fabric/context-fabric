"""Tests for the MCP server of the browser (`/mcp`) and the stdio proxy
(`cfabric-browser mcp`), on the miniature BHSA of `test_lexicon.py`."""

import io
import json

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.assistant import stdio
from cfabric_browser.corpora import CorpusService, resolve_corpus
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient
from test_lexicon import write_tf

HEADERS = {"Accept": "application/json, text/event-stream"}


@pytest.fixture(scope="module")
def client(tmp_path_factory):
    root = tmp_path_factory.mktemp("mcp")
    write_tf(root / "Hebrew" / "bible" / "tf" / "1.0")
    roots = (str(root),)
    spec = resolve_corpus("Hebrew/bible", roots)
    service = CorpusService([spec], roots=roots)
    pool = SearchPool(
        [(spec.name, str(spec.path))],
        sort_key=lambda c: service.api(c).N.sortKeyTuple,
        workers=1,
        timeout=20,
    )
    pool.start(wait=True)
    app = create_app(service, pool, manage_pool=False, secret="s")
    with TestClient(app, base_url="http://127.0.0.1:8107") as c:
        yield c
    pool.shutdown()


def rpc(client, method, params=None, id=1, **headers):
    message = {"jsonrpc": "2.0", "id": id, "method": method, "params": params or {}}
    return client.post("/mcp", json=message, headers={**HEADERS, **headers})


def call(client, tool, **args):
    answer = rpc(client, "tools/call", {"name": tool, "arguments": args}).json()
    return json.loads(answer["result"]["content"][0]["text"])


def test_initialize_and_tools(client):
    init = rpc(
        client,
        "initialize",
        {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"},
        },
    )
    assert init.status_code == 200
    result = init.json()["result"]
    assert result["serverInfo"]["name"] == "CACCHT Explorer"
    assert "propose_search" in result["instructions"]
    tools = rpc(client, "tools/list").json()["result"]["tools"]
    assert {t["name"] for t in tools} == {
        "list_corpora",
        "corpus_guide",
        "describe_corpus",
        "list_features",
        "describe_feature",
        "search_syntax_guide",
        "check_template",
        "try_search",
        "propose_search",
    }


def test_tools_through_mcp(client):
    assert call(client, "list_corpora")["corpora"][0]["name"] == "bible"
    found = call(client, "try_search", corpus="bible", template="word")
    assert found["count"] == 8
    shown = call(
        client,
        "propose_search",
        corpus="bible",
        template="lex lex=BR>[",
        request="the lexeme create",
        explanation="lex=BR>[ is the verb create",
        model="Claude",
    )
    assert shown["ok"] and shown["count"] == 1
    listed = client.get("/api/proposals").json()["proposals"]
    assert (
        listed[0]["request"] == "the lexeme create" and listed[0]["model"] == "Claude"
    )
    # an unknown corpus is an answer, not a failure
    assert call(client, "describe_corpus", corpus="bhsa")["ok"] is False


def test_web_pages_cannot_use_it(client):
    page = rpc(client, "tools/list", Origin="http://example.org")
    assert page.status_code in (400, 403)
    own = rpc(client, "tools/list", Origin="http://127.0.0.1:8107")
    assert own.status_code in (400, 403)
    elsewhere = rpc(client, "tools/list", Host="example.org")
    assert elsewhere.status_code in (400, 403, 421)


def test_the_page_still_works(client):
    # /mcp is a route of its own, before the interface's catch-all one
    assert client.get("/api/health").status_code == 200


class TestStdio:
    def messages(self, *items):
        return io.BytesIO(b"".join(json.dumps(m).encode() + b"\n" for m in items))

    def test_forwards_each_message(self, client, monkeypatch):
        monkeypatch.setattr(
            stdio, "find_browser", lambda path: "http://127.0.0.1:8107/"
        )

        def forward(url, message):
            answer = client.post(
                "/mcp",
                content=message,
                headers={**HEADERS, "Content-Type": "application/json"},
            )
            return answer.content if answer.content.strip() else None

        monkeypatch.setattr(stdio, "forward", forward)
        out = io.BytesIO()
        stdio.serve(
            self.messages(
                {"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}},
                # a notification has no answer
                {"jsonrpc": "2.0", "method": "notifications/initialized"},
                {
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {"name": "list_corpora", "arguments": {}},
                },
            ),
            out,
            start=False,
        )
        lines = [json.loads(line) for line in out.getvalue().splitlines()]
        assert [line["id"] for line in lines] == [1, 2]
        assert "bible" in lines[1]["result"]["content"][0]["text"]

    def test_without_a_browser(self, monkeypatch):
        monkeypatch.setattr(stdio, "find_browser", lambda path: None)
        out = io.BytesIO()
        stdio.serve(
            self.messages(
                {"jsonrpc": "2.0", "id": 7, "method": "tools/list"},
                {"jsonrpc": "2.0", "method": "notifications/initialized"},
            ),
            out,
            start=False,
        )
        [line] = [json.loads(x) for x in out.getvalue().splitlines()]
        assert line["id"] == 7 and "not running" in line["error"]["message"]

    def test_finds_it_again_after_a_restart(self, monkeypatch):
        urls = iter(["http://127.0.0.1:1/", "http://127.0.0.1:2/"])
        monkeypatch.setattr(stdio, "find_browser", lambda path: next(urls))

        def forward(url, message):
            if url.endswith(":1/"):
                raise OSError("refused")
            return b'{"jsonrpc":"2.0","id":1,"result":{}}'

        monkeypatch.setattr(stdio, "forward", forward)
        out = io.BytesIO()
        stdio.serve(
            self.messages({"jsonrpc": "2.0", "id": 1, "method": "ping"}),
            out,
            start=False,
        )
        assert json.loads(out.getvalue()) == {"jsonrpc": "2.0", "id": 1, "result": {}}
