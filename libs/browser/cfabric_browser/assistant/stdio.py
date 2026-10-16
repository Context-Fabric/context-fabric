"""`cfabric-browser mcp`: the browser's MCP server over stdio, for AI apps
that start their own servers (Claude Desktop).

It is a small proxy: each JSON-RPC message from the app (one per line on
stdin) goes to the running browser's `/mcp` as an HTTP request, and the answer
comes back as a line on stdout. The browser's `/mcp` is stateless and answers
with JSON, so nothing else is needed. The browser is found in
`~/.cfabric/running.json`; when none answers, one is started.

Nothing but those messages may be written to stdout.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, BinaryIO

from cfabric_browser.assistant import running

START_TIMEOUT = 120.0  # seconds to wait for a browser that was started


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


def forward(url: str, message: bytes) -> bytes | None:
    """One message to the browser's /mcp; its answer, None for none (a
    notification)."""
    request = urllib.request.Request(
        url + "mcp",
        data=message,
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream",
        },
    )
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
    url: str | None = None
    for line in source:
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except ValueError:
            continue
        answer: bytes | None
        for attempt in (1, 2):
            if url is None:
                url = find_browser(path) or (start_browser(path) if start else None)
            if url is None:
                answer = error(
                    message,
                    "The Context-Fabric browser is not running and could not be "
                    "started. Start it, then try again.",
                )
                break
            try:
                answer = forward(url, line)
                break
            except urllib.error.HTTPError as e:
                answer = error(message, f"The browser answered {e.code}")
                break
            except OSError:
                # it stopped, or restarted on another port: look again
                url = None
                answer = error(message, "Lost the connection to the browser")
        if answer is not None:
            sink.write(answer.rstrip() + b"\n")
            sink.flush()
    return 0
