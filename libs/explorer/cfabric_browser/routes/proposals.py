"""Searches proposed by a language model (`cfabric_browser.assistant`)."""

from __future__ import annotations

import json
from typing import Any

from anyio import to_thread
from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import StreamingResponse

from cfabric_browser.assistant.tools import ToolError
from cfabric_browser.routes.common import (
    Context,
    ProposalRequest,
    ProposalState,
)


def build(ctx: Context) -> APIRouter:
    router = APIRouter()
    tools = ctx.tools

    @router.get("/proposals")
    def list_proposals() -> dict[str, Any]:
        """The proposals, the newest first."""
        return tools.proposals.snapshot()

    @router.post("/proposals")
    def add_proposal(body: ProposalRequest) -> dict[str, Any]:
        """Propose a search, as the model's `propose_search` does (other web
        pages are kept out by `cfabric_browser.guard`)."""
        try:
            return tools.propose_search(
                body.corpus, body.template, body.request, body.explanation, body.model
            )
        except ToolError as e:
            raise HTTPException(404, str(e)) from None

    @router.post("/proposals/{pid}")
    def set_proposal(pid: str, body: ProposalState) -> dict[str, Any]:
        """The user ran or dismissed a proposal."""
        found = tools.proposals.set_state(pid, body.state)
        if found is None:
            raise HTTPException(404, f"No proposal {pid}")
        return found

    @router.get("/proposals/events")
    async def proposal_events(request: Request) -> StreamingResponse:
        """Server-sent events with all proposals whenever they change."""
        store = tools.proposals

        async def stream():
            version = -1
            while True:
                if await request.is_disconnected():
                    return
                now = await to_thread.run_sync(store.wait_change, version, 5.0)
                if now != version:
                    version = now
                    yield f"event: proposals\ndata: {json.dumps(store.snapshot())}\n\n"
                else:
                    # keeps the connection open through proxies
                    yield ": waiting\n\n"

        return StreamingResponse(
            stream(),
            media_type="text/event-stream",
            headers={"Cache-Control": "no-cache", "X-Accel-Buffering": "no"},
        )

    return router
