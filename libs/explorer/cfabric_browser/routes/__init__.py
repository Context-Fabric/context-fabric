"""The browser's JSON API, all routes under `/api`.

Endpoints mirror the MCP tools of `cfabric-mcp` where they overlap (corpus
description, features, passages, search with paging), so that people in the
browser and language models through MCP work with the same operations.

All corpus-specific routes are under `/api/corpora/{corpus}`; searches have
their own ids under `/api/searches/{id}`. Nothing in the API exposes paths on
the server's file system.

One module per area; each builds its routes on the shared `Context`.
"""

from __future__ import annotations

from fastapi import APIRouter

from cfabric_browser.assistant.tools import Tools
from cfabric_browser.corpora import CorpusService
from cfabric_browser.dictionaries.manage import DictionaryJobs
from cfabric_browser.dictionaries.store import Dictionaries
from cfabric_browser.downloads import DownloadJobs
from cfabric_browser.routes import (
    lexicon,
    parallel,
    proposals,
    reading,
    search,
)
from cfabric_browser.routes import library as library_routes
from cfabric_browser.routes.common import Context
from cfabric_browser.translations.manage import Jobs
from cfabric_browser.workers import SearchPool


def build_router(
    service: CorpusService,
    pool: SearchPool,
    library: bool = True,
    translations: Jobs | None = None,
    downloads: DownloadJobs | None = None,
    dictionaries: DictionaryJobs | None = None,
    assistant: Tools | None = None,
) -> APIRouter:
    lookups = dictionaries if dictionaries is not None else DictionaryJobs()
    ctx = Context(
        service=service,
        pool=pool,
        library=library,
        tools=assistant if assistant is not None else Tools(service, pool),
        lookups=lookups,
        store=Dictionaries(lookups.root),
        jobs=translations if translations is not None else Jobs(service.roots[0]),
        fetches=downloads if downloads is not None else DownloadJobs(service.roots[0]),
    )
    router = APIRouter(prefix="/api")
    for area in (library_routes, reading, parallel, search, proposals, lexicon):
        router.include_router(area.build(ctx))
    return router
