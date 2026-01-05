"""Unit tests for core.timestamp module.

This module tests the Timestamp class which provides timed logging
with various verbosity levels and indentation.
"""

import pytest
import time
from io import StringIO
import sys

from cfabric.utils.timestamp import (
    Timestamp,
    silentConvert,
    VERBOSE,
    AUTO,
    TERSE,
    DEEP,
    SILENT_D,
)


class TestSilentConvert:
    """Tests for silentConvert() function."""

    def test_none_returns_default(self):
        """None should return default silent level."""
        assert silentConvert(None) == SILENT_D

    def test_false_returns_verbose(self):
        """False should return VERBOSE."""
        assert silentConvert(False) == VERBOSE

    def test_true_returns_deep(self):
        """True should return DEEP."""
        assert silentConvert(True) == DEEP

    def test_valid_strings(self):
        """Valid string levels should be returned as-is."""
        assert silentConvert(VERBOSE) == VERBOSE
        assert silentConvert(AUTO) == AUTO
        assert silentConvert(TERSE) == TERSE
        assert silentConvert(DEEP) == DEEP

    def test_invalid_string(self):
        """Invalid strings should return default silent level."""
        assert silentConvert("invalid") == SILENT_D


class TestTimestampInit:
    """Tests for Timestamp initialization."""

    def test_default_initialization(self):
        """Timestamp should initialize with default settings."""
        ts = Timestamp()

        assert ts.level == 0
        assert ts.silent == SILENT_D
        assert ts.log == []

    def test_custom_silent_level(self):
        """Should accept custom silent level."""
        ts = Timestamp(silent=DEEP)
        assert ts.silent == DEEP

    def test_custom_level(self):
        """Should accept custom indentation level."""
        ts = Timestamp(level=2)
        assert ts.level == 2


class TestTimestampMessages:
    """Tests for Timestamp message methods."""

    def test_info_verbose(self, capsys):
        """info() should output in verbose mode."""
        ts = Timestamp(silent=VERBOSE)
        ts.info("test message", tm=False)

        captured = capsys.readouterr()
        assert "test message" in captured.out

    def test_info_deep_silent(self, capsys):
        """info() should be silent in deep mode."""
        ts = Timestamp(silent=DEEP)
        ts.info("test message", tm=False)

        captured = capsys.readouterr()
        assert captured.out == ""

    def test_error_always_outputs(self, capsys):
        """error() should always output regardless of silent mode."""
        ts = Timestamp(silent=DEEP)
        ts.error("error message", tm=False)

        captured = capsys.readouterr()
        assert "error message" in captured.err

    def test_warning_verbose(self, capsys):
        """warning() should output in verbose mode."""
        ts = Timestamp(silent=VERBOSE)
        ts.warning("warning message", tm=False)

        captured = capsys.readouterr()
        assert "warning message" in captured.out

    def test_warning_deep_silent(self, capsys):
        """warning() should be silent in deep mode."""
        ts = Timestamp(silent=DEEP)
        ts.warning("warning message", tm=False)

        captured = capsys.readouterr()
        assert captured.out == ""


class TestTimestampIndent:
    """Tests for Timestamp.indent() method."""

    def test_set_level(self):
        """indent() should set indentation level."""
        ts = Timestamp()
        ts.indent(level=3)
        assert ts.level == 3

    def test_increment_level(self):
        """indent(level=True) should increment level."""
        ts = Timestamp()
        ts.indent(level=1)
        ts.indent(level=True)
        assert ts.level == 2

    def test_decrement_level(self):
        """indent(level=False) should decrement level."""
        ts = Timestamp()
        ts.indent(level=2)
        ts.indent(level=False)
        assert ts.level == 1

    def test_level_not_negative(self):
        """Level should not go below 0."""
        ts = Timestamp()
        ts.indent(level=False)
        ts.indent(level=False)
        assert ts.level == 0

    def test_reset_timer(self):
        """indent(reset=True) should reset the timer."""
        ts = Timestamp()
        ts.indent(level=1, reset=True)
        assert 1 in ts.timestamp


class TestTimestampSilentMethods:
    """Tests for Timestamp silent mode methods."""

    def test_isSilent(self):
        """isSilent() should return current silent level."""
        ts = Timestamp(silent=VERBOSE)
        assert ts.isSilent() == VERBOSE

    def test_setSilent(self):
        """setSilent() should change silent level."""
        ts = Timestamp(silent=VERBOSE)
        ts.setSilent(DEEP)
        assert ts.silent == DEEP

    def test_silentOn(self):
        """silentOn() should suppress info messages."""
        ts = Timestamp(silent=VERBOSE)
        ts.silentOn()
        assert ts.silent == TERSE

    def test_silentOn_deep(self):
        """silentOn(deep=True) should suppress warnings too."""
        ts = Timestamp(silent=VERBOSE)
        ts.silentOn(deep=True)
        assert ts.silent == DEEP

    def test_silentOff(self):
        """silentOff() should restore previous silent level."""
        ts = Timestamp(silent=VERBOSE)
        ts.silentOn()
        ts.silentOff()
        assert ts.silent == VERBOSE


class TestTimestampCache:
    """Tests for Timestamp caching functionality."""

    def test_cache_stores_messages(self):
        """Messages with cache=1 should be stored in log."""
        ts = Timestamp(silent=VERBOSE)
        ts.info("cached message", tm=False, cache=1)

        assert len(ts.log) == 1
        assert "cached message" in ts.log[0][2]

    def test_reset_clears_cache(self):
        """reset() should clear the log."""
        ts = Timestamp(silent=VERBOSE)
        ts.info("message", tm=False, cache=1)
        ts.reset()

        assert ts.log == []

    def test_cache_output(self, capsys):
        """cache() should output cached messages."""
        ts = Timestamp(silent=VERBOSE)
        ts.info("cached", tm=False, cache=1)
        ts.reset()  # Clear stdout from info
        capsys.readouterr()  # Clear captured output

        ts.log = [(False, True, "cached message")]
        ts.cache()

        captured = capsys.readouterr()
        assert "cached message" in captured.out

    def test_cache_as_string(self):
        """cache(_asString=True) should return string."""
        ts = Timestamp(silent=VERBOSE)
        ts.log = [(False, True, "test message")]

        result = ts.cache(_asString=True)
        assert "test message" in result


class TestTimestampTiming:
    """Tests for Timestamp elapsed time functionality."""

    def test_elapsed_format_seconds(self):
        """Elapsed time should format correctly for seconds."""
        ts = Timestamp()
        ts.indent(reset=True)
        time.sleep(0.1)

        elapsed = ts._elapsed()
        # Should be formatted as seconds
        assert "s" in elapsed

    def test_elapsed_resets_per_level(self):
        """Each level should have independent timing."""
        ts = Timestamp()
        ts.indent(level=0, reset=True)
        time.sleep(0.05)
        ts.indent(level=1, reset=True)

        # Level 1 timer should be fresher than level 0
        assert ts.timestamp[0] < ts.timestamp[1]


class TestTimestampNonStringMessages:
    """Tests for handling non-string messages."""

    def test_repr_of_non_string(self, capsys):
        """Non-string messages should be repr'd."""
        ts = Timestamp(silent=VERBOSE)
        ts.info({"key": "value"}, tm=False)

        captured = capsys.readouterr()
        assert "key" in captured.out or "value" in captured.out


class TestTimestampMultiline:
    """Tests for multiline message handling."""

    def test_multiline_indentation(self, capsys):
        """Multiline messages should have indent on each line."""
        ts = Timestamp(silent=VERBOSE)
        ts.indent(level=1)
        ts.info("line1\nline2", tm=False)

        captured = capsys.readouterr()
        # Both lines should appear
        assert "line1" in captured.out
        assert "line2" in captured.out


class TestTimestampDebug:
    """Tests for debug() method."""

    def test_debug_verbose_only(self, capsys):
        """debug() should only output in verbose mode."""
        ts = Timestamp(silent=VERBOSE)
        ts.debug("debug message", tm=False)

        captured = capsys.readouterr()
        assert "debug message" in captured.out

    def test_debug_silent_in_auto(self, capsys):
        """debug() should be silent in auto mode."""
        ts = Timestamp(silent=AUTO)
        ts.debug("debug message", tm=False)

        captured = capsys.readouterr()
        assert captured.out == ""

    def test_debug_force(self, capsys):
        """debug(force=True) should output regardless of mode."""
        ts = Timestamp(silent=DEEP)
        ts.debug("forced debug", tm=False, force=True)

        captured = capsys.readouterr()
        assert "forced debug" in captured.out
