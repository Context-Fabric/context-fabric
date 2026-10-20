"""Searches proposed by a language model, kept until the user runs or
dismisses them.

A proposal has the user's request in their own words, the template, the
model's explanation, which model made it and when, and the trial's count.
The open page follows the proposals live (`/api/proposals/events`); nothing
runs until the user presses *Run*.
"""

from __future__ import annotations

import datetime
import threading
import uuid
from dataclasses import asdict, dataclass, field
from typing import Any

NEW = "new"
RUN = "run"
DISMISSED = "dismissed"
STATES = (NEW, RUN, DISMISSED)
KEEP = 50  # proposals kept, the newest


@dataclass
class Proposal:
    id: str
    corpus: str
    template: str
    request: str
    explanation: str
    model: str | None
    created: str
    # the trial before proposing: its count, and whether that is a lower bound
    count: int | None = None
    atLeast: bool = False
    state: str = NEW


@dataclass
class ProposalStore:
    proposals: list[Proposal] = field(default_factory=list)
    version: int = 0
    _cond: threading.Condition = field(default_factory=threading.Condition, repr=False)

    def add(
        self,
        corpus: str,
        template: str,
        request: str,
        explanation: str,
        model: str | None = None,
        count: int | None = None,
        atLeast: bool = False,
    ) -> dict[str, Any]:
        proposal = Proposal(
            id=uuid.uuid4().hex[:12],
            corpus=corpus,
            template=template,
            request=request,
            explanation=explanation,
            model=model,
            created=datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
            count=count,
            atLeast=atLeast,
        )
        with self._cond:
            self.proposals.append(proposal)
            del self.proposals[:-KEEP]
            self.version += 1
            self._cond.notify_all()
        return asdict(proposal)

    def set_state(self, pid: str, state: str) -> dict[str, Any] | None:
        if state not in STATES:
            raise ValueError(f"Unknown state {state!r}")
        with self._cond:
            for p in self.proposals:
                if p.id == pid:
                    p.state = state
                    self.version += 1
                    self._cond.notify_all()
                    return asdict(p)
        return None

    def snapshot(self) -> dict[str, Any]:
        """All proposals, the newest first, and the version they are at."""
        with self._cond:
            return {
                "version": self.version,
                "proposals": [asdict(p) for p in reversed(self.proposals)],
            }

    def wait_change(self, version: int, timeout: float) -> int:
        """Block until the proposals change after `version`, or the timeout."""
        with self._cond:
            if self.version == version:
                self._cond.wait(timeout)
            return self.version
