"""Unit tests for core.search.search module.

This module tests the Search class that provides the top-level search API.
"""

import pytest
from unittest.mock import MagicMock, patch


class TestSearchInit:
    """Tests for Search class initialization."""

    def test_search_creation(self):
        """Search should initialize with an API object."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        s = Search(mock_api)
        assert s.api is mock_api
        assert s.exe is None

    def test_search_perf_params_initialized(self):
        """Search should initialize performance parameters from defaults."""
        from cfabric.search.search import Search
        from cfabric.search.searchexe import SearchExe

        mock_api = MagicMock()
        s = Search(mock_api)
        assert s.perfParams is not None
        # Should contain defaults from SearchExe
        for key in SearchExe.perfDefaults:
            assert key in s.perfParams

    def test_search_silent_mode(self):
        """Search should accept silent parameter."""
        from cfabric.search.search import Search
        from cfabric.utils.logging import SILENT_D

        mock_api = MagicMock()
        s = Search(mock_api, silent=SILENT_D)
        assert s.silent == SILENT_D


class TestTweakPerformance:
    """Tests for tweakPerformance method."""

    def test_invalid_parameter_name(self):
        """Should report error for invalid parameter name."""
        from cfabric.search.search import Search

        mock_api = MagicMock()

        s = Search(mock_api)

        with patch("cfabric.search.search.logger") as mock_logger:
            s.tweakPerformance(invalidParam=100)
            mock_logger.error.assert_called()

    def test_reset_to_default(self):
        """Passing None should reset parameter to default."""
        from cfabric.search.search import Search
        from cfabric.search.searchexe import SearchExe

        mock_api = MagicMock()

        s = Search(mock_api)
        # Set a non-default value first
        s.perfParams["tryLimitFrom"] = 9999

        # Reset to default
        s.tweakPerformance(tryLimitFrom=None)

        assert s.perfParams["tryLimitFrom"] == SearchExe.perfDefaults["tryLimitFrom"]


class TestSearchMethod:
    """Tests for the search() method."""

    def test_search_stores_exe(self):
        """search() should store the SearchExe in self.exe when here=True."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        mock_api.CF = MagicMock()
        mock_api.TF = mock_api.CF  # Alias

        s = Search(mock_api)

        # Patch SearchExe to avoid full initialization
        with patch("cfabric.search.search.SearchExe") as MockSearchExe:
            mock_exe = MagicMock()
            mock_exe.search.return_value = []
            MockSearchExe.return_value = mock_exe

            s.search("word", here=True)

            assert s.exe is mock_exe

    def test_search_does_not_store_exe_when_here_false(self):
        """search() should not store exe when here=False."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        mock_api.CF = MagicMock()
        mock_api.TF = mock_api.CF  # Alias

        s = Search(mock_api)

        with patch("cfabric.search.search.SearchExe") as MockSearchExe:
            mock_exe = MagicMock()
            mock_exe.search.return_value = []
            MockSearchExe.return_value = mock_exe

            s.search("word", here=False)

            assert s.exe is None


class TestStudyMethod:
    """Tests for the study() method."""

    def test_study_stores_exe(self):
        """study() should store the SearchExe in self.exe when here=True."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        mock_api.CF = MagicMock()
        mock_api.TF = mock_api.CF  # Alias

        s = Search(mock_api)

        with patch("cfabric.search.search.SearchExe") as MockSearchExe:
            mock_exe = MagicMock()
            mock_exe.study.return_value = None
            MockSearchExe.return_value = mock_exe

            s.study("word", here=True)

            assert s.exe is mock_exe


class TestFetchMethod:
    """Tests for the fetch() method."""

    def test_fetch_without_study_reports_error(self):
        """fetch() should report error if no previous study()."""
        from cfabric.search.search import Search

        mock_api = MagicMock()

        s = Search(mock_api)
        s.exe = None

        with patch("cfabric.search.search.logger") as mock_logger:
            s.fetch()
            mock_logger.error.assert_called_once()

    def test_fetch_with_exe_calls_fetch(self):
        """fetch() should call exe.fetch() when exe exists."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        mock_api.CF = MagicMock()
        mock_api.TF = mock_api.CF  # Alias

        s = Search(mock_api)
        mock_exe = MagicMock()
        mock_exe.fetch.return_value = [(1, 2, 3)]
        s.exe = mock_exe

        result = s.fetch(limit=10)

        mock_exe.fetch.assert_called_once_with(limit=10)


class TestCountMethod:
    """Tests for the count() method."""

    def test_count_without_study_reports_error(self):
        """count() should report error if no previous study()."""
        from cfabric.search.search import Search

        mock_api = MagicMock()

        s = Search(mock_api)
        s.exe = None

        with patch("cfabric.search.search.logger") as mock_logger:
            s.count()
            mock_logger.error.assert_called_once()

    def test_count_with_exe_calls_count(self):
        """count() should call exe.count() when exe exists."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        mock_api.CF = MagicMock()
        mock_api.TF = mock_api.CF  # Alias

        s = Search(mock_api)
        mock_exe = MagicMock()
        s.exe = mock_exe

        s.count(progress=50, limit=100)

        mock_exe.count.assert_called_once_with(progress=50, limit=100)


class TestShowPlanMethod:
    """Tests for the showPlan() method."""

    def test_showplan_without_study_reports_error(self):
        """showPlan() should report error if no previous study()."""
        from cfabric.search.search import Search

        mock_api = MagicMock()

        s = Search(mock_api)
        s.exe = None

        with patch("cfabric.search.search.logger") as mock_logger:
            s.showPlan()
            mock_logger.error.assert_called_once()

    def test_showplan_with_exe_calls_showplan(self):
        """showPlan() should call exe.showPlan() when exe exists."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        mock_api.CF = MagicMock()
        mock_api.TF = mock_api.CF  # Alias

        s = Search(mock_api)
        mock_exe = MagicMock()
        s.exe = mock_exe

        s.showPlan(details=True)

        mock_exe.showPlan.assert_called_once_with(details=True)


class TestRelationsLegend:
    """Tests for the relationsLegend() method."""

    def test_relations_legend_creates_exe_if_none(self):
        """relationsLegend() should create an exe if none exists."""
        from cfabric.search.search import Search

        mock_api = MagicMock()

        s = Search(mock_api)
        s.exe = None

        with patch("cfabric.search.search.SearchExe") as MockSearchExe:
            mock_exe = MagicMock()
            mock_exe.relationLegend = "Legend text"
            MockSearchExe.return_value = mock_exe

            with patch("cfabric.search.search.console") as mock_console:
                s.relationsLegend()
                mock_console.assert_called_once_with("Legend text")


class TestGleanMethod:
    """Tests for the glean() method."""

    def test_glean_empty_tuple(self):
        """glean() should return empty string for empty tuple."""
        from cfabric.search.search import Search

        mock_api = MagicMock()
        s = Search(mock_api)

        result = s.glean(())

        assert result == ""

    def test_glean_with_nodes(self):
        """glean() should format tuple of nodes."""
        from cfabric.search.search import Search

        # Set up mock API with required attributes
        mock_api = MagicMock()

        # Mock F.otype
        mock_otype = MagicMock()
        mock_otype.v.return_value = "word"
        mock_otype.slotType = "word"
        mock_otype.maxSlot = 10
        mock_api.F.otype = mock_otype

        # Mock E.oslots
        mock_oslots = MagicMock()
        mock_oslots.data = []
        mock_api.E.oslots = mock_oslots

        # Mock T (text API)
        mock_T = MagicMock()
        mock_T.sectionTypes = ["book", "chapter", "verse"]
        mock_T.text.return_value = "hello"
        mock_api.T = mock_T

        s = Search(mock_api)

        result = s.glean((1,))

        assert result == "hello"
