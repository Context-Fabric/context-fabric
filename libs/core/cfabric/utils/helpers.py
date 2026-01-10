from __future__ import annotations

import os
import sys
from sys import getsizeof, stderr
import re
from itertools import chain
from collections import deque
from subprocess import run as run_cmd, CalledProcessError
from datetime import datetime as dt, timezone
from typing import TYPE_CHECKING, Any, Callable, Generator

if TYPE_CHECKING:
    from re import Match

from cfabric.core.config import OMAP
from cfabric.utils.files import readYaml, unexpanduser as ux
from cfabric.utils.attrs import AttrDict


NBSP = "\u00a0"  # non-breaking space

TO_SYM = "↦"
FROM_SYM = "⇥"


LETTER = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
VALID = set("_0123456789") | LETTER
MQL_KEYWORDS = dict(
    database="dbase",
    default="dfault",
    first="frst",
    focus="fcus",
    gap="gp",
    last="lst",
    notexist="notexst",
    object="objct",
    retrieve="retriev",
    noretrieve="noretriev",
    type="typ",
)
MQL_KEYWORDS["as"] = "as_"
MQL_KEYWORDS["or"] = "or_"

WARN32 = """WARNING: you are not running a 64-bit implementation of Python.
You may run into memory problems if you load a big data set.
Consider installing a 64-bit Python.
"""

MSG64 = """Running on 64-bit Python"""

SEP_RE = re.compile(r"[\n\t ,]+")
STRIP_RE = re.compile(r"(?:^[\n\t ,]+)|(?:[\n\t ,]+$)", re.S)
VAR_RE = re.compile(r"\{([^}]+?)(:[^}]*)?\}")
MSG_LINE_RE = re.compile(r"^( *[0-9]+) (.*)$")
NUM_ALFA_RE = re.compile(r"^([0-9]*)([^0-9]*)(.*)$")

QUAD = "    "


def safe_rank_key(Crank: Any) -> Callable[[int], int]:
    """Create a sort key function that safely handles out-of-bounds nodes.

    Parameters
    ----------
    Crank : array-like
        The canonical rank array (0-indexed, so node n has rank Crank[n-1])

    Returns
    -------
    Callable[[int], int]
        A function that returns the rank for a node, or a large value for
        out-of-bounds nodes (so they sort to the end).
    """
    max_idx = len(Crank)
    fallback = max_idx + 1  # Out-of-bounds nodes sort to end

    def get_rank(n: int) -> int:
        idx = n - 1
        if idx < 0 or idx >= max_idx:
            return fallback
        return Crank[idx]

    return get_rank


def utcnow() -> dt:
    return dt.now(timezone.utc)


def versionSort(x: str) -> tuple[tuple[int, str, str], ...]:
    parts: list[tuple[int, str, str]] = []

    for p in x.split("."):
        match = NUM_ALFA_RE.match(p)
        if match:
            (num, alfa, rest) = match.group(1, 2, 3)
            parts.append((int(num) if num else 0, alfa, rest))
        else:
            parts.append((0, p, ""))

    return tuple(parts)


def var(envVar: str) -> str | None:
    """Retrieves the value of an environment variable.

    Parameters
    ----------
    envVar: string
        The name of the environment variable.

    Returns
    -------
    string or void
        The value of the environment variable if it exists, otherwise `None`.
    """
    return os.environ.get(envVar, None)


def isInt(val: Any) -> bool:
    try:
        val = int(val)
    except Exception:
        return False
    return True


def mathEsc(val: Any) -> str:
    """Escape dollar signs to `<span>$</span>`.

    To prevent them from being interpreted as math in a Jupyter notebook
    in cases where you need them literally.
    """

    return "" if val is None else (str(val).replace("$", "<span>$</span>"))


def mdEsc(val: Any, math: bool = False) -> str:
    """Escape certain markdown characters.

    Parameters
    ----------
    val: string
        The input value
    math: boolean, optional False
        Whether retain TeX notation.
        If True, `$` is not escaped, if False, it is not escaped.
    """
    if val is None:
        return ""

    val = (
        str(val)
        .replace("!", "&#33;")
        .replace("#", "&#35;")
        .replace("*", "&#42;")
        .replace("[", "&#91;")
        .replace("_", "&#95;")
        .replace("|", "&#124;")
        .replace("~", "&#126;")
    )

    return val if math else val.replace("$", "<span>$</span>")


def htmlEsc(val: Any, math: bool = False) -> str:
    """Escape certain HTML characters by HTML entities.

    To prevent them to be interpreted as HTML
    in cases where you need them literally.

    Parameters
    ----------
    val: string
        The input value
    math: boolean, optional False
        Whether retain TeX notation.
        If True, `$` is not escaped, if False, it is not escaped.
    """

    return (
        ""
        if val is None
        else (
            (str(val).replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))
            if math
            else (
                str(val)
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("$", "<span>$</span>")
            )
        )
    )


def xmlEsc(val: Any) -> str:
    """Escape certain HTML characters by XML entities.

    To prevent them to be interpreted as XML
    in cases where you need them literally.
    """

    return (
        ""
        if val is None
        else (
            str(val)
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("'", "&apos;")
            .replace('"', "&quot;")
        )
    )


def mdhtmlEsc(val: Any, math: bool = False) -> str:
    """Escape certain Markdown characters by HTML entities or span elements.

    To prevent them to be interpreted as Markdown
    in cases where you need them literally.

    Parameters
    ----------
    val: string
        The input value
    math: boolean, optional False
        Whether retain TeX notation.
        If True, `$` is not escaped, if False, it is not escaped.
    """

    return (
        ""
        if val is None
        else (
            (
                str(val)
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("|", "&#124;")
            )
            if math
            else (
                str(val)
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("|", "&#124;")
                .replace("$", "<span>$</span>")
            )
        )
    )


def tsvEsc(x: Any) -> str:
    """Escapes a double quote for strings to be included in TSV data.

    Only `"` and `'` at the beginning of the string are escaped.
    The escaping is realized by putting a backslash at the beginning.
    """
    s = str(x)
    return s if s == "" else f"\\{s}" if s[0] in {"'", '"'} else s


PANDAS_QUOTE = '"'
PANDAS_ESCAPE = "\u0001"


def pandasEsc(x: Any) -> str:
    """Escapes the character that will be used as the `pandas` quote char.

    The escaping is realized by prepending a special char the quote char.
    Also: all tab characters will be replaced by single spaces.
    """
    return (
        x
        if x == ""
        else str(x)
        .replace("\t", " ")
        .replace(PANDAS_QUOTE, PANDAS_ESCAPE + PANDAS_QUOTE)
    )


def camel(name: str) -> str:
    if not name:
        return name
    temp = name.replace("_", " ").title().replace(" ", "")
    return temp[0].lower() + temp[1:]


def check32() -> tuple[bool, str, str]:
    warn = ""
    msg = ""
    on32 = sys.maxsize < 2**63 - 1
    if on32 < 2**63 - 1:
        warn = WARN32
    else:
        msg = MSG64
    return (on32, warn, msg)


def console(
    *msg: Any,
    error: bool = False,
    newline: bool = True,
    file: Any = None,
) -> None:
    msg_str = " ".join(m if type(m) is str else repr(m) for m in msg)
    msg_str = "" if not msg_str else ux(msg_str)
    msg_str = msg_str[1:] if msg_str.startswith("\n") else msg_str
    msg_str = msg_str[0:-1] if msg_str.endswith("\n") else msg_str
    if file is not None:
        target = file
    else:
        target = sys.stderr if error else sys.stdout
    nl = "\n" if newline else ""
    target.write(f"{msg_str}{nl}")
    target.flush()


def cleanName(name: str) -> str:
    clean = "".join(c if c in VALID else "_" for c in name)
    if clean == "" or not clean[0] in LETTER:
        clean = "x" + clean
    return MQL_KEYWORDS.get(clean, clean)


def isClean(name: str | None) -> bool:
    if name is None or len(name) == 0 or name[0] not in LETTER:
        return False
    return all(c in VALID for c in name[1:])


def flattenToSet(features: str | list[Any] | tuple[Any, ...]) -> set[str]:
    theseFeatures: set[str] = set()
    if type(features) is str:
        theseFeatures |= setFromStr(features)
    else:
        for feature in features:
            if type(feature) is str:
                theseFeatures.add(feature)
            else:
                feature = feature[1]
                theseFeatures |= setFromValue(feature)
    return theseFeatures


def setFromSpec(spec: str) -> set[int]:
    covered: set[int] = set()
    for r_str in spec.split(","):
        bounds = r_str.split("-")
        if len(bounds) == 1:
            covered.add(int(r_str))
        else:
            b = int(bounds[0])
            e = int(bounds[1])
            if e < b:
                (b, e) = (e, b)
            for n in range(b, e + 1):
                covered.add(n)
    return covered


def rangesFromSet(nodeSet: set[int]) -> Generator[tuple[int, int], None, None]:
    curstart: int = -1
    curend: int = -1
    started = False
    for n in sorted(nodeSet):
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


def rangesFromList(nodeList: list[int]) -> Generator[tuple[int, int], None, None]:  # the list must be sorted
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


def specFromRanges(ranges: list[tuple[int, int]] | Generator[tuple[int, int], None, None]) -> str:  # ranges must be normalized
    return ",".join(
        "{}".format(r[0]) if r[0] == r[1] else "{}-{}".format(*r) for r in ranges
    )


def specFromRangesLogical(ranges: list[tuple[int, int]] | Generator[tuple[int, int], None, None]) -> list[int | list[int]]:  # ranges must be normalized
    return [r[0] if r[0] == r[1] else [r[0], r[1]] for r in ranges]


def valueFromTf(tf: str) -> str:
    return "\\".join(
        x.replace("\\t", "\t").replace("\\n", "\n") for x in tf.split("\\\\")
    )


def tfFromValue(val: str | int) -> str | None:
    if isinstance(val, int):
        return str(val)
    if isinstance(val, str):
        return val.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")
    console(f"Wrong type for a TF value: {type(val)}: {val}", error=True)
    return None


def makeIndex(data: dict[int, int]) -> dict[int, set[int]]:
    inv: dict[int, set[int]] = {}
    for n, m in data.items():
        inv.setdefault(m, set()).add(n)
    return inv


def makeInverse(data: dict[int, list[int] | set[int]]) -> dict[int, set[int]]:
    inverse: dict[int, set[int]] = {}
    for n in data:
        for m in data[n]:
            inverse.setdefault(m, set()).add(n)
    return inverse


def makeInverseVal(data: dict[int, dict[int, Any]]) -> dict[int, dict[int, Any]]:
    inverse: dict[int, dict[int, Any]] = {}
    for n in data:
        for m, val in data[n].items():
            inverse.setdefault(m, {})[n] = val
    return inverse


def nbytes(by: int | float) -> str:
    units = ["B", "KB", "MB", "GB", "TB"]
    for i in range(len(units)):
        if by < 1024 or i == len(units) - 1:
            fmt = "{:>5}{}" if i == 0 else "{:>5.1f}{}"
            return fmt.format(by, units[i])
        by /= 1024
    return ""


def collectFormats(config: dict[str, str]) -> tuple[dict[str, tuple[str, str, tuple[tuple[tuple[str, ...], str], ...]]], list[str]]:
    featureSet: set[str] = set()

    def collectFormat(tpl: str) -> tuple[str, str, tuple[tuple[tuple[str, ...], str], ...]]:
        features: list[tuple[tuple[str, ...], str]] = []
        default = ""

        def varReplace(match: Match[str]) -> str:
            nonlocal default
            varText = match.group(1)
            default = (match.group(2) or ":")[1:]
            fts = tuple(varText.split("/"))
            features.append((fts, default))
            for ft in fts:
                featureSet.add(ft)
            return "{}"

        rtpl = VAR_RE.sub(varReplace, tpl)
        return (tpl, rtpl, tuple(features))

    formats: dict[str, tuple[str, str, tuple[tuple[tuple[str, ...], str], ...]]] = {}
    for fmt, tpl in sorted(config.items()):
        if fmt.startswith("fmt:"):
            formats[fmt[4:]] = collectFormat(tpl)
    return (formats, sorted(featureSet))


def itemize(string: str | None, sep: str | None = None) -> list[str]:
    if not string:
        return []
    if not sep:
        return string.strip().split()
    return string.strip().split(sep)


def fitemize(value: Any) -> list[str]:
    if not value:
        return []
    if type(value) is str:
        return SEP_RE.split(STRIP_RE.sub("", value))
    if type(value) in {bool, int, float}:
        return [str(value)]
    return list(str(v) for v in value)


def project(iterableOfTuples: set[tuple[Any, ...]] | list[tuple[Any, ...]], maxDimension: int) -> set[Any] | set[tuple[Any, ...]]:
    if maxDimension == 1:
        return {r[0] for r in iterableOfTuples}
    return {r[0:maxDimension] for r in iterableOfTuples}


def wrapMessages(messages: str | list[str | tuple[bool, bool, str]]) -> tuple[bool, str]:
    if type(messages) is str:
        messages = messages.split("\n")
    html: list[str] = []
    status = True
    for msg in messages:
        if type(msg) is tuple:
            (error, nl, msgRep) = msg
            if error:
                status = False
            match = MSG_LINE_RE.match(msgRep)
            msg_str = msgRep + ("<br>" if nl else "")
            clsName = "eline" if error and not match else "tline"
        else:
            match = MSG_LINE_RE.match(msg)
            clsName = "tline" if match else "eline"
            if clsName == "eline":
                status = False
            msg_str = msg.replace("\n", "<br>")
        html.append(f'<span class="{clsName.lower()}">{msg_str}</span>')
    return (status, "".join(html))


def makeExamples(nodeList: list[int] | tuple[int, ...]) -> str:
    lN = len(nodeList)
    if lN <= 10:
        return f"{lN:>7} x: " + (", ".join(str(n) for n in nodeList))
    else:
        return (
            f"{lN:>7} x: "
            + (", ".join(str(n) for n in nodeList[0:5]))
            + " ... "
            + (", ".join(str(n) for n in nodeList[-5:]))
        )


def setFromValue(x: Any, asInt: bool = False) -> set[Any]:
    if x is None:
        return set()

    typeX = type(x)
    if typeX in {set, frozenset}:
        return x
    elif typeX in {str, dict, list, tuple}:
        if typeX is str:
            x = SEP_RE.split(x)
        return {int(p) for p in x if p.isdecimal()} if asInt else {p for p in x if p}

    return {x}


def setFromStr(x: str | None) -> set[str]:
    if x is None:
        return set()

    return {p for p in SEP_RE.split(x) if p}


def mergeDictOfSets(d1: dict[Any, set[Any]], d2: dict[Any, set[Any]]) -> None:
    for n, ms in d2.items():
        if n in d1:
            d1[n] |= ms
        else:
            d1[n] = ms


def mergeDict(source: dict[str, Any], overrides: dict[str, Any]) -> None:
    """Merge overrides into a source dictionary recursively.

    Parameters
    ----------
    source: dict
        The source dictionary, which will be modified by the overrides.
    overrides: dict
        The overrides, itself a dictionary.
    """

    for k, v in overrides.items():
        if k in source and type(source[k]) is dict:
            mergeDict(source[k], v)
        else:
            source[k] = v


def getAllRealFeatures(api: Any) -> set[str]:
    """Get all configuration features and all loaded node and edge features.

    Except `omap@v-w` features.
    When we take volumes or collections from works,
    we need to pass these features on.

    This will exclude the computed features and the node / edge features
    that are not loaded by default.
    """

    CF = api.CF
    allFeatures: set[str] = set()

    for feat, fObj in CF.features.items():
        if fObj.method:
            continue
        if fObj.isConfig:
            allFeatures.add(feat)

    allFeatures |= set(api.Fall())
    allFeatures |= {e for e in api.Eall() if not e.startswith(OMAP)}
    return allFeatures


def formatMeta(featureMeta: dict[str, dict[str, str]]) -> dict[str, dict[str, str]]:
    """Reorder meta data.

    Parameters
    ----------
    meta: dict
        Dictionary of meta data: keyed by feature, valued by a dict
        of metadata in the form of key values

    Returns
    -------
    dict
        A copy of the dict but with the values for metadata keys
        `desc` and `eg` merged under a new key `description`,
        and the keys `desc` and `eg` deleted.
    """

    result: dict[str, dict[str, str]] = {}
    for f, meta in featureMeta.items():
        fmeta: dict[str, str] = {}
        for k, v in meta.items():
            if k == "eg" and "desc" in meta:
                continue
            if k == "desc":
                eg = meta.get("eg", "")
                egRep = f" ({eg})" if eg else ""
                fmeta["description"] = f"{v}{egRep}"
            else:
                fmeta[k] = v
        result[f] = fmeta

    return result


def deepSize(o: Any, handlers: dict[type, Callable[[Any], Any]] = {}, verbose: bool = False, seen: set[int] | None = None) -> int:
    """Returns the approximate memory footprint an object and all of its contents.

    Automatically finds the contents of the following builtin containers and
    their subclasses:
    `tuple`, `list`, `deque`, `dict`, `set` and `frozenset`.
    To search other containers, add handlers to iterate over their contents:

    ```
    handlers = {SomeContainerClass: iter,
                OtherContainerClass: OtherContainerClass.get_elements}
    ```

    """

    def dict_handler(d: dict[Any, Any]) -> chain[Any]:
        return chain.from_iterable(d.items())

    all_handlers: dict[type, Callable[[Any], Any]] = {
        tuple: iter,
        list: iter,
        deque: iter,
        dict: dict_handler,
        set: iter,
        frozenset: iter,
    }
    all_handlers.update(handlers)  # user handlers take precedence
    if seen is None:
        seen = set()  # track which object id's have already been seen
    default_size = getsizeof(0)  # estimate sizeof object without __sizeof__

    def sizeof(o: Any) -> int:
        if id(o) in seen:  # do not double count the same object
            return 0
        seen.add(id(o))
        s = getsizeof(o, default_size)

        if verbose:
            console(s, type(o), repr(o), file=stderr)

        for typ, handler in all_handlers.items():
            if isinstance(o, typ):
                s += sum(map(sizeof, handler(o)))
                break
        return s

    return sizeof(o)


def run(cmdline: str, workDir: str | None = None) -> tuple[bool, int, str, str]:
    """Runs a shell command and returns all relevant info.

    The function runs a command-line in a shell, and returns
    whether the command was successful, and also what the output was, separately for
    standard error and standard output.

    Parameters
    ----------
    cmdline: string
        The command-line to execute.
    workDir: string, optional None
        The working directory where the command should be executed.
        If `None` the current directory is used.
    """
    try:
        result = run_cmd(
            cmdline,
            shell=True,
            cwd=workDir,
            check=True,
            capture_output=True,
        )
        stdOut = result.stdout.decode("utf8").strip()
        stdErr = result.stderr.decode("utf8").strip()
        returnCode = 0
        good = True

    except CalledProcessError as e:
        stdOut = e.stdout.decode("utf8").strip()
        stdErr = e.stderr.decode("utf8").strip()
        returnCode = e.returncode
        good = False

    return (good, returnCode, stdOut, stdErr)


def readCfg(folder: str, file: str, label: str, verbose: int = 0, **kwargs: Any) -> tuple[bool, AttrDict | dict[str, Any] | None]:
    settingsFile = f"{folder}/config/{file}.yml"
    settings = readYaml(asFile=settingsFile, **kwargs)

    if settings:
        if verbose == 1:
            console(f"{label} settings read from {settingsFile}")
        good = True
    else:
        console(f"No {label} settings found, looked for {settingsFile}", error=True)
        good = False

    return (good, settings)
