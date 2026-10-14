"""The browser in a window of its own (pywebview), instead of a web browser."""

from __future__ import annotations

import socket
import sys
import threading
import time
from pathlib import Path
from typing import Any

# Web storage of the window (settings, search history, saved workspaces). It is
# tied to the address, so the window uses a fixed port where it can.
STORAGE = Path.home() / ".cfabric" / "window"


def free_port(host: str, start: int, tries: int = 20) -> int:
    """The first port from `start` on which nothing listens yet."""
    for port in range(start, start + tries):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            try:
                s.bind((host, port))
            except OSError:
                continue
            return port
    raise OSError(f"No free port in {start}-{start + tries - 1}")


def run_window(app: Any, host: str, port: int, url: str) -> int:
    """Serve `app` in a background thread and show it in a window until closed."""
    try:
        import webview
    except ImportError:
        print(
            "cfabric-browser: --window needs pywebview:"
            " pip install cfabric-browser[desktop]",
            file=sys.stderr,
        )
        return 2
    import uvicorn

    config = uvicorn.Config(
        app,
        host=host,
        port=port,
        log_level="warning",
        # without a console (the standalone program) there is nowhere to log to
        log_config=None if sys.stdout is None else uvicorn.config.LOGGING_CONFIG,
    )
    server = uvicorn.Server(config)
    thread = threading.Thread(target=server.run, name="cfabric-server", daemon=True)
    thread.start()
    while not server.started:
        if not thread.is_alive():
            return 1
        time.sleep(0.05)

    webview.settings["ALLOW_DOWNLOADS"] = True  # TSV export, saved workspaces
    webview.create_window(
        "Context-Fabric", url, width=1400, height=900, min_size=(640, 480)
    )
    STORAGE.mkdir(parents=True, exist_ok=True)
    webview.start(private_mode=False, storage_path=str(STORAGE))

    # the window is closed: stop the server, which stops the search workers
    server.should_exit = True
    thread.join(timeout=15)
    return 0
