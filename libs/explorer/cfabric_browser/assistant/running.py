"""Where a running browser can be reached, for a language model's tools.

At its start the browser writes its address to `~/.cfabric/running.json`;
the stdio helper (`cfabric-browser mcp`) reads it to forward a model's tool
calls. (Other web pages are kept out by `cfabric_browser.guard`, and `/mcp`
by its own checks of host and origin.) Several browsers can run at once
(windows on other ports): each has an entry, the newest last, and removes it
when it stops. An entry of a browser that was killed stays; readers try the
newest that answers.
"""

from __future__ import annotations

import datetime
import json
import os
import tempfile
from pathlib import Path
from typing import Any

RUNNING = "~/.cfabric/running.json"


def read(path: str | Path = RUNNING) -> list[dict[str, Any]]:
    """The browsers that said they run, the newest last."""
    try:
        data = json.loads(Path(path).expanduser().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return []
    found = data.get("browsers") if isinstance(data, dict) else None
    return [b for b in found or [] if isinstance(b, dict) and "url" in b]


def _write(path: Path, browsers: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # written next to it and renamed, readable by this user only
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".running-", suffix=".json")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump({"browsers": browsers}, f, indent=1)
        os.chmod(tmp, 0o600)
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


def announce(url: str, path: str | Path = RUNNING) -> dict[str, Any]:
    """Add this browser (its process and address)."""
    entry = {
        "pid": os.getpid(),
        "url": url,
        "started": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
    }
    place = Path(path).expanduser()
    others = [b for b in read(place) if b.get("url") != url]
    _write(place, [*others, entry])
    return entry


def withdraw(url: str, path: str | Path = RUNNING) -> None:
    """Remove this browser's entry, when it stops."""
    place = Path(path).expanduser()
    found = read(place)
    left = [
        b for b in found if not (b.get("url") == url and b.get("pid") == os.getpid())
    ]
    if len(left) != len(found):
        _write(place, left)
