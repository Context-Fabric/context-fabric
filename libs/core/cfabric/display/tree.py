"""Render trees: structured, HTML-free displays of nodes.

A *render tree* shows a node with everything inside it, as nested fragments.
Clients (the Context-Fabric browser, notebooks, exporters) turn it into
visual displays; nothing here produces HTML.

Nodes in a corpus do not nest neatly: a phrase may have gaps, a clause may
cross a verse boundary. *Unraveling* cuts nodes into fragments at the
boundaries of other nodes, so that the fragments do nest. Each fragment
records whether its edges are the real start/end of its node (`node`),
the edge of a gap in a discontinuous node (`gap`), or a cut made by
unraveling (`split`).

The algorithm is a port of `tf.advanced.unravel` (Text-Fabric, MIT licensed,
by Dirk Roorda); see `tf.about.displaydesign` for its rationale.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass, field
from itertools import chain
from typing import TYPE_CHECKING, Any

from cfabric.display.sections import section_ref, structure_ref
from cfabric.utils.helpers import rangesFromList

if TYPE_CHECKING:
    from cfabric.core.api import Api
    from cfabric.display.config import DisplayConfig

PRETTY = "pretty"
PLAIN = "plain"

NODE = "node"
GAP = "gap"
SPLIT = "split"


@dataclass
class RenderOptions:
    """Options that change the shape or content of a render tree.

    Options that only change how a tree *looks* (show node numbers, show
    standard features, ...) are left to the client.
    """

    fmt: str | None = None
    """Text format; defaults to the corpus' default format."""
    full: bool = False
    """Unfold big nodes (books, chapters) instead of showing only their heading."""
    hide_types: bool = True
    """Leave out nodes of `hidden_types`."""
    hidden_types: Iterable[str] | None = None
    """Types to hide; defaults to the types marked hidden in the configuration."""
    base_types: Iterable[str] | None = None
    """Pretty displays do not unfold nodes of these types; defaults to the
    configured base types (normally the slot type)."""
    condense_type: str | None = None
    """Container type for tuples (e.g. `verse`); defaults to the configured one."""
    extra_features: Iterable[str] = ()
    """Features to show besides the configured ones, e.g. those of a query.
    Entries may be `type:feature` to take the value from an enclosing node."""
    highlights: dict[int, str] | None = None
    """Nodes to highlight, mapped to a color (`""` for the default color)."""
    lang: str = "en"
    """Language for section names."""


@dataclass
class Fragment:
    """A piece of a node that nests properly inside its parent fragment."""

    node: int
    type: str
    begin: int
    end: int
    start: str
    """`node`, `gap` or `split`: what the begin of this fragment is."""
    stop: str
    """`node`, `gap` or `split`: what the end of this fragment is."""
    children: list[Fragment] = field(default_factory=list)
    label: str | None = None
    heading: str | None = None
    text: str | None = None
    features: list[dict[str, Any]] | None = None
    highlight: str | None = None
    base: bool = False

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "node": self.node,
            "type": self.type,
            "range": [self.begin, self.end],
            "start": self.start,
            "end": self.stop,
        }
        for key in ("label", "heading", "text", "features", "highlight"):
            value = getattr(self, key)
            if value is not None:
                result[key] = value
        if self.base:
            result["base"] = True
        if self.children:
            result["children"] = [c.to_dict() for c in self.children]
        return result


@dataclass
class RenderTree:
    """The display of one node: its fragments plus what is needed to draw them."""

    node: int
    type: str
    mode: str
    fmt: str
    direction: str
    heading: str
    fragments: list[Fragment]

    def to_dict(self) -> dict[str, Any]:
        return {
            "node": self.node,
            "type": self.type,
            "mode": self.mode,
            "fmt": self.fmt,
            "direction": self.direction,
            "heading": self.heading,
            "fragments": [f.to_dict() for f in self.fragments],
        }

    def walk(self) -> Iterable[tuple[int, Fragment]]:
        """All fragments, depth first, with their depth."""

        def visit(frags: list[Fragment], depth: int):
            for f in frags:
                yield (depth, f)
                yield from visit(f.children, depth + 1)

        yield from visit(self.fragments, 0)

    def outline(self) -> str:
        """A compact text view of the tree, for debugging and tests."""
        lines = []
        for depth, f in self.walk():
            rng = f"{f.begin}" if f.begin == f.end else f"{f.begin}-{f.end}"
            base = "*" if f.base else ""
            edges = "" if f.start == NODE and f.stop == NODE else f" {f.start}/{f.stop}"
            lines.append(f"{'  ' * depth}{f.type}{base} {f.node} {{{rng}}}{edges}")
        return "\n".join(lines)


# unraveling


def _is_big(
    api: Api,
    config: DisplayConfig,
    pretty: bool,
    full: bool,
    condenseType: str | None,
    nType: str,
) -> bool:
    if full:
        return False
    otypeRank = api.N.otypeRank
    if not pretty and nType in config.big_types:
        return True
    sectional = set(config.section_types) | set(config.structure_types)
    if sectional and nType in sectional:
        return condenseType is None or otypeRank[nType] > otypeRank[condenseType]
    return condenseType is not None and otypeRank[nType] > otypeRank[condenseType]


def _split_points(p: tuple[int, int], q: tuple[int, int]) -> set[int]:
    """Where the boundaries of range `p` cut through range `q`.

    A split point is the slot where the second part starts.
    """
    (b1, e1) = p
    (b2, e2) = q
    if b2 == e2 or (b1 <= b2 and e1 >= e2):
        return set()
    points = set()
    if b2 < b1 <= e2:
        points.add(b1)
    if b2 <= e1 < e2:
        points.add(e1 + 1)
    return points


def _apply_splits(
    chunks: set[tuple[int, tuple[int, int]]],
    splits: dict[tuple[int, tuple[int, int]], set[int]],
) -> None:
    for target, points in splits.items():
        if not points:
            continue
        chunks.remove(target)
        (m, (b, e)) = target
        prev = b
        for sp in sorted(points):
            chunks.add((m, (prev, sp - 1)))
            prev = sp
        chunks.add((m, (prev, e)))


def unravel(
    api: Api,
    config: DisplayConfig,
    n: int,
    pretty: bool = True,
    options: RenderOptions | None = None,
    in_tuple: bool = False,
) -> list[Fragment]:
    """Cut a node and the nodes inside it into properly nested fragments.

    Returns the top-level fragments: those of `n` itself (more than one if
    `n` has gaps). Fragments are bare: no labels, texts or features yet;
    see `render_node` for dressed-up trees.
    """
    options = options or RenderOptions()
    F = api.F
    L = api.L
    N = api.N
    Fs = api.Fs
    eOslots = api.E.oslots.s
    fOtypeV = F.otype.v
    nTypes = F.otype.all

    n = int(n)
    nType = fOtypeV(n)
    lexTypes = config.lex_types
    descendantTypes = config.descendant_types

    condenseType = options.condense_type or config.condense_type

    if not pretty and nType in config.verse_types and not config.show_verse_in_tuple:
        isBig = in_tuple
    else:
        isBig = _is_big(api, config, pretty, options.full, condenseType, nType)

    # the nodes that take part

    if isBig and not options.full:
        iNodes: set[int] = set()
    elif nType in descendantTypes:
        iNodes = {int(m) for m in L.i(n, otype=descendantTypes[nType])}
    elif nType in lexTypes:
        iNodes = {n}
    else:
        iNodes = {int(m) for m in L.i(n)}

    if options.hide_types:
        hidden = (
            config.hidden_types
            if options.hidden_types is None
            else frozenset(options.hidden_types)
        )
        if hidden:
            iNodes = {m for m in iNodes if fOtypeV(m) not in hidden}

    iNodes.add(n)

    # chunks: maximal contiguous slot ranges of each node, and their true boundaries

    nSlots = tuple(int(s) for s in eOslots(n))
    if nType in lexTypes:
        nSlots = nSlots[:1]
    nSlotSet = set(nSlots)

    chunks: dict[str, set[tuple[int, tuple[int, int]]]] = {}
    boundaries: dict[int, dict[int, tuple[bool | None, bool | None]]] = {}
    exclusions = config.exclusions

    for m in iNodes:
        mType = fOtypeV(m)
        if mType in exclusions and any(
            Fs(feat).v(m) == value for (feat, value) in exclusions[mType].items()
        ):
            continue

        slots = [int(s) for s in eOslots(m)]
        if nType in lexTypes:
            slots = slots[:1]
        if m != n and mType == nType and nSlotSet <= set(slots):
            continue
        minSlot = slots[0]
        maxSlot = slots[-1]
        bounds: dict[int, tuple[bool | None, bool | None]] = {}
        for b, e in rangesFromList(slots):
            chunks.setdefault(mType, set()).add((m, (b, e)))
            bounds[b] = (b == minSlot, None if b != e else e == maxSlot)
            bounds[e] = (b == minSlot if b == e else None, e == maxSlot)
        boundaries[m] = bounds

    # fragmentize: bigger types cut smaller types, and same-type nodes cut each other

    sortKeyChunk = N.sortKeyChunk
    sortKeyChunkLength = N.sortKeyChunkLength
    typeLen = len(nTypes) - 1

    for p, pType in enumerate(nTypes):
        pChunks = chunks.get(pType)
        if not pChunks:
            continue
        splits: dict[tuple[int, tuple[int, int]], set[int]] = {}
        pSorted = sorted(pChunks, key=sortKeyChunkLength)
        for i, pChunk in enumerate(pSorted):
            for p2Chunk in pSorted[i + 1 :]:
                splits.setdefault(p2Chunk, set()).update(
                    _split_points(pChunk[1], p2Chunk[1])
                )
        _apply_splits(pChunks, splits)

        for q in range(p + 1, typeLen):
            qChunks = chunks.get(nTypes[q])
            if not qChunks:
                continue
            splits = {}
            for qChunk in qChunks:
                for pChunk in pChunks:
                    splits.setdefault(qChunk, set()).update(
                        _split_points(pChunk[1], qChunk[1])
                    )
            _apply_splits(qChunks, splits)

    # each fragment now lies either inside the top node or outside it;
    # testing its first slot suffices

    ordered = sorted(
        (c for c in chain.from_iterable(chunks.values()) if c[1][0] in nSlotSet),
        key=sortKeyChunk,
    )

    # stack the fragments

    def edge(m: int, slot: int, side: int) -> str:
        bounds = boundaries[m].get(slot)
        code = None if bounds is None else bounds[side]
        return SPLIT if code is None else NODE if code else GAP

    top: list[Fragment] = []
    stack: list[Fragment] = []
    for m, (b, e) in ordered:
        frag = Fragment(
            node=m,
            type=fOtypeV(m),
            begin=b,
            end=e,
            start=edge(m, b, 0),
            stop=edge(m, e, 1),
        )
        while stack and e > stack[-1].end:
            stack.pop()
        (stack[-1].children if stack else top).append(frag)
        stack.append(frag)

    return top


# dressing


class _Dresser:
    """Adds labels, texts, headings, features and highlights to fragments."""

    def __init__(
        self,
        api: Api,
        config: DisplayConfig,
        pretty: bool,
        options: RenderOptions,
    ) -> None:
        self.api = api
        self.config = config
        self.pretty = pretty
        self.options = options
        T = api.T
        self.fmt = options.fmt or config.default_format
        self.descendType = T.formats.get(self.fmt, config.slot_type) or config.slot_type

        baseTypes = (
            frozenset(options.base_types)
            if options.base_types is not None
            else config.base_types
        )
        subBaseTypes: set[str] = set()
        if baseTypes and baseTypes != {config.slot_type}:
            for bt in baseTypes:
                subBaseTypes |= config.descendant_types.get(bt, frozenset())
        self.baseTypes = baseTypes
        self.subBaseTypes = subBaseTypes

        (extra, extraIndirect) = _parse_extra(options.extra_features)
        self.extra = extra
        self.extraIndirect = extraIndirect
        self.edgeFeatures = {e for e in api.Eall() if e != "oslots"}
        self.highlights = options.highlights or {}
        self._cache: dict[int, dict[str, Any]] = {}

    def is_base(self, nType: str) -> bool:
        return nType != self.config.slot_type and (
            nType in self.baseTypes or nType in self.subBaseTypes
        )

    def dress(self, fragments: list[Fragment]) -> None:
        for frag in fragments:
            self._dress(frag)

    def _dress(self, frag: Fragment) -> None:
        props = self._node_props(frag.node, frag.type)
        frag.label = props["label"]
        frag.heading = props["heading"]
        frag.features = props["features"]
        frag.highlight = self.highlights.get(frag.node)

        config = self.config
        nType = frag.type
        isSlot = nType == config.slot_type
        isDescend = nType == self.descendType
        isBase = self.pretty and self.is_base(nType)

        if isSlot or isDescend or nType in config.lex_types:
            frag.text = props["text"]
            frag.children = []
        elif isBase:
            frag.base = True
            frag.text = self._fragment_text(frag)
            frag.children = []
        for child in frag.children:
            self._dress(child)

    def _fragment_text(self, frag: Fragment) -> str:
        eOslots = self.api.E.oslots.s
        slots = [int(s) for s in eOslots(frag.node) if frag.begin <= s <= frag.end]
        return str(self.api.T.text(slots, fmt=self.fmt))

    def _node_props(self, n: int, nType: str) -> dict[str, Any]:
        cached = self._cache.get(n)
        if cached is not None:
            return cached

        api = self.api
        config = self.config
        T = api.T
        typeDisplay = config.types.get(nType)
        tpl: str | bool = ""
        if typeDisplay is not None:
            tpl = typeDisplay.label if self.pretty else typeDisplay.template

        heading = None
        label = None
        text = None
        if nType in config.section_types:
            heading = section_ref(
                api,
                n,
                sep1=config.section_sep1,
                sep2=config.section_sep2,
                lang=self.options.lang,
            )
        elif nType in config.structure_types:
            heading = structure_ref(api, n)

        if (
            nType == config.slot_type
            or nType == self.descendType
            or nType in config.lex_types
        ):
            descend = (
                False
                if nType == self.descendType or nType in config.lex_types
                else None
            )
            text = T.text(n, fmt=self.fmt, descend=descend)

        if isinstance(tpl, str) and tpl:
            label = self._fill(n, nType, tpl)

        features = self._features(n, nType) if self.pretty else None

        props = {"label": label, "heading": heading, "text": text, "features": features}
        self._cache[n] = props
        return props

    def _value(self, n: int, feat: str, keep_none_values: bool = False) -> Any:
        api = self.api
        if feat in self.edgeFeatures:
            return None
        fObj = api.Fs(feat) if hasattr(api.F, feat) else None
        if fObj is None:
            return None
        value = fObj.v(n)
        if not keep_none_values and value in self.config.none_values:
            return None
        return value

    def _fill(self, n: int, nType: str, tpl: str) -> str:
        from cfabric.display.config import VAR_PATTERN

        # As in TF, templates show values like `NA` that feature lists suppress.
        def replace(match):
            value = self._value(n, match.group(1), keep_none_values=True)
            return "" if value is None else str(value)

        return VAR_PATTERN.sub(replace, tpl)

    def _features(self, n: int, nType: str) -> list[dict[str, Any]] | None:
        typeDisplay = self.config.types.get(nType)
        entries: list[tuple[str, str, str | None]] = []
        if typeDisplay is not None:
            entries += [
                (f, "bare", typeDisplay.features_bare_indirect.get(f))
                for f in typeDisplay.features_bare
            ]
            entries += [
                (f, "standard", typeDisplay.features_indirect.get(f))
                for f in typeDisplay.features
            ]
        configured = {f for (f, _, _) in entries}
        entries += [
            (f, "extra", self.extraIndirect.get(f))
            for f in self.extra
            if f not in configured
        ]

        L = self.api.L
        result = []
        seen = set()
        for name, kind, refType in entries:
            if name in seen:
                continue
            seen.add(name)
            refNode: int | None = n
            if refType is not None:
                ups = L.u(n, otype=refType)
                refNode = int(ups[0]) if ups else None
            if refNode is None:
                continue
            value = self._value(refNode, name)
            if value is None:
                continue
            item: dict[str, Any] = {
                "name": name,
                "value": value if isinstance(value, (int, str)) else str(value),
                "kind": kind,
            }
            if refType is not None:
                item["on"] = refType
            if name in self.extra:
                item["extra"] = True
            result.append(item)
        return result or None


def _parse_extra(features: Iterable[str]) -> tuple[list[str], dict[str, str]]:
    names: list[str] = []
    indirect: dict[str, str] = {}
    for spec in features or ():
        if not spec:
            continue
        parts = str(spec).split(":", 1)
        name = parts[-1]
        if name not in names:
            names.append(name)
        if len(parts) > 1:
            indirect[name] = parts[0]
    return (names, indirect)


# public API


def render_node(
    api: Api,
    config: DisplayConfig,
    n: int,
    mode: str = PRETTY,
    options: RenderOptions | None = None,
    in_tuple: bool = False,
) -> RenderTree:
    """Build the render tree of a node.

    Parameters
    ----------
    api: Api
        The loaded corpus.
    config: DisplayConfig
        Its display configuration.
    n: int
        The node to display.
    mode: str
        `pretty` (nested boxes with labels and features)
        or `plain` (running text with nested highlights).
    options: RenderOptions | None
        See `RenderOptions`.
    """
    if mode not in (PRETTY, PLAIN):
        raise ValueError(f"mode must be '{PRETTY}' or '{PLAIN}', not '{mode}'")
    options = options or RenderOptions()
    pretty = mode == PRETTY
    n = int(n)
    fragments = unravel(
        api, config, n, pretty=pretty, options=options, in_tuple=in_tuple
    )
    dresser = _Dresser(api, config, pretty, options)
    dresser.dress(fragments)
    nType = api.F.otype.v(n)
    return RenderTree(
        node=n,
        type=nType,
        mode=mode,
        fmt=dresser.fmt,
        direction=config.ltr(dresser.fmt),
        heading=section_ref(
            api,
            n,
            sep1=config.section_sep1,
            sep2=config.section_sep2,
            lang=options.lang,
        ),
        fragments=fragments,
    )


def tuple_containers(
    api: Api,
    config: DisplayConfig,
    tup: Iterable[int],
    condense_type: str | None = None,
) -> list[int]:
    """The nodes to display a result tuple in, e.g. the verses that contain it.

    Nodes of the condense type are their own container; smaller nodes are shown
    in the container(s) of their first and last slot; bigger nodes are shown
    as themselves.
    """
    F = api.F
    L = api.L
    N = api.N
    eOslots = api.E.oslots.s
    fOtypeV = F.otype.v
    slotType = F.otype.slotType
    otypeRank = N.otypeRank
    condenseType = condense_type or config.condense_type or slotType
    condenseRank = otypeRank[condenseType]

    containers: set[int] = set()
    for n in tup:
        n = int(n)
        nType = fOtypeV(n)
        if nType == condenseType:
            containers.add(n)
        elif nType == slotType:
            up = L.u(n, otype=condenseType)
            if up:
                containers.add(int(up[0]))
        elif otypeRank[nType] < condenseRank:
            slots = eOslots(n)
            for s in (slots[0], slots[-1]):
                up = L.u(int(s), otype=condenseType)
                if up:
                    containers.add(int(up[0]))
        else:
            containers.add(n)
    return [int(c) for c in N.sortNodes(containers)]


def render_tuple(
    api: Api,
    config: DisplayConfig,
    tup: Iterable[int],
    mode: str = PRETTY,
    options: RenderOptions | None = None,
    colors: dict[int, str] | None = None,
) -> list[RenderTree]:
    """Render a result tuple.

    In pretty mode, the tuple is shown inside its containers (see
    `tuple_containers`), with the tuple's nodes highlighted. In plain mode,
    each node of the tuple is rendered on its own.

    Parameters
    ----------
    colors: dict[int, str] | None
        Highlight color per position in the tuple (1-based, as in query
        templates). Positions without a color get the default highlight.
    """
    options = options or RenderOptions()
    tup = tuple(int(n) for n in tup)
    colors = colors or {}
    highlights = dict(options.highlights or {})
    for i, n in enumerate(tup):
        highlights.setdefault(n, colors.get(i + 1, ""))
    opts = RenderOptions(**{**options.__dict__, "highlights": highlights})

    if mode == PLAIN:
        return [render_node(api, config, n, PLAIN, opts, in_tuple=True) for n in tup]
    return [
        render_node(api, config, c, PRETTY, opts)
        for c in tuple_containers(api, config, tup, opts.condense_type)
    ]
