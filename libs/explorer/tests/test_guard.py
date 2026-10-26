"""Requests from other web sites, and under foreign host names, are refused."""

from cfabric_browser.app import create_app
from cfabric_browser.corpora import CorpusService
from cfabric_browser.guard import LOCAL_HOSTS
from cfabric_browser.workers import SearchPool
from fastapi.testclient import TestClient


def client(tmp_path, hosts=None) -> TestClient:
    app = create_app(
        CorpusService([]),
        SearchPool([]),
        static_dir=tmp_path,
        manage_pool=False,
        hosts=hosts,
    )
    return TestClient(app, base_url="http://127.0.0.1:8107")


def test_own_page_and_plain_requests_pass(tmp_path):
    c = client(tmp_path, hosts=list(LOCAL_HOSTS))
    assert c.get("/api/health").status_code == 200
    own = {"Origin": "http://127.0.0.1:8107"}
    assert c.get("/api/health", headers=own).status_code == 200
    local = {"Host": "localhost:5173", "Origin": "http://localhost:5173"}
    assert c.get("/api/health", headers=local).status_code == 200


def test_other_sites_are_refused(tmp_path):
    c = client(tmp_path)
    for origin in ("https://example.com", "http://127.0.0.1:9999", "null"):
        r = c.post("/api/downloads/ETCBC/bhsa", headers={"Origin": origin})
        assert r.status_code == 403, origin
        assert "other sites" in r.json()["detail"]


def test_foreign_host_names_are_refused_when_local(tmp_path):
    c = client(tmp_path, hosts=list(LOCAL_HOSTS))
    r = c.get("/api/health", headers={"Host": "evil.example:8107"})
    assert r.status_code == 403
    assert c.get("/api/health", headers={"Host": "[::1]:8107"}).status_code == 200
    # reachable from other machines: any name
    assert (
        client(tmp_path)
        .get("/api/health", headers={"Host": "192.168.1.5:8107"})
        .status_code
        == 200
    )
