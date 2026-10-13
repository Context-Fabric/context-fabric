"""Checking search templates and reporting problems as structured data.

The browser and (later) language models both need errors they can act on:
which line, what is wrong, and which feature or type is involved. The search
engine reports problems as `(line, message)` pairs with 0-based lines; here
they become dictionaries with 1-based lines.
"""

from __future__ import annotations

import logging
import re
import threading
from collections.abc import Iterator
from contextlib import contextmanager
from typing import Any

from cfabric.search.searchexe import SearchExe

LANGUAGES = ("cf-template",)
"""Query languages the browser accepts. A natural-language mode will later be
translated into `cf-template` before searching."""

MISSING_FEATURE = re.compile(r'Missing feature "([^"]+)" in line\(s\) ([\d, ]+)')
UNKNOWN_TYPE = re.compile(r'Unknown object type: "([^"]+)"')
TEMPLATE_ECHO = re.compile(r"^\s*\d+ ")
QUANTIFIER_ERROR = re.compile(r"^line\s+(\d+): Error under (/\w+/):")
LINE_MESSAGE = re.compile(r"^line \d+:")


def _structure(kind: str, line: int | None, message: str) -> list[dict[str, Any]]:
    """One engine message as one or more error records."""
    m = MISSING_FEATURE.search(message)
    if m:
        lines = [int(x) for x in re.findall(r"\d+", m.group(2))]
        return [
            {
                "kind": kind,
                "line": ln + 1,
                "message": f'Unknown feature "{m.group(1)}"',
                "feature": m.group(1),
            }
            for ln in lines
        ]
    record: dict[str, Any] = {
        "kind": kind,
        "line": None if line is None else int(line) + 1,
        "message": message,
    }
    m = UNKNOWN_TYPE.search(message)
    if m:
        record["nodeType"] = m.group(1)
    return [record]


def check_template(
    api, template: str, sets: dict[str, set[int]] | None = None
) -> list[dict[str, Any]]:
    """Check a template without searching; return its problems.

    Only the cheap phases run: parsing, checking types, features and names,
    and checking that the template is connected. Problems inside quantifiers
    surface only when searching; the search worker reports them in the same
    form (see `problems_of`).

    Each problem is a dict with `kind` (`syntax` or `semantics`), `line`
    (1-based, or `None` for general remarks), `message`, and where it applies
    `feature` or `nodeType`. An empty list means the template is fine.
    """
    if not template.strip():
        return [{"kind": "syntax", "line": None, "message": "Empty search template"}]

    exe = SearchExe(
        api,
        template,
        outerTemplate=template,
        quKind=None,
        offset=0,
        sets=sets,
        shallow=False,
        silent="deep",
        setInfo={},
    )
    with capture_engine_messages() as messages:
        exe._parse()
        exe._prepare()
    return problems_of(exe, messages)


def template_features(api, template: str) -> list[list[str]]:
    """The node features mentioned per atom of a template, in result-tuple order.

    `clause typ*` mentions `typ`, `word sp=verb gloss~a` mentions `sp` and
    `gloss`. As in Text-Fabric, mentioning a feature with `*` (any value) is the
    way to get it into exports without restricting the search. Atoms inside
    quantifiers are not part of results and are left out. Edge features are
    left out too.
    """
    exe = SearchExe(
        api,
        template,
        outerTemplate=template,
        quKind=None,
        offset=0,
        sets=None,
        shallow=False,
        silent="deep",
        setInfo={},
    )
    with capture_engine_messages():
        exe._parse()
        exe._prepare()
    if not exe.good:
        return []
    nodeFeatures = set(api.Fall())
    return [
        [name for name in qnode[1] if name in nodeFeatures and name != "otype"]
        for qnode in exe.qnodes
    ]


def problems_of(
    exe: SearchExe, messages: list[str] | None = None
) -> list[dict[str, Any]]:
    """The problems recorded on a search execution, as structured records.

    Some problems (such as a template that falls apart into unconnected parts)
    are only logged by the engine, not recorded; pass the messages captured
    with `capture_engine_messages` to include them.
    """
    if exe.good:
        return []
    problems: list[dict[str, Any]] = []
    for kind, attr in (("syntax", "badSyntax"), ("semantics", "badSemantics")):
        for line, message in getattr(exe, attr, None) or ():
            problems.extend(_structure(kind, line, message))
    if not problems:
        for message in messages or ():
            if TEMPLATE_ECHO.match(message) or LINE_MESSAGE.match(message):
                continue
            problems.append({"kind": "semantics", "line": None, "message": message})
    if not problems:
        problems.append(
            {"kind": "semantics", "line": None, "message": "Invalid search template"}
        )
    return problems


def quantifier_problems(messages: list[str]) -> list[dict[str, Any]]:
    """Problems inside quantifiers, found in the engine's captured messages.

    The engine (like Text-Fabric) reports an error inside a quantifier such as
    `/without/` only in its log, and then carries on as if the quantifier did
    not exclude anything. That gives wrong results without warning, so the
    browser treats these messages as errors. Lines refer to the quantifier's
    line in the full template; the engine's own line numbers inside the
    quantifier refer to a sub-template and are dropped.
    """
    problems: list[dict[str, Any]] = []
    context: tuple[int, str] | None = None
    for message in messages:
        m = QUANTIFIER_ERROR.match(message)
        if m:
            context = (int(m.group(1)) + 1, m.group(2))
            continue
        if context is None or TEMPLATE_ECHO.match(message):
            continue
        (line, kind) = context
        for problem in _structure("semantics", None, message):
            problem["line"] = line
            problem["message"] = f"{problem['message']} (inside {kind})"
            problem["quantifier"] = kind
            problems.append(problem)
    return problems


class _ThreadCapture(logging.Handler):
    def __init__(self, thread: int) -> None:
        super().__init__(level=logging.ERROR)
        self.thread = thread
        self.messages: list[str] = []

    def emit(self, record: logging.LogRecord) -> None:
        if record.thread == self.thread:
            for line in record.getMessage().splitlines():
                if line.strip():
                    self.messages.append(line.rstrip())


@contextmanager
def capture_engine_messages() -> Iterator[list[str]]:
    """Collect the engine's error messages of this thread instead of printing them.

    Other threads keep logging as usual.
    """
    thread = threading.get_ident()
    cfLogger = logging.getLogger("cfabric")
    capture = _ThreadCapture(thread)
    silence = logging.Filter()
    silence.filter = lambda record: record.thread != thread  # type: ignore[method-assign]
    handlers = list(cfLogger.handlers)
    for handler in handlers:
        handler.addFilter(silence)
    cfLogger.addHandler(capture)
    try:
        yield capture.messages
    finally:
        cfLogger.removeHandler(capture)
        for handler in handlers:
            handler.removeFilter(silence)
