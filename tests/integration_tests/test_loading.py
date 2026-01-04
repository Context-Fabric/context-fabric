"""Integration tests for FabricCore loading.

Tests the full data loading pipeline with real TF files.
"""

import pytest
from pathlib import Path


class TestFabricCoreLoading:
    """Tests for FabricCore initialization and loading."""

    def test_load_all_features(self, loaded_api):
        """loadAll() should load all features and return API."""
        assert loaded_api is not None
        assert loaded_api is not False

    def test_required_features_loaded(self, loaded_api):
        """Required features otype and oslots should be loaded."""
        # otype is accessed via F
        assert hasattr(loaded_api.F, "otype")
        # oslots is accessed via E
        assert hasattr(loaded_api.E, "oslots")

    def test_optional_features_loaded(self, loaded_api):
        """Optional features from mini_corpus should be loaded."""
        assert hasattr(loaded_api.F, "word")
        assert hasattr(loaded_api.E, "parent")

    def test_api_has_all_components(self, loaded_api):
        """API should have all standard components."""
        assert hasattr(loaded_api, "F")  # Features
        assert hasattr(loaded_api, "E")  # Edges
        assert hasattr(loaded_api, "C")  # Computed
        assert hasattr(loaded_api, "N")  # Nodes
        assert hasattr(loaded_api, "L")  # Locality
        assert hasattr(loaded_api, "S")  # Search


class TestFabricCoreExplore:
    """Tests for FabricCore.explore() method."""

    def test_explore_returns_categories(self, mini_corpus_path):
        """explore() should return dict with feature categories."""
        from core.fabric import FabricCore

        TF = FabricCore(locations=mini_corpus_path, silent="deep")
        result = TF.explore(silent="deep", show=True)

        assert isinstance(result, dict)
        assert "nodes" in result
        assert "edges" in result

    def test_explore_lists_features(self, mini_corpus_path):
        """explore() should list available features."""
        from core.fabric import FabricCore

        TF = FabricCore(locations=mini_corpus_path, silent="deep")
        result = TF.explore(silent="deep", show=True)

        # Should find node features
        assert "word" in result["nodes"]
        assert "otype" in result["nodes"]

        # Should find edge features
        assert "parent" in result["edges"]
        assert "oslots" in result["edges"]


class TestFabricCoreLoadSpecific:
    """Tests for loading specific features."""

    def test_load_specific_features(self, mini_corpus_path):
        """load() should load only specified features."""
        from core.fabric import FabricCore

        TF = FabricCore(locations=mini_corpus_path, silent="deep")
        api = TF.load("word", silent="deep")

        assert api is not False
        assert hasattr(api.F, "word")

    def test_load_with_add(self, mini_corpus_path):
        """load(add=True) should add features to existing API."""
        from core.fabric import FabricCore

        TF = FabricCore(locations=mini_corpus_path, silent="deep")
        api = TF.load("word", silent="deep")
        assert hasattr(api.F, "word")

        # Add another feature - returns True on success, not API
        result = TF.load("pos", add=True, silent="deep")
        assert result is True
        # Original API should now have both features
        assert hasattr(api.F, "word")
        assert hasattr(api.F, "pos")


class TestFabricCoreErrors:
    """Tests for error handling during loading."""

    def test_load_nonexistent_path(self, tmp_path):
        """Loading from non-existent path should handle gracefully."""
        from core.fabric import FabricCore

        nonexistent = str(tmp_path / "nonexistent")
        TF = FabricCore(locations=nonexistent, silent="deep")
        api = TF.loadAll(silent="deep")

        # Should return False when loading fails (no features found)
        # Note: FabricCore may return False or an API with no features
        assert api is False or (api is not None and not hasattr(api.F, "otype"))

    def test_load_empty_directory(self, tmp_path):
        """Loading from empty directory should handle gracefully."""
        from core.fabric import FabricCore

        empty_dir = tmp_path / "empty"
        empty_dir.mkdir()

        TF = FabricCore(locations=str(empty_dir), silent="deep")
        api = TF.loadAll(silent="deep")

        # Should return False when no features found
        assert api is False or (api is not None and not hasattr(api.F, "otype"))
