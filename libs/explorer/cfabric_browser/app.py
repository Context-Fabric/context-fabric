"""The browser's web application: the JSON API plus the single-page interface."""

from __future__ import annotations

from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from importlib import metadata
from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse, HTMLResponse, RedirectResponse
from fastapi.staticfiles import StaticFiles
from mcp.server.fastmcp.server import StreamableHTTPASGIApp
from starlette.routing import Route

from cfabric_browser import __version__
from cfabric_browser.api import build_router
from cfabric_browser.assistant.mcp import build_server as build_mcp
from cfabric_browser.assistant.mcp import local
from cfabric_browser.assistant.tools import Tools
from cfabric_browser.corpora import CorpusService
from cfabric_browser.dictionaries.manage import DictionaryJobs
from cfabric_browser.downloads import DownloadJobs
from cfabric_browser.guard import Guard
from cfabric_browser.translations.manage import Jobs
from cfabric_browser.workers import SearchPool

STATIC = Path(__file__).parent / "static"

NOT_BUILT = """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>CACCHT Explorer</title></head>
<body style="font-family: system-ui; max-width: 40rem; margin: 4rem auto">
<h1>CACCHT Explorer</h1>
<p>The server is running, but the interface has not been built.</p>
<p>Build it with <code>npm install</code> and <code>npm run build</code> in
<code>libs/explorer/frontend</code>, or use the API at <a href="/docs">/docs</a>.</p>
</body></html>
"""


def create_app(
    service: CorpusService,
    pool: SearchPool,
    static_dir: Path = STATIC,
    manage_pool: bool = True,
    library: bool = True,
    ui: str = "classic",
    hidden_uis: list[str] | None = None,
    translations: Jobs | None = None,
    downloads: DownloadJobs | None = None,
    ask_downloads: bool = False,
    dictionaries: DictionaryJobs | None = None,
    assistant: Tools | None = None,
    secret: str | None = None,
    hosts: list[str] | None = None,
) -> FastAPI:
    """Build the application.

    Parameters
    ----------
    service: CorpusService
        The corpora to serve.
    pool: SearchPool
        The search workers.
    static_dir: Path
        Directory with the built interface (`index.html` and `assets/`).
    manage_pool: bool
        Start the pool when the app starts and stop it when it stops.
    library: bool
        Let people open other corpora found on this computer. Switch this off
        when the browser is reachable by others.
    ui: str
        The interface to start in when the user has not chosen one yet:
        `classic` (panes), `parallel`, `reader`, `interlinear` or
        `concordance` (see `cfabric_browser.interfaces`).
    hidden_uis: list[str] | None
        Interfaces to leave out of the menu until the user chooses otherwise.
    translations: Jobs | None
        Where translations are built and how (`cfabric_browser.translations`);
        by default into the first place the library looks.
    downloads: DownloadJobs | None
        Where corpora are downloaded to (`cfabric_browser.downloads`); by
        default the first place the library looks.
    ask_downloads: bool
        At the start, offer to download the corpora that can be downloaded and
        are missing (the standalone program).
    dictionaries: DictionaryJobs | None
        Where dictionaries (BDB) are kept (`cfabric_browser.dictionaries`); by
        default `~/.cfabric/dictionaries`.
    assistant: Tools | None
        The tools of searching in plain language and their proposals
        (`cfabric_browser.assistant`); by default on these corpora and pool.
    secret: str | None
        What a language model's tools must send to propose a search; without
        one, nothing can propose.
    hosts: list[str] | None
        The host names the server answers to (`cfabric_browser.guard`), such
        as `guard.LOCAL_HOSTS` when it listens only on this computer; `None`
        for any. Requests from other web sites are refused either way.
    """

    # searching in plain language: the tools, also as an MCP server at /mcp
    tools = assistant if assistant is not None else Tools(service, pool)
    mcpServer = build_mcp(local(tools))
    mcpServer.streamable_http_app()  # makes its session manager

    @asynccontextmanager
    async def lifespan(app: FastAPI) -> AsyncIterator[None]:
        if manage_pool:
            pool.start(wait=False)
        try:
            async with mcpServer.session_manager.run():
                yield
        finally:
            if manage_pool:
                pool.shutdown()

    app = FastAPI(
        title="CACCHT Explorer",
        version=__version__,
        lifespan=lifespan,
    )
    app.add_middleware(Guard, hosts=hosts)
    app.include_router(
        build_router(
            service,
            pool,
            library=library,
            translations=translations,
            downloads=downloads,
            dictionaries=dictionaries,
            assistant=tools,
            secret=secret,
        )
    )
    # before the interface's catch-all route
    app.router.routes.insert(
        0, Route("/mcp", endpoint=StreamableHTTPASGIApp(mcpServer.session_manager))
    )

    try:
        coreVersion = metadata.version("context-fabric")
    except metadata.PackageNotFoundError:
        coreVersion = None

    @app.get("/api/health")
    def health() -> dict:
        return {
            "ok": True,
            "searchReady": pool.ready,
            "version": __version__,
            "coreVersion": coreVersion,
            "ui": ui,
            "hiddenUis": list(hidden_uis or []),
            # offer the missing corpora at the start (/api/downloads)
            "askDownloads": ask_downloads and library,
        }

    index = static_dir / "index.html"
    assets = static_dir / "assets"
    if assets.is_dir():
        app.mount("/assets", StaticFiles(directory=assets), name="assets")

    @app.get("/{path:path}", include_in_schema=False)
    def spa(path: str):
        # the interface does its own routing: every non-API path gets the page
        if path.startswith("api/"):
            raise HTTPException(404, "Not found")
        candidate = (static_dir / path).resolve()
        inside = bool(path) and candidate.is_relative_to(static_dir.resolve())
        if inside and candidate.is_file():
            return FileResponse(candidate)
        # a folder of pages (the guide, /guide/): its index.html, at an address
        # ending in / so that the pages' relative links work
        if inside and (candidate / "index.html").is_file():
            if not path.endswith("/"):
                return RedirectResponse(f"/{path}/")
            return FileResponse(candidate / "index.html")
        if index.is_file():
            return FileResponse(index)
        return HTMLResponse(NOT_BUILT)

    return app
