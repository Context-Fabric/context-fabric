"""
# Pre-compute data.

For CF to work efficiently, some derived data needs to be pre-computed.
The pre-computed data has a similar function as indexes in a database.

Pre-computation is triggered when `cfabric.fabric.Fabric` loads features, and
the order and nature of the steps is configured in
`cfabric.fabric.PRECOMPUTE`.

The functions in this module implement those tasks.
"""

from __future__ import annotations

import collections
import functools
import array
from collections.abc import Callable
from typing import TYPE_CHECKING, Any

from cfabric.utils.helpers import itemize

if TYPE_CHECKING:
    from cfabric.core.api import Api

# Type aliases for clarity
InfoFunc = Callable[..., None]
ErrorFunc = Callable[..., None]
OtypeData = tuple[tuple[str, ...], int, int, str]
OslotsData = tuple[tuple[tuple[int, ...], ...], dict[str, str]]
OtextData = dict[str, str]
LevelsData = tuple[tuple[str, float, int, int], ...]
OrderData = tuple[int, ...]
RankData = array.array[int]
LevUpData = tuple[tuple[int, ...], ...]
LevDownData = tuple[array.array[int], ...]
BoundaryData = tuple[tuple[tuple[int, ...], ...], tuple[tuple[int, ...], ...]]
SectionsResult = dict[str, Any]
StructureResult = tuple[
    dict[int, tuple[tuple[str, str | int | None], ...]],
    dict[tuple[tuple[str, str | int | None], ...], int],
    dict[tuple[tuple[str, str | int | None], ...], tuple[int, ...]],
    tuple[int, ...],
    dict[int, int],
    dict[int, tuple[int, ...]],
]
CharactersResult = dict[str, list[tuple[str, int]]]


def levels(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    oslots: OslotsData,
    otext: OtextData,
) -> LevelsData:
    """Computes level data.

    For each node type, compute the average number of slots occupied by its nodes,
    and order the node types on that.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    oslots: iterable
        The data of the `oslots` feature.
    otext: iterable
        The data of the `otext` feature.

    Returns
    -------
    tuple
        An ordered tuple, each member with the information of a node type:

        *   node type name
        *   average number of slots contained in the nodes of this type
        *   first node of this type
        *   last node of this type

    The order of the tuple is descending by average number of slots per node of that
    type.

    Notes
    -----
    !!! explanation "Level computation and customization"
        All node types have a level, defined by the average amount of slots object of
        that type usually occupy. The bigger the average object, the lower the levels.
        Books have the lowest level, words the highest level.

        However, this can be overruled. Suppose you have a node type `phrase` and above
        it a node type `cluster`, i.e. phrases are contained in clusters, but not vice
        versa. If all phrases are contained in clusters, and some clusters have more
        than one phrase, the automatic level ranking of node types works out well in
        this case. But if clusters only have very small phrases, and the big phrases do
        not occur in clusters, then the algorithm may assign a lower rank to clusters
        than to phrases.

        In general, it is too expensive to try to compute the levels in a sophisticated
        way. In order to remedy cases where the algorithm assigns wrong levels, you can
        add a `@levels` and / or `@levelsConstraint` key to the `otext`
        configuration feature.
        See `cfabric.text`.
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    oslots_data = oslots[0]
    levelOrder = otext.get("levels", None)

    levelRank: dict[str, int] = {}
    if levelOrder is not None:
        levelRank = {level: i for (i, level) in enumerate(levelOrder.split(","))}
    otypeCount: collections.Counter[str] = collections.Counter()
    otypeMin: dict[str, int] = {}
    otypeMax: dict[str, int] = {}
    slotSetLengths: collections.Counter[str] = collections.Counter()
    info("get ranking of otypes")
    for k in range(len(oslots_data)):
        ntp = otype_data[k]
        otypeCount[ntp] += 1
        slotSetLengths[ntp] += len(oslots_data[k])
        tfn = k + maxSlot + 1
        if ntp not in otypeMin:
            otypeMin[ntp] = tfn
        if ntp not in otypeMax or otypeMax[ntp] < tfn:
            otypeMax[ntp] = tfn
    sortKey: Callable[[tuple[str, float, int, int]], tuple[int, ...] | tuple[float]] = (
        (lambda x: (-x[1],))
        if levelOrder is None
        else (lambda x: (-1, levelRank[x[0]]) if x[0] in levelRank else (1, int(-x[1])))
    )
    result: list[tuple[str, float, int, int]] = sorted(
        (
            (
                ntp,
                slotSetLengths[ntp] / otypeCount[ntp],
                otypeMin[ntp],
                otypeMax[ntp],
            )
            for ntp in otypeCount
        ),
        key=sortKey,
    ) + [(slotType, 1, 1, maxSlot)]
    resultIndex: dict[str, int] = {r[0]: i for (i, r) in enumerate(result)}

    levelConstraintsSpec = otext.get("levelConstraints", None)

    if levelConstraintsSpec:
        levelConstraints = levelConstraintsSpec.split(";")

        for levelConstraint in levelConstraints:
            (smaller, biggerSpec) = levelConstraint.strip().split("<")
            smaller = smaller.strip()
            biggers = {b.strip() for b in biggerSpec.strip().split(",")}
            highestBigIndex = max(resultIndex.get(tp, 0) for tp in biggers)
            smallerIndex = resultIndex.get(smaller, len(result))
            if smallerIndex <= highestBigIndex:
                x = result.pop(smallerIndex)
                result.insert(highestBigIndex, x)
            resultIndex = {r[0]: i for (i, r) in enumerate(result)}

    info("results:")
    for otp, av, omin, omax in result:
        info(f"{otp:<15}: {round(av, 2):>8} {{{omin}-{omax}}}", tm=False)
    return tuple(result)


def order(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    oslots: OslotsData,
    levels: LevelsData,
) -> OrderData:
    """Computes order data for the canonical ordering.

    The canonical ordering between nodes is defined in terms of the slots that
    nodes contain, and if that is not decisive, the rank of the node type is taken
    into account, and if that is still not decisive, the node itself is taken into
    account.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    oslots: iterable
        The data of the `oslots` feature.
    levels: tuple
        The data of the `levels` pre-computation step.

    Returns
    -------
    tuple
        All nodes, slot and nonslot, in canonical order.

    See Also
    --------
    cfabric.nodes: canonical ordering
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    oslots_data = oslots[0]
    info("assigning otype levels to nodes")
    otypeLevels: dict[str, int] = dict(((x[0], i) for (i, x) in enumerate(reversed(levels))))

    def otypeRank(n: int) -> int:
        return otypeLevels[slotType if n < maxSlot + 1 else otype_data[n - maxSlot - 1]]

    def before(na: int, nb: int) -> int:
        if na < maxSlot + 1:
            a = na
            sa = {a}
        else:
            a = na - maxSlot
            sa = set(oslots_data[a - 1])
        if nb < maxSlot + 1:
            b = nb
            sb = {b}
        else:
            b = nb - maxSlot
            sb = set(oslots_data[b - 1])
        oa = otypeRank(na)
        ob = otypeRank(nb)
        if sa == sb:
            return (
                (-1 if na < nb else 1 if na > nb else 0)
                if oa == ob
                else -1
                if oa > ob
                else 1
            )
        if sa > sb:
            return -1
        if sa < sb:
            return 1
        am = min(sa - sb)
        bm = min(sb - sa)
        return -1 if am < bm else 1 if bm < am else 0

    canonKey = functools.cmp_to_key(before)
    info("sorting nodes")
    nodes = sorted(range(1, maxNode + 1), key=canonKey)
    # return array.array("I", nodes)
    return tuple(nodes)


def rank(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    order: OrderData,
) -> RankData:
    """Computes rank data.

    The rank of a node is its place in among the other nodes in the
    canonical order (see `cfabric.nodes`).

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    order: tuple
        The data of the `order` feature.

    Returns
    -------
    tuple
        The ranks of all nodes, slot and nonslot, with respect to the canonical order.
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    info("ranking nodes")
    nodesRank: dict[int, int] = dict(((n, i) for (i, n) in enumerate(order)))
    return array.array("I", (nodesRank[n] for n in range(1, maxNode + 1)))
    # return tuple((nodesRank[n] for n in range(1, maxNode + 1)))


def levUp(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    oslots: OslotsData,
    rank: RankData,
) -> LevUpData:
    """Computes level-up data.

    Level-up data is used by the API function `cfabric.locality.Locality.u`.

    This function computes the embedders of a node by looking them up from
    the level-up data.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    oslots: iterable
        The data of the `oslots` feature.
    rank: tuple
        The data of the `rank` pre-computation step.

    Returns
    -------
    tuple
        The `n`-th member is a tuple of the embedder nodes of `n`.
        Those tuples are sorted in canonical order (`cfabric.nodes`).

    Notes
    -----
    !!! hint "Memory efficiency"
        Many nodes have the same tuple of embedders.
        Those embedder tuples will be reused for those nodes.

    Warnings
    --------
    It is not advisable to use this data directly by `C.levUp.data`,
    it is far better to use the `cfabric.locality.Locality.u` function.

    Only when every bit of performance waste has to be squeezed out,
    this raw data might be a deal.
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    oslots_data = oslots[0]
    info("making inverse of edge feature oslots")
    oslotsInv: dict[int, set[int]] = {}
    for k, mList in enumerate(oslots_data):
        for m in mList:
            oslotsInv.setdefault(m, set()).add(k + 1 + maxSlot)
    info("listing embedders of all nodes")
    embedders: list[tuple[int, ...]] = []
    for n in range(1, maxSlot + 1):
        contentEmbedders = oslotsInv.get(n, tuple())
        embedders.append(
            tuple(
                sorted(
                    (m for m in contentEmbedders if m != n),
                    key=lambda k: -rank[k - 1],
                )
            )
        )
    for n in range(maxSlot + 1, maxNode + 1):
        mList = oslots_data[n - maxSlot - 1]
        if len(mList) == 0:
            embedders.append(tuple())
        else:
            contentEmbedders = functools.reduce(
                lambda x, y: x & oslotsInv[y],
                mList[1:],
                oslotsInv[mList[0]],
            )
            embedders.append(
                tuple(
                    sorted(
                        (m for m in contentEmbedders if m != n),
                        key=lambda k: -rank[k - 1],
                    )
                )
            )
    # reuse embedder tuples, because lots of nodes share embedders
    seen: dict[tuple[int, ...], tuple[int, ...]] = {}
    embeddersx: list[tuple[int, ...]] = []
    for t in embedders:
        if t not in seen:
            # seen[t] = array.array("I", t)
            seen[t] = tuple(t)
        embeddersx.append(seen[t])
    return tuple(embeddersx)


def levDown(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    levUp: LevUpData,
    rank: RankData,
) -> LevDownData:
    """Computes level-down data.

    Level-down data is used by the API function `cfabric.locality.Locality.d`.

    This function computes the embedded nodes of a node by looking them up from
    the level-down data.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    levUp: iterable
        The data of the `levUp` pre-computation step.
    rank: tuple
        The data of the `rank` pre-computation step.

    Returns
    -------
    tuple
        The `n`-th member is an tuple of the embedded nodes of `n + maxSlot`.
        Those tuples are sorted in canonical order (`cfabric.nodes`).

    !!! hint "Memory efficiency"
        Slot nodes do not have embedded nodes, so they do not have to occupy
        space in this tuple. Hence the first member are the embedded nodes
        of node `maxSlot + 1`.

    !!! caution "Use with care"
        It is not advisable to use this data directly by `C.levDown.data`,
        it is far better to use the `cfabric.locality.Locality.d` function.

        Only when every bit of performance waste has to be squeezed out,
        this raw data might be a deal.
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    info("inverting embedders")
    inverse: dict[int, set[int]] = {}
    for n in range(maxSlot + 1, maxNode + 1):
        for m in levUp[n - 1]:
            inverse.setdefault(m, set()).add(n)
    info("turning embeddees into list")
    embeddees: list[array.array[int]] = []
    for n in range(maxSlot + 1, maxNode + 1):
        embeddees.append(
            array.array("I", sorted(inverse.get(n, []), key=lambda m: rank[m - 1]))
            # tuple(sorted(inverse.get(n, []), key=lambda m: rank[m - 1]))
        )
    return tuple(embeddees)


def characters(
    info: InfoFunc,
    error: ErrorFunc,
    otext: OtextData,
    tFormats: dict[str, tuple[str, ...]],
    *tFeats: tuple[str, dict[int, Any] | None],
) -> CharactersResult:
    """Computes character data.

    For each text format, a frequency list of the characters in that format
    is made.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otext: iterable
        The data of the `otext` feature.
    tFormats: dict
        Dictionary keyed by text format and valued by the tuple of features
        used in that format.
    tFeats: iterable
        Each `tFeat` is the name and the data of a text feature.
        i.e. a feature used in text formats.

    Returns
    -------
    dict
        Keyed by format valued by a frequency dict, which is
        itself keyed by single characters and valued by the frequency
        of that character in the whole corpus when rendered with that format.
    """

    charFreqsByFeature: dict[str, dict[str, int]] = {}

    for tFeat, data in tFeats:
        freqList: collections.Counter[Any] = collections.Counter()
        if data is not None:
            for v in data.values():
                freqList[v] += 1
        charFreq: dict[str, int] = collections.defaultdict(lambda: 0)
        for v, freq in freqList.items():
            for c in str(v):
                charFreq[c] += freq
        charFreqsByFeature[tFeat] = charFreq

    charFreqsByFmt: dict[str, list[tuple[str, int]]] = {}

    for fmt, tFeatures in sorted(tFormats.items()):
        charFreq = collections.defaultdict(lambda: 0)
        for tFeat in tFeatures:
            thisCharFreq = charFreqsByFeature[tFeat]
            for c, freq in thisCharFreq.items():
                charFreq[c] += freq
        charFreqsByFmt[fmt] = sorted(x for x in charFreq.items())

    return charFreqsByFmt


def boundary(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    oslots: OslotsData,
    rank: RankData,
) -> BoundaryData:
    """Computes boundary data.

    For each slot, the nodes that start at that slot and the nodes that end
    at that slot are collected.

    Boundary data is used by the API functions
    `cfabric.locality.Locality.p`.
    and
    `cfabric.locality.Locality.n`.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    oslots: iterable
        The data of the `oslots` feature.
    rank: tuple
        The data of the `rank` pre-computation step.

    Returns
    -------
    tuple
        *   first: tuple of tuple
            The `n`-th member is the tuple of nodes that start at slot `n`,
            ordered in *reversed* canonical order (`cfabric.nodes`);
        *   last: tuple of tuple
            The `n`-th member is the tuple of nodes that end at slot `n`,
            ordered in canonical order;

    Notes
    -----
    !!! hint "why  reversed canonical order?"
        Just for symmetry.
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    oslots_data = oslots[0]
    firstSlotsD: dict[int, list[int]] = {}
    lastSlotsD: dict[int, list[int]] = {}

    for node, slots in enumerate(oslots_data):
        realNode = node + 1 + maxSlot
        firstSlotsD.setdefault(slots[0], []).append(realNode)
        lastSlotsD.setdefault(slots[-1], []).append(realNode)

    firstSlots: tuple[tuple[int, ...], ...] = tuple(
        tuple(sorted(firstSlotsD.get(n, []), key=lambda node: -rank[node - 1]))
        # array.array("I", sorted(firstSlotsD.get(n, []),
        # key=lambda node: -rank[node - 1]))
        for n in range(1, maxSlot + 1)
    )
    lastSlots: tuple[tuple[int, ...], ...] = tuple(
        tuple(sorted(lastSlotsD.get(n, []), key=lambda node: rank[node - 1]))
        # array.array("I", sorted(lastSlotsD.get(n, []),
        # key=lambda node: rank[node - 1]))
        for n in range(1, maxSlot + 1)
    )
    return (firstSlots, lastSlots)


def sections(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    oslots: OslotsData,
    otext: OtextData,
    levUp: LevUpData,
    levDown: LevDownData,
    levels: LevelsData,
    *sFeats: dict[int, Any],
) -> SectionsResult:
    """Computes section data.

    CF datasets may define up to three section levels, roughly corresponding
    with a volume, a chapter, a paragraph.

    If the corpus has a richer section structure, it is also possible
    a different, more flexible and more extensive nest of structural sections.
    See `structure`.

    CF must be able to go from sections at one level to the sections
    at one level lower. It must also be able to map section headings
    to nodes. For this, the section features are needed, since they
    contain the section headings.

    We also map the sections to sequence numbers and back, at each level, e.g.
    in the Hebrew Bible `Genesis` is mapped to 1, `Exodus` to 2, etc.
    We also do it for integer values components, and we make sure that the first section
    at each level gets sequence number `1`.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    oslots: iterable
        The data of the `oslots` feature.
    otext: iterable
        The data of the `otext` feature.
    levUp: tuple
        The data of the `levUp` pre-computation step.
    levDown: tuple
        The data of the `levDown` pre-computation step.
    levels: tuple
        The data of the `levels` pre-computation step.
    sFeats: iterable
        Each `sFeat` is the data of a section feature.

    Returns
    -------
    dict
        We have the following items:

        *   `sec1`:
            Mapping from section-level-1 nodes to mappings from
            section-level-2 headings to section-level-2 nodes.
        *   `sec2`:
            Mapping from section-level-1 nodes to mappings from
            section-level-2 headings to mappings from
            section-level-3 headings to section-level-3 nodes.
        *   `seqFromNode`:
            Mapping from tuples of section nodes to tuples of sequence numbers.
            Only if there are precisely 3 section levels, otherwise this is an
            empty dictionary.
        *   `nodeFromSeq`:
            Mapping from tuples of section sequence numbers to tuples of nodes.
            Only if there are precisely 3 section levels, otherwise this is an
            empty dictionary.

    Warnings
    --------
    Note that the terms `book`, `chapter`, `verse` are not baked into CF.
    It is the corpus data, especially the `otext` configuration feature that
    spells out the names of the sections.

    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    oslots_data = oslots[0]
    support: dict[str, tuple[int, int]] = {level[0]: (level[2], level[3]) for level in levels}
    sTypes: list[str] = itemize(otext["sectionTypes"], ",")
    sec1: dict[int, dict[Any, int]] = {}
    sec2: dict[int, dict[Any, dict[Any, int]]] = {}
    seqFromNode: dict[int, tuple[int, ...]] = {}
    nodeFromSeq: dict[tuple[int, ...], int] = {}
    nestingProblems: collections.Counter[str] = collections.Counter()

    if len(sTypes) < 2:
        pass

    elif len(sTypes) < 3:
        c1 = 0
        support1 = support[sTypes[1]]
        for n1 in range(support1[0], support1[1] + 1):
            n0s = tuple(x for x in levUp[n1 - 1] if otype_data[x - maxSlot - 1] == sTypes[0])
            if not n0s:
                nestingProblems[
                    f"section {sTypes[1]} without containing {sTypes[0]}"
                ] += 1
                continue
            n0 = n0s[0]
            n1head = sFeats[1].get(n1, None)
            if n1head is None:
                nestingProblems[f"{sTypes[1]}-node {n1} has no section heading"] += 1
            if n0 not in sec1:
                sec1[n0] = {}
            if n1head not in sec1[n0]:
                sec1[n0][n1head] = n1
                c1 += 1
        info(f"{c1} {sTypes[1]}s indexed")

    else:
        c1 = 0
        c2 = 0
        support2 = support[sTypes[2]]
        for n2 in range(support2[0], support2[1] + 1):
            n0s = tuple(x for x in levUp[n2 - 1] if otype_data[x - maxSlot - 1] == sTypes[0])
            if not n0s:
                nestingProblems[
                    f"section {sTypes[2]} without containing {sTypes[0]}"
                ] += 1
                continue
            n0 = n0s[0]

            n1s = tuple(x for x in levUp[n2 - 1] if otype_data[x - maxSlot - 1] == sTypes[1])
            if not n1s:
                nestingProblems[
                    f"section {sTypes[2]} without containing {sTypes[1]}"
                ] += 1
                continue
            n1 = n1s[0]

            n1head = sFeats[1].get(n1, None)
            if n1head is None:
                nestingProblems[f"{sTypes[1]}-node {n1} has no section heading"] += 1
            n2head = sFeats[2].get(n2, None)
            if n2head is None:
                nestingProblems[f"{sTypes[2]}-node {n2} has no section heading"] += 1

            if n0 not in sec1:
                sec1[n0] = {}
            if n1head not in sec1[n0]:
                sec1[n0][n1head] = n1
                c1 += 1
            sec2.setdefault(n0, {}).setdefault(n1head, {})[n2head] = n2
            c2 += 1
        info(f"{c1} {sTypes[1]}s and {c2} {sTypes[2]}s indexed")

    if nestingProblems:
        for msg, amount in sorted(nestingProblems.items()):
            error(f"WARNING: {amount:>4} x {msg}")

    c0 = 0

    if len(sTypes) == 3:
        support0 = support[sTypes[0]]

        for n0 in range(support0[0], support0[1] + 1):
            c0 += 1
            seqFromNode[n0] = (c0,)
            nodeFromSeq[(c0,)] = n0

            c1 = 0

            for n1 in (
                x
                for x in levDown[n0 - maxSlot - 1]
                if otype_data[x - maxSlot - 1] == sTypes[1]
            ):
                c1 += 1
                c2 = 0
                seqFromNode[n1] = (c0, c1)
                nodeFromSeq[(c0, c1)] = n1

                for n2 in (
                    x
                    for x in levDown[n1 - maxSlot - 1]
                    if otype_data[x - maxSlot - 1] == sTypes[2]
                ):
                    c2 += 1
                    seqFromNode[n2] = (c0, c1, c2)
                    nodeFromSeq[(c0, c1, c2)] = n2

    return dict(sec1=sec1, sec2=sec2, seqFromNode=seqFromNode, nodeFromSeq=nodeFromSeq)


def sectionsFromApi(
    api: Api,
    sectionTypes: list[str],
    sectionFeats: list[str],
) -> SectionsResult | None:
    """Compute sections data using API methods.

    This is an alternative to `sections()` that works with the high-level API
    rather than raw data structures. Used when loading from .cfm format.

    Parameters
    ----------
    api : Api
        The CF API object with F, L, Fs attributes
    sectionTypes : list
        Section type names, e.g. ['book', 'chapter', 'verse']
    sectionFeats : list
        Section feature names, e.g. ['book', 'chapter', 'verse']

    Returns
    -------
    dict
        Same structure as sections(): {sec1, sec2, seqFromNode, nodeFromSeq}
    """
    if len(sectionTypes) < 2 or len(sectionFeats) < 2:
        return None

    sec1: dict[int, dict[Any, int]] = {}
    sec2: dict[int, dict[Any, dict[Any, int]]] = {}
    seqFromNode: dict[int, tuple[int, ...]] = {}
    nodeFromSeq: dict[tuple[int, ...], int] = {}

    F = api.F
    L = api.L
    Fs = api.Fs

    feat1 = Fs(sectionFeats[1])
    feat2 = Fs(sectionFeats[2]) if len(sectionFeats) > 2 else None

    # Index level-1 sections (e.g., chapters)
    for n1 in F.otype.s(sectionTypes[1]):
        n0_list = [x for x in L.u(n1) if F.otype.v(x) == sectionTypes[0]]
        if not n0_list:
            continue
        n0 = n0_list[0]
        heading1 = feat1.v(n1)

        if n0 not in sec1:
            sec1[n0] = {}
        if heading1 not in sec1[n0]:
            sec1[n0][heading1] = n1

    # Index level-2 sections (e.g., verses) if present
    if len(sectionTypes) >= 3 and feat2 is not None:
        for n2 in F.otype.s(sectionTypes[2]):
            embedders = L.u(n2)
            n0_list = [x for x in embedders if F.otype.v(x) == sectionTypes[0]]
            n1_list = [x for x in embedders if F.otype.v(x) == sectionTypes[1]]

            if not n0_list or not n1_list:
                continue

            n0 = n0_list[0]
            n1 = n1_list[0]
            heading1 = feat1.v(n1)
            heading2 = feat2.v(n2)

            sec2.setdefault(n0, {}).setdefault(heading1, {})[heading2] = n2

    return dict(sec1=sec1, sec2=sec2, seqFromNode=seqFromNode, nodeFromSeq=nodeFromSeq)


def structure(
    info: InfoFunc,
    error: ErrorFunc,
    otype: OtypeData,
    oslots: OslotsData,
    otext: OtextData,
    rank: RankData,
    levUp: LevUpData,
    *sFeats: dict[int, Any],
) -> StructureResult | tuple[dict[Any, Any], dict[Any, Any]]:
    """Computes structure data.

    If the corpus has a rich section structure, it is possible to define
    a flexible and extensive nest of structural sections.

    Independent of this,
    CF datasets may also define up to three section levels,
    roughly corresponding with a volume, a chapter, a paragraph.
    See `sections`.

    CF must be able to go from sections at one level to the sections
    at one level lower. It must also be able to map section headings
    to nodes. For this, the section features are needed, since they
    contain the section headings.

    Parameters
    ----------
    info: function
        Method to write informational messages to the console.
    error: function
        Method to write error messages to the console.
    otype: iterable
        The data of the `otype` feature.
    oslots: iterable
        The data of the `oslots` feature.
    otext: iterable
        The data of the `otext` feature.
    rank: tuple
        The data of the `rank` pre-computation step.
    levUp: tuple
        The data of the `levUp` pre-computation step.
    sFeats: iterable
        Each `sFeat` the data of a section feature.

    Returns
    -------
    tuple
        *   `headingFromNode` (Mapping from nodes to section keys)
        *   `nodeFromHeading` (Mapping from section keys to nodes)
        *   `multiple`
        *   `top`
        *   `up`
        *   `down`

    Notes
    -----
    A section key of a structural node is obtained by going a level up from
    that node, retrieving the heading of that structural node, then going up again,
    and so on till a top node is reached. The tuple of headings obtained in this way
    is the  section key.
    """

    (otype_data, maxSlot, maxNode, slotType) = otype
    oslots_data = oslots[0]
    sTypeList: list[str] = itemize(otext["structureTypes"], ",")
    nsTypes = len(sTypeList)
    nsFeats = len(sFeats)

    if nsTypes != nsFeats:
        error(
            f"WARNING: {nsTypes} structure levels but {nsFeats} corresponding features"
        )
        return ({}, {})

    sTypes = set(sTypeList)
    if len(sTypes) != nsTypes:
        error("WARNING: duplicate structure levels")
        return ({}, {})

    higherTypes: dict[str, set[str]] = collections.defaultdict(set)
    for i, highType in enumerate(sTypeList):
        for lowType in sTypeList[i:]:
            higherTypes[lowType].add(highType)

    featFromType: dict[str, dict[int, Any]] = {sTypeList[i]: sFeats[i] for i in range(nsTypes)}

    multiple: dict[tuple[tuple[str, Any], ...], list[int]] = collections.defaultdict(list)
    headingFromNode: dict[int, tuple[tuple[str, Any], ...]] = {}
    nodeFromHeading: dict[tuple[tuple[str, Any], ...], int] = {}

    for n in range(maxSlot + 1, maxNode + 1):
        nType = otype_data[n - maxSlot - 1]
        if nType not in sTypes:
            continue
        ups = (u for u in levUp[n - 1] if otype_data[u - maxSlot - 1] in higherTypes[nType])
        sKey: tuple[tuple[str, Any], ...] = tuple(
            reversed(
                tuple(
                    (
                        otype_data[x - maxSlot - 1],
                        featFromType[otype_data[x - maxSlot - 1]].get(x, None),
                    )
                    for x in (n, *ups)
                )
            )
        )

        if sKey in nodeFromHeading:
            if sKey not in multiple:
                multiple[sKey].append(nodeFromHeading[sKey])
            multiple[sKey].append(n)
        nodeFromHeading[sKey] = n
        headingFromNode[n] = sKey
    multiple_final: dict[tuple[tuple[str, Any], ...], tuple[int, ...]] = {
        sKey: tuple(sorted(ns, key=lambda n: rank[n - 1]))
        for (sKey, ns) in multiple.items()
    }

    top: tuple[int, ...] = tuple(
        sorted(
            (n for (n, h) in headingFromNode.items() if len(h) == 1),
            key=lambda n: rank[n - 1],
        )
    )

    up: dict[int, int] = {}
    for n, heading in headingFromNode.items():
        lHeading = len(heading)
        if lHeading == 1:
            continue
        upNode: int | None = None
        for i in range(lHeading - 1, 0, -1):
            upHeading = heading[0:i]
            upNode = nodeFromHeading.get(upHeading, None)
            if upNode is not None:
                up[n] = upNode
                break

    down: dict[int, list[int]] = {}
    for n, heading in headingFromNode.items():
        if len(heading) == 1:
            continue
        down.setdefault(up[n], []).append(n)

    down_final: dict[int, tuple[int, ...]] = {n: tuple(sorted(ms, key=lambda m: rank[m - 1])) for (n, ms) in down.items()}

    return (headingFromNode, nodeFromHeading, multiple_final, top, up, down_final)
