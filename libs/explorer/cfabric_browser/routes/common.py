"""What the routes of all areas share: request models, display options,
the shared state (`Context`) and helpers."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Literal

from fastapi import HTTPException, Query
from fastapi.responses import Response
from pydantic import BaseModel, Field

from cfabric_browser.assistant.tools import Tools
from cfabric_browser.corpora import CorpusService
from cfabric_browser.dictionaries.manage import DictionaryJobs
from cfabric_browser.dictionaries.store import Dictionaries
from cfabric_browser.display import (
    RenderOptions,
    render_node,
    section_ref_for,
)
from cfabric_browser.downloads import DownloadJobs
from cfabric_browser.queries import LANGUAGES
from cfabric_browser.translations.manage import Jobs
from cfabric_browser.witnesses import dress_signs, place
from cfabric_browser.workers import SearchJob, SearchPool

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


class ProposalRequest(BaseModel):
    """A search proposed by a language model."""

    corpus: str
    template: str
    request: str = Field(description="The user's request in their own words")
    explanation: str = ""
    model: str | None = None


class ProposalState(BaseModel):
    state: Literal["new", "run", "dismissed"]


class SearchRequest(BaseModel):
    template: str
    language: str = Field(default="cf-template", description=f"One of {LANGUAGES}")
    limit: int | None = Field(default=None, ge=1)
    timeout: float | None = Field(default=None, gt=0)


def split_items(value: str | None) -> list[str] | None:
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
    """Display options as query parameters; see `cfabric_browser.display.RenderOptions`."""
    return RenderOptions(
        fmt=fmt or None,
        full=full,
        hide_types=hide_types,
        hidden_types=split_items(hidden_types),
        base_types=split_items(base_types),
        condense_type=condense_type or None,
        extra_features=split_items(extra_features) or (),
        lang=lang,
    )


@dataclass
class Context:
    """What the routes of all areas share."""

    service: CorpusService
    pool: SearchPool
    # whether people may open other corpora found on this computer
    library: bool
    # searching in plain language: the model's tools, and its proposals
    tools: Tools
    # dictionaries (BDB): downloaded into ~/.cfabric/dictionaries, read when
    # first needed
    lookups: DictionaryJobs
    store: Dictionaries
    # translations are built, and corpora downloaded, into the first place
    # the library looks
    jobs: Jobs
    fetches: DownloadJobs

    def corpus_api(self, corpus: str):
        if not self.service.has(corpus):
            raise HTTPException(404, f"Unknown corpus: {corpus}")
        return self.service.api(corpus)

    @staticmethod
    def check_node(api, node: int) -> int:
        if not 1 <= node <= int(api.F.otype.maxNode):
            raise HTTPException(404, f"No node {node}")
        return node

    def search_job(self, search_id: str) -> SearchJob:
        job = self.pool.get(search_id)
        if job is None:
            raise HTTPException(404, f"Unknown or expired search: {search_id}")
        return job

    def allowed(self, corpusId: str) -> bool:
        """Whether a corpus may be offered (`only` limits the library)."""
        only = {o.lower() for o in self.service.only}
        return not only or corpusId.lower() in only


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


def edge_item(x: Any) -> dict[str, Any]:
    if isinstance(x, tuple):
        return {"node": int(x[0]), "value": x[1]}
    return {"node": int(x)}
