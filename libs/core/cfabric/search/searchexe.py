"""
# Search execution management
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, Callable, Generator

if TYPE_CHECKING:
    from cfabric.core.api import Api

from cfabric.search.relations import basicRelations
from cfabric.search.syntax import syntax
from cfabric.search.semantics import semantics
from cfabric.search.graph import connectedness, displayPlan
from cfabric.search.spin import spinAtoms, spinEdges
from cfabric.search.stitch import setStrategy, stitch
from cfabric.core.config import SEARCH_FAIL_FACTOR, YARN_RATIO, TRY_LIMIT_FROM, TRY_LIMIT_TO
from cfabric.utils.logging import DEEP

logger = logging.getLogger(__name__)


PROGRESS: int = 100


class SearchExe:
    perfDefaults: dict[str, int | float] = dict(
        yarnRatio=YARN_RATIO,
        tryLimitFrom=TRY_LIMIT_FROM,
        tryLimitTo=TRY_LIMIT_TO,
    )
    perfParams: dict[str, int | float] = dict(**perfDefaults)

    @classmethod
    def setPerfParams(cls, params: dict[str, int | float]) -> None:
        cls.perfParams = params

    def __init__(
        self,
        api: Api,
        searchTemplate: str,
        outerTemplate: str | None = None,
        quKind: str | None = None,
        offset: int = 0,
        level: int = 0,
        sets: dict[str, set[int]] | None = None,
        shallow: bool | int = False,
        silent: str = DEEP,
        showQuantifiers: bool = False,
        _msgCache: bool | list[Any] = False,
        setInfo: dict[str, bool | None] | None = None,
    ) -> None:
        if setInfo is None:
            setInfo = {}
        self.api: Api = api
        self.searchTemplate: str = searchTemplate
        self.outerTemplate: str | None = outerTemplate
        self.quKind: str | None = quKind
        self.level: int = level
        self.offset: int = offset
        self.sets: dict[str, set[int]] | None = sets
        self.shallow: int = 0 if not shallow else 1 if shallow is True else shallow
        self.silent: str = silent
        self.showQuantifiers: bool = showQuantifiers
        self._msgCache: list[Any] | int = (
            _msgCache if type(_msgCache) is list else -1 if _msgCache else 0
        )
        self.good: bool = True
        self.setInfo: dict[str, bool | None] = setInfo
        basicRelations(self, api)

    # API METHODS ###

    def search(
        self, limit: int | None = None
    ) -> (
        tuple[tuple[int, ...], ...] |
        set[int] |
        set[tuple[int, ...]] |
        Generator[tuple[int, ...], None, None]
    ):
        self.study()
        return self.fetch(limit=limit)

    def study(self, strategy: str | None = None) -> None:
        self.good = True

        setStrategy(self, strategy)
        if not self.good:
            return

        logger.info("Checking search template ...")

        self._parse()
        self._prepare()
        if not self.good:
            return
        logger.info(f"Setting up search space for {len(self.qnodes)} objects ...")
        spinAtoms(self)
        logger.info(f"Constraining search space with {len(self.qedges)} relations ...")
        spinEdges(self)
        logger.info(f"\t{len(self.thinned)} edges thinned")
        logger.info(f"Setting up retrieval plan with strategy {self.strategyName} ...")
        stitch(self)
        if self.good:
            yarnContent = sum(len(y) for y in self.yarns.values())
            logger.info(f"Ready to deliver results from {yarnContent} nodes")
            logger.debug("Iterate over S.fetch() to get the results")
            logger.debug("See S.showPlan() to interpret the results")

    def fetch(
        self, limit: int | None = None
    ) -> (
        tuple[tuple[int, ...], ...] |
        set[int] |
        set[tuple[int, ...]] |
        Generator[tuple[int, ...], None, None]
    ):
        api = self.api
        F = api.F

        if limit and limit < 0:
            limit = 0

        if not self.good:
            queryResults = set() if self.shallow else []
        elif self.shallow:
            queryResults = self.results
        else:
            failLimit = limit if limit else SEARCH_FAIL_FACTOR * F.otype.maxNode

            def limitedResults():
                for i, result in enumerate(self.results()):
                    if i < failLimit:
                        yield result
                    else:
                        if not limit:
                            logger.error(
                                f"cut off at {failLimit} results. There are more ..."
                            )
                        return

            queryResults = (
                limitedResults() if limit is None else tuple(limitedResults())
            )

        return queryResults

    def count(self, progress: int | None = None, limit: int | None = None) -> None:
        if limit and limit < 0:
            limit = 0

        if not self.good:
            logger.error("This search has problems. No results to count.")
            return

        if progress is None:
            progress = PROGRESS

        if limit:
            failLimit = limit
            msg = f" up to {failLimit}"
        else:
            failLimit = SEARCH_FAIL_FACTOR * self.api.F.otype.maxNode
            msg = ""

        logger.info(f"Counting results per {progress}{msg} ...")

        j = 0
        good = True
        for i, r in enumerate(self.results(remap=False)):
            if i >= failLimit:
                if not limit:
                    good = False
                break
            j += 1
            if j == progress:
                j = 0
                logger.info(str(i + 1))

        if good:
            logger.info(f"Done: {i + 1} results")
        else:
            logger.error(f"cut off at {failLimit} results. There are more ...")

    # SHOWING WITH THE SEARCH GRAPH ###

    def showPlan(self, details: bool = False) -> None:
        displayPlan(self, details=details)

    def showOuterTemplate(self, _msgCache: list[Any] | int) -> None:
        offset = self.offset
        outerTemplate = self.outerTemplate
        quKind = self.quKind
        if offset and outerTemplate is not None:
            for i, line in enumerate(outerTemplate.split("\n")):
                logger.error(f"{i:>2} {line}")
            logger.error(f"line {offset:>2}: Error under {quKind}:")

    # TOP-LEVEL IMPLEMENTATION METHODS

    def _parse(self) -> None:
        syntax(self)
        semantics(self)

    def _prepare(self) -> None:
        if not self.good:
            return
        self.yarns: dict[int, set[int]] = {}
        self.spreads: dict[int, float] = {}
        self.spreadsC: dict[int, float] = {}
        self.uptodate: dict[int, bool] = {}
        self.results: (
            set[int] |
            set[tuple[int, ...]] |
            Callable[[bool], Generator[tuple[int, ...], None, None]] |
            None
        ) = None
        connectedness(self)
