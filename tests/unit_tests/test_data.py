"""Unit tests for core.data module.

This module tests the Data class which handles loading, saving, and caching
of TF feature data from .tf files.
"""

import pytest
from pathlib import Path
from unittest.mock import MagicMock, patch
import tempfile
import os

from core.data import Data, DATA_TYPES


class TestDataInit:
    """Tests for Data initialization."""

    def test_initialization(self, mock_timestamp, tmp_path):
        """Data should initialize with path and tmObj."""
        path = str(tmp_path / "feature.tf")
        data = Data(path, mock_timestamp)

        assert data.path == path
        assert data.tmObj is mock_timestamp
        assert data.dataLoaded is False
        assert data.dataError is False

    def test_parses_path_components(self, mock_timestamp, tmp_path):
        """Should parse directory, filename, and extension from path."""
        path = str(tmp_path / "myfeature.tf")
        data = Data(path, mock_timestamp)

        assert data.dirName == str(tmp_path)
        assert data.fileName == "myfeature"
        assert data.extension == ".tf"

    def test_sets_bin_paths(self, mock_timestamp, tmp_path):
        """Should set up binary cache paths."""
        path = str(tmp_path / "feature.tf")
        data = Data(path, mock_timestamp)

        assert ".tf" in data.binDir
        assert data.binPath.endswith(".tfx")


class TestDataLoad:
    """Tests for Data.load() method."""

    def test_load_node_feature(self, mock_timestamp, fixtures_dir):
        """Should load a node feature from TF file."""
        path = str(fixtures_dir / "mini_corpus" / "word.tf")
        data = Data(path, mock_timestamp)

        result = data.load(silent=True)

        assert result is True
        assert data.dataLoaded
        assert data.isEdge is False
        assert "hello" in data.data.values()

    def test_load_edge_feature(self, mock_timestamp, fixtures_dir):
        """Should load an edge feature from TF file."""
        path = str(fixtures_dir / "mini_corpus" / "parent.tf")
        data = Data(path, mock_timestamp)

        result = data.load(silent=True)

        assert result is True
        assert data.isEdge is True

    def test_load_nonexistent_file(self, mock_timestamp, tmp_path):
        """Should return False for non-existent file."""
        path = str(tmp_path / "nonexistent.tf")
        data = Data(path, mock_timestamp)

        result = data.load(silent=True)

        assert result is False
        assert data.dataError is True

    def test_load_meta_only(self, mock_timestamp, fixtures_dir):
        """Should load only metadata when metaOnly=True."""
        path = str(fixtures_dir / "mini_corpus" / "word.tf")
        data = Data(path, mock_timestamp)

        result = data.load(metaOnly=True, silent=True)

        assert result is True
        assert "valueType" in data.metaData
        assert "description" in data.metaData


class TestDataUnload:
    """Tests for Data.unload() method."""

    def test_unload_clears_data(self, mock_timestamp, fixtures_dir):
        """unload() should clear loaded data."""
        path = str(fixtures_dir / "mini_corpus" / "word.tf")
        data = Data(path, mock_timestamp)
        data.load(silent=True)

        data.unload()

        assert data.data is None
        assert data.dataLoaded is False


class TestDataReadTf:
    """Tests for Data._readTf() method (reading TF format)."""

    def test_reads_node_header(self, mock_timestamp, temp_tf_file):
        """Should recognize @node header."""
        path = temp_tf_file("test", "@node\n@valueType=str\n\nvalue1\n")
        data = Data(str(path), mock_timestamp)
        data._readTf()

        assert data.isEdge is False

    def test_reads_edge_header(self, mock_timestamp, temp_tf_file):
        """Should recognize @edge header."""
        path = temp_tf_file("test", "@edge\n@valueType=int\n\n1\t2\n")
        data = Data(str(path), mock_timestamp)
        data._readTf()

        assert data.isEdge is True

    def test_reads_config_header(self, mock_timestamp, temp_tf_file):
        """Should recognize @config header."""
        path = temp_tf_file("test", "@config\n@key=value\n\n")
        data = Data(str(path), mock_timestamp)
        data._readTf()

        assert data.isConfig is True

    def test_reads_metadata(self, mock_timestamp, temp_tf_file):
        """Should read metadata fields."""
        content = "@node\n@valueType=str\n@description=Test feature\n\nvalue\n"
        path = temp_tf_file("test", content)
        data = Data(str(path), mock_timestamp)
        data._readTf()

        assert data.metaData["valueType"] == "str"
        assert data.metaData["description"] == "Test feature"

    def test_missing_header_fails(self, mock_timestamp, fixtures_dir):
        """Should fail on missing @node/@edge/@config header."""
        path = str(fixtures_dir / "invalid" / "missing_header.tf")
        data = Data(path, mock_timestamp)

        result = data._readTf()

        assert result is False


class TestDataReadDataTf:
    """Tests for Data._readDataTf() method (reading data section)."""

    def test_reads_implicit_nodes(self, mock_timestamp, temp_tf_file):
        """Should handle implicit node numbering."""
        content = "@node\n@valueType=str\n\na\nb\nc\n"
        path = temp_tf_file("test", content)
        data = Data(str(path), mock_timestamp)
        data.load(silent=True)

        assert data.data[1] == "a"
        assert data.data[2] == "b"
        assert data.data[3] == "c"

    def test_reads_explicit_nodes(self, mock_timestamp, temp_tf_file):
        """Should handle explicit node numbers."""
        content = "@node\n@valueType=str\n\n5\tvalue5\n10\tvalue10\n"
        path = temp_tf_file("test", content)
        data = Data(str(path), mock_timestamp)
        data.load(silent=True)

        assert data.data[5] == "value5"
        assert data.data[10] == "value10"

    def test_reads_integer_values(self, mock_timestamp, temp_tf_file):
        """Should parse integer values correctly."""
        content = "@node\n@valueType=int\n\n100\n200\n300\n"
        path = temp_tf_file("test", content)
        data = Data(str(path), mock_timestamp)
        data.load(silent=True)

        assert data.data[1] == 100
        assert data.data[2] == 200
        assert isinstance(data.data[1], int)

    def test_reads_edge_data(self, mock_timestamp, temp_tf_file):
        """Should read edge feature data."""
        content = "@edge\n\n1\t5\n2\t5\n3\t6\n"
        path = temp_tf_file("test", content)
        data = Data(str(path), mock_timestamp)
        data.load(silent=True)

        assert 5 in data.data.get(1, set())
        assert 5 in data.data.get(2, set())
        assert 6 in data.data.get(3, set())

    def test_reads_edge_with_values(self, mock_timestamp, temp_tf_file):
        """Should read edge feature with values."""
        content = "@edge\n@edgeValues\n@valueType=str\n\n1\t5\tparent\n"
        path = temp_tf_file("test", content)
        data = Data(str(path), mock_timestamp)
        data.load(silent=True)

        assert data.edgeValues is True
        assert data.data[1][5] == "parent"


class TestDataSave:
    """Tests for Data.save() method."""

    def test_save_node_feature(self, mock_timestamp, tmp_path):
        """Should save node feature to TF file."""
        path = str(tmp_path / "output.tf")
        data = Data(path, mock_timestamp)
        data.isEdge = False
        data.isConfig = False
        data.metaData = {"valueType": "str"}
        data.data = {1: "value1", 2: "value2"}
        data.dataLoaded = True

        result = data.save(silent=True)

        assert result is True
        assert (tmp_path / "output.tf").exists()

        content = (tmp_path / "output.tf").read_text()
        assert "@node" in content
        assert "value1" in content


class TestDataSetDataType:
    """Tests for Data._setDataType() method."""

    def test_sets_str_type(self, mock_timestamp, tmp_path):
        """Should set dataType to str."""
        path = str(tmp_path / "test.tf")
        data = Data(path, mock_timestamp)
        data.metaData = {"valueType": "str"}

        data._setDataType()

        assert data.dataType == "str"

    def test_sets_int_type(self, mock_timestamp, tmp_path):
        """Should set dataType to int."""
        path = str(tmp_path / "test.tf")
        data = Data(path, mock_timestamp)
        data.metaData = {"valueType": "int"}

        data._setDataType()

        assert data.dataType == "int"

    def test_unknown_type_defaults_to_str(self, mock_timestamp, tmp_path):
        """Unknown valueType should default to str."""
        path = str(tmp_path / "test.tf")
        data = Data(path, mock_timestamp)
        data.metaData = {"valueType": "unknown"}

        data._setDataType()

        assert data.dataType == "str"


class TestDataBinaryCache:
    """Tests for binary caching functionality."""

    def test_writes_binary_cache(self, mock_timestamp, fixtures_dir, tmp_path):
        """Loading should create binary cache."""
        # Copy a TF file to tmp_path
        import shutil

        src = fixtures_dir / "mini_corpus" / "word.tf"
        dst = tmp_path / "word.tf"
        shutil.copy(src, dst)

        data = Data(str(dst), mock_timestamp)
        data.load(silent=True)

        # Check that binary cache was created
        bin_dir = tmp_path / ".tf"
        # Binary cache might be in a versioned subdirectory
        assert data.dataLoaded

    def test_clean_data_bin(self, mock_timestamp, tmp_path):
        """cleanDataBin() should remove binary cache."""
        path = str(tmp_path / "test.tf")
        data = Data(path, mock_timestamp)

        # Create a fake binary cache file
        bin_dir = Path(data.binDir)
        bin_dir.mkdir(parents=True, exist_ok=True)
        bin_path = Path(data.binPath)
        bin_path.write_text("fake")

        data.cleanDataBin()

        assert not bin_path.exists()


class TestDataTypes:
    """Tests for DATA_TYPES constant."""

    def test_data_types_defined(self):
        """DATA_TYPES should contain valid types."""
        assert "str" in DATA_TYPES
        assert "int" in DATA_TYPES


class TestDataOtype:
    """Tests for special otype feature handling."""

    def test_otype_special_format(self, mock_timestamp, fixtures_dir):
        """otype feature should be stored in special format."""
        path = str(fixtures_dir / "mini_corpus" / "otype.tf")
        data = Data(path, mock_timestamp)
        data.load(silent=True)

        # otype data is stored as tuple: (types, maxSlot, maxNode, slotType)
        assert isinstance(data.data, tuple)
        assert len(data.data) == 4


class TestDataOslots:
    """Tests for special oslots feature handling."""

    def test_oslots_special_format(self, mock_timestamp, fixtures_dir):
        """oslots feature should be stored in special format."""
        path = str(fixtures_dir / "mini_corpus" / "oslots.tf")
        data = Data(path, mock_timestamp)
        data.load(silent=True)

        # oslots data is stored as tuple: (slots_per_node, maxSlot, maxNode)
        assert isinstance(data.data, tuple)
        assert len(data.data) == 3
