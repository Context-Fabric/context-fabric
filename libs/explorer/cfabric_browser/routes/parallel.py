"""The Parallel interface: verse rows across corpora, also for search results."""

from __future__ import annotations

from dataclasses import replace
from typing import Annotated, Any, Literal

from fastapi import APIRouter, Depends, HTTPException, Query

from cfabric_browser import formats
from cfabric_browser.concordance import Spans
from cfabric_browser.display import (
    RenderOptions,
    node_from_section_ref,
    render_node,
    section_ref_for,
)
from cfabric_browser.routes.common import (
    MAX_PAGE,
    Context,
    render_options,
    section_neighbours,
    section_path,
    split_items,
    witness_row,
)
from cfabric_browser.witnesses import dress_signs, format_ref, parse_ref


def build(ctx: Context) -> APIRouter:
    router = APIRouter()
    service = ctx.service
    corpus_api = ctx.corpus_api
    search_job = ctx.search_job

    @router.get("/parallel")
    def parallel(
        ref: str,
        lead: str,
        opts: Annotated[RenderOptions, Depends(render_options)],
        texts: str = "",
        mode: Literal["pretty", "plain"] = "plain",
    ) -> dict[str, Any]:
        """A chapter in several corpora, as rows of verses with a cell per corpus.

        `ref` is a chapter of `lead` (`Genesis 1`), whose numbering the rows
        follow; `texts` names the corpora of the columns (comma separated),
        with or without `lead`. Books are matched through the book concordance,
        chapters and verses by number. There is a row for every verse number
        in any of the texts, in numeric order, and a cell only where a text has
        that verse. A corpus read by biblical book (DSS) gets, per verse, the
        text of each scroll that has it, as in `witnesses`. `fmt` applies to
        the corpora that have that format; the others use their default.

        Per column: its `kind` (`text` or `witnesses`) and the `ref` of its
        chapter, or None when it has no counterpart of `ref`.
        """
        names = split_items(texts) or []
        lApi = corpus_api(lead)
        for name in names:
            corpus_api(name)
        lConfig = service.config(lead)
        try:
            node = node_from_section_ref(
                lApi, ref, lConfig.section_sep1, lConfig.section_sep2, opts.lang
            )
        except ValueError as e:
            raise HTTPException(404, str(e)) from None
        chapterRef = section_ref_for(lApi, lConfig, node, lang=opts.lang)

        columns: list[dict[str, Any]] = []
        cells: dict[int, dict[str, Any]] = {}
        for name in names:
            api = service.api(name)
            config = service.config(name)
            own = formats.options(config, opts)
            index = service.book_index(name)
            if index is not None:
                aligned = service.align(lead, chapterRef, name, opts.lang, "books")
                parsed = parse_ref(aligned["ref"]) if aligned else None
                if parsed is None or not index.has(parsed[0], parsed[1]):
                    columns.append({"name": name, "kind": "witnesses", "ref": None})
                    continue
                (book, chapter, _) = parsed
                columns.append(
                    {
                        "name": name,
                        "kind": "witnesses",
                        "ref": format_ref(book, chapter),
                    }
                )
                for v, witnesses in index.books[book][chapter].items():
                    cells.setdefault(int(v), {})[name] = {
                        "witnesses": [
                            witness_row(api, config, index, w, mode, own)
                            for w in witnesses
                        ]
                    }
                continue

            sectionTypes = list(api.T.sectionTypes or ())
            if name == lead:
                cNode: int | None = node
            else:
                aligned = service.align(lead, chapterRef, name, opts.lang)
                exact = aligned is not None and aligned["exact"]
                cNode = aligned["node"] if exact else None
            if cNode is None or len(sectionTypes) < 3:
                columns.append({"name": name, "kind": "text", "ref": None})
                continue
            columns.append(
                {
                    "name": name,
                    "kind": "text",
                    "ref": section_ref_for(api, config, cNode, lang=opts.lang),
                }
            )
            for v in api.L.d(cNode, otype=sectionTypes[2]):
                number = api.T.sectionFromNode(int(v), lang="en")[2]
                try:
                    key = int(number)
                except (TypeError, ValueError):
                    continue
                tree = render_node(api, config, int(v), mode, own).to_dict()
                dress_signs(api, tree["fragments"])
                cell = cells.setdefault(key, {}).setdefault(name, {"trees": []})
                cell["trees"].append(tree)
                # a translation: a note printed after the verse, and the verse's
                # number in the print where it is divided differently
                for extra in ("note", "printed"):
                    if extra in api.Fall() and (value := api.Fs(extra).v(int(v))):
                        cell[extra] = value

        rows = [
            {
                "verse": v,
                "ref": f"{chapterRef}{lConfig.section_sep2}{v}",
                "cells": cells[v],
            }
            for v in sorted(cells)
        ]
        return {
            "ref": chapterRef,
            "path": section_path(lApi, lConfig, node, opts.lang),
            **section_neighbours(lApi, lConfig, node, opts.lang),
            "columns": columns,
            "rows": rows,
        }

    @router.get("/searches/{search_id}/parallel")
    def search_parallel(
        search_id: str,
        opts: Annotated[RenderOptions, Depends(render_options)],
        offset: int = Query(0, ge=0),
        limit: int = Query(25, ge=1, le=MAX_PAGE),
        texts: str = "",
        mode: Literal["pretty", "plain"] = "plain",
    ) -> dict[str, Any]:
        """A page of results as rows of the parallel interface.

        Per result: the verse (the lowest section) its first node lies in, in
        the searched corpus with the result's innermost nodes highlighted, and
        the same verse in each of `texts`, as in `/parallel` (for the DSS, the
        text of each scroll that has it). Cells are left out where a text has
        no such verse.
        """
        job = search_job(search_id)
        corpus = job.corpus
        api = corpus_api(corpus)
        config = service.config(corpus)
        names = [n for n in (split_items(texts) or []) if n != corpus]
        for name in names:
            corpus_api(name)
        sectionTypes = list(api.T.sectionTypes or ())
        unitType = sectionTypes[-1] if sectionTypes else None
        rank = api.N.otypeRank
        spans = Spans(api)

        rows = []
        for i, tup in enumerate(job.page(offset, limit)):
            nodes = [int(n) for n in tup]
            if unitType is None:
                break
            # the first node that lies within a verse: not a book or chapter
            # that the template starts with
            first = next(
                (n for n in nodes if rank[api.F.otype.v(n)] <= rank[unitType]),
                nodes[0],
            )
            if api.F.otype.v(first) == unitType:
                unit = first
            else:
                ups = api.L.u(spans.span(first)[0], otype=unitType)
                if not ups:
                    continue
                unit = int(ups[0])
            # highlight the innermost nodes: those that contain no other node of the result
            inner = [n for n in spans.innermost(nodes) if n != unit]
            own = replace(opts, highlights={n: "" for n in inner})
            tree = render_node(api, config, unit, mode, own).to_dict()
            dress_signs(api, tree["fragments"])
            ref = section_ref_for(api, config, unit, lang=opts.lang)
            cells: dict[str, Any] = {corpus: {"trees": [tree]}}
            for name in names:
                cell = verse_cell(corpus, ref, name, opts, mode)
                if cell is not None:
                    cells[name] = cell
            rows.append(
                {"index": offset + i, "nodes": nodes, "ref": ref, "cells": cells}
            )
        return {
            "search": job.snapshot(),
            "offset": offset,
            "columns": [corpus, *names],
            "rows": rows,
        }

    def verse_cell(
        source: str, ref: str, target: str, opts: RenderOptions, mode: str
    ) -> dict[str, Any] | None:
        """The verse `ref` of `source` in `target`, as a cell of the parallel
        interface, or None when `target` does not have it."""
        api = service.api(target)
        config = service.config(target)
        own = formats.options(config, opts)
        own = replace(own, highlights=None)
        index = service.book_index(target)
        if index is not None:
            aligned = service.align(source, ref, target, opts.lang, "books")
            parsed = parse_ref(aligned["ref"]) if aligned and aligned["exact"] else None
            if parsed is None or parsed[2] is None or not index.has(*parsed):
                return None
            (book, chapter, verse) = parsed
            return {
                "witnesses": [
                    witness_row(api, config, index, w, mode, own)
                    for w in index.books[book][chapter][verse]
                ]
            }
        aligned = service.align(source, ref, target, opts.lang)
        if aligned is None or not aligned["exact"]:
            return None
        tree = render_node(api, config, aligned["node"], mode, own).to_dict()
        dress_signs(api, tree["fragments"])
        return {"trees": [tree]}

    # the concordance: results as lines with the hit in context

    return router
