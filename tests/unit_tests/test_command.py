"""Unit tests for core.command module.

This module tests the readArgs function for CLI argument parsing.
"""

import pytest
from unittest.mock import patch, MagicMock


class TestReadArgsHelp:
    """Tests for help display behavior."""

    def test_no_args_shows_help(self):
        """No arguments should show help and return early."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd"]):
            with patch("cfabric.core.command.console") as mock_console:
                result = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {"param1": ("Param desc", "default")},
                    {"flag1": ("Flag desc", False, 2)},
                )

        # Should return (True, {}, {}, {}) with message shown
        assert result == (True, {}, {}, {})
        mock_console.assert_called()

    def test_help_flag_shows_help(self):
        """--help flag should show help."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "--help"]):
            with patch("cfabric.core.command.console") as mock_console:
                result = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {},
                )

        assert result == (True, {}, {}, {})
        mock_console.assert_called()

    def test_h_flag_shows_help(self):
        """-h flag should show help."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "-h"]):
            with patch("cfabric.core.command.console") as mock_console:
                result = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {},
                )

        assert result == (True, {}, {}, {})


class TestReadArgsTasks:
    """Tests for task argument parsing."""

    def test_single_task(self):
        """Should parse single task."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task", "task2": "Second task"},
                    {},
                    {},
                )

        assert good is True
        assert tasks == {"task1": True}

    def test_multiple_tasks(self):
        """Should parse multiple tasks."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "task2"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task", "task2": "Second task"},
                    {},
                    {},
                )

        assert good is True
        assert tasks == {"task1": True, "task2": True}

    def test_all_task_expands(self):
        """'all' task should expand to all tasks."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "all"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task", "task2": "Second task"},
                    {},
                    {},
                )

        assert good is True
        assert tasks == {"task1": True, "task2": True}

    def test_all_excludes_notinall(self):
        """'all' should exclude tasks in notInAll set."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "all"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task", "task2": "Second task", "special": "Special"},
                    {},
                    {},
                    notInAll={"special"},
                )

        assert good is True
        assert "task1" in tasks
        assert "task2" in tasks
        assert "special" not in tasks


class TestReadArgsParams:
    """Tests for parameter argument parsing."""

    def test_param_with_value(self):
        """Should parse param=value."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "myParam=myValue"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {"myParam": ("Param desc", "default")},
                    {},
                )

        assert good is True
        assert params["myParam"] == "myValue"

    def test_param_empty_value_uses_default(self):
        """param= without value should use default."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "myParam="]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {"myParam": ("Param desc", "defaultValue")},
                    {},
                )

        assert params["myParam"] == "defaultValue"

    def test_unspecified_param_uses_default(self):
        """Unspecified params should get default values."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {"param1": ("Desc1", "def1"), "param2": ("Desc2", "def2")},
                    {},
                )

        assert params["param1"] == "def1"
        assert params["param2"] == "def2"


class TestReadArgsFlags:
    """Tests for flag argument parsing."""

    def test_binary_flag_plus(self):
        """Should parse +flag as True for binary flags."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "+myflag"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {"myflag": ("Flag desc", False, 2)},
                )

        assert good is True
        assert flags["myflag"] is True

    def test_binary_flag_minus(self):
        """Should parse -flag as False for binary flags."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "-myflag"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {"myflag": ("Flag desc", True, 2)},
                )

        assert good is True
        assert flags["myflag"] is False

    def test_ternary_flag_minus(self):
        """Should parse -flag as -1 for ternary flags."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "-myflag"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {"myflag": ("Flag desc", 0, 3)},
                )

        assert flags["myflag"] == -1

    def test_ternary_flag_plus(self):
        """Should parse +flag as 0 for ternary flags."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "+myflag"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {"myflag": ("Flag desc", -1, 3)},
                )

        assert flags["myflag"] == 0

    def test_ternary_flag_plusplus(self):
        """Should parse ++flag as 1 for ternary flags."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "++myflag"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {"myflag": ("Flag desc", 0, 3)},
                )

        assert flags["myflag"] == 1

    def test_unspecified_flag_uses_default(self):
        """Unspecified flags should get default values."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {"flag1": ("Desc1", True, 2), "flag2": ("Desc2", 0, 3)},
                )

        assert flags["flag1"] is True
        assert flags["flag2"] == 0


class TestReadArgsErrors:
    """Tests for error handling."""

    def test_illegal_argument(self):
        """Illegal arguments should return (False, {}, {}, {})."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "unknown_arg"]):
            with patch("cfabric.core.command.console") as mock_console:
                result = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {},
                    {},
                )

        assert result == (False, {}, {}, {})
        # Should have shown help and error message
        assert mock_console.call_count >= 2

    def test_illegal_param(self):
        """Unknown param should be illegal."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "unknown=value"]):
            with patch("cfabric.core.command.console"):
                result = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First task"},
                    {"known": ("Desc", "default")},
                    {},
                )

        assert result[0] is False


class TestReadArgsCombined:
    """Tests for combined arguments."""

    def test_tasks_params_flags_combined(self):
        """Should handle tasks, params, and flags together."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "task1", "task2", "param1=val1", "+flag1"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First", "task2": "Second"},
                    {"param1": ("Param desc", "default")},
                    {"flag1": ("Flag desc", False, 2)},
                )

        assert good is True
        assert tasks == {"task1": True, "task2": True}
        assert params["param1"] == "val1"
        assert flags["flag1"] is True

    def test_order_independence(self):
        """Argument order should not matter."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "+flag1", "param1=val1", "task1"]):
            with patch("cfabric.core.command.console"):
                (good, tasks, params, flags) = readArgs(
                    "test-cmd",
                    "Test description",
                    {"task1": "First"},
                    {"param1": ("Param desc", "default")},
                    {"flag1": ("Flag desc", False, 2)},
                )

        assert good is True
        assert "task1" in tasks
        assert params["param1"] == "val1"
        assert flags["flag1"] is True


class TestReadArgsHelpText:
    """Tests for help text generation."""

    def test_help_includes_command(self):
        """Help text should include command name."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "--help"]):
            with patch("cfabric.core.command.console") as mock_console:
                readArgs(
                    "my-command",
                    "My description",
                    {"task1": "First task"},
                    {},
                    {},
                )

        # Check that console was called with text containing command name
        call_args = mock_console.call_args_list[0][0][0]
        assert "my-command" in call_args

    def test_help_includes_description(self):
        """Help text should include description."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "--help"]):
            with patch("cfabric.core.command.console") as mock_console:
                readArgs(
                    "cmd",
                    "This is my description",
                    {"task1": "First task"},
                    {},
                    {},
                )

        call_args = mock_console.call_args_list[0][0][0]
        assert "This is my description" in call_args

    def test_help_includes_task_names(self):
        """Help text should list task names."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "--help"]):
            with patch("cfabric.core.command.console") as mock_console:
                readArgs(
                    "cmd",
                    "Description",
                    {"myTask": "My task description"},
                    {},
                    {},
                )

        call_args = mock_console.call_args_list[0][0][0]
        assert "myTask" in call_args

    def test_help_includes_all_option(self):
        """Help text should include 'all' task option."""
        from cfabric.core.command import readArgs

        with patch("sys.argv", ["cmd", "--help"]):
            with patch("cfabric.core.command.console") as mock_console:
                readArgs(
                    "cmd",
                    "Description",
                    {"task1": "First task"},
                    {},
                    {},
                )

        call_args = mock_console.call_args_list[0][0][0]
        assert "all" in call_args.lower()
