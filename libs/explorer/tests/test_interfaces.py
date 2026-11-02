"""Tests for the settings that choose the browser's interfaces."""

import pytest
from cfabric_browser.cli import parse_args
from cfabric_browser.interfaces import read_ui_settings, ui_id


def test_names():
    assert ui_id("parallel") == "parallel"
    # Panes is "classic" in the interface
    assert ui_id("Panes") == "classic"
    with pytest.raises(ValueError, match="unknown interface"):
        ui_id("tabs")


def test_settings_as_a_name():
    assert read_ui_settings(None) == (None, [])
    assert read_ui_settings("reader") == ("reader", [])


def test_settings_with_default_and_hide():
    value = {"default": "parallel", "hide": ["panes", "concordance"]}
    assert read_ui_settings(value) == ("parallel", ["classic", "concordance"])
    assert read_ui_settings({"hide": "reader"}) == (None, ["reader"])
    with pytest.raises(ValueError):
        read_ui_settings({"hide": ["nothing"]})
    with pytest.raises(ValueError):
        read_ui_settings(["parallel"])


def test_command_line():
    assert parse_args(["--ui", "interlinear"]).ui == "interlinear"
    with pytest.raises(SystemExit):
        parse_args(["--ui", "tabs"])
