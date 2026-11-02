"""Integration tests for `cfabric_browser.registry.CorpusRegistry`."""

import threading
from unittest import mock

import pytest
from cfabric_browser.registry import CorpusRegistry


@pytest.fixture
def registry():
    return CorpusRegistry()


class TestRegister:
    def test_register_does_not_load(self, registry, mini_corpus_path):
        name = registry.register(mini_corpus_path)
        assert name == "mini_corpus"
        assert name == "mini_corpus"
        assert registry.list_corpora() == []

    def test_first_registered_becomes_current(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "a")
        registry.register(mini_corpus_path, "b")
        assert registry.current == "a"

    def test_missing_path(self, registry, tmp_path):
        with pytest.raises(FileNotFoundError):
            registry.register(tmp_path / "nope")

    def test_same_name_same_path_is_idempotent(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "c")
        assert registry.register(mini_corpus_path, "c") == "c"

    def test_same_name_other_path_is_refused(
        self, registry, mini_corpus_path, fixtures_dir
    ):
        registry.register(mini_corpus_path, "c")
        with pytest.raises(ValueError):
            registry.register(fixtures_dir / "edge_cases", "c")


class TestLoading:
    def test_get_loads_lazily(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "mini")
        api = registry.get_api("mini")
        assert api.F.otype.v(1) == "word"
        assert registry.list_corpora() == ["mini"]

    def test_get_defaults_to_current(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "mini")
        assert registry.get_api() is registry.get_api("mini")

    def test_no_corpus(self, registry):
        with pytest.raises(RuntimeError):
            registry.get()

    def test_unknown_corpus(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path)
        with pytest.raises(KeyError):
            registry.get("unknown")

    def test_concurrent_get_loads_once(self, registry, mini_corpus_path):
        from cfabric import Fabric

        registry.register(mini_corpus_path, "mini")
        original = Fabric.loadAll
        calls = []

        def counting(self, *args, **kwargs):
            calls.append(1)
            return original(self, *args, **kwargs)

        apis = []
        with mock.patch.object(Fabric, "loadAll", counting):
            threads = [
                threading.Thread(target=lambda: apis.append(registry.get_api("mini")))
                for _ in range(8)
            ]
            for t in threads:
                t.start()
            for t in threads:
                t.join()

        assert len(calls) == 1
        assert len(apis) == 8
        assert all(api is apis[0] for api in apis)
