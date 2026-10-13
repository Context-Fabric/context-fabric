"""The browser's JSON API.

Endpoints mirror the MCP tools of `cfabric-mcp` where they overlap (corpus
description, features, passages, search with paging), so that people in the
browser and language models through MCP work with the same operations.

All corpus-specific routes are under `/api/corpora/{corpus}`; searches have
their own ids under `/api/searches/{id}`. Nothing in the API exposes paths on
the server's file system.
"""

from __future__ import annotations

import asyncio
import json
from typing import Annotated, Any, Literal

from anyio import to_thread
from cfabric.display import (
    RenderOptions,
    node_from_section_ref,
    render_node,
    render_tuple,
    section_ref_for,
)
from cfabric.utils.helpers import tsvEsc
from fastapi import APIRouter, Depends, HTTPException, Query, Request
from fastapi.responses import PlainTextResponse, StreamingResponse
from pydantic import BaseModel, Field

from cfabric_browser.corpora import CorpusService
from cfabric_browser.queries import LANGUAGES, check_template
from cfabric_browser.workers import FINISHED, SearchJob, SearchPool

MAX_PAGE = 500
MAX_DOWN = 200
MAX_PASSAGE = 200


class SearchRequest(BaseModel):
    template: str
    language: str = Field(default="cf-template", description=f"One of {LANGUAGES}")
    limit: int | None = Field(default=None, ge=1)
    timeout: float | None = Field(default=None, gt=0)


def _items(value: str | None) -> list[str] | None:
    if value is None:
        return None
    return [x for x in value.replace(",", " ").split() if x]


def render_options(
    fmt: str | None = None,
    full: bool = False,
    hide_types: bool = True,
    hidden_types: str | None = Query(None, description="Comma separated types"),
    base_types: str | None = Query(None, description="Comma separated types"),
    condense_type: str | None = None,
    extra_features: str | None = Query(
        None, description="Comma separated features, `type:feature` allowed"
    ),
    lang: str = "en",
) -> RenderOptions:
    """Display options as query parameters; see `cfabric.display.RenderOptions`."""
    return RenderOptions(
        fmt=fmt or None,
        full=full,
        hide_types=hide_types,
        hidden_types=_items(hidden_types),
        base_types=_items(base_types),
        condense_type=condense_type or None,
        extra_features=_items(extra_features) or (),
        lang=lang,
    )


def build_router(service: CorpusService, pool: SearchPool) -> APIRouter:
    router = APIRouter(prefix="/api")

    def corpus_api(corpus: str):
        if not service.has(corpus):
            raise HTTPException(404, f"Unknown corpus: {corpus}")
        return service.api(corpus)

    def check_node(api, node: int) -> int:
        if not 1 <= node <= int(api.F.otype.maxNode):
            raise HTTPException(404, f"No node {node}")
        return node

    def search_job(search_id: str) -> SearchJob:
        job = pool.get(search_id)
        if job is None:
            raise HTTPException(404, f"Unknown or expired search: {search_id}")
        return job

    # corpora

    @router.get("/corpora")
    def list_corpora() -> dict[str, Any]:
        return {
            "corpora": [
                {
                    "name": name,
                    "title": service.config(name).provenance.get("corpus") or name,
                }
                for name in service.names()
            ]
        }

    @router.get("/corpora/{corpus}")
    def describe_corpus(corpus: str) -> dict[str, Any]:
        corpus_api(corpus)
        return service.card(corpus)

    @router.get("/corpora/{corpus}/features")
    def list_features(corpus: str) -> dict[str, Any]:
        from cfabric.describe import list_features as listFeatures

        api = corpus_api(corpus)
        excluded = service.config(corpus).excluded_features
        features = [
            {**f.to_dict(), "excluded": f.name in excluded} for f in listFeatures(api)
        ]
        return {"features": features}

    @router.get("/corpora/{corpus}/features/{feature}")
    def describe_feature(
        corpus: str, feature: str, sample_limit: int = Query(20, ge=0, le=500)
    ) -> dict[str, Any]:
        from cfabric.describe import describe_feature as describeFeature

        api = corpus_api(corpus)
        result: dict[str, Any] = describeFeature(
            api, feature, sample_limit=sample_limit
        ).to_dict()
        if "error" in result:
            raise HTTPException(404, f"Unknown feature: {feature}")
        return result

    # sections and passages

    @router.get("/corpora/{corpus}/sections")
    def sections(corpus: str, ref: str | None = None, lang: str = "en") -> dict:
        """The section tree, one level at a time: books, then their chapters, ..."""
        api = corpus_api(corpus)
        config = service.config(corpus)
        T = api.T
        L = api.L
        sectionTypes = list(T.sectionTypes or ())
        if not sectionTypes:
            return {"node": None, "heading": "", "children": []}
        if ref:
            try:
                node = node_from_section_ref(
                    api, ref, config.section_sep1, config.section_sep2, lang
                )
            except ValueError as e:
                raise HTTPException(404, str(e)) from None
            level = sectionTypes.index(api.F.otype.v(node))
            if level + 1 >= len(sectionTypes):
                childNodes: list[int] = []
            else:
                childNodes = [int(n) for n in L.d(node, otype=sectionTypes[level + 1])]
            heading = section_ref_for(api, config, node, lang=lang)
        else:
            node = None
            childNodes = [int(n) for n in api.F.otype.s(sectionTypes[0])]
            heading = ""
        children = []
        for n in childNodes:
            section = T.sectionFromNode(n, lang=lang)
            children.append(
                {
                    "node": n,
                    "type": api.F.otype.v(n),
                    "label": str(section[-1]) if section else str(n),
                    "ref": section_ref_for(api, config, n, lang=lang),
                }
            )
        return {"node": node, "heading": heading, "children": children}

    @router.get("/corpora/{corpus}/passage")
    def passage(
        corpus: str,
        ref: str,
        opts: Annotated[RenderOptions, Depends(render_options)],
        mode: Literal["pretty", "plain"] = "pretty",
    ) -> dict[str, Any]:
        api = corpus_api(corpus)
        config = service.config(corpus)
        try:
            node = node_from_section_ref(
                api, ref, config.section_sep1, config.section_sep2, opts.lang
            )
        except ValueError as e:
            raise HTTPException(404, str(e)) from None
        nType = api.F.otype.v(node)
        # a section above the condense type (book, chapter) is shown as its
        # condense-type parts, e.g. the verses of a chapter
        condense = opts.condense_type or config.condense_type
        rank = api.N.otypeRank
        if not opts.full and condense and rank[nType] > rank[condense]:
            nodes = [int(n) for n in api.L.d(node, otype=condense)]
        else:
            nodes = [node]
        trees = [
            render_node(api, config, n, mode, opts).to_dict()
            for n in nodes[:MAX_PASSAGE]
        ]
        return {
            "node": node,
            "ref": ref,
            "total": len(nodes),
            "truncated": len(nodes) > MAX_PASSAGE,
            "trees": trees,
        }

    # nodes

    @router.get("/corpora/{corpus}/nodes/{node}")
    def node_render(
        corpus: str,
        node: int,
        opts: Annotated[RenderOptions, Depends(render_options)],
        mode: Literal["pretty", "plain"] = "pretty",
    ) -> dict[str, Any]:
        api = corpus_api(corpus)
        check_node(api, node)
        tree: dict[str, Any] = render_node(
            api, service.config(corpus), node, mode, opts
        ).to_dict()
        return tree

    @router.get("/corpora/{corpus}/nodes/{node}/info")
    def node_info(corpus: str, node: int, lang: str = "en") -> dict[str, Any]:
        """Everything about one node, for the node inspector."""
        api = corpus_api(corpus)
        check_node(api, node)
        config = service.config(corpus)
        F, E, L, T = api.F, api.E, api.L, api.T
        nType = F.otype.v(node)
        slots = E.oslots.s(node)

        features = {}
        for name in sorted(api.Fall()):
            if name == "otype":
                continue
            value = api.Fs(name).v(node)
            if value is not None:
                features[name] = value if isinstance(value, (int, str)) else str(value)

        edges: dict[str, dict[str, list]] = {}
        for name in sorted(api.Eall()):
            if name == "oslots":
                continue
            es = api.Es(name)
            out = [_edge_item(x) for x in (es.f(node) or ())]
            inc = [_edge_item(x) for x in (es.t(node) or ())]
            if out or inc:
                edges[name] = {"from": out, "to": inc}

        up = [{"node": int(n), "type": F.otype.v(int(n))} for n in L.u(node)]
        downCounts: dict[str, int] = {}
        down: list[dict[str, Any]] = []
        for n in L.d(node):
            t = F.otype.v(int(n))
            downCounts[t] = downCounts.get(t, 0) + 1
            if len(down) < MAX_DOWN:
                down.append({"node": int(n), "type": t})

        text = T.text(node) if len(slots) <= 200 else None
        return {
            "node": node,
            "type": nType,
            "ref": section_ref_for(api, config, node, lang=lang),
            "slots": [int(slots[0]), int(slots[-1])],
            "slotCount": len(slots),
            "text": text,
            "features": features,
            "edges": edges,
            "up": up,
            "down": down,
            "downCounts": downCounts,
        }

    # search

    @router.post("/corpora/{corpus}/search", status_code=202)
    def start_search(corpus: str, request: SearchRequest) -> dict[str, Any]:
        api = corpus_api(corpus)
        if request.language not in LANGUAGES:
            raise HTTPException(
                422,
                f"Unsupported query language {request.language!r};"
                f" supported: {', '.join(LANGUAGES)}",
            )
        problems = check_template(api, request.template)
        if problems:
            raise HTTPException(
                400,
                {
                    "message": "Invalid search template",
                    "problems": problems,
                    "template": request.template,
                },
            )
        job = pool.submit(
            corpus, request.template, limit=request.limit, timeout=request.timeout
        )
        return job.snapshot()

    @router.post("/corpora/{corpus}/check")
    def check(corpus: str, request: SearchRequest) -> dict[str, Any]:
        """Check a template without searching, e.g. while typing."""
        api = corpus_api(corpus)
        problems = check_template(api, request.template)
        return {"ok": not problems, "problems": problems}

    @router.get("/searches/{search_id}")
    def search_status(search_id: str) -> dict[str, Any]:
        return search_job(search_id).snapshot()

    @router.delete("/searches/{search_id}")
    def cancel_search(search_id: str) -> dict[str, Any]:
        job = search_job(search_id)
        pool.cancel(search_id)
        return job.snapshot()

    @router.get("/searches/{search_id}/events")
    async def search_events(search_id: str, request: Request) -> StreamingResponse:
        """Server-sent events with the status of a search until it finishes."""
        job = search_job(search_id)

        async def stream():
            version = -1
            lastSent = 0.0
            loop = asyncio.get_running_loop()
            while True:
                if await request.is_disconnected():
                    return
                version = await to_thread.run_sync(job.wait_change, version, 1.0)
                now = loop.time()
                snap = job.snapshot()
                finished = snap["state"] in FINISHED
                # at most ~10 updates per second while running
                if finished or now - lastSent >= 0.1:
                    lastSent = now
                    yield f"event: status\ndata: {json.dumps(snap)}\n\n"
                if finished:
                    return

        return StreamingResponse(
            stream(),
            media_type="text/event-stream",
            headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"},
        )

    @router.get("/searches/{search_id}/results")
    def search_results(
        search_id: str,
        offset: int = Query(0, ge=0),
        limit: int = Query(50, ge=1, le=MAX_PAGE),
        render: Literal["none", "plain"] = "none",
        fmt: str | None = None,
        lang: str = "en",
    ) -> dict[str, Any]:
        """A page of results: node tuples with their passage reference.

        With `render=plain`, each row also has a plain render tree per node.
        While a search runs, rows come in the order they were found; once
        it is done they are in canonical order (`sorted` is then true).
        """
        job = search_job(search_id)
        api = corpus_api(job.corpus)
        config = service.config(job.corpus)
        F = api.F
        opts = RenderOptions(fmt=fmt or None, lang=lang)
        rows = []
        for i, tup in enumerate(job.page(offset, limit)):
            row: dict[str, Any] = {
                "index": offset + i,
                "nodes": list(tup),
                "types": [F.otype.v(n) for n in tup],
                "ref": section_ref_for(api, config, tup[0], lang=lang),
            }
            if render == "plain":
                row["plain"] = [
                    t.to_dict()
                    for t in render_tuple(api, config, tup, mode="plain", options=opts)
                ]
            rows.append(row)
        return {"search": job.snapshot(), "offset": offset, "rows": rows}

    @router.get("/searches/{search_id}/results/{index}")
    def search_result(
        search_id: str,
        index: int,
        opts: Annotated[RenderOptions, Depends(render_options)],
        mode: Literal["pretty", "plain"] = "pretty",
    ) -> dict[str, Any]:
        """One result rendered in its context, with its nodes highlighted."""
        job = search_job(search_id)
        page = job.page(index, 1)
        if not page:
            raise HTTPException(404, f"No result {index}")
        api = corpus_api(job.corpus)
        trees = render_tuple(api, service.config(job.corpus), page[0], mode, opts)
        return {
            "index": index,
            "nodes": list(page[0]),
            "trees": [t.to_dict() for t in trees],
        }

    @router.get("/searches/{search_id}/export.tsv")
    def export_tsv(search_id: str, fmt: str | None = None, lang: str = "en"):
        """All results as tab-separated values, one row per result."""
        job = search_job(search_id)
        if job.state not in FINISHED:
            raise HTTPException(409, "The search has not finished yet")
        api = corpus_api(job.corpus)
        F, T = api.F, api.T
        sectionTypes = list(T.sectionTypes or ())
        rows = job.page(0, job.count)
        width = max((len(r) for r in rows), default=0)

        header = ["R"] + [f"S{i + 1}" for i in range(len(sectionTypes))]
        for i in range(width):
            header += [f"NODE{i + 1}", f"TYPE{i + 1}", f"TEXT{i + 1}"]
        lines = ["\t".join(header)]
        for r, tup in enumerate(rows, 1):
            section = T.sectionFromNode(tup[0], lang=lang) if sectionTypes else ()
            cells = [str(r)] + [tsvEsc("" if s is None else s) for s in (section or ())]
            cells += [""] * (len(header) - len(cells) - 3 * width)
            for n in tup:
                text = (
                    T.text(n, fmt=fmt)
                    if F.otype.v(n) == F.otype.slotType or len(api.E.oslots.s(n)) <= 100
                    else ""
                )
                cells += [str(n), F.otype.v(n), tsvEsc(text)]
            lines.append("\t".join(cells))
        return PlainTextResponse(
            "\n".join(lines) + "\n",
            media_type="text/tab-separated-values; charset=utf-8",
            headers={
                "Content-Disposition": (
                    f'attachment; filename="{job.corpus}-results-{job.id}.tsv"'
                )
            },
        )

    return router


def _edge_item(x: Any) -> dict[str, Any]:
    if isinstance(x, tuple):
        return {"node": int(x[0]), "value": x[1]}
    return {"node": int(x)}
