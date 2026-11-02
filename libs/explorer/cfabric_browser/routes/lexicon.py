"""The lexicon of corpora with lexeme nodes (BHSA), with BDB."""

from __future__ import annotations

from typing import Any

from fastapi import APIRouter, HTTPException, Query
from fastapi.responses import Response

from cfabric_browser import formats
from cfabric_browser.dictionaries.manage import DICTIONARIES
from cfabric_browser.helpers import tsvEsc
from cfabric_browser.lexicon import Lexicon, render_verse
from cfabric_browser.routes.common import (
    MAX_PAGE,
    Context,
    tsv_response,
)


def build(ctx: Context) -> APIRouter:
    router = APIRouter()
    service = ctx.service
    store = ctx.store
    corpus_api = ctx.corpus_api
    allowed = ctx.allowed

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
        With BDB, a linked lexeme also has `bdbShort` (its main entry's short
        definition), `bdbOrder` (that entry's place in BDB), `bdbLetter` (the
        letter of its part, its root's) and `bdbRoot` (whether it is a root).
        """
        lexicon = corpus_lexicon(corpus)
        bdb = store.get("bdb") if allowed(DICTIONARIES["bdb"].corpus) else None
        if bdb is None:
            return {"features": lexicon.features, "entries": lexicon.entries}
        return {
            "features": lexicon.features,
            "entries": [
                {**e, **bdb.for_list(e.get("language"), e.get("lex"))}
                for e in lexicon.entries
            ],
            "bdb": True,
        }

    @router.get("/corpora/{corpus}/lexicon/{node}")
    def lexicon_entry(corpus: str, node: int, lang: str = "en") -> dict[str, Any]:
        """One lexeme, how often it occurs per book, and in how many verses;
        `bdb`: its entries in BDB when that has been added, else None."""
        lexicon = corpus_lexicon(corpus)
        entry = lexeme(lexicon, node)
        bdb = store.get("bdb") if allowed(DICTIONARIES["bdb"].corpus) else None
        return {
            **entry,
            "verses": len(lexicon.verses(node)),
            "books": lexicon.distribution(node, lang=lang),
            "bdb": bdb.for_lexeme(entry.get("language"), entry.get("lex"))
            if bdb
            else None,
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
        fmt = formats.chosen(config, fmt)
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
        fmt = formats.chosen(service.config(corpus), fmt)
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

    return router
