"""The browser's web application: the JSON API plus the single-page interface."""

from __future__ import annotations

from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from importlib import metadata
from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse, HTMLResponse
from fastapi.staticfiles import StaticFiles

from cfabric_browser import __version__
from cfabric_browser.api import build_router
from cfabric_browser.corpora import CorpusService
from cfabric_browser.downloads import DownloadJobs
from cfabric_browser.translations.manage import Jobs
from cfabric_browser.workers import SearchPool

STATIC = Path(__file__).parent / "static"

NOT_BUILT = """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Context-Fabric Browser</title></head>
<body style="font-family: system-ui; max-width: 40rem; margin: 4rem auto">
<h1>Context-Fabric Browser</h1>
<p>The server is running, but the interface has not been built.</p>
<p>Build it with <code>npm install</code> and <code>npm run build</code> in
<code>libs/browser/frontend</code>, or use the API at <a href="/docs">/docs</a>.</p>
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
    """

    @asynccontextmanager
    async def lifespan(app: FastAPI) -> AsyncIterator[None]:
        if manage_pool:
            pool.start(wait=False)
        try:
            yield
        finally:
            if manage_pool:
                pool.shutdown()

    app = FastAPI(
        title="Context-Fabric Browser",
        version=__version__,
        lifespan=lifespan,
    )
    app.include_router(
        build_router(
            service,
            pool,
            library=library,
            translations=translations,
            downloads=downloads,
        )
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
        if (
            path
            and candidate.is_file()
            and candidate.is_relative_to(static_dir.resolve())
        ):
            return FileResponse(candidate)
        if index.is_file():
            return FileResponse(index)
        return HTMLResponse(NOT_BUILT)

    return app
