"""`cfabric-browser mcp`: the browser's MCP server over stdio, for AI apps
that start their own servers (Claude Desktop, the ChatGPT desktop app).

It is a small proxy: each JSON-RPC message from the app (one per line on
stdin) goes to the running browser's `/mcp` as an HTTP request, and the answer
comes back as a line on stdout. The browser's `/mcp` is stateless and answers
with JSON, so nothing else is needed. The browser is found in
`~/.cfabric/running.json`; when none answers, one is started.

What doesn't need the browser is answered here: `initialize`, `ping`, the
lists of tools, prompts and resources (from the same server definition,
`mcp.build_server`), and notifications. Apps give a server only seconds to
start (Codex: 10), and starting the browser takes longer, so it is looked
for, or started, in the background from the first message, and only a tool
call waits for it. The app's name from `initialize` goes with each tool call
(`mcp.CLIENT_HEADER`), for proposals that don't name their model.

Nothing but those messages may be written to stdout.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from collections.abc import Awaitable, Callable, Sequence
from pathlib import Path
from typing import Any, BinaryIO

import anyio
from mcp.server.fastmcp import FastMCP
from mcp.shared.version import SUPPORTED_PROTOCOL_VERSIONS
from mcp.types import LATEST_PROTOCOL_VERSION

from cfabric_browser.assistant import mcp, running

START_TIMEOUT = 120.0  # seconds to wait for a browser that was started
RETRY_START = 300.0  # seconds before trying again to start one that failed


def find_browser(path: str | Path = running.RUNNING) -> str | None:
    """The address of the newest running browser that answers."""
    for entry in reversed(running.read(path)):
        try:
            with urllib.request.urlopen(entry["url"] + "api/health", timeout=3):
                return str(entry["url"])
        except (OSError, ValueError):
            continue
    return None


def start_browser(path: str | Path = running.RUNNING) -> str | None:
    """Start the browser in the background (the program itself when frozen),
    and wait until it announces itself."""
    if getattr(sys, "frozen", False):
        command = [sys.executable]
    else:
        command = [sys.executable, "-m", "cfabric_browser.cli"]
    options: dict[str, Any] = {
        "stdin": subprocess.DEVNULL,
        "stdout": subprocess.DEVNULL,
        "stderr": subprocess.DEVNULL,
    }
    if os.name == "nt":
        options["creationflags"] = (
            subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP
        )
    else:
        options["start_new_session"] = True
    subprocess.Popen(command, **options)
    deadline = time.monotonic() + START_TIMEOUT
    while time.monotonic() < deadline:
        found = find_browser(path)
        if found:
            return found
        time.sleep(1)
    return None


class Browser:
    """The running browser's address, looked for (and the browser started if
    need be) in a thread of its own, from the first time it is asked for."""

    def __init__(self, path: str | Path, start: bool) -> None:
        self.path = path
        self.start = start
        self._url: str | None = None
        self._looking: threading.Thread | None = None
        # when starting it last failed: not tried again for a while, so that
        # every message doesn't start (and wait for) another one
        self._failedStart: float | None = None

    def look(self) -> None:
        """Start looking, unless it is found or being looked for."""
        if self._url is None and self._looking is None:
            self._looking = threading.Thread(target=self._find, daemon=True)
            self._looking.start()

    def _find(self) -> None:
        self._url = find_browser(self.path)
        if self._url is not None or not self.start:
            return
        failed = self._failedStart
        if failed is not None and time.monotonic() - failed < RETRY_START:
            return
        self._url = start_browser(self.path)
        self._failedStart = None if self._url else time.monotonic()

    def url(self) -> str | None:
        """The address, after waiting for the looking; None if there is no
        browser."""
        self.look()
        if self._looking is not None:
            self._looking.join()
            self._looking = None
        return self._url

    def lost(self) -> None:
        """It stopped, or restarted on another port: look again next time."""
        self._url = None


def _not_here(name: str, args: dict[str, Any]) -> Any:
    raise RuntimeError("tools run in the browser")


class Local:
    """The answers that don't need the browser, from the server definition
    the browser itself serves."""

    def __init__(self) -> None:
        self.server: FastMCP = mcp.build_server(_not_here)

    def answer(self, message: dict[str, Any]) -> dict[str, Any] | None:
        """The result for a message answered here, None for one that goes to
        the browser."""
        method = message.get("method")
        if method == "initialize":
            return self.initialize(message.get("params") or {})
        if method == "ping":
            return {}
        lists: dict[str, tuple[str, Callable[[], Awaitable[Sequence[Any]]]]] = {
            "tools/list": ("tools", self.server.list_tools),
            "prompts/list": ("prompts", self.server.list_prompts),
            "resources/list": ("resources", self.server.list_resources),
            "resources/templates/list": (
                "resourceTemplates",
                self.server.list_resource_templates,
            ),
        }
        if method in lists:
            key, get = lists[method]
            items = anyio.run(get)
            return {key: [dump(item) for item in items]}
        return None

    def initialize(self, params: dict[str, Any]) -> dict[str, Any]:
        """As the SDK answers it: the version the app asks for if we know it,
        else our latest."""
        options = self.server._mcp_server.create_initialization_options()
        asked = params.get("protocolVersion")
        result = {
            "protocolVersion": asked
            if asked in SUPPORTED_PROTOCOL_VERSIONS
            else LATEST_PROTOCOL_VERSION,
            "capabilities": dump(options.capabilities),
            "serverInfo": {
                "name": options.server_name,
                "version": options.server_version,
            },
        }
        if options.instructions:
            result["instructions"] = options.instructions
        return result


def dump(item: Any) -> Any:
    """A pydantic object of the SDK as JSON, as it sends them."""
    return item.model_dump(by_alias=True, mode="json", exclude_none=True)


def forward(url: str, message: bytes, client: str | None = None) -> bytes | None:
    """One message to the browser's /mcp; its answer, None for none (a
    notification). `client`: the app's name, from its `initialize`."""
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
    }
    if client:
        headers[mcp.CLIENT_HEADER] = client
    request = urllib.request.Request(url + "mcp", data=message, headers=headers)
    with urllib.request.urlopen(request, timeout=180) as response:
        body = response.read()
    return body if body.strip() else None


def error(message: dict[str, Any], text: str) -> bytes | None:
    """A JSON-RPC error for a request (none for a notification)."""
    if "id" not in message:
        return None
    answer = {
        "jsonrpc": "2.0",
        "id": message["id"],
        "error": {"code": -32603, "message": text},
    }
    return json.dumps(answer).encode()


def serve(
    stdin: BinaryIO | None = None,
    stdout: BinaryIO | None = None,
    path: str | Path = running.RUNNING,
    start: bool = True,
) -> int:
    source = stdin or sys.stdin.buffer
    sink = stdout or sys.stdout.buffer
    browser = Browser(path, start)
    local = Local()
    client: str | None = None
    for line in source:
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if not isinstance(message, dict):
            continue
        browser.look()
        if message.get("method") == "initialize":
            info = (message.get("params") or {}).get("clientInfo") or {}
            client = str(info.get("name") or "") or None
        if "id" not in message:
            # a notification: the stateless /mcp does nothing with one
            continue
        answer: bytes | None
        result = local.answer(message)
        if result is not None:
            answer = json.dumps(
                {"jsonrpc": "2.0", "id": message["id"], "result": result}
            ).encode()
        else:
            answer = call_browser(browser, message, line, client)
        if answer is not None:
            sink.write(answer.rstrip() + b"\n")
            sink.flush()
    return 0


def call_browser(
    browser: Browser, message: dict[str, Any], line: bytes, client: str | None
) -> bytes | None:
    """A message for the browser, with a second try when it was lost."""
    answer: bytes | None = None
    for _ in range(2):
        url = browser.url()
        if url is None:
            return error(
                message,
                "CACCHT Explorer is not running and could not be "
                "started. Start it, then try again.",
            )
        try:
            return forward(url, line, client)
        except urllib.error.HTTPError as e:
            return error(message, f"The browser answered {e.code}")
        except OSError:
            # it stopped, or restarted on another port: look again
            browser.lost()
            answer = error(message, "Lost the connection to the browser")
    return answer
