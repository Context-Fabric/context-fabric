"""Searching: starting, checking and following searches; results, the
Concordance, statistics and export."""

from __future__ import annotations

import asyncio
import json
import threading
from collections import Counter, OrderedDict
from typing import Annotated, Any, Literal

from anyio import to_thread
from fastapi import APIRouter, Depends, HTTPException, Query, Request
from fastapi.responses import Response, StreamingResponse

from cfabric_browser import formats
from cfabric_browser.concordance import MAX_SIDE, Concordance, parse_filters
from cfabric_browser.display import (
    RenderOptions,
    render_tuple,
    section_ref_for,
)
from cfabric_browser.helpers import tsvEsc
from cfabric_browser.queries import (
    LANGUAGES,
    check_template,
    template_constraints,
    template_features,
)
from cfabric_browser.routes.common import (
    MAX_PAGE,
    STATS_SAMPLE,
    Context,
    SearchRequest,
    render_options,
    section_item,
    split_items,
    tsv_response,
)
from cfabric_browser.workers import FINISHED, SearchJob


def build(ctx: Context) -> APIRouter:
    router = APIRouter()
    service = ctx.service
    pool = ctx.pool
    corpus_api = ctx.corpus_api
    search_job = ctx.search_job

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

    # searches proposed by a language model (cfabric_browser.assistant)

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
        opts = RenderOptions(fmt=formats.chosen(config, fmt), lang=lang)
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
        config = service.config(job.corpus)
        trees = render_tuple(api, config, page[0], mode, formats.options(config, opts))
        return {
            "index": index,
            "nodes": list(page[0]),
            "trees": [t.to_dict() for t in trees],
        }

    concordances: OrderedDict[str, tuple[int, bool, Concordance]] = OrderedDict()
    concordanceLock = threading.Lock()

    def concordance(job: SearchJob) -> Concordance:
        """The concordance of a search's results. While the search runs, new
        results are added to it; once they are sorted, it is built again."""
        snapshot = job.snapshot()
        with concordanceLock:
            cached = concordances.get(job.id)
            if cached is not None:
                concordances.move_to_end(job.id)
            if cached and cached[0] == snapshot["version"]:
                return cached[2]
        if cached and cached[1] == snapshot["sorted"]:
            # the same order: only results were added
            built = cached[2]
            with concordanceLock:
                built.extend(job.page(len(built.hits), snapshot["count"]))
        else:
            api = corpus_api(job.corpus)
            config = service.config(job.corpus)
            built = Concordance(api, config, job.page(0, snapshot["count"]))
        with concordanceLock:
            concordances[job.id] = (snapshot["version"], snapshot["sorted"], built)
            concordances.move_to_end(job.id)
            # keep a few, the last used: they hold a list of words per result
            while len(concordances) > 8:
                concordances.popitem(last=False)
        return built

    def concordance_selection(
        conc: Concordance,
        filters: str | None,
        per: str,
        sample: int | None,
        seed: int,
    ) -> list[int]:
        return conc.select(parse_filters(filters), per == "verse", sample, seed)

    @router.get("/searches/{search_id}/kwic")
    def kwic(
        search_id: str,
        offset: int = Query(0, ge=0),
        limit: int = Query(50, ge=1, le=MAX_PAGE),
        left: int = Query(6, ge=0, le=MAX_SIDE),
        right: int = Query(6, ge=0, le=MAX_SIDE),
        sort: str | None = Query(
            None, description="`<position>.<feature>`: hit.lex, L1.text, R2.sp, ..."
        ),
        filter: str | None = Query(
            None, description="Features of the hit: vs:qal,book:Genesis"
        ),
        per: Literal["result", "verse"] = "result",
        sample: int | None = Query(None, ge=1),
        seed: int = 1,
        features: str | None = Query(
            None, description="Comma separated features to give per word"
        ),
        fmt: str | None = None,
        lang: str = "en",
    ) -> dict[str, Any]:
        """A page of the results as concordance lines (KWIC): per result its
        hit, the words before it (`left`) and after it (`right`), and its verse.
        Sorting, filtering, one line per verse and sampling apply to all
        results. See `cfabric_browser.concordance`."""
        job = search_job(search_id)
        conc = concordance(job)
        selection = concordance_selection(conc, filter, per, sample, seed)
        try:
            selection = conc.sort(selection, sort)
        except ValueError as e:
            raise HTTPException(400, str(e)) from None
        api = conc.api
        config = service.config(job.corpus)
        fmt = formats.chosen(config, fmt)
        names = split_items(features) or []
        lines = []
        for i in selection[offset : offset + limit]:
            line = conc.line(i, left, right, names, fmt)
            verse = line.pop("verse")
            line["ref"] = (
                section_ref_for(api, config, verse, lang=lang) if verse else None
            )
            lines.append(line)
        return {
            "search": job.snapshot(),
            "total": len(selection),
            "words": conc.last - conc.first + 1,
            "offset": offset,
            "lines": lines,
        }

    @router.get("/searches/{search_id}/kwic/facets")
    def kwic_facets(
        search_id: str,
        feature: str,
        filter: str | None = None,
        per: Literal["result", "verse"] = "result",
        top: int = Query(50, ge=1, le=500),
    ) -> dict[str, Any]:
        """How often each value of a feature of the hit's first word occurs in
        the (filtered) results, most frequent first; `book` gives the books in
        their order."""
        job = search_job(search_id)
        conc = concordance(job)
        selection = concordance_selection(conc, filter, per, None, 1)
        return {
            "feature": feature,
            "total": len(selection),
            "values": conc.facet(selection, feature, top),
        }

    @router.get("/searches/{search_id}/collocations")
    def collocations(
        search_id: str,
        feature: str = "lex",
        window: int = Query(3, ge=1, le=10),
        filter: str | None = None,
        per: Literal["result", "verse"] = "result",
        min_count: int = Query(2, ge=1, alias="min"),
        top: int = Query(40, ge=1, le=500),
    ) -> dict[str, Any]:
        """The values of a feature of the words within `window` words of the
        hits, by their logDice score: how typical they are near the hit,
        compared with the whole corpus."""
        job = search_job(search_id)
        conc = concordance(job)
        selection = concordance_selection(conc, filter, per, None, 1)
        return {
            "feature": feature,
            "window": window,
            "hits": len(selection),
            "values": conc.collocations(
                selection, feature, window, id(conc.api), min_count, top
            ),
        }

    # statistics over the results of a search

    def position_nodes(job: SearchJob, position: int) -> list[int]:
        rows = job.page(0, job.count)
        if rows and not 0 <= position < len(rows[0]):
            raise HTTPException(404, f"Results have no position {position + 1}")
        return [row[position] for row in rows]

    @router.get("/searches/{search_id}/stats/features")
    def stats_features(
        search_id: str, position: int = Query(0, ge=0)
    ) -> dict[str, Any]:
        """Features that have values on the nodes at one position of the results.

        Looks at a sample of the results; features the template mentions for
        this position come first.
        """
        job = search_job(search_id)
        api = corpus_api(job.corpus)
        config = service.config(job.corpus)
        nodes = position_nodes(job, position)
        if not nodes:
            return {
                "position": position,
                "type": None,
                "features": [],
                "inTemplate": [],
                "open": [],
            }
        sample = nodes[:: max(1, len(nodes) // STATS_SAMPLE)][:STATS_SAMPLE]
        nType = api.F.otype.v(nodes[0])
        features = []
        for name in sorted(api.Fall()):
            if name == "otype" or name in config.excluded_features:
                continue
            fObj = api.Fs(name)
            if any(fObj.v(n) not in (None, "") for n in sample):
                features.append(name)
        perAtom = template_constraints(api, job.template)
        mentioned = perAtom[position] if position < len(perAtom) else []
        inTemplate = [name for (name, _) in mentioned if name in features]
        # features the template leaves open (`lex*`) are the interesting ones to count
        openFeatures = [
            name for (name, fixed) in mentioned if not fixed and name in features
        ]
        return {
            "position": position,
            "type": nType,
            "features": features,
            "inTemplate": inTemplate,
            "open": openFeatures,
        }

    @router.get("/searches/{search_id}/stats/values")
    def stats_values(
        search_id: str,
        feature: str,
        position: int = Query(0, ge=0),
        top: int = Query(25, ge=1, le=500),
    ) -> dict[str, Any]:
        """How often each value of a feature occurs at one position of the results.

        Counts distinct nodes, so a word that occurs in several results counts
        once. Values the corpus marks as "no value" (NA, none, ...) are counted
        separately.
        """
        job = search_job(search_id)
        api = corpus_api(job.corpus)
        if feature not in api.Fall() or feature == "otype":
            raise HTTPException(404, f"Unknown feature: {feature}")
        config = service.config(job.corpus)
        nodes = set(position_nodes(job, position))
        fObj = api.Fs(feature)
        counts: Counter[str] = Counter()
        empty = 0
        for n in nodes:
            value = fObj.v(n)
            if value is None or value == "" or value in config.none_values:
                empty += 1
            else:
                counts[str(value)] += 1
        ranked = counts.most_common()
        shown = ranked[:top]
        rest = ranked[top:]
        return {
            "search": job.snapshot(),
            "position": position,
            "type": api.F.otype.v(next(iter(nodes))) if nodes else None,
            "feature": feature,
            "nodes": len(nodes),
            "withoutValue": empty,
            "distinct": len(counts),
            "values": [{"value": v, "count": c} for (v, c) in shown],
            "other": {"values": len(rest), "count": sum(c for (_, c) in rest)},
        }

    @router.get("/searches/{search_id}/stats/sections")
    def stats_sections(
        search_id: str,
        level: int = Query(0, ge=0, le=2),
        lang: str = "en",
    ) -> dict[str, Any]:
        """Results per section (book, chapter, ...) of their first node, in text order."""
        job = search_job(search_id)
        api = corpus_api(job.corpus)
        config = service.config(job.corpus)
        T, L = api.T, api.L
        sectionTypes = list(T.sectionTypes or ())
        if level >= len(sectionTypes):
            raise HTTPException(404, "This corpus has no sections at that level")
        sType = sectionTypes[level]
        slotType = api.F.otype.slotType
        eOslots = api.E.oslots.s
        counts: Counter[int] = Counter()
        for row in job.page(0, job.count):
            first = row[0]
            if api.F.otype.v(first) == sType:
                counts[first] += 1
                continue
            # the section of the node's first slot, as in references; a node
            # that crosses sections lies in none of them as a whole
            slot = first if api.F.otype.v(first) == slotType else int(eOslots(first)[0])
            ups = L.u(slot, otype=sType)
            if ups:
                counts[int(ups[0])] += 1
        sections = [
            {**section_item(api, config, n, lang), "count": counts[n]}
            for n in sorted(counts)
        ]
        return {
            "search": job.snapshot(),
            "level": level,
            "type": sType,
            "sections": sections,
        }

    @router.get("/searches/{search_id}/export.tsv")
    def export_tsv(
        search_id: str,
        fmt: str | None = None,
        lang: str = "en",
        excel: bool = Query(
            False,
            description="UTF-16 with byte-order mark, which Excel opens correctly",
        ),
        extra: Annotated[
            list[str] | None,
            Query(
                description="More feature columns for one part of the results:"
                " `3:lex,gloss` adds lex and gloss for the third node of each"
                " result. Can be given several times."
            ),
        ] = None,
    ) -> Response:
        """All results as tab-separated values, one row per result.

        Columns: result number, the section of the result's first node, and for
        each node of the result its number, type and text, followed by the
        features mentioned for it in the template (use `feature*` in a
        template to add a column without restricting the search), and then
        any `extra` features asked for.
        """
        job = search_job(search_id)
        if job.state not in FINISHED:
            raise HTTPException(409, "The search has not finished yet")
        api = corpus_api(job.corpus)
        fmt = formats.chosen(service.config(job.corpus), fmt)
        F, T, Fs = api.F, api.T, api.Fs
        sectionTypes = list(T.sectionTypes or ())
        rows = job.page(0, job.count)
        width = max((len(r) for r in rows), default=0)
        perAtom = template_features(api, job.template)
        features = [list(perAtom[i]) if i < len(perAtom) else [] for i in range(width)]
        nodeFeatures = set(api.Fall()) - {"otype"}
        for spec in extra or ():
            (pos, _, names) = spec.partition(":")
            if not pos.strip().isdigit() or not 1 <= int(pos) <= width:
                raise HTTPException(
                    422, f"Not a part of the results: {spec!r} (use 1 to {width})"
                )
            for name in (n.strip() for n in names.split(",")):
                if not name:
                    continue
                if name not in nodeFeatures:
                    raise HTTPException(422, f"Unknown feature: {name}")
                if name not in features[int(pos) - 1]:
                    features[int(pos) - 1].append(name)

        header = ["R"] + [f"S{i + 1}" for i in range(len(sectionTypes))]
        for i in range(width):
            header += [f"NODE{i + 1}", f"TYPE{i + 1}", f"TEXT{i + 1}"]
            header += [f"{name}{i + 1}" for name in features[i]]
        lines = ["\t".join(header)]
        for r, tup in enumerate(rows, 1):
            section = T.sectionFromNode(tup[0], lang=lang) if sectionTypes else ()
            cells = [str(r)]
            cells += [tsvEsc("" if s is None else s) for s in (section or ())]
            cells += [""] * (1 + len(sectionTypes) - len(cells))
            for i, n in enumerate(tup):
                small = (
                    F.otype.v(n) == F.otype.slotType or len(api.E.oslots.s(n)) <= 100
                )
                text = T.text(n, fmt=fmt) if small else ""
                cells += [str(n), F.otype.v(n), tsvEsc(text)]
                for name in features[i]:
                    value = Fs(name).v(n)
                    cells.append("" if value is None else tsvEsc(value))
            lines.append("\t".join(cells))
        content = "\n".join(lines) + "\n"
        suffix = "-excel" if excel else ""
        return tsv_response(content, f"{job.corpus}-results-{job.id}{suffix}", excel)

    # the lexicon (BHSA): its lexemes, and per lexeme the verses it occurs in

    return router
