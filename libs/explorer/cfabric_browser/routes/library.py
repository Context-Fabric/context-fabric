"""The library: corpora on this computer to open, translations to build,
dictionaries and corpora to download."""

from __future__ import annotations

import re
import threading
from typing import Any

from fastapi import APIRouter, HTTPException

from cfabric_browser.corpora import discover, resolve_corpus
from cfabric_browser.dictionaries.manage import (
    DICTIONARIES,
    Catalogued,
    is_added,
)
from cfabric_browser.dictionaries.manage import remove as remove_dictionary
from cfabric_browser.downloads import DOWNLOADS, Download, is_present
from cfabric_browser.routes.common import (
    Context,
    OpenRequest,
)
from cfabric_browser.translations import TRANSLATIONS, Translation
from cfabric_browser.translations.manage import (
    cancel_removal,
    is_built,
    is_pending,
    remove,
    repo_of,
    schedule_removal,
)


def build(ctx: Context) -> APIRouter:
    router = APIRouter()
    service = ctx.service
    pool = ctx.pool
    library = ctx.library
    lookups = ctx.lookups
    store = ctx.store
    jobs = ctx.jobs
    fetches = ctx.fetches
    allowed = ctx.allowed

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

    def open_from_library(requestedId: str, name: str | None = None) -> dict[str, Any]:
        """Open a corpus of the library by its id (`ETCBC/dss`, optionally with
        `@version`); `opened` is False when it was open already."""
        (requested, _, version) = requestedId.partition("@")
        # any corpus on disk can be opened by its id, also copies and hidden ones,
        # but not one outside a library limited with `only`
        ids = {
            entry["id"].lower(): entry["id"]
            for entry in discover(service.roots, collapse=False, only=service.only)
        }
        corpusId = ids.get(requested.lower())
        if corpusId is None:
            raise HTTPException(404, f"Not in the library: {requestedId}")
        target = f"{corpusId}@{version}" if version else corpusId
        try:
            spec = resolve_corpus(f"{name}={target}" if name else target, service.roots)
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
        return {"name": spec.name, "opened": True}

    @router.post("/corpora", status_code=201)
    def open_corpus(request: OpenRequest) -> dict[str, Any]:
        """Open a corpus from the library. Only library ids are accepted, no paths."""
        if not library:
            raise HTTPException(403, "Opening corpora is switched off on this server")
        result = open_from_library(request.id, request.name)
        if result["opened"]:
            # start loading now, so the corpus is ready by the time it is shown
            threading.Thread(
                target=service.config, args=(result["name"],), daemon=True
            ).start()
        return result

    # translations: built from their source into a corpus, then in the library

    def translation_of(tid: str) -> Translation:
        translation = TRANSLATIONS.get(tid)
        if translation is None or not offered(translation):
            raise HTTPException(404, f"Unknown translation: {tid}")
        return translation

    def offered(translation: Translation) -> bool:
        return allowed(translation.corpus_id)

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

    # dictionaries: one file each, built once and published, from the library

    def dictionary_of(did: str) -> Catalogued:
        catalogued = DICTIONARIES.get(did)
        if catalogued is None or not allowed(catalogued.corpus):
            raise HTTPException(404, f"Unknown dictionary: {did}")
        return catalogued

    def dictionary_entry(catalogued: Catalogued) -> dict[str, Any]:
        found = store.get(catalogued.id)
        return {
            "id": catalogued.id,
            "title": catalogued.title,
            "name": catalogued.name,
            "licence": catalogued.licence,
            "licenceNote": catalogued.licenceNote,
            "sources": catalogued.sources,
            "corpus": catalogued.corpus,
            "size": catalogued.size,
            # can be downloaded (not before it is published)
            "available": catalogued.url is not None,
            "added": found is not None,
            "built": found.meta.get("built") if found else None,
            "job": lookups.status(catalogued.id),
        }

    @router.get("/dictionaries")
    def list_dictionaries() -> dict[str, Any]:
        return {
            "enabled": library,
            "dictionaries": [
                dictionary_entry(d) for d in DICTIONARIES.values() if allowed(d.corpus)
            ],
        }

    @router.get("/dictionaries/{did}")
    def describe_dictionary(did: str) -> dict[str, Any]:
        """A dictionary, and when it has been added its metadata (title,
        edition, sources, figures of the build) and abbreviations."""
        catalogued = dictionary_of(did)
        result = dictionary_entry(catalogued)
        found = store.get(did)
        if found is not None:
            result["meta"] = found.meta
            result["abbreviations"] = found.abbreviations
        return result

    @router.post("/dictionaries/{did}", status_code=202)
    def add_dictionary(did: str) -> dict[str, Any]:
        """Download a dictionary; `job` shows the progress."""
        if not library:
            raise HTTPException(403, "Adding is switched off on this server")
        catalogued = dictionary_of(did)
        if catalogued.url is None:
            raise HTTPException(409, f"{catalogued.name} has not been published yet")
        if not is_added(catalogued, lookups.root):
            lookups.start(catalogued)
        return dictionary_entry(catalogued)

    @router.delete("/dictionaries/{did}")
    def remove_dictionary_route(did: str) -> dict[str, Any]:
        if not library:
            raise HTTPException(403, "Removing is switched off on this server")
        catalogued = dictionary_of(did)
        if not remove_dictionary(catalogued, lookups.root):
            raise HTTPException(404, f"{catalogued.name} has not been added")
        return dictionary_entry(catalogued)

    def linked_lexemes(keys: list[str], corpus: str | None) -> list[dict[str, Any]]:
        """Lexeme keys (`Hebrew BR>[`) as lexemes of a corpus: node, form,
        gloss; those the corpus doesn't have are left out."""
        lexicon = service.lexicon(corpus) if corpus and service.has(corpus) else None
        if lexicon is None:
            return []
        found = []
        for key in keys:
            language, _, lex = key.partition(" ")
            e = lexicon.find(lex, language)
            if e is not None:
                found.append(
                    {k: e.get(k) for k in ("node", "lex", "form", "gloss", "language")}
                )
        return found

    def passages(text: str, corpus: str | None) -> dict[str, str]:
        """The verses an article cites (`Gen.1.1`) as references in `corpus`
        (`Genesis 1:1`), for those of its books the corpus has."""
        if not corpus or not service.has(corpus):
            return {}
        books = service.books(corpus)
        found = {}
        for osis in set(re.findall(r'data-osis="([^"]+)"', text)):
            book, _, place = osis.partition(".")
            if book in books and place:
                chapter, _, verse = place.partition(".")
                found[osis] = f"{books[book]} {chapter}" + (
                    f":{verse}" if verse else ""
                )
        return found

    @router.get("/dictionaries/{did}/entries/{eid}")
    def dictionary_article(
        did: str, eid: str, corpus: str | None = None
    ) -> dict[str, Any]:
        """An entry: its text (HTML with `<a data-osis="Gen.1.1">` for verses
        and `<a data-entry="id">` for other entries), its neighbours, its root
        family, and the lexemes of `corpus` linked to it and to its family."""
        dictionary_of(did)
        found = store.get(did)
        if found is None:
            raise HTTPException(404, f"{did} has not been added")
        article = found.article(eid)
        if article is None:
            raise HTTPException(404, f"No entry {eid}")
        article["lexemes"] = linked_lexemes(article["lexemes"], corpus)
        article["passages"] = passages(article["html"], corpus)
        for member in article["family"]:
            member["lexemes"] = linked_lexemes(
                found.lexemes.get(member["id"], []), corpus
            )
        return article

    # corpora that can be downloaded: the BHSA, the SP and the DSS, from their
    # own releases (cfabric_browser.downloads)

    def download_of(org: str, repo: str) -> Download:
        download = DOWNLOADS.get(f"{org}/{repo}")
        if download is None or not allowed(download.id):
            raise HTTPException(
                404, f"Not a corpus that can be downloaded: {org}/{repo}"
            )
        return download

    def download_entry(download: Download) -> dict[str, Any]:
        return {
            "id": download.id,
            "name": download.name,
            "title": download.title,
            "version": download.version,
            "size": download.size,
            "unpacked": download.unpacked,
            "terms": download.terms,
            "licence": download.licence,
            # in any of the places the library looks
            "present": any(is_present(download, root) for root in service.roots),
            "job": fetches.status(download.id),
        }

    @router.get("/downloads")
    def list_downloads() -> dict[str, Any]:
        if not library:
            return {"enabled": False, "downloads": []}
        return {
            "enabled": True,
            "downloads": [
                download_entry(d) for d in DOWNLOADS.values() if allowed(d.id)
            ],
        }

    @router.get("/downloads/{org}/{repo}")
    def describe_download(org: str, repo: str) -> dict[str, Any]:
        return download_entry(download_of(org, repo))

    @router.post("/downloads/{org}/{repo}", status_code=202)
    def start_download(org: str, repo: str) -> dict[str, Any]:
        """Download a corpus in the background; `job` in the answer, and in
        `GET /downloads/{org}/{repo}`, shows the progress."""
        if not library:
            raise HTTPException(403, "Downloading is switched off on this server")
        download = download_of(org, repo)
        if not download_entry(download)["present"]:

            def prepare(progress) -> None:
                # open it and load it once, which compiles it: minutes for the
                # BHSA, after which it opens in about a second
                progress("prepare", 0, 1)
                name = open_from_library(download.id)["name"]
                service.config(name)
                progress("prepare", 1, 1)

            fetches.start(download, then=prepare)
        return download_entry(download)

    @router.delete("/downloads/{org}/{repo}")
    def cancel_download(org: str, repo: str) -> dict[str, Any]:
        """Stop a download; a corpus being prepared is not stopped."""
        download = download_of(org, repo)
        fetches.cancel(download.id)
        return download_entry(download)

    return router
