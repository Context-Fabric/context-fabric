"""Checking search templates and reporting problems as structured data.

The browser and (later) language models both need errors they can act on:
which line, what is wrong, and which feature or type is involved. The search
engine reports problems as `(line, message)` pairs with 0-based lines; here
they become dictionaries with 1-based lines.

The browser works with either core. With the Python one, a template is
checked by running only the engine's cheap phases (parsing and preparing,
not the search); the Rust one (0.6) checks it with `S.study`, which is fast
there. The features per atom of a template (for exports and statistics) come
from parsing the template here (`template_atoms`), the same for both.
"""

from __future__ import annotations

import logging
import re
import threading
from collections.abc import Iterator
from contextlib import contextmanager
from typing import Any

try:  # the Python core's engine
    from cfabric.search.searchexe import SearchExe
except ImportError:  # the Rust core (0.6) has none
    SearchExe = None

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
    if SearchExe is None:
        return study(api, template, sets)

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


def template_constraints(api, template: str) -> list[list[tuple[str, bool]]]:
    """The node features mentioned per atom of a template, in result-tuple order.

    Each feature comes with whether the template fixes it to a single value
    (`sp=verb`) rather than leaving it open (`lex*`, `sp=verb|subs`,
    `gloss~a`, `sp#verb`). Atoms inside quantifiers are not part of results
    and are left out; so are edge features, and features the corpus doesn't
    have.
    """
    nodeFeatures = set(api.Fall())
    return [
        [
            (name, fixed)
            for (name, fixed) in atom
            if name in nodeFeatures and name != "otype"
        ]
        for atom in template_atoms(template)
    ]


def template_features(api, template: str) -> list[list[str]]:
    """The node features mentioned per atom of a template, in result-tuple order.

    `clause typ*` mentions `typ`, `word sp=verb gloss~a` mentions `sp` and
    `gloss`. As in Text-Fabric, mentioning a feature with `*` (any value) is the
    way to get it into exports without restricting the search.
    """
    return [
        [name for (name, _) in atom] for atom in template_constraints(api, template)
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


# parsing templates, only as far as the features of each atom

QUANTIFIER = re.compile(r"^/(without|where|have|with|or|-)/$")
OPENING = {"without", "where", "with"}
ATOM_HEAD = re.compile(r"^(?:[A-Za-z_]\w*:)?[A-Za-z_][\w.]*$")
FEATURE = re.compile(r"^([A-Za-z_][\w@-]*)(\*|[=#~<>].*)?$")


def _tokens(line: str) -> list[str]:
    r"""The words of a line; `\ ` is a space inside a value."""
    return [t.replace("\0", " ") for t in line.replace("\\ ", "\0").split()]


def template_atoms(template: str) -> list[list[tuple[str, bool]]]:
    """Per atom of a template outside quantifiers, in order, the features it
    mentions with whether it fixes them to one value.

    Comments (`%`), quantifier blocks (`/without/` … `/-/`), relation lines
    between named atoms (`p1 < p2`) and feature comparisons (`.f=g.`) are
    left out; an atom may start with a relation (`<: word`) and a name
    (`w1:word`). A feature written without a value (`gloss`) or with `*`
    is open, as are `#`, `~`, `<` and `>`; `=` with one value (no `|`) is
    fixed.
    """
    names: set[str] = set()
    depth = 0
    atoms: list[list[tuple[str, bool]]] = []
    for raw in template.splitlines():
        line = raw.strip()
        if not line or line.startswith("%"):
            continue
        q = QUANTIFIER.match(line)
        if q:
            if q.group(1) in OPENING:
                depth += 1
            elif q.group(1) == "-":
                depth = max(0, depth - 1)
            continue
        tokens = _tokens(line)
        if len(tokens) == 3 and tokens[0] in names and tokens[2] in names:
            continue  # a relation between named atoms
        if tokens and not ATOM_HEAD.match(tokens[0]):
            tokens = tokens[1:]  # a relation before the atom
        if not tokens or not ATOM_HEAD.match(tokens[0]):
            continue
        if ":" in tokens[0]:
            names.add(tokens[0].split(":", 1)[0])
        if depth:
            continue
        features: list[tuple[str, bool]] = []
        seen: set[str] = set()
        for token in tokens[1:]:
            if token.startswith("."):
                continue  # comparing features of two atoms
            m = FEATURE.match(token)
            if not m or m.group(1) in seen:
                continue
            seen.add(m.group(1))
            spec = m.group(2) or ""
            features.append((m.group(1), spec.startswith("=") and "|" not in spec))
        atoms.append(features)
    return atoms


_studyLocks: dict[int, threading.Lock] = {}
_studyGuard = threading.Lock()


def study(
    api, template: str, sets: dict[str, set[int]] | None = None
) -> list[dict[str, Any]]:
    """Check a template with the core's own `S.study` (the Rust core): its
    problems, as `check_template` gives them. One study at a time per corpus,
    since `S` keeps the template it studied."""
    with _studyGuard:
        lock = _studyLocks.setdefault(id(api), threading.Lock())
    with lock, capture_engine_messages() as messages:
        S = api.S
        try:
            S.study(template, sets=sets, silent="deep")
        except ValueError as e:  # the Rust core's SearchTemplateError is one
            return _from_error(e)
        exe = S.exe
        if exe is None or exe.good:
            return []
        # problems inside a quantifier come as "Error under /without/:"
        # followed by the problems themselves, as the Python engine logs them
        semantics = list(exe.badSemantics)
        if any(str(m).startswith("Error under /") for (_, m) in semantics):
            lines = [
                f"line {line}: {m}" if str(m).startswith("Error under /") else str(m)
                for (line, m) in semantics
            ]
            nested = quantifier_problems(lines)
            if nested:
                return nested
        problems = problems_of(exe, messages)
        generic = [
            {"kind": "semantics", "line": None, "message": "Invalid search template"}
        ]
        if problems == generic and not exe.badSyntax and not semantics:
            return unconnected(template) or problems
        return problems


def unconnected(template: str) -> list[dict[str, Any]]:
    """For an engine that rejects a template without saying why: whether its
    objects at the top level are related. Several of them without a relation
    between them can't be searched (Text-Fabric calls that not connected)."""
    names: set[str] = set()
    tops = 0
    relations = 0
    depth = 0
    for raw in template.splitlines():
        line = raw.strip()
        if not line or line.startswith("%"):
            continue
        q = QUANTIFIER.match(line)
        if q:
            depth += 1 if q.group(1) in OPENING else -1 if q.group(1) == "-" else 0
            continue
        tokens = _tokens(line)
        if len(tokens) == 3 and tokens[0] in names and tokens[2] in names:
            relations += 1
            continue
        if tokens and ":" in tokens[0] and ATOM_HEAD.match(tokens[0]):
            names.add(tokens[0].split(":", 1)[0])
        if not depth and raw[: len(raw) - len(raw.lstrip())] == "":
            tops += 1
    if tops > 1 and relations == 0:
        return [
            {
                "kind": "semantics",
                "line": None,
                "message": "The template is not connected: its objects at the "
                "top level have no relation between them; put them inside one "
                "object, or relate them by name (p1 < p2)",
            }
        ]
    return []


def _from_error(error: Exception) -> list[dict[str, Any]]:
    """An engine's exception as problems: one per line of its message."""
    problems: list[dict[str, Any]] = []
    for text in str(error).splitlines() or [repr(error)]:
        m = re.match(r"^\s*line\s+(\d+):\s*(.*)$", text)
        if m:
            problems.extend(_structure("syntax", int(m.group(1)) - 1, m.group(2)))
        elif text.strip():
            problems.extend(_structure("syntax", None, text.strip()))
    return problems or [
        {"kind": "syntax", "line": None, "message": "Invalid search template"}
    ]
