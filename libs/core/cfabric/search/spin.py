"""
# Search pre-processing
"""

from __future__ import annotations

import logging
import types
from random import randrange
from inspect import signature
from typing import TYPE_CHECKING, Any

import numpy as np

if TYPE_CHECKING:
    from cfabric.search.searchexe import SearchExe

from cfabric.search.syntax import (
    reTp,
    cleanParent,
    QWHERE,
    QWITHOUT,
    QWITH,
    QHAVE,
    QOR,
    QEND,
)
from cfabric.utils.helpers import project
from cfabric.storage.string_pool import StringPool, IntFeatureArray

logger = logging.getLogger(__name__)

# SPINNING ###


def _spinAtom(searchExe: SearchExe, q: int) -> None:
    """Build the initial candidate set (yarn) for a search atom.

    This function filters nodes by type and feature constraints. When features
    are backed by mmap storage (StringPool/IntFeatureArray), we use vectorized
    numpy operations for significant performance improvement.
    """
    F = searchExe.api.F
    Fs = searchExe.api.Fs
    maxNode = F.otype.maxNode
    qnodes = searchExe.qnodes
    sets = searchExe.sets

    (otype, features, src, quantifiers) = qnodes[q]
    featureList = sorted(features.items())

    # Get initial node set based on type
    nodeSet = (
        range(1, maxNode + 1)
        if otype == "."
        else sets[otype]
        if sets is not None and otype in sets
        else F.otype.s(otype)
    )

    # Convert to set for fast operations
    if isinstance(nodeSet, range):
        yarn = set(nodeSet)
    else:
        yarn = set(nodeSet)

    # Apply feature constraints
    for ft, val in featureList:
        feature = Fs(ft)
        feature_data = feature._data if hasattr(feature, '_data') else None
        is_mmap = isinstance(feature_data, (StringPool, IntFeatureArray))

        if is_mmap and _can_vectorize_constraint(val):
            # Use vectorized filtering for mmap-backed features
            yarn = _vectorized_filter(yarn, feature_data, val)
        else:
            # Fall back to per-node lookup for complex constraints
            yarn = _scalar_filter(yarn, feature, val)

        # Early exit if no candidates remain
        if not yarn:
            break

    if quantifiers:
        for quantifier in quantifiers:
            yarn = _doQuantifier(searchExe, yarn, src, quantifier)
    searchExe.yarns[q] = yarn


def _can_vectorize_constraint(val: Any) -> bool:
    """Check if a constraint can be handled with vectorized operations.

    Vectorizable constraints:
    - None (feature must be missing)
    - True (feature must exist)
    - (True, set) for value in set (ident=True)
    - (False, set) for value not in set (ident=False)
    - (None, True) for any value exists

    Non-vectorizable:
    - Functions (custom predicates)
    - Regex patterns
    """
    if val is None or val is True:
        return True
    if isinstance(val, (types.FunctionType, reTp)):
        return False
    if isinstance(val, tuple) and len(val) == 2:
        ident, inner_val = val
        # Can vectorize set membership checks
        if ident is True or ident is False:
            return isinstance(inner_val, (set, frozenset))
        # (None, True) means any value exists - vectorizable
        if ident is None and inner_val is True:
            return True
    return False


def _vectorized_filter(
    yarn: set[int],
    feature_data: StringPool | IntFeatureArray,
    val: Any
) -> set[int]:
    """Apply constraint using vectorized numpy operations.

    Returns filtered yarn as a set.
    """
    nodes = list(yarn)
    if not nodes:
        return set()

    if val is None:
        # Feature must be missing
        result = feature_data.filter_missing_value(nodes)
    elif val is True:
        # Feature must exist (have any value)
        result = feature_data.filter_has_value(nodes)
    elif isinstance(val, tuple):
        ident, inner_val = val
        if ident is None and inner_val is True:
            # Any value exists
            result = feature_data.filter_has_value(nodes)
        elif ident is True:
            # Value must be in set
            if len(inner_val) == 1:
                # Single value - use filter_by_value
                single_val = next(iter(inner_val))
                result = feature_data.filter_by_value(nodes, single_val)
            else:
                result = feature_data.filter_by_values(nodes, inner_val)
        elif ident is False:
            # Value must NOT be in set (exclusion)
            # Get nodes that have the excluded values, then subtract
            if len(inner_val) == 1:
                single_val = next(iter(inner_val))
                excluded = set(feature_data.filter_by_value(nodes, single_val))
            else:
                excluded = set(feature_data.filter_by_values(nodes, inner_val))
            return yarn - excluded
        else:
            # Fallback - shouldn't reach here if _can_vectorize_constraint is correct
            return yarn
    else:
        return yarn

    return set(result)


def _scalar_filter(yarn: set[int], feature: Any, val: Any) -> set[int]:
    """Apply constraint using per-node lookup (fallback for complex constraints)."""
    result = set()
    for n in yarn:
        fval = feature.v(n)
        if val is None:
            if fval is None:
                result.add(n)
        elif val is True:
            if fval is not None:
                result.add(n)
        elif isinstance(val, types.FunctionType):
            if val(fval):
                result.add(n)
        elif isinstance(val, reTp):
            if fval is not None and val.search(fval):
                result.add(n)
        else:
            (ident, inner_val) = val
            if ident is None and inner_val is True:
                if fval is not None:
                    result.add(n)
            elif ident:
                if fval in inner_val:
                    result.add(n)
            else:
                if fval not in inner_val:
                    result.add(n)
    return result


def _doQuantifier(
    searchExe: SearchExe,
    yarn: set[int],
    atom: str,
    quantifier: tuple[str, list[str], str, int],
) -> set[int]:
    from .searchexe import SearchExe

    (quKind, quTemplates, parentName, ln) = quantifier
    showQuantifiers = searchExe.showQuantifiers
    silent = searchExe.silent
    level = searchExe.level
    universe = yarn
    cleanAtom = cleanParent(atom, parentName)
    offset = searchExe.offset + ln

    if showQuantifiers:
        logger.info(f'"Quantifier on "{cleanAtom}"')

    if quKind == QWITHOUT:
        queryN = "\n".join((cleanAtom, quTemplates[0]))
        exe = SearchExe(
            searchExe.api,
            queryN,
            outerTemplate=searchExe.outerTemplate,
            quKind=quKind,
            level=level + 1,
            offset=offset,
            sets=searchExe.sets,
            shallow=True,
            showQuantifiers=showQuantifiers,
            silent=silent,
            setInfo=searchExe.setInfo,
        )
        if showQuantifiers:
            logger.info(f"{quKind}\n{queryN}\n{QEND}")
        noResults = exe.search()
        resultYarn = universe - noResults
        if showQuantifiers:
            logger.info(f"{len(noResults)} nodes to exclude")
    elif quKind == QWHERE:
        # compute the atom+antecedent:
        #   as result tuples
        queryA = "\n".join((cleanAtom, quTemplates[0]))
        exe = SearchExe(
            searchExe.api,
            queryA,
            outerTemplate=searchExe.outerTemplate,
            quKind=quKind,
            offset=offset,
            level=level + 1,
            sets=searchExe.sets,
            shallow=False,
            showQuantifiers=showQuantifiers,
            silent=silent,
            setInfo=searchExe.setInfo,
        )
        if showQuantifiers:
            logger.info(f"{quKind}\n{queryA}")
        aResultTuples = exe.search(limit=0)
        if showQuantifiers:
            logger.info(f"{len(aResultTuples)} matching nodes")
        if not aResultTuples:
            resultYarn = yarn
        else:
            sizeA = len(aResultTuples[0])

            # compute the atom+antecedent+consequent:
            #   as shallow result tuples (same length as atom+antecedent)
            queryAH = "\n".join((cleanAtom, *quTemplates))
            offset += len(quTemplates[0].split("\n"))
            exe = SearchExe(
                searchExe.api,
                queryAH,
                outerTemplate=searchExe.outerTemplate,
                quKind=QHAVE,
                offset=offset,
                level=level + 1,
                sets=searchExe.sets,
                shallow=sizeA,
                showQuantifiers=showQuantifiers,
                silent=silent,
                setInfo=searchExe.setInfo,
            )
            if showQuantifiers:
                logger.info(f"{QHAVE}\n{queryAH}\n{QEND}")
            ahResults = exe.search()
            if showQuantifiers:
                logger.info(f"{len(ahResults)} matching nodes")

            # determine the shallow tuples that correspond to
            #   atom+antecedent but not consequent
            #   and then take the projection to their first components
            resultsAnotH = project(set(aResultTuples) - ahResults, 1)
            if showQuantifiers:
                logger.info(f"{len(resultsAnotH)} match antecedent but not consequent")

            # now have the atoms that do NOT qualify:
            #   we subtract them from the universe
            resultYarn = universe - resultsAnotH
    elif quKind == QWITH:
        # compute the atom+alternative for all alternatives and union them
        resultYarn = set()
        nAlts = len(quTemplates)
        for i, alt in enumerate(quTemplates):
            queryAlt = "\n".join((cleanAtom, alt))
            exe = SearchExe(
                searchExe.api,
                queryAlt,
                outerTemplate=searchExe.outerTemplate,
                quKind=quKind if i == 0 else QOR,
                offset=offset,
                level=level + 1,
                sets=searchExe.sets,
                shallow=True,
                showQuantifiers=showQuantifiers,
                silent=silent,
                setInfo=searchExe.setInfo,
            )
            offset += len(alt.split("\n")) + 1
            if showQuantifiers:
                logger.info(f"{quKind if i == 0 else QOR}\n{queryAlt}")
            altResults = exe.search()
            altResults &= universe
            nAlt = len(altResults)
            nYarn = len(resultYarn)
            resultYarn |= altResults
            nNew = len(resultYarn)
            if showQuantifiers:
                logger.info(f"adding {nAlt} to {nYarn} yields {nNew} nodes")
                if i == nAlts - 1:
                    logger.info(QEND)
    if showQuantifiers:
        logger.info(f"reduction from {len(yarn)} to {len(resultYarn)} nodes")
    return resultYarn


def spinAtoms(searchExe: SearchExe) -> None:
    qnodes = searchExe.qnodes
    for q in range(len(qnodes)):
        _spinAtom(searchExe, q)


def estimateSpreads(searchExe: SearchExe, both: bool = False) -> None:
    TRY_LIMIT_F = searchExe.perfParams["tryLimitFrom"]
    TRY_LIMIT_T = searchExe.perfParams["tryLimitTo"]
    qnodes = searchExe.qnodes
    relations = searchExe.relations
    converse = searchExe.converse
    qedges = searchExe.qedges
    yarns = searchExe.yarns

    spreadsC = {}
    spreads = {}

    for e, (f, rela, t) in enumerate(qedges):
        tasks = [(f, rela, t, 1)]
        if both:
            tasks.append((t, converse[rela], f, -1))
        for tf, trela, tt, dir in tasks:
            s = relations[trela]["spin"]
            yarnF = yarns[tf]
            yarnT = yarns[tt]
            dest = spreads if dir == 1 else spreadsC
            if type(s) is float:
                # fixed estimates
                dest[e] = len(yarnT) * s
                continue
            yarnF = list(yarnF)
            yarnT = yarns[tt]
            yarnFl = len(yarnF)
            if yarnFl < TRY_LIMIT_F:
                triesn = yarnF
            else:
                triesn = {yarnF[randrange(yarnFl)] for n in range(TRY_LIMIT_F)}

            if len(triesn) == 0:
                dest[e] = 0
            else:
                r = relations[trela]["func"](qnodes[tf][0], qnodes[tt][0])
                nparams = len(signature(r).parameters)
                totalSpread = 0
                if nparams == 1:
                    for n in triesn:
                        mFromN = {m for m in r(n) or () if m in yarnT}
                        totalSpread += len(mFromN)
                else:
                    yarnTl = len(yarnT)
                    yarnTL = list(yarnT)
                    for n in triesn:
                        triesm = (
                            yarnT
                            if yarnTl < TRY_LIMIT_T
                            else set(
                                yarnTL[randrange(yarnTl)] for m in range(TRY_LIMIT_T)
                            )
                        )
                        if len(triesm) == 0:
                            thisSpread = 0
                        else:
                            thisSpread = 0
                            for m in triesm:
                                if r(n, m):
                                    thisSpread += 1
                            thisSpread = thisSpread / len(triesm)
                        totalSpread += yarnTl * thisSpread
                dest[e] = totalSpread / len(triesn)
    searchExe.spreads = spreads
    searchExe.spreadsC = spreadsC


def _chooseEdge(searchExe: SearchExe) -> int:
    qedges = searchExe.qedges
    yarns = searchExe.yarns
    spreads = searchExe.spreads
    yarnSize = {}
    for e, (f, rela, t) in enumerate(qedges):
        if searchExe.uptodate[e]:
            continue
        yarnFl = len(yarns[f])
        yarnTl = len(yarns[t])
        yarnSize[e] = yarnFl * yarnTl * spreads[e]
    firstEdge = sorted(yarnSize.items(), key=lambda x: x[1])[0][0]
    return firstEdge


def _spinEdge(searchExe: SearchExe, e: int) -> bool:
    YARN_RATIO = searchExe.perfParams["yarnRatio"]
    qnodes = searchExe.qnodes
    relations = searchExe.relations
    yarns = searchExe.yarns
    spreads = searchExe.spreads
    qedges = searchExe.qedges
    uptodate = searchExe.uptodate

    (f, rela, t) = qedges[e]
    yarnF = yarns[f]
    yarnT = yarns[t]
    uptodate[e] = True

    # if the yarns around an edge are big,
    # and the spread of the relation is
    # also big, spinning costs an enormous amount of time,
    # and will not help in reducing the search space.
    # condition for skipping: spread times length from-yarn >= SPIN_LIMIT
    yarnFl = len(yarnF)
    yarnTl = len(yarnT)
    thisYarnRatio = (
        max((yarnFl / yarnTl, yarnTl / yarnFl)) / spreads[e]
        if yarnFl and yarnTl and spreads[e]
        else -YARN_RATIO
    )
    if thisYarnRatio < YARN_RATIO:
        return False
    # if spreads[e] * len(yarnF) >= SPIN_LIMIT:
    #   return False

    # for some basic relations we know that spinning is useless
    s = relations[rela]["spin"]
    if type(s) is float:
        return False

    # for other basic relations we have an optimised spin function
    # if type(s) is types.FunctionType:
    if isinstance(s, types.FunctionType):
        (newYarnF, newYarnT) = s(qnodes[f][0], qnodes[t][0])(yarnF, yarnT)
    else:
        r = relations[rela]["func"](qnodes[f][0], qnodes[t][0])
        nparams = len(signature(r).parameters)
        newYarnF = set()
        newYarnT = set()

        if nparams == 1:
            for n in yarnF:
                found = False
                for m in r(n):
                    if m not in yarnT:
                        continue
                    newYarnT.add(m)
                    found = True
                if found:
                    newYarnF.add(n)
        else:
            for n in yarnF:
                found = False
                for m in yarnT:
                    if r(n, m):
                        newYarnT.add(m)
                        found = True
                if found:
                    newYarnF.add(n)

    affectedF = len(newYarnF) != len(yarns[f])
    affectedT = len(newYarnT) != len(yarns[t])

    uptodate[e] = True
    for oe, (of, orela, ot) in enumerate(qedges):
        if oe == e:
            continue
        if (affectedF and f in {of, ot}) or (affectedT and t in {of, ot}):
            uptodate[oe] = False
    searchExe.yarns[f] = newYarnF
    searchExe.yarns[t] = newYarnT

    return affectedF or affectedT


def spinEdges(searchExe: SearchExe) -> None:
    qnodes = searchExe.qnodes
    qedges = searchExe.qedges
    yarns = searchExe.yarns
    uptodate = searchExe.uptodate
    thinned = {}

    estimateSpreads(searchExe, both=True)

    for e in range(len(qedges)):
        uptodate[e] = False
    it = 0
    while 1:
        if min(len(yarns[q]) for q in range(len(qnodes))) == 0:
            break
        if all(uptodate[e] for e in range(len(qedges))):
            break
        e = _chooseEdge(searchExe)
        (f, rela, t) = qedges[e]
        affected = _spinEdge(searchExe, e)
        if affected:
            thinned[e] = 1
        it += 1
    searchExe.thinned = thinned
