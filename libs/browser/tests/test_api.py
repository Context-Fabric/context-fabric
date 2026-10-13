"""Tests for the browser API, with real search workers on a tiny corpus.

The corpus is the `display_corpus` fixture of the core package: 12 words in one
book ("Alpha Book") of two chapters and three verses, a clause with a gap
(node 19) and a clause that crosses a verse boundary (node 20).
"""

import shutil
import time
from pathlib import Path

import pytest
from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService, CorpusSpec, resolve_corpus
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient

FIXTURE = (
    Path(__file__).resolve().parents[2]
    / "core"
    / "tests"
    / "fixtures"
    / "display_corpus"
)
DATA = FIXTURE / "tf" / "1.0"


@pytest.fixture(scope="module")
def client():
    spec = CorpusSpec(name="alpha", path=DATA)
    service = CorpusService([spec])
    pool = SearchPool(
        [(spec.name, str(spec.path))],
        sort_key=lambda c: service.api(c).N.sortKeyTuple,
        workers=2,
        timeout=20,
        max_results=1000,
    )
    pool.start(wait=True)
    app = create_app(service, pool, manage_pool=False)
    with TestClient(app) as c:
        yield c
    pool.shutdown()


def wait_for(client, search_id, timeout=20.0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        status = client.get(f"/api/searches/{search_id}").json()
        if status["state"] in ("done", "failed", "timeout", "cancelled"):
            return status
        time.sleep(0.05)
    raise AssertionError("search did not finish")


class TestCorpora:
    def test_resolve_repository_layout(self):
        spec = resolve_corpus(str(FIXTURE))
        assert spec.name == "display_corpus"
        assert spec.path == DATA.resolve()

    def test_resolve_with_name_and_data_dir(self):
        spec = resolve_corpus(f"Mine={DATA}")
        assert spec.name == "mine"
        assert spec.path == DATA.resolve()

    def test_resolve_unknown(self, tmp_path):
        with pytest.raises(FileNotFoundError):
            resolve_corpus("no-such-corpus-anywhere", roots=(str(tmp_path),))

    def test_resolve_by_name_in_roots(self, tmp_path):
        repo = tmp_path / "Org" / "alpha"
        for version in ("0.9", "1.0"):
            shutil.copytree(
                DATA, repo / "tf" / version, ignore=shutil.ignore_patterns(".cfm")
            )
        assert resolve_corpus("alpha", roots=(str(tmp_path),)).path.name == "1.0"
        assert resolve_corpus("org/ALPHA", roots=(str(tmp_path),)).name == "alpha"

    def test_resolve_versions(self, tmp_path):
        roots = (str(tmp_path),)
        repo = tmp_path / "Org" / "alpha"
        for version in ("0.9", "1.0", "1.10"):
            shutil.copytree(
                DATA, repo / "tf" / version, ignore=shutil.ignore_patterns(".cfm")
            )
        # no app config here, so the newest version (1.10, not 1.0) by default
        assert resolve_corpus("alpha", roots=roots).path.name == "1.10"
        assert resolve_corpus("alpha@0.9", roots=roots).path.name == "0.9"
        assert resolve_corpus("Org/alpha@latest", roots=roots).path.name == "1.10"
        assert resolve_corpus("old=alpha@0.9", roots=roots).name == "old"
        assert resolve_corpus(f"{repo}@0.9").path.name == "0.9"
        with pytest.raises(FileNotFoundError, match="on this computer: 0.9, 1.0, 1.10"):
            resolve_corpus("alpha@2.0", roots=roots)

    def test_app_config_version_wins_unless_asked(self):
        # the fixture's app config names version 1.0
        assert resolve_corpus(str(FIXTURE)).path.name == "1.0"
        assert resolve_corpus(f"{FIXTURE}@latest").path.name == "1.0"


class TestDescribe:
    def test_health(self, client):
        data = client.get("/api/health").json()
        assert data["ok"] is True
        assert data["searchReady"] is True

    def test_list_corpora(self, client):
        data = client.get("/api/corpora").json()
        assert data == {"corpora": [{"name": "alpha", "title": "Display test corpus"}]}

    def test_corpus_card(self, client):
        card = client.get("/api/corpora/alpha").json()
        assert card["slotType"] == "word"
        assert card["maxSlot"] == 12
        assert card["sectionTypes"] == ["book", "chapter", "verse"]
        assert {"type": "clause", "count": 3, "isSlot": False} in card["nodeTypes"]
        assert card["display"]["direction"] == "rtl"
        assert card["hasAppConfig"] is True
        assert card["version"] == "1.0"  # the data directory that is loaded
        assert str(DATA.resolve()) not in str(card)  # no server paths in the API

    def test_unknown_corpus(self, client):
        assert client.get("/api/corpora/nope").status_code == 404

    def test_features(self, client):
        features = client.get("/api/corpora/alpha/features").json()["features"]
        byName = {f["name"]: f for f in features}
        assert byName["sp"]["kind"] == "node"
        assert byName["trans"]["excluded"] is True
        assert byName["sp"]["excluded"] is False

    def test_feature(self, client):
        data = client.get("/api/corpora/alpha/features/sp").json()
        assert data["unique_values"] == 6
        assert client.get("/api/corpora/alpha/features/nope").status_code == 404


class TestPassages:
    def test_top_level_sections(self, client):
        data = client.get("/api/corpora/alpha/sections").json()
        assert data["children"] == [
            {"node": 13, "type": "book", "label": "Alpha Book", "ref": "Alpha Book"}
        ]

    def test_chapters_and_verses(self, client):
        chapters = client.get(
            "/api/corpora/alpha/sections", params={"ref": "Alpha Book"}
        ).json()
        assert [c["ref"] for c in chapters["children"]] == [
            "Alpha Book 1",
            "Alpha Book 2",
        ]
        verses = client.get(
            "/api/corpora/alpha/sections", params={"ref": "Alpha Book 1"}
        ).json()
        assert [c["label"] for c in verses["children"]] == ["1", "2"]
        leaf = client.get(
            "/api/corpora/alpha/sections", params={"ref": "Alpha Book 1.2"}
        ).json()
        assert leaf["children"] == []

    def test_bad_section(self, client):
        response = client.get("/api/corpora/alpha/sections", params={"ref": "Zeta 1"})
        assert response.status_code == 404

    def test_passage_of_a_verse(self, client):
        data = client.get(
            "/api/corpora/alpha/passage", params={"ref": "Alpha Book 1.1"}
        ).json()
        assert data["total"] == 1
        tree = data["trees"][0]
        assert tree["heading"] == "Alpha Book 1.1"
        assert tree["fragments"][0]["children"][0]["label"] == "Main"

    def test_passage_of_a_chapter_gives_its_verses(self, client):
        data = client.get(
            "/api/corpora/alpha/passage", params={"ref": "Alpha Book 1"}
        ).json()
        assert [t["node"] for t in data["trees"]] == [16, 17]
        assert data["truncated"] is False

    def test_passage_path_and_neighbours(self, client):
        data = client.get(
            "/api/corpora/alpha/passage", params={"ref": "Alpha Book 1.2"}
        ).json()
        assert data["ref"] == "Alpha Book 1.2"
        assert [p["ref"] for p in data["path"]] == [
            "Alpha Book",
            "Alpha Book 1",
            "Alpha Book 1.2",
        ]
        assert data["prev"]["ref"] == "Alpha Book 1.1"
        # the next verse is in the next chapter
        assert data["next"]["ref"] == "Alpha Book 2.1"
        last = client.get(
            "/api/corpora/alpha/passage", params={"ref": "Alpha Book 2"}
        ).json()
        assert last["prev"]["ref"] == "Alpha Book 1"
        assert last["next"] is None

    def test_passage_options(self, client):
        data = client.get(
            "/api/corpora/alpha/passage",
            params={
                "ref": "Alpha Book 1.1",
                "mode": "plain",
                "fmt": "text-trans-plain",
                "hidden_types": "phrase",
            },
        ).json()
        tree = data["trees"][0]
        assert tree["mode"] == "plain"
        assert tree["direction"] == "ltr"
        clause = tree["fragments"][0]["children"][0]
        assert clause["children"][0]["type"] == "word"
        assert clause["children"][0]["text"] == "a "

    def test_node_render_and_info(self, client):
        tree = client.get(
            "/api/corpora/alpha/nodes/20", params={"extra_features": "sp"}
        ).json()
        assert tree["type"] == "clause"
        info = client.get("/api/corpora/alpha/nodes/20/info").json()
        assert info["type"] == "clause"
        assert info["features"]["typ"] == "Rel"
        assert info["slots"] == [3, 6]
        assert info["downCounts"] == {"phrase": 2, "word": 4}
        assert {"node": 13, "type": "book"} in info["up"]
        assert client.get("/api/corpora/alpha/nodes/999/info").status_code == 404


class TestSearch:
    def test_search_lifecycle(self, client):
        response = client.post(
            "/api/corpora/alpha/search",
            json={"template": "clause\n  phrase\n    word sp=verb\n"},
        )
        assert response.status_code == 202
        status = wait_for(client, response.json()["id"])
        assert status["state"] == "done"
        assert status["count"] == 3
        assert status["sorted"] is True

        page = client.get(
            f"/api/searches/{status['id']}/results", params={"render": "plain"}
        ).json()
        rows = page["rows"]
        assert [r["nodes"] for r in rows] == [[20, 23, 3], [20, 24, 6], [21, 26, 9]]
        assert rows[0]["ref"] == "Alpha Book 1.1"
        assert rows[0]["types"] == ["clause", "phrase", "word"]
        assert len(rows[0]["plain"]) == 3

        # clause 20 crosses from verse 16 into verse 17: shown in both
        result = client.get(f"/api/searches/{status['id']}/results/0").json()
        assert [t["node"] for t in result["trees"]] == [16, 17]

        tsv = client.get(f"/api/searches/{status['id']}/export.tsv")
        assert tsv.status_code == 200
        lines = tsv.text.strip().split("\n")
        assert lines[0].split("\t")[:5] == ["R", "S1", "S2", "S3", "NODE1"]
        assert lines[1].split("\t")[:5] == ["1", "Alpha Book", "1", "1", "20"]
        assert len(lines) == 4

    def test_export_feature_columns(self, client):
        # `typ*` and `sp*` match any value: they only add columns
        response = client.post(
            "/api/corpora/alpha/search",
            json={"template": "clause typ*\n  word sp* gloss~a\n"},
        )
        status = wait_for(client, response.json()["id"])
        assert status["state"] == "done"
        tsv = client.get(f"/api/searches/{status['id']}/export.tsv").text
        lines = tsv.strip().split("\n")
        assert lines[0].split("\t") == [
            "R", "S1", "S2", "S3",
            "NODE1", "TYPE1", "TEXT1", "typ1",
            "NODE2", "TYPE2", "TEXT2", "sp2", "gloss2",
        ]  # fmt: skip
        first = lines[1].split("\t")
        assert first[4:8] == ["19", "clause", "α β η θ ", "Main"]
        # the first word of clause 19 whose gloss contains an "a" is word 7
        assert first[8:] == ["7", "word", "η ", "conj", "and"]

    def test_export_extra_columns(self, client):
        response = client.post(
            "/api/corpora/alpha/search", json={"template": "clause\n  word sp*\n"}
        )
        status = wait_for(client, response.json()["id"])
        url = f"/api/searches/{status['id']}/export.tsv"
        tsv = client.get(url, params={"extra": ["2:gloss,sp", "1:typ"]}).text
        header = tsv.split("\n")[0].split("\t")
        # template features first (sp2), extras after, without repeating sp2
        assert header[4:] == [
            "NODE1", "TYPE1", "TEXT1", "typ1",
            "NODE2", "TYPE2", "TEXT2", "sp2", "gloss2",
        ]  # fmt: skip
        assert client.get(url, params={"extra": "3:gloss"}).status_code == 422
        assert client.get(url, params={"extra": "2:nope"}).status_code == 422

    def test_export_without_template_features(self, client):
        response = client.post("/api/corpora/alpha/search", json={"template": "verse"})
        status = wait_for(client, response.json()["id"])
        header = client.get(f"/api/searches/{status['id']}/export.tsv").text
        assert header.split("\n")[0] == "R\tS1\tS2\tS3\tNODE1\tTYPE1\tTEXT1"

    def test_export_for_excel(self, client):
        response = client.post("/api/corpora/alpha/search", json={"template": "word"})
        status = wait_for(client, response.json()["id"])
        excel = client.get(
            f"/api/searches/{status['id']}/export.tsv", params={"excel": True}
        )
        assert excel.content[:2] == b"\xff\xfe"  # UTF-16 little endian, with BOM
        assert excel.content.decode("utf-16").startswith("R\tS1")
        assert "-excel.tsv" in excel.headers["content-disposition"]

    def test_statistics(self, client):
        # 11 (clause, word) pairs: clause 19 has words 1, 2, 7, 8 (it has a
        # gap), clause 20 words 3-6 (it crosses into verse 2), clause 21 words 9-11
        response = client.post(
            "/api/corpora/alpha/search", json={"template": "clause\n  word sp*\n"}
        )
        status = wait_for(client, response.json()["id"])
        assert status["count"] == 11
        base = f"/api/searches/{status['id']}/stats"

        features = client.get(f"{base}/features", params={"position": 1}).json()
        assert features["type"] == "word"
        assert features["inTemplate"] == ["sp"]
        assert features["open"] == ["sp"]  # `sp*` leaves it open
        assert {"sp", "gloss", "g_word"} <= set(features["features"])
        assert "trans" not in features["features"]  # excluded by the app config

        values = client.get(
            f"{base}/values", params={"position": 1, "feature": "sp", "top": 2}
        ).json()
        assert values["nodes"] == 11
        assert values["distinct"] == 5
        assert values["values"] == [
            {"value": "subs", "count": 4},
            {"value": "verb", "count": 3},
        ]
        assert values["other"] == {"values": 3, "count": 4}

        # each clause counts once per result; clause 20 is counted in verse 1
        # (its first word), clause 19 too
        verses = client.get(f"{base}/sections", params={"level": 2}).json()
        assert [(s["ref"], s["count"]) for s in verses["sections"]] == [
            ("Alpha Book 1.1", 8),
            ("Alpha Book 2.1", 3),
        ]
        books = client.get(f"{base}/sections", params={"level": 0}).json()
        assert [(s["label"], s["count"]) for s in books["sections"]] == [
            ("Alpha Book", 11)
        ]

    def test_open_and_fixed_features(self, client):
        response = client.post(
            "/api/corpora/alpha/search",
            json={"template": "word sp=verb gloss* g_word~."},
        )
        status = wait_for(client, response.json()["id"])
        features = client.get(f"/api/searches/{status['id']}/stats/features").json()
        assert features["inTemplate"] == ["sp", "gloss", "g_word"]
        assert features["open"] == ["gloss", "g_word"]

    def test_statistics_errors(self, client):
        response = client.post("/api/corpora/alpha/search", json={"template": "word"})
        status = wait_for(client, response.json()["id"])
        base = f"/api/searches/{status['id']}/stats"
        assert (
            client.get(f"{base}/values", params={"feature": "nope"}).status_code == 404
        )
        assert (
            client.get(
                f"{base}/values", params={"feature": "sp", "position": 3}
            ).status_code
            == 404
        )

    def test_invalid_template(self, client):
        response = client.post(
            "/api/corpora/alpha/search", json={"template": "word spp=verb"}
        )
        assert response.status_code == 400
        detail = response.json()["detail"]
        assert detail["problems"] == [
            {
                "kind": "semantics",
                "line": 1,
                "message": 'Unknown feature "spp"',
                "feature": "spp",
            }
        ]

    def test_check_endpoint(self, client):
        ok = client.post("/api/corpora/alpha/check", json={"template": "word"}).json()
        assert ok == {"ok": True, "problems": []}
        bad = client.post(
            "/api/corpora/alpha/check", json={"template": "word\nphrase"}
        ).json()
        assert bad["ok"] is False
        assert "connected" in bad["problems"][0]["message"]

    def test_unsupported_language(self, client):
        response = client.post(
            "/api/corpora/alpha/search",
            json={"template": "find verbs", "language": "natural-language"},
        )
        assert response.status_code == 422

    def test_problems_inside_quantifiers_come_from_the_worker(self, client):
        response = client.post(
            "/api/corpora/alpha/search",
            json={"template": "phrase\n/without/\n  word spp=verb\n/-/\n"},
        )
        assert response.status_code == 202
        status = wait_for(client, response.json()["id"])
        # the engine itself only logs this and returns every phrase
        assert status["state"] == "failed"
        assert status["problems"] == [
            {
                "kind": "semantics",
                "line": 2,
                "message": 'Unknown feature "spp" (inside /without/)',
                "feature": "spp",
                "quantifier": "/without/",
            }
        ]

    def test_timeout(self, client):
        response = client.post(
            "/api/corpora/alpha/search", json={"template": "word", "timeout": 1e-9}
        )
        assert wait_for(client, response.json()["id"])["state"] == "timeout"

    def test_limit_truncates(self, client):
        response = client.post(
            "/api/corpora/alpha/search", json={"template": "word", "limit": 5}
        )
        status = wait_for(client, response.json()["id"])
        assert status["count"] == 5
        assert status["truncated"] is True

    def test_events_stream(self, client):
        response = client.post("/api/corpora/alpha/search", json={"template": "word"})
        searchId = response.json()["id"]
        with client.stream("GET", f"/api/searches/{searchId}/events") as events:
            body = "".join(events.iter_text())
        assert "event: status" in body
        assert '"state": "done"' in body

    def test_cancel_and_unknown_search(self, client):
        response = client.post("/api/corpora/alpha/search", json={"template": "word"})
        searchId = response.json()["id"]
        assert client.delete(f"/api/searches/{searchId}").status_code == 200
        assert wait_for(client, searchId)["state"] in ("done", "cancelled")
        assert client.get("/api/searches/nope").status_code == 404


class TestInterface:
    def test_spa_fallback(self, client):
        response = client.get("/some/route/in/the/app")
        assert response.status_code == 200
        assert "Context-Fabric Browser" in response.text

    def test_unknown_api_route(self, client):
        assert client.get("/api/does-not-exist").status_code == 404
