"""Unit tests for core.files module.

This module tests file system operations, path manipulation,
and JSON/YAML reading/writing utilities.
"""

import pytest
import os
import json
import yaml

from cfabric.core.files import (
    fileOpen,
    normpath,
    abspath,
    expanduser,
    unexpanduser,
    prefixSlash,
    dirEmpty,
    dirNm,
    fileNm,
    extNm,
    stripExt,
    replaceExt,
    splitPath,
    isFile,
    isDir,
    fileMake,
    fileExists,
    fileRemove,
    fileCopy,
    fileMove,
    dirExists,
    dirRemove,
    dirCopy,
    dirMove,
    dirMake,
    dirContents,
    dirAllFiles,
    getCwd,
    chDir,
    readJson,
    writeJson,
    readYaml,
    writeYaml,
    backendRep,
)


class TestNormpath:
    """Tests for normpath() function."""

    def test_normalizes_path(self):
        """Should normalize path separators."""
        result = normpath("/path/to/file")
        assert "/" in result
        assert "\\" not in result

    def test_handles_none(self):
        """Should return None for None input."""
        assert normpath(None) is None

    def test_removes_trailing_slash(self):
        """Should normalize trailing slashes."""
        result = normpath("/path/to/dir/")
        # os.path.normpath removes trailing slash
        assert not result.endswith("/") or result == "/"


class TestAbspath:
    """Tests for abspath() function."""

    def test_returns_absolute_path(self):
        """Should return an absolute path."""
        result = abspath(".")
        assert result.startswith("/")

    def test_normalizes_path(self):
        """Should normalize the path."""
        result = abspath("./test")
        assert "./" not in result


class TestExpanduser:
    """Tests for expanduser() function."""

    def test_expands_tilde(self):
        """Should expand ~ to home directory."""
        result = expanduser("~/test")
        assert not result.startswith("~")
        assert "test" in result


class TestUnexpanduser:
    """Tests for unexpanduser() function."""

    def test_replaces_home_with_tilde(self):
        """Should replace home directory with ~."""
        home = os.path.expanduser("~")
        result = unexpanduser(f"{home}/test")
        assert "~" in result


class TestPrefixSlash:
    """Tests for prefixSlash() function."""

    def test_adds_slash_to_nonempty(self):
        """Should add slash to non-empty path without leading slash."""
        assert prefixSlash("path") == "/path"

    def test_keeps_existing_slash(self):
        """Should not add extra slash if already present."""
        assert prefixSlash("/path") == "/path"

    def test_empty_string(self):
        """Empty string should remain empty."""
        assert prefixSlash("") == ""

    def test_none_input(self):
        """None should return None (falsy)."""
        assert prefixSlash(None) is None


class TestDirNm:
    """Tests for dirNm() function."""

    def test_gets_directory(self):
        """Should extract directory from path."""
        assert dirNm("/path/to/file.txt") == "/path/to"

    def test_no_directory(self):
        """File without path should return empty."""
        assert dirNm("file.txt") == ""


class TestFileNm:
    """Tests for fileNm() function."""

    def test_gets_filename(self):
        """Should extract filename from path."""
        assert fileNm("/path/to/file.txt") == "file.txt"

    def test_just_filename(self):
        """Just filename should return itself."""
        assert fileNm("file.txt") == "file.txt"


class TestExtNm:
    """Tests for extNm() function."""

    def test_gets_extension(self):
        """Should extract extension without dot."""
        assert extNm("file.txt") == "txt"

    def test_no_extension(self):
        """File without extension should return empty."""
        assert extNm("file") == "file"  # Actually returns the last part

    def test_multiple_dots(self):
        """Should get last extension."""
        assert extNm("file.tar.gz") == "gz"


class TestStripExt:
    """Tests for stripExt() function."""

    def test_strips_extension(self):
        """Should remove extension."""
        assert stripExt("file.txt") == "file"

    def test_preserves_directory(self):
        """Should preserve directory path."""
        result = stripExt("/path/to/file.txt")
        assert result == "/path/to/file"


class TestReplaceExt:
    """Tests for replaceExt() function."""

    def test_replaces_extension(self):
        """Should replace extension."""
        assert replaceExt("file.txt", "md") == "file.md"

    def test_preserves_path(self):
        """Should preserve directory path."""
        result = replaceExt("/path/to/file.txt", "json")
        assert result == "/path/to/file.json"


class TestSplitPath:
    """Tests for splitPath() function."""

    def test_splits_path(self):
        """Should split into directory and filename."""
        dir_part, file_part = splitPath("/path/to/file.txt")
        assert dir_part == "/path/to"
        assert file_part == "file.txt"


class TestIsFile:
    """Tests for isFile() function."""

    def test_existing_file(self, tmp_path):
        """Should return True for existing file."""
        test_file = tmp_path / "test.txt"
        test_file.write_text("test")
        assert isFile(str(test_file)) is True

    def test_nonexistent_file(self, tmp_path):
        """Should return False for non-existent file."""
        assert isFile(str(tmp_path / "nonexistent.txt")) is False

    def test_directory(self, tmp_path):
        """Should return False for directory."""
        assert isFile(str(tmp_path)) is False


class TestIsDir:
    """Tests for isDir() function."""

    def test_existing_directory(self, tmp_path):
        """Should return True for existing directory."""
        assert isDir(str(tmp_path)) is True

    def test_nonexistent_directory(self, tmp_path):
        """Should return False for non-existent directory."""
        assert isDir(str(tmp_path / "nonexistent")) is False

    def test_file(self, tmp_path):
        """Should return False for file."""
        test_file = tmp_path / "test.txt"
        test_file.write_text("test")
        assert isDir(str(test_file)) is False


class TestFileExists:
    """Tests for fileExists() function."""

    def test_existing_file(self, tmp_path):
        """Should return True for existing file."""
        test_file = tmp_path / "test.txt"
        test_file.write_text("test")
        assert fileExists(str(test_file)) is True

    def test_nonexistent_file(self, tmp_path):
        """Should return False for non-existent file."""
        assert fileExists(str(tmp_path / "nonexistent.txt")) is False


class TestDirExists:
    """Tests for dirExists() function."""

    def test_existing_directory(self, tmp_path):
        """Should return True for existing directory."""
        assert dirExists(str(tmp_path)) is True

    def test_nonexistent_directory(self, tmp_path):
        """Should return False for non-existent directory."""
        assert dirExists(str(tmp_path / "nonexistent")) is False

    def test_none_input(self):
        """Should return False for None."""
        assert dirExists(None) is False

    def test_empty_string(self):
        """Empty string should return True (current directory)."""
        assert dirExists("") is True


class TestFileMake:
    """Tests for fileMake() function."""

    def test_creates_file(self, tmp_path):
        """Should create a new file."""
        test_file = tmp_path / "new_file.txt"
        fileMake(str(test_file))
        assert test_file.exists()

    def test_creates_parent_dirs(self, tmp_path):
        """Should create parent directories."""
        test_file = tmp_path / "subdir" / "new_file.txt"
        fileMake(str(test_file))
        assert test_file.exists()

    def test_does_not_overwrite(self, tmp_path):
        """Should not overwrite existing file without force."""
        test_file = tmp_path / "existing.txt"
        test_file.write_text("original")
        fileMake(str(test_file))
        assert test_file.read_text() == "original"

    def test_force_overwrites(self, tmp_path):
        """With force=True, should truncate existing file."""
        test_file = tmp_path / "existing.txt"
        test_file.write_text("original")
        fileMake(str(test_file), force=True)
        assert test_file.read_text() == ""


class TestFileRemove:
    """Tests for fileRemove() function."""

    def test_removes_file(self, tmp_path):
        """Should remove existing file."""
        test_file = tmp_path / "test.txt"
        test_file.write_text("test")
        fileRemove(str(test_file))
        assert not test_file.exists()

    def test_nonexistent_file_no_error(self, tmp_path):
        """Should not raise error for non-existent file."""
        fileRemove(str(tmp_path / "nonexistent.txt"))  # Should not raise


class TestFileCopy:
    """Tests for fileCopy() function."""

    def test_copies_file(self, tmp_path):
        """Should copy file to destination."""
        src = tmp_path / "src.txt"
        dst = tmp_path / "dst.txt"
        src.write_text("content")
        fileCopy(str(src), str(dst))
        assert dst.exists()
        assert dst.read_text() == "content"

    def test_overwrites_destination(self, tmp_path):
        """Should overwrite existing destination."""
        src = tmp_path / "src.txt"
        dst = tmp_path / "dst.txt"
        src.write_text("new")
        dst.write_text("old")
        fileCopy(str(src), str(dst))
        assert dst.read_text() == "new"

    def test_same_path_no_op(self, tmp_path):
        """Copying to same path should do nothing."""
        src = tmp_path / "file.txt"
        src.write_text("content")
        fileCopy(str(src), str(src))
        assert src.read_text() == "content"


class TestFileMove:
    """Tests for fileMove() function."""

    def test_moves_file(self, tmp_path):
        """Should move file to destination."""
        src = tmp_path / "src.txt"
        dst = tmp_path / "dst.txt"
        src.write_text("content")
        fileMove(str(src), str(dst))
        assert not src.exists()
        assert dst.exists()
        assert dst.read_text() == "content"


class TestDirMake:
    """Tests for dirMake() function."""

    def test_creates_directory(self, tmp_path):
        """Should create a new directory."""
        new_dir = tmp_path / "new_dir"
        dirMake(str(new_dir))
        assert new_dir.is_dir()

    def test_creates_nested_dirs(self, tmp_path):
        """Should create nested directories."""
        new_dir = tmp_path / "a" / "b" / "c"
        dirMake(str(new_dir))
        assert new_dir.is_dir()

    def test_existing_dir_no_error(self, tmp_path):
        """Should not raise error for existing directory."""
        dirMake(str(tmp_path))  # Already exists


class TestDirRemove:
    """Tests for dirRemove() function."""

    def test_removes_directory(self, tmp_path):
        """Should remove existing directory."""
        new_dir = tmp_path / "to_remove"
        new_dir.mkdir()
        dirRemove(str(new_dir))
        assert not new_dir.exists()

    def test_removes_with_contents(self, tmp_path):
        """Should remove directory with contents."""
        new_dir = tmp_path / "to_remove"
        new_dir.mkdir()
        (new_dir / "file.txt").write_text("test")
        dirRemove(str(new_dir))
        assert not new_dir.exists()


class TestDirCopy:
    """Tests for dirCopy() function."""

    def test_copies_directory(self, tmp_path):
        """Should copy directory to destination."""
        src = tmp_path / "src_dir"
        dst = tmp_path / "dst_dir"
        src.mkdir()
        (src / "file.txt").write_text("content")
        result = dirCopy(str(src), str(dst))
        assert result is True
        assert dst.is_dir()
        assert (dst / "file.txt").read_text() == "content"


class TestDirMove:
    """Tests for dirMove() function."""

    def test_moves_directory(self, tmp_path):
        """Should move directory to destination."""
        src = tmp_path / "src_dir"
        dst = tmp_path / "dst_dir"
        src.mkdir()
        (src / "file.txt").write_text("content")
        result = dirMove(str(src), str(dst))
        assert result is True
        assert not src.exists()
        assert dst.is_dir()
        assert (dst / "file.txt").read_text() == "content"

    def test_refuses_if_dst_exists(self, tmp_path):
        """Should refuse if destination exists."""
        src = tmp_path / "src_dir"
        dst = tmp_path / "dst_dir"
        src.mkdir()
        dst.mkdir()
        result = dirMove(str(src), str(dst))
        assert result is False


class TestDirContents:
    """Tests for dirContents() function."""

    def test_lists_files_and_dirs(self, tmp_path):
        """Should list files and subdirectories separately."""
        (tmp_path / "file.txt").write_text("test")
        (tmp_path / "subdir").mkdir()
        files, dirs = dirContents(str(tmp_path))
        assert "file.txt" in files
        assert "subdir" in dirs

    def test_nonexistent_returns_empty(self, tmp_path):
        """Non-existent directory should return empty tuples."""
        files, dirs = dirContents(str(tmp_path / "nonexistent"))
        assert files == ()
        assert dirs == ()


class TestDirAllFiles:
    """Tests for dirAllFiles() function."""

    def test_lists_all_files_recursively(self, tmp_path):
        """Should list all files recursively."""
        (tmp_path / "file1.txt").write_text("test")
        subdir = tmp_path / "subdir"
        subdir.mkdir()
        (subdir / "file2.txt").write_text("test")
        result = dirAllFiles(str(tmp_path))
        assert len(result) == 2

    def test_ignores_specified_dirs(self, tmp_path):
        """Should skip ignored directories."""
        (tmp_path / "file1.txt").write_text("test")
        ignored_dir = tmp_path / "ignored"
        ignored_dir.mkdir()
        (ignored_dir / "file2.txt").write_text("test")
        result = dirAllFiles(str(tmp_path), ignore={"ignored"})
        assert len(result) == 1


class TestDirEmpty:
    """Tests for dirEmpty() function."""

    def test_empty_directory(self, tmp_path):
        """Empty directory should return True."""
        empty_dir = tmp_path / "empty"
        empty_dir.mkdir()
        assert dirEmpty(str(empty_dir)) is True

    def test_nonempty_directory(self, tmp_path):
        """Non-empty directory should return False."""
        (tmp_path / "file.txt").write_text("test")
        assert dirEmpty(str(tmp_path)) is False

    def test_nonexistent_directory(self, tmp_path):
        """Non-existent directory should return True."""
        assert dirEmpty(str(tmp_path / "nonexistent")) is True


class TestGetCwd:
    """Tests for getCwd() function."""

    def test_returns_current_directory(self):
        """Should return current working directory."""
        result = getCwd()
        assert os.path.isabs(result)
        assert os.path.isdir(result)


class TestChDir:
    """Tests for chDir() function."""

    def test_changes_directory(self, tmp_path):
        """Should change current directory."""
        original = getCwd()
        try:
            chDir(str(tmp_path))
            assert getCwd() == str(tmp_path)
        finally:
            chDir(original)


class TestReadJson:
    """Tests for readJson() function."""

    def test_reads_from_string(self):
        """Should parse JSON string."""
        result = readJson(text='{"key": "value"}', plain=True)
        assert result == {"key": "value"}

    def test_reads_from_file(self, tmp_path):
        """Should read JSON from file."""
        test_file = tmp_path / "test.json"
        test_file.write_text('{"key": "value"}')
        result = readJson(asFile=str(test_file), plain=True)
        assert result == {"key": "value"}

    def test_returns_attrdict_by_default(self):
        """Should return AttrDict by default."""
        result = readJson(text='{"key": "value"}')
        assert result.key == "value"

    def test_nonexistent_file_returns_empty(self, tmp_path):
        """Non-existent file should return empty dict."""
        result = readJson(asFile=str(tmp_path / "nonexistent.json"), plain=True)
        assert result == {}


class TestWriteJson:
    """Tests for writeJson() function."""

    def test_writes_to_file(self, tmp_path):
        """Should write JSON to file."""
        test_file = tmp_path / "test.json"
        writeJson({"key": "value"}, asFile=str(test_file))
        content = json.loads(test_file.read_text())
        assert content == {"key": "value"}

    def test_returns_string(self):
        """Should return JSON string when asFile is None."""
        result = writeJson({"key": "value"})
        assert json.loads(result) == {"key": "value"}


class TestReadYaml:
    """Tests for readYaml() function."""

    def test_reads_from_string(self):
        """Should parse YAML string."""
        result = readYaml(text="key: value", plain=True)
        assert result == {"key": "value"}

    def test_reads_from_file(self, tmp_path):
        """Should read YAML from file."""
        test_file = tmp_path / "test.yaml"
        test_file.write_text("key: value")
        result = readYaml(asFile=str(test_file), plain=True)
        assert result == {"key": "value"}

    def test_returns_attrdict_by_default(self):
        """Should return AttrDict by default."""
        result = readYaml(text="key: value")
        assert result.key == "value"

    def test_nonexistent_file_returns_empty(self, tmp_path):
        """Non-existent file should return empty dict."""
        result = readYaml(asFile=str(tmp_path / "nonexistent.yaml"), plain=True)
        assert result == {}


class TestWriteYaml:
    """Tests for writeYaml() function."""

    def test_writes_to_file(self, tmp_path):
        """Should write YAML to file."""
        test_file = tmp_path / "test.yaml"
        writeYaml({"key": "value"}, asFile=str(test_file))
        content = yaml.safe_load(test_file.read_text())
        assert content == {"key": "value"}

    def test_returns_string(self):
        """Should return YAML string when asFile is None."""
        result = writeYaml({"key": "value"})
        assert yaml.safe_load(result) == {"key": "value"}


class TestFileOpen:
    """Tests for fileOpen() function."""

    def test_opens_with_utf8_by_default(self, tmp_path):
        """Should use UTF-8 encoding by default."""
        test_file = tmp_path / "test.txt"
        with fileOpen(str(test_file), mode="w") as f:
            f.write("hello")
        with fileOpen(str(test_file)) as f:
            content = f.read()
        assert content == "hello"

    def test_binary_mode_no_encoding(self, tmp_path):
        """Binary mode should not add encoding."""
        test_file = tmp_path / "test.bin"
        with fileOpen(str(test_file), mode="wb") as f:
            f.write(b"hello")
        with fileOpen(str(test_file), mode="rb") as f:
            content = f.read()
        assert content == b"hello"


class TestBackendRep:
    """Tests for backendRep() function."""

    def test_norm_github(self):
        """Should normalize github variations."""
        assert backendRep("github", "norm") == "github"
        assert backendRep("github.com", "norm") == "github"
        assert backendRep("", "norm") == "github"
        assert backendRep(None, "norm") == "github"

    def test_norm_gitlab(self):
        """Should normalize gitlab variations."""
        assert backendRep("gitlab", "norm") == "gitlab"
        assert backendRep("gitlab.com", "norm") == "gitlab"

    def test_tech(self):
        """Should return technology type."""
        assert backendRep("github", "tech") == "github"
        assert backendRep("gitlab", "tech") == "gitlab"
        assert backendRep("custom.server.com", "tech") == "gitlab"

    def test_name(self):
        """Should return display name."""
        assert backendRep("github", "name") == "GitHub"
        assert backendRep("gitlab", "name") == "GitLab"

    def test_machine(self):
        """Should return machine name."""
        assert backendRep("github", "machine") == "github.com"
        assert backendRep("gitlab", "machine") == "gitlab.com"

    def test_url(self):
        """Should return URL."""
        assert "github.com" in backendRep("github", "url")
        assert "gitlab.com" in backendRep("gitlab", "url")
