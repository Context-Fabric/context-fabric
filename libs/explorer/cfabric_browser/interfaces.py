"""The browser's interfaces, and the settings that choose among them.

The interface itself is chosen in the browser (and remembered there); the
server only says which one to start in and which ones to leave out of the menu:
`--ui NAME`, or in `~/.cfabric/browser.yaml` either `ui: parallel` or

    ui:
      default: parallel
      hide: [panes, concordance]
"""

from __future__ import annotations

from typing import Any

# the ids the interface uses; Panes is "classic" there
UIS = ("classic", "parallel", "reader", "interlinear", "concordance")
ALIASES = {"panes": "classic"}


def ui_id(name: str) -> str:
    """The id of an interface named in the settings (`panes` is `classic`)."""
    key = str(name).strip().lower()
    key = ALIASES.get(key, key)
    if key not in UIS:
        names = ", ".join(["panes", *UIS[1:]])
        raise ValueError(f"unknown interface {name!r}; choose from {names}")
    return key


def read_ui_settings(value: Any) -> tuple[str | None, list[str]]:
    """The default interface and the hidden ones from the `ui:` setting.

    Raises
    ------
    ValueError
        For an unknown interface or a setting of another shape.
    """
    if value is None:
        return (None, [])
    if isinstance(value, str):
        return (ui_id(value), [])
    if isinstance(value, dict):
        default = value.get("default")
        hide = value.get("hide") or []
        if isinstance(hide, str):
            hide = [hide]
        return (ui_id(default) if default else None, [ui_id(h) for h in hide])
    raise ValueError("ui: should be an interface name, or default: and hide:")
