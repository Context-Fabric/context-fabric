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
import threading
from collections import Counter
from dataclasses import replace
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
from fastapi.responses import Response, StreamingResponse
from pydantic import BaseModel, Field

from cfabric_browser.compare import (
    compare_passages,
    compare_three,
    compare_witnesses,
)
from cfabric_browser.concordance import MAX_SIDE, Concordance, parse_filters
from cfabric_browser.corpora import CorpusService, discover, resolve_corpus
from cfabric_browser.interlinear import interlinear_verse
from cfabric_browser.lexicon import Lexicon, render_verse
from cfabric_browser.queries import (
    LANGUAGES,
    check_template,
    template_constraints,
    template_features,
)
from cfabric_browser.summary import summary
from cfabric_browser.translations import TRANSLATIONS, Translation
from cfabric_browser.translations.manage import (
    Jobs,
    cancel_removal,
    is_built,
    is_pending,
    remove,
    repo_of,
    schedule_removal,
)
from cfabric_browser.witnesses import dress_signs, format_ref, parse_ref, place
from cfabric_browser.workers import FINISHED, SearchJob, SearchPool

MAX_PAGE = 500
MAX_DOWN = 200
MAX_PASSAGE = 200
STATS_SAMPLE = 300


class OpenRequest(BaseModel):
    id: str = Field(
        description="A corpus id from the library, e.g. ETCBC/dss, optionally with"
        " a version: DT-UCPH/sp@7.2.4 or DT-UCPH/sp@latest"
    )
    name: str | None = Field(default=None, description="Name to show it under")


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


def build_router(
    service: CorpusService,
    pool: SearchPool,
    library: bool = True,
    translations: Jobs | None = None,
) -> APIRouter:
    router = APIRouter(prefix="/api")
    # translations are built into the first place the library looks
    jobs = translations if translations is not None else Jobs(service.roots[0])

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

    # the library: corpora on this computer that can be opened

    @router.get("/library")
    def corpus_library() -> dict[str, Any]:
        if not library:
            return {"enabled": False, "corpora": []}
        # a corpus counts as open when its data directory is being served
        opened = {spec.path for spec in service.specs}
        corpora = []
        for entry in discover(service.roots, hide=service.hide, only=service.only):
            try:
                path = resolve_corpus(entry["id"], service.roots).path
            except FileNotFoundError:
                continue
            corpora.append({**entry, "open": path in opened})
        return {"enabled": True, "corpora": corpora}

    @router.post("/corpora", status_code=201)
    def open_corpus(request: OpenRequest) -> dict[str, Any]:
        """Open a corpus from the library. Only library ids are accepted, no paths."""
        if not library:
            raise HTTPException(403, "Opening corpora is switched off on this server")
        (requested, _, version) = request.id.partition("@")
        # any corpus on disk can be opened by its id, also copies and hidden ones,
        # but not one outside a library limited with `only`
        ids = {
            entry["id"].lower(): entry["id"]
            for entry in discover(service.roots, collapse=False, only=service.only)
        }
        corpusId = ids.get(requested.lower())
        if corpusId is None:
            raise HTTPException(404, f"Not in the library: {request.id}")
        target = f"{corpusId}@{version}" if version else corpusId
        try:
            spec = resolve_corpus(
                f"{request.name}={target}" if request.name else target, service.roots
            )
        except FileNotFoundError as e:
            raise HTTPException(404, str(e)) from None
        if service.has(spec.name):
            if service.spec(spec.name).path != spec.path:
                raise HTTPException(
                    409, f"Another corpus is already open as {spec.name!r}"
                )
            return {"name": spec.name, "opened": False}
        service.add(spec)
        pool.add_corpus(spec.name, str(spec.path))
        # start loading now, so the corpus is ready by the time it is shown
        threading.Thread(target=service.config, args=(spec.name,), daemon=True).start()
        return {"name": spec.name, "opened": True}

    # translations: built from their source into a corpus, then in the library

    def translation_of(tid: str) -> Translation:
        translation = TRANSLATIONS.get(tid)
        if translation is None or not offered(translation):
            raise HTTPException(404, f"Unknown translation: {tid}")
        return translation

    def offered(translation: Translation) -> bool:
        only = {o.lower() for o in service.only}
        return not only or translation.corpus_id.lower() in only

    def translation_entry(translation: Translation) -> dict[str, Any]:
        added = is_built(translation, jobs.root)
        repo = repo_of(translation, jobs.root).resolve()
        return {
            "id": translation.id,
            "title": translation.title,
            "name": translation.name,
            "language": translation.language,
            "corpus": translation.corpus_id,
            "licence": translation.licence,
            "licenceNote": translation.licence_note,
            "source": translation.source,
            "added": added,
            "open": any(spec.path.is_relative_to(repo) for spec in service.specs),
            # removed at the next start, because it is open
            "removing": added and is_pending(translation, jobs.pending),
            "job": jobs.status(translation.id),
        }

    @router.get("/translations")
    def list_translations() -> dict[str, Any]:
        if not library:
            return {"enabled": False, "translations": []}
        return {
            "enabled": True,
            "translations": [
                translation_entry(t) for t in TRANSLATIONS.values() if offered(t)
            ],
        }

    @router.get("/translations/{tid}")
    def describe_translation(tid: str) -> dict[str, Any]:
        return translation_entry(translation_of(tid))

    @router.post("/translations/{tid}", status_code=202)
    def add_translation(tid: str) -> dict[str, Any]:
        """Download and build a translation; `job` in the answer, and in
        `GET /translations/{tid}`, shows the progress."""
        if not library:
            raise HTTPException(403, "Adding texts is switched off on this server")
        translation = translation_of(tid)
        if is_built(translation, jobs.root):
            # still there: adding it again only cancels a removal
            cancel_removal(translation, jobs.pending)
        else:
            jobs.start(translation)
        return translation_entry(translation)

    @router.delete("/translations/{tid}")
    def remove_translation(tid: str) -> dict[str, Any]:
        if not library:
            raise HTTPException(403, "Removing texts is switched off on this server")
        translation = translation_of(tid)
        if not is_built(translation, jobs.root):
            raise HTTPException(404, f"{translation.title} has not been added")
        if translation_entry(translation)["open"]:
            schedule_removal(translation, jobs.pending)
        else:
            try:
                remove(translation, jobs.root)
            except OSError as e:
                raise HTTPException(409, f"Could not remove it: {e}") from None
        return translation_entry(translation)

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
        children = [section_item(api, config, n, lang) for n in childNodes]
        return {"node": node, "heading": heading, "children": children}

    @router.get("/corpora/{corpus}/align")
    def align(
        corpus: str,
        ref: str,
        source: Annotated[str, Query(alias="from")],
        lang: str = "en",
        view: Literal["sections", "books"] = "sections",
    ) -> dict[str, Any]:
        """The passage of this corpus that corresponds to `ref` in `from`.

        Books are matched with a concordance of book names; chapters and
        verses by number. `exact` is false when only an enclosing section
        exists here (a verse missing in this corpus, for instance).
        With `view=books`, the passage in this corpus' book view (DSS).
        """
        corpus_api(corpus)
        corpus_api(source)
        try:
            result = service.align(source, ref, corpus, lang, view)
        except ValueError as e:
            raise HTTPException(404, str(e)) from None
        if result is None:
            raise HTTPException(404, f"{corpus} has no counterpart of {ref}")
        return result

    @router.get("/compare")
    def compare(
        a: str,
        aref: str,
        b: str,
        bref: str,
        lang: str = "en",
        bview: Literal["sections", "books"] = "sections",
    ) -> dict[str, Any]:
        """Words that differ between passage `aref` of `a` and `bref` of `b`.

        `a` and `b` map word nodes of each corpus to `matres` (only matres
        lectionis differ) or `variant`. With `bview=books`, `bref` is a passage
        in the book view of `b` (DSS): each scroll is compared with `a`, and
        `aWitnesses` names, per marked word of `a`, the scrolls that differ.
        See `cfabric_browser.compare`.
        """
        corpus_api(a)
        corpus_api(b)
        try:
            if bview == "books":
                return compare_witnesses(service, a, aref, b, bref, lang)
            return compare_passages(service, a, aref, b, bref, lang)
        except ValueError as e:
            raise HTTPException(404, str(e)) from None

    # reading by biblical book (DSS)

    def book_index(corpus: str):
        corpus_api(corpus)
        index = service.book_index(corpus)
        if index is None:
            raise HTTPException(404, f"{corpus} cannot be read by biblical book")
        return index

    @router.get("/corpora/{corpus}/books")
    def books(corpus: str) -> dict[str, Any]:
        """The biblical books in this corpus, with chapters, verses and scrolls."""
        return {"books": book_index(corpus).summary()}

    def witness_row(api, config, index, w, mode: str, opts: RenderOptions) -> dict:
        """The text of one verse in one scroll, for the book view."""
        # a witness links to its fragment in the scroll view, for context
        types = list(api.T.sectionTypes or [])
        placeType = types[1] if len(types) > 1 else (types[0] if types else None)
        fragments: list[dict[str, Any]] = []
        tree: dict[str, Any] = {"fmt": None, "direction": config.direction}
        for word in w.words:
            tree = render_node(api, config, word, mode, opts).to_dict()
            fragments.extend(tree["fragments"])
        dress_signs(api, fragments)
        where = api.L.u(w.words[0], otype=placeType)[0] if placeType else None
        return {
            "scroll": w.scroll,
            "place": place(api, w.words),
            "ref": section_ref_for(api, config, int(where), lang=opts.lang)
            if where
            else None,
            "words": w.words,
            "alsoNonBiblical": w.also_non_biblical,
            "tree": {
                "node": w.words[0],
                "type": index.word_type,
                "mode": mode,
                "fmt": tree["fmt"],
                "direction": tree["direction"],
                "heading": f"{w.scroll} {place(api, w.words)}",
                "fragments": fragments,
            },
        }

    @router.get("/corpora/{corpus}/witnesses")
    def witnesses(
        corpus: str,
        ref: str,
        opts: Annotated[RenderOptions, Depends(render_options)],
        mode: Literal["pretty", "plain"] = "plain",
    ) -> dict[str, Any]:
        """A chapter or verse in the book view: per verse, the text of each scroll.

        Each witness has the scroll, the place of its words (`f1i:1–2`), a
        reference to open its fragment in the scroll, its words and a render
        tree of them, in which reconstructed text is flagged `dim`.
        `alsoNonBiblical` is set for text that the DSS also lists among
        non-biblical material (`biblical=2`).
        """
        index = book_index(corpus)
        parsed = parse_ref(ref)
        if parsed is None or not index.has(*parsed):
            raise HTTPException(404, f"{corpus} has no {ref}")
        (book, chapter, verse) = parsed
        api = corpus_api(corpus)
        config = service.config(corpus)
        chapterVerses = index.books[book][chapter]
        verses = []
        for v in [verse] if verse is not None else list(chapterVerses):
            rows = [
                witness_row(api, config, index, w, mode, opts) for w in chapterVerses[v]
            ]
            verses.append(
                {"verse": v, "ref": format_ref(book, chapter, v), "witnesses": rows}
            )
        return {
            "ref": format_ref(book, chapter, verse),
            "book": book,
            "chapter": chapter,
            "chapterRef": format_ref(book, chapter),
            "verses": verses,
            **index.neighbours(book, chapter),
        }

    # the interlinear: a passage word by word

    @router.get("/corpora/{corpus}/interlinear")
    def interlinear(
        corpus: str,
        ref: str,
        opts: Annotated[RenderOptions, Depends(render_options)],
        features: str | None = Query(
            None, description="Comma separated features to add as lines"
        ),
        under: str | None = Query(
            None, description="Another corpus to show word by word under this one"
        ),
    ) -> dict[str, Any]:
        """A passage (a chapter or a verse) word by word, for the interlinear.

        Per verse its words, each with its text and its lines in words
        (`translit`, `lexeme`, `gloss`, `pos`, `morph`, read by role as on the
        hover card), the values of `features`, and the indexes of its phrase
        and clause in `phrases` and `clauses` (with the phrase function and
        clause type). With `under`, also that corpus' verse, its words under
        the words of this one, and the words that differ. See
        `cfabric_browser.interlinear`.
        """
        api = corpus_api(corpus)
        if under:
            corpus_api(under)
        config = service.config(corpus)
        try:
            node = node_from_section_ref(
                api, ref, config.section_sep1, config.section_sep2, opts.lang
            )
        except ValueError as e:
            raise HTTPException(404, str(e)) from None
        verseType = (api.T.sectionTypes or [None])[-1]
        if api.F.otype.v(node) == verseType:
            verses = [node]
        else:
            verses = [int(v) for v in api.L.d(node, otype=verseType)]
        names = _items(features) or []
        return {
            "ref": section_ref_for(api, config, node, lang=opts.lang),
            "path": section_path(api, config, node, opts.lang),
            **section_neighbours(api, config, node, opts.lang),
            "truncated": len(verses) > MAX_PASSAGE,
            "verses": [
                interlinear_verse(service, corpus, v, opts, names, under)
                for v in verses[:MAX_PASSAGE]
            ],
        }

    # the parallel interface: a chapter in several corpora, verse by verse

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
        names = _items(texts) or []
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
            own = (
                opts
                if opts.fmt in (None, *config.text_formats)
                else replace(opts, fmt=None)
            )
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
                cells.setdefault(key, {}).setdefault(name, {"trees": []})[
                    "trees"
                ].append(tree)

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

    @router.get("/agreement")
    def agreement(
        a: str, aref: str, b: str, bref: str, c: str, cref: str, lang: str = "en"
    ) -> dict[str, Any]:
        """Which scrolls of `c` (DSS, book view) side with `a` (MT) or `b` (SP).

        Per verse, every place where `a` and `b` differ, with the words of each
        and, per scroll that preserves it, its reading there and whether it
        `agrees` with `a`, `b` or neither (`other`). See
        `cfabric_browser.compare.compare_three`.
        """
        for corpus in (a, b, c):
            corpus_api(corpus)
        try:
            return compare_three(service, a, aref, b, bref, c, cref, lang)
        except ValueError as e:
            raise HTTPException(404, str(e)) from None

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
        for tree in trees:
            dress_signs(api, tree["fragments"])
        return {
            "node": node,
            "ref": section_ref_for(api, config, node, lang=opts.lang),
            "total": len(nodes),
            "truncated": len(nodes) > MAX_PASSAGE,
            "path": section_path(api, config, node, opts.lang),
            **section_neighbours(api, config, node, opts.lang),
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

    def word_summary(corpus: str, node: int) -> list[dict[str, str]]:
        """The essentials of a word; in a corpus without glosses (the DSS),
        with the gloss from the lexicon of another open corpus (the BHSA)."""
        api = corpus_api(corpus)
        config = service.config(corpus)
        empty = {str(v) for v in config.none_values if v is not None}
        glosses = service.glosses(corpus)
        if glosses is None:
            return summary(api, node, empty)
        (gloss_for, source) = glosses
        return summary(api, node, empty, gloss_for, f"gloss in {source}")

    @router.get("/corpora/{corpus}/nodes/{node}/summary")
    def node_summary(corpus: str, node: int, lang: str = "en") -> dict[str, Any]:
        """The essentials of a node, for the card shown when hovering over a
        word: its text, and its gloss, lexeme, part of speech and morphology in
        words (see `cfabric_browser.summary`)."""
        api = corpus_api(corpus)
        check_node(api, node)
        config = service.config(corpus)
        slots = api.E.oslots.s(node)
        return {
            "node": node,
            "type": api.F.otype.v(node),
            "ref": section_ref_for(api, config, node, lang=lang),
            "text": api.T.text(node) if len(slots) <= 200 else None,
            "summary": word_summary(corpus, node),
        }

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
            # oslots is structure; omap@... maps nodes between corpus versions
            if name == "oslots" or name.startswith("omap@"):
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
            "summary": word_summary(corpus, node),
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
        names = [n for n in (_items(texts) or []) if n != corpus]
        for name in names:
            corpus_api(name)
        sectionTypes = list(api.T.sectionTypes or ())
        unitType = sectionTypes[-1] if sectionTypes else None
        oslots = api.E.oslots.s
        maxSlot = api.F.otype.maxSlot
        rank = api.N.otypeRank

        def slots(n: int) -> set[int]:
            return {n} if n <= maxSlot else {int(s) for s in oslots(n)}

        rows = []
        for i, tup in enumerate(job.page(offset, limit)):
            nodes = [int(n) for n in tup]
            if unitType is None:
                continue
            # the first node that lies within a verse: not a book or chapter
            # that the template starts with
            first = next(
                (n for n in nodes if rank[api.F.otype.v(n)] <= rank[unitType]),
                nodes[0],
            )
            if api.F.otype.v(first) == unitType:
                unit = first
            else:
                ups = api.L.u(min(slots(first)), otype=unitType)
                if not ups:
                    continue
                unit = int(ups[0])
            # highlight the innermost nodes: those that contain no other node of the result
            sets = {n: slots(n) for n in nodes}
            inner = [
                n
                for n in nodes
                if n != unit and not any(m != n and sets[m] < sets[n] for m in nodes)
            ]
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
        own = (
            opts
            if opts.fmt in (None, *config.text_formats)
            else replace(opts, fmt=None)
        )
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

    concordances: dict[str, tuple[int, Concordance]] = {}
    concordanceLock = threading.Lock()

    def concordance(job: SearchJob) -> Concordance:
        """The concordance of a search's results, rebuilt when they change."""
        snapshot = job.snapshot()
        with concordanceLock:
            cached = concordances.get(job.id)
            if cached and cached[0] == snapshot["version"]:
                return cached[1]
        api = corpus_api(job.corpus)
        built = Concordance(api, service.config(job.corpus), job.page(0, job.count))
        with concordanceLock:
            concordances[job.id] = (snapshot["version"], built)
            # keep a few: they hold a list of words per result
            for old in list(concordances)[:-8]:
                del concordances[old]
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
        names = _items(features) or []
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

    def corpus_lexicon(corpus: str) -> Lexicon:
        corpus_api(corpus)
        lexicon = service.lexicon(corpus)
        if lexicon is None:
            raise HTTPException(404, f"{corpus} has no lexicon")
        return lexicon

    def lexeme(lexicon: Lexicon, node: int) -> dict[str, Any]:
        entry = lexicon.entry(node)
        if entry is None:
            raise HTTPException(404, f"No lexeme {node}")
        return entry

    @router.get("/corpora/{corpus}/lexicon")
    def lexicon_entries(corpus: str) -> dict[str, Any]:
        """All lexemes, in alphabetical order per language.

        Each has its node, `lex` (transliteration), `form` (vocalised),
        `cons` (consonants), `letter` (the first one), `gloss`, `sp` (part of
        speech), `language`, `freq`, `rank` and `nametype`, as far as the
        corpus has them; `features` names the corpus feature behind each.
        """
        lexicon = corpus_lexicon(corpus)
        return {"features": lexicon.features, "entries": lexicon.entries}

    @router.get("/corpora/{corpus}/lexicon/{node}")
    def lexicon_entry(corpus: str, node: int, lang: str = "en") -> dict[str, Any]:
        """One lexeme, how often it occurs per book, and in how many verses."""
        lexicon = corpus_lexicon(corpus)
        entry = lexeme(lexicon, node)
        return {
            **entry,
            "verses": len(lexicon.verses(node)),
            "books": lexicon.distribution(node, lang=lang),
        }

    @router.get("/corpora/{corpus}/lexicon/{node}/occurrences")
    def lexicon_occurrences(
        corpus: str,
        node: int,
        offset: int = Query(0, ge=0),
        limit: int = Query(50, ge=1, le=MAX_PAGE),
        book: str | None = None,
        fmt: str | None = None,
        lang: str = "en",
    ) -> dict[str, Any]:
        """A page of the verses a lexeme occurs in, as running text with its
        words marked; with `book`, only the verses in that book."""
        lexicon = corpus_lexicon(corpus)
        lexeme(lexicon, node)
        api = corpus_api(corpus)
        config = service.config(corpus)
        verses = lexicon.verses(node, book=book, lang=lang)
        return {
            "total": len(verses),
            "offset": offset,
            "rows": [
                render_verse(api, config, verse, words, fmt=fmt or None, lang=lang)
                for (verse, words) in verses[offset : offset + limit]
            ],
        }

    @router.get("/corpora/{corpus}/lexicon/{node}/occurrences.tsv")
    def lexicon_tsv(
        corpus: str,
        node: int,
        book: str | None = None,
        fmt: str | None = None,
        lang: str = "en",
        excel: bool = Query(
            False,
            description="UTF-16 with byte-order mark, which Excel opens correctly",
        ),
    ) -> Response:
        """Every occurrence of a lexeme as tab-separated values: its section,
        the word (node and text) and the text of its verse."""
        lexicon = corpus_lexicon(corpus)
        entry = lexeme(lexicon, node)
        api = corpus_api(corpus)
        T = api.T
        sectionTypes = list(T.sectionTypes or ())
        header = [*(t.upper() for t in sectionTypes), "WORD", "TEXT", "VERSE_TEXT"]
        lines = ["\t".join(header)]
        for verse, words in lexicon.verses(node, book=book, lang=lang):
            section = T.sectionFromNode(verse, lang=lang)
            verseText = tsvEsc(T.text(verse, fmt=fmt or None))
            for w in words:
                cells = [tsvEsc("" if s is None else s) for s in section]
                cells += [str(w), tsvEsc(T.text(w, fmt=fmt or None)), verseText]
                lines.append("\t".join(cells))
        content = "\n".join(lines) + "\n"
        name = "".join(c if c.isalnum() else "_" for c in entry.get("lex") or "")
        suffix = "-excel" if excel else ""
        filename = f"{corpus}-{name.strip('_') or node}{suffix}"
        return tsv_response(content, filename, excel)

    return router


def tsv_response(content: str, filename: str, excel: bool) -> Response:
    """Tab-separated values as a download, as UTF-8 or as UTF-16 for Excel."""
    return Response(
        content.encode("utf-16") if excel else content.encode("utf-8"),
        media_type="text/tab-separated-values; charset="
        + ("utf-16" if excel else "utf-8"),
        headers={"Content-Disposition": f'attachment; filename="{filename}.tsv"'},
    )


def section_item(api, config, node: int, lang: str) -> dict[str, Any]:
    """A section node as shown in navigation: its label and full reference."""
    T = api.T
    section = T.sectionFromNode(node, lang=lang)
    nType = api.F.otype.v(node)
    item = {
        "node": node,
        "type": nType,
        "label": str(section[-1]) if section else str(node),
        "ref": section_ref_for(api, config, node, lang=lang),
    }
    # the feature that names it, with its own value, for search templates: the
    # BHSA's book feature says Jesaia where the label (book@en) says Isaiah
    types = list(T.sectionTypes or ())
    feats = list(T.sectionFeats or ())
    if nType in types and types.index(nType) < len(feats):
        feature = feats[types.index(nType)]
        value = api.Fs(feature).v(node) if api.Fs(feature) is not None else None
        if value is not None:
            item["feature"] = feature
            item["value"] = str(value)
    return item


def section_path(api, config, node: int, lang: str) -> list[dict[str, Any]]:
    """The sections a section node lies in, from the top down, ending with itself."""
    F, L, T = api.F, api.L, api.T
    nType = F.otype.v(node)
    path = []
    for sType in T.sectionTypes:
        if sType == nType:
            path.append(section_item(api, config, node, lang))
            break
        ups = L.u(node, otype=sType)
        if ups:
            path.append(section_item(api, config, int(ups[0]), lang))
    return path


def section_neighbours(api, config, node: int, lang: str) -> dict[str, Any]:
    """The previous and next section of the same level, across parents."""
    F = api.F
    siblings = F.otype.s(F.otype.v(node))
    (first, last) = (int(siblings[0]), int(siblings[-1]))
    # section nodes of one type are numbered consecutively in text order
    prev = node - 1 if node > first else None
    nxt = node + 1 if node < last else None
    return {
        "prev": None if prev is None else section_item(api, config, prev, lang),
        "next": None if nxt is None else section_item(api, config, nxt, lang),
    }


def _edge_item(x: Any) -> dict[str, Any]:
    if isinstance(x, tuple):
        return {"node": int(x[0]), "value": x[1]}
    return {"node": int(x)}
