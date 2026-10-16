"""The tools a language model gets, on the browser's open corpora.

Describing corpora and features and the template language are `cfabric-mcp`'s
tools, run on the browser's registry (`cfabric_mcp.tools.using`), so the two
don't drift apart. Checking a template is the browser's own check, and trial
searches go through the browser's search pool, with its result cap, timeout
and cancelling, as any typed search does.
"""

from __future__ import annotations

import re
import time
from importlib import resources
from typing import Any

from cfabric.display import section_ref_for
from cfabric_mcp import tools as shared

from cfabric_browser import formats
from cfabric_browser.assistant.proposals import ProposalStore
from cfabric_browser.corpora import CorpusService
from cfabric_browser.queries import check_template
from cfabric_browser.workers import DONE, FINISHED, SearchPool

# a trial: the count up to this many results, within this many seconds, and
# a few examples
TRIAL_LIMIT = 10_000
TRIAL_TIMEOUT = 20.0
EXAMPLES = 5
# the text shown of a node in an example
EXAMPLE_TEXT = 120


# `feature=value` in a template (not `feature~regex`, `feature<n` and the like)
EQUALS = re.compile(r"(?<![\w.])([A-Za-z_][\w@-]*)=([^\s|]+)")


class ToolError(Exception):
    """A tool was asked something it can't do (an unknown corpus)."""


class Tools:
    def __init__(
        self,
        service: CorpusService,
        pool: SearchPool,
        proposals: ProposalStore | None = None,
    ) -> None:
        self.service = service
        self.pool = pool
        self.proposals = proposals if proposals is not None else ProposalStore()
        # per (corpus, feature) the values it has
        self._values: dict[tuple[str, str], set[str]] = {}

    def _corpus(self, corpus: str) -> str:
        if not self.service.has(corpus):
            open_ = ", ".join(self.service.names()) or "none"
            raise ToolError(f"No corpus {corpus!r} is open; open: {open_}")
        return corpus

    # describing

    def list_corpora(self) -> dict[str, Any]:
        """The open corpora: name, title, id and whether it is a translation."""
        found = []
        for name in self.service.names():
            card = self.service.card(name)
            found.append(
                {
                    "name": name,
                    "title": card["title"],
                    "id": card["id"],
                    "translation": bool(card["translation"]),
                    "sectionTypes": card["sectionTypes"],
                }
            )
        return {"corpora": found}

    def describe_corpus(self, corpus: str) -> dict[str, Any]:
        with shared.using(self.service.registry):
            return shared.describe_corpus(self._corpus(corpus))

    def list_features(
        self, corpus: str, node_types: list[str] | None = None
    ) -> dict[str, Any]:
        with shared.using(self.service.registry):
            return shared.list_features(
                node_types=node_types, corpus=self._corpus(corpus)
            )

    def describe_feature(self, corpus: str, features: list[str]) -> dict[str, Any]:
        with shared.using(self.service.registry):
            return shared.describe_features(features, corpus=self._corpus(corpus))

    def search_syntax_guide(self, section: str | None = None) -> dict[str, Any]:
        return shared.search_syntax_guide(section)

    def corpus_guide(self, corpus: str) -> dict[str, Any]:
        """The hand-written guide to searching this corpus, if there is one
        (`guides/<repo>.md`, by the corpus' id `org/repo`)."""
        card = self.service.card(self._corpus(corpus))
        repo = (card["id"] or "").split("/")[-1].lower()
        guide = resources.files("cfabric_browser.assistant").joinpath(
            "guides", f"{repo}.md"
        )
        if repo and guide.is_file():
            return {"corpus": corpus, "guide": guide.read_text(encoding="utf-8")}
        return {
            "corpus": corpus,
            "guide": None,
            "note": "No guide for this corpus yet: use describe_corpus, "
            "list_features and describe_feature.",
        }

    # checking and trying

    def check_template(self, corpus: str, template: str) -> dict[str, Any]:
        """The template's problems (syntax, unknown types, features and values)
        without searching; `ok` when there are none."""
        api = self.service.api(self._corpus(corpus))
        problems = check_template(api, template)
        return {"ok": not problems, "problems": problems}

    def try_search(
        self, corpus: str, template: str, examples: int = EXAMPLES
    ) -> dict[str, Any]:
        """Run a template as a trial: how many results (up to `TRIAL_LIMIT`,
        within `TRIAL_TIMEOUT` seconds) and the first few, with their passage
        and text, for the model to judge its template by."""
        corpus = self._corpus(corpus)
        checked = self.check_template(corpus, template)
        if not checked["ok"]:
            return {"ok": False, "problems": checked["problems"]}
        job = self.pool.submit(
            corpus, template, limit=TRIAL_LIMIT, timeout=TRIAL_TIMEOUT
        )
        version = -1
        deadline = time.monotonic() + TRIAL_TIMEOUT + 10
        while job.state not in FINISHED and time.monotonic() < deadline:
            version = job.wait_change(version, 1.0)
        if job.state not in FINISHED:
            self.pool.cancel(job.id)
        found = job.snapshot()
        result: dict[str, Any] = {
            "ok": found["state"] == DONE and not found["problems"],
            "state": found["state"],
            "count": found["count"],
            # more results than the trial counts: `count` is a lower bound
            "atLeast": bool(found["truncated"]) or found["state"] != DONE,
            "seconds": found["elapsed"],
            "problems": found["problems"],
            "examples": [
                self._example(corpus, result)
                for result in job.page(0, max(0, min(examples, 20)))
            ],
        }
        if found["state"] == DONE and not found["count"]:
            result["hints"] = self.unknown_values(corpus, template)
        return result

    def unknown_values(self, corpus: str, template: str) -> list[str]:
        """For an empty result: the `feature=value` conditions whose value the
        feature never has (a misspelled lexeme such as `>LHM/`)."""
        api = self.service.api(corpus)
        hints = []
        for feature, value in EQUALS.findall(template):
            if feature not in api.Fall():
                continue
            key = (corpus, feature)
            if key not in self._values:
                self._values[key] = {str(v) for (v, _) in api.Fs(feature).freqList()}
            if value not in self._values[key]:
                hints.append(f"{feature} never has the value {value}")
        return hints

    def _example(self, corpus: str, nodes: tuple[int, ...]) -> dict[str, Any]:
        api = self.service.api(corpus)
        config = self.service.config(corpus)
        fmt = formats.default(config)
        F, T = api.F, api.T
        return {
            "ref": section_ref_for(api, config, nodes[0]),
            "nodes": [
                {
                    "node": int(n),
                    "type": F.otype.v(n),
                    "text": T.text(n, fmt=fmt).strip()[:EXAMPLE_TEXT],
                }
                for n in nodes
            ],
        }

    # proposing: the only tool that changes what the user sees

    def propose_search(
        self,
        corpus: str,
        template: str,
        request: str,
        explanation: str,
        model: str | None = None,
    ) -> dict[str, Any]:
        """Show a search in the browser, for the user to run, edit or dismiss:
        the template, the user's request in their words, and an explanation
        of the template. A template with problems is not shown; they come
        back to be fixed. Nothing runs until the user presses *Run*."""
        corpus = self._corpus(corpus)
        trial = self.try_search(corpus, template, examples=0)
        if trial.get("problems") or trial.get("state") not in (DONE, None):
            return {
                "ok": False,
                "problems": trial.get("problems"),
                "state": trial.get("state"),
            }
        proposal = self.proposals.add(
            corpus,
            template,
            request.strip(),
            explanation.strip(),
            model,
            count=trial["count"],
            atLeast=trial["atLeast"],
        )
        return {
            "ok": True,
            "proposal": proposal["id"],
            "count": trial["count"],
            "atLeast": trial["atLeast"],
            "note": "Shown in the browser; the user runs it with Run.",
        }
