"""Keeping other web sites out of the browser's server.

The server listens on this computer, but any web page open in the user's web
browser can send it requests. Two checks stop that:

*   **Origin:** a request a web page sends to another site carries its own
    origin; requests from the browser's own page carry the server's (or none,
    for plain page loads). A foreign origin is refused, so another site can't
    start a download or a build, or read the answers.
*   **Host:** a site can make its own name point at 127.0.0.1 (DNS rebinding),
    after which its requests look like the browser's own. When the server
    listens only on this computer, it answers only to the names of this
    computer (`127.0.0.1`, `localhost`, `[::1]`, on any port).
"""

from __future__ import annotations

import json
from collections.abc import Iterable
from typing import Any
from urllib.parse import urlsplit

LOCAL_HOSTS = ("127.0.0.1", "localhost", "[::1]")

Scope = dict[str, Any]


def _host_name(host: str) -> str:
    """`localhost:8107` -> `localhost`; `[::1]:8107` -> `[::1]`."""
    if host.startswith("["):
        return host.split("]", 1)[0] + "]"
    return host.rsplit(":", 1)[0] if ":" in host else host


class Guard:
    """ASGI middleware that refuses requests from other sites.

    Parameters
    ----------
    app:
        The application to protect.
    hosts: Iterable[str] | None
        The host names (without port) the server answers to; `None` for any
        (the server is reachable from other machines under names we can't
        know).
    """

    def __init__(self, app: Any, hosts: Iterable[str] | None = LOCAL_HOSTS) -> None:
        self.app = app
        self.hosts = None if hosts is None else {h.lower() for h in hosts}

    async def __call__(self, scope: Scope, receive: Any, send: Any) -> None:
        if scope["type"] == "http":
            problem = self.problem(scope)
            if problem is not None:
                await _refuse(send, problem)
                return
        await self.app(scope, receive, send)

    def problem(self, scope: Scope) -> str | None:
        headers = {
            k.decode("latin-1").lower(): v.decode("latin-1")
            for (k, v) in scope.get("headers", [])
        }
        host = headers.get("host", "")
        if self.hosts is not None and _host_name(host).lower() not in self.hosts:
            return "Unknown host"
        origin = headers.get("origin")
        if origin is not None and urlsplit(origin).netloc.lower() != host.lower():
            return "Requests from other sites are not allowed"
        return None


async def _refuse(send: Any, message: str) -> None:
    body = json.dumps({"detail": message}).encode()
    await send(
        {
            "type": "http.response.start",
            "status": 403,
            "headers": [
                (b"content-type", b"application/json"),
                (b"content-length", str(len(body)).encode()),
            ],
        }
    )
    await send({"type": "http.response.body", "body": body})
