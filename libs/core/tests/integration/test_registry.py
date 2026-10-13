"""Integration tests for cfabric.registry.CorpusRegistry."""

import threading
from unittest import mock

import pytest

from cfabric.registry import CorpusRegistry


@pytest.fixture
def registry():
    return CorpusRegistry()


class TestRegister:
    def test_register_does_not_load(self, registry, mini_corpus_path):
        name = registry.register(mini_corpus_path)
        assert name == "mini_corpus"
        assert registry.is_registered(name)
        assert not registry.is_loaded(name)
        assert registry.names() == ["mini_corpus"]
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
        assert registry.names() == ["c"]

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
        assert registry.is_loaded("mini")
        assert registry.list_corpora() == ["mini"]

    def test_get_defaults_to_current(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "mini")
        assert registry.get_api() is registry.get_api("mini")

    def test_load_returns_info_and_sets_current(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "first")
        info = registry.load(mini_corpus_path, "second")
        assert info.name == "second"
        assert "word" in {t["type"] for t in info.node_types}
        assert registry.current == "second"

    def test_load_with_feature_subset(self, registry, mini_corpus_path):
        registry.load(mini_corpus_path, "mini", features=["word"])
        api = registry.get_api("mini")
        assert api.F.word.v(1) == "hello"
        assert not hasattr(api.F, "pos")

    def test_no_corpus(self, registry):
        with pytest.raises(RuntimeError):
            registry.get()

    def test_unknown_corpus(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path)
        with pytest.raises(KeyError):
            registry.get("unknown")

    def test_concurrent_get_loads_once(self, registry, mini_corpus_path):
        from cfabric.core.fabric import Fabric

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


class TestBookkeeping:
    def test_set_current(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "a")
        registry.register(mini_corpus_path, "b")
        registry.set_current("b")
        assert registry.current == "b"
        with pytest.raises(KeyError):
            registry.set_current("zzz")

    def test_unload_moves_current(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "a")
        registry.register(mini_corpus_path, "b")
        registry.unload("a")
        assert registry.names() == ["b"]
        assert registry.current == "b"
        registry.unload("b")
        assert registry.current is None
        registry.unload("b")  # unknown names are ignored

    def test_search_locks_are_per_corpus(self, registry, mini_corpus_path):
        registry.register(mini_corpus_path, "a")
        registry.register(mini_corpus_path, "b")
        assert registry.search_lock("a") is registry.search_lock("a")
        assert registry.search_lock("a") is not registry.search_lock("b")

    def test_path(self, registry, mini_corpus_path):
        from pathlib import Path

        registry.register(mini_corpus_path, "a")
        assert registry.path("a") == Path(mini_corpus_path).resolve()
