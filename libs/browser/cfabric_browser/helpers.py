"""Small helpers the browser needs, copied from the Python core
(`cfabric.utils.helpers`), so that the browser works with any core: the
Python one and the Rust one (0.6), which has no `cfabric.utils`."""

from __future__ import annotations

import re
from collections.abc import Generator
from typing import Any

NUM_ALFA_RE = re.compile(r"^([0-9]*)([^0-9]*)(.*)$")


def rangesFromList(nodeList: list[int]) -> Generator[tuple[int, int]]:
    """Consecutive runs in a sorted list of nodes, as (first, last) pairs."""
    curstart: int = -1
    curend: int = -1
    started = False
    for n in nodeList:
        if not started:
            curstart = n
            curend = n
            started = True
        elif n == curend + 1:
            curend = n
        else:
            yield (curstart, curend)
            curstart = n
            curend = n
    if started:
        yield (curstart, curend)


def tsvEsc(x: Any) -> str:
    """A value for TSV data: a `"` or `'` at its start gets a backslash before
    it (only there), as Text-Fabric does."""
    s = str(x)
    return s if s == "" else f"\\{s}" if s[0] in {"'", '"'} else s


def versionSort(x: str) -> tuple[tuple[int, str, str], ...]:
    """A sort key for version numbers such as `2021`, `c` or `1.10.2`."""
    parts: list[tuple[int, str, str]] = []
    for p in x.split("."):
        match = NUM_ALFA_RE.match(p)
        if match:
            (num, alfa, rest) = match.group(1, 2, 3)
            parts.append((int(num) if num else 0, alfa, rest))
        else:
            parts.append((0, p, ""))
    return tuple(parts)
