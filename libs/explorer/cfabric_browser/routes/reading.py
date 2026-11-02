"""Reading: corpora, features, sections, passages, alignment, differences,
the book view, the Interlinear, and words."""

from __future__ import annotations

from typing import Annotated, Any, Literal

from fastapi import APIRouter, Depends, HTTPException, Query

from cfabric_browser import formats
from cfabric_browser.compare import (
    compare_passages,
    compare_three,
    compare_witnesses,
)
from cfabric_browser.dictionaries.manage import DICTIONARIES
from cfabric_browser.display import (
    RenderOptions,
    node_from_section_ref,
    render_node,
    section_ref_for,
)
from cfabric_browser.interlinear import interlinear_verse
from cfabric_browser.routes.common import (
    MAX_DOWN,
    MAX_PASSAGE,
    Context,
    edge_item,
    render_options,
    section_item,
    section_neighbours,
    section_path,
    split_items,
    witness_row,
)
from cfabric_browser.summary import lexeme_of, summary
from cfabric_browser.witnesses import dress_signs, format_ref, parse_ref


def build(ctx: Context) -> APIRouter:
    router = APIRouter()
    service = ctx.service
    store = ctx.store
    corpus_api = ctx.corpus_api
    check_node = ctx.check_node
    allowed = ctx.allowed

    @router.get("/corpora")
    def list_corpora() -> dict[str, Any]:
        return {
            "corpora": [
                {
                    "name": name,
                    # without loading: a corpus may be compiling for minutes
                    "title": service.title(name),
                    # translations are not compared with the other texts
                    "translation": service.translation(name) is not None,
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
        opts = formats.options(config, opts)
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
        opts = formats.options(config, opts)
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
        names = split_items(features) or []
        return {
            "ref": section_ref_for(api, config, node, lang=opts.lang),
            "path": section_path(api, config, node, opts.lang),
            **section_neighbours(api, config, node, opts.lang),
            "truncated": len(verses) > MAX_PASSAGE,
            "verses": [
                interlinear_verse(
                    service,
                    corpus,
                    v,
                    opts,
                    names,
                    under,
                    lambda w: word_short(corpus, w),
                )
                for v in verses[:MAX_PASSAGE]
            ],
        }

    # the parallel interface: a chapter in several corpora, verse by verse

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
        opts = formats.options(config, opts)
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
        # a translation: notes printed after verses, and verse numbers in the
        # print where it divides differently (`cfabric_browser.translations`)
        extras = {
            extra: {
                str(n): value
                for n in nodes[:MAX_PASSAGE]
                if (value := api.Fs(extra).v(n))
            }
            for extra in ("note", "printed")
            if service.translation(corpus) is not None and extra in api.Fall()
        }
        return {
            "node": node,
            "ref": section_ref_for(api, config, node, lang=opts.lang),
            **{f"{extra}s": values for (extra, values) in extras.items()},
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
        config = service.config(corpus)
        tree: dict[str, Any] = render_node(
            api, config, node, mode, formats.options(config, opts)
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

    def word_bdb(corpus: str, node: int) -> dict[str, Any] | None:
        """A word's entries in BDB, with its lexeme in the corpus whose
        lexicon has it (the BHSA, also for the DSS): `{corpus, node, lex,
        entries}`; None without BDB or a lexeme."""
        bdb = store.get("bdb") if allowed(DICTIONARIES["bdb"].corpus) else None
        if bdb is None:
            return None
        api = corpus_api(corpus)
        own = service.lexicon(corpus)
        entry = None
        if own is not None:
            # the lexeme node the word is in
            up = api.L.u(node, "lex") if "lex" in api.F.otype.all else ()
            entry = own.entry(int(up[0])) if len(up) else None
            source = corpus
        else:
            # by its ETCBC lexeme, in the corpus it borrows glosses from (DSS)
            glosses = service.glosses(corpus)
            source = glosses[1] if glosses else None
            config = service.config(corpus)
            empty = {str(v) for v in config.none_values if v}
            found = lexeme_of(api, node, empty)
            lexicon = service.lexicon(source) if source else None
            entry = lexicon.lookup(*found) if lexicon and found else None
        if entry is None:
            return None
        link = bdb.for_lexeme(entry.get("language"), entry.get("lex"))
        if not link["entries"]:
            return None
        return {
            "corpus": source,
            "node": entry["node"],
            "lex": entry.get("lex"),
            **link,
        }

    def word_short(corpus: str, node: int) -> str | None:
        """BDB's short definition of a word (its main entry), or None."""
        found = word_bdb(corpus, node)
        if found is None:
            return None
        return found["entries"][0]["short"] or None

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
            "bdb": word_bdb(corpus, node),
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
            out = [edge_item(x) for x in (es.f(node) or ())]
            inc = [edge_item(x) for x in (es.t(node) or ())]
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
            "bdb": word_bdb(corpus, node),
            "features": features,
            "edges": edges,
            "up": up,
            "down": down,
            "downCounts": downCounts,
        }

    # search

    return router
