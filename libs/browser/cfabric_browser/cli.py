"""The `cfabric-browser` command."""

from __future__ import annotations

import argparse
import logging
import sys
import threading
import time
import webbrowser

from cfabric_browser import __version__

DEFAULT_PORT = 8107


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="cfabric-browser",
        description="Browse, search and compare Context-Fabric corpora, offline.",
        epilog=(
            "Corpora can be given as a data directory, a repository in "
            "Text-Fabric layout, or a name such as 'bhsa' or 'ETCBC/dss' that "
            "is looked up in ~/text-fabric-data/github and ~/github. "
            "Prefix with 'name=' to choose the name shown in the browser. "
            "Without corpora, the browser starts empty and offers the corpora on "
            "this computer to open."
        ),
    )
    parser.add_argument("corpora", nargs="*", metavar="CORPUS")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument(
        "--port",
        type=int,
        default=None,
        help=f"default {DEFAULT_PORT}; with --window the first free one from there",
    )
    parser.add_argument("--workers", type=int, default=2, help="parallel searches")
    parser.add_argument(
        "--timeout", type=float, default=60.0, help="time limit per search (seconds)"
    )
    parser.add_argument(
        "--max-results", type=int, default=500_000, help="results kept per search"
    )
    parser.add_argument(
        "--no-open", action="store_true", help="do not open a web browser"
    )
    parser.add_argument(
        "--window",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="show the browser in a window of its own (needs pywebview);"
        " closing it stops the server",
    )
    parser.add_argument(
        "--hide",
        action="append",
        default=[],
        metavar="ORG/REPO",
        help="leave this corpus out of the library (can be repeated); see also"
        " library: hide: in ~/.cfabric/browser.yaml",
    )
    parser.add_argument(
        "--no-library",
        action="store_true",
        help="do not let users open other corpora found on this computer",
    )
    parser.add_argument("--version", action="version", version=__version__)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if not args.corpora and args.no_library:
        # nothing to show and no way to open anything
        print(
            "cfabric-browser: give at least one corpus, or leave out --no-library",
            file=sys.stderr,
        )
        return 2
    logging.basicConfig(level=logging.WARNING, format="%(message)s")

    import uvicorn

    from cfabric_browser.app import create_app
    from cfabric_browser.corpora import CorpusService, load_settings, resolve_corpus
    from cfabric_browser.workers import SearchPool

    try:
        specs = [resolve_corpus(c) for c in args.corpora]
    except FileNotFoundError as e:
        print(f"cfabric-browser: {e}", file=sys.stderr)
        return 2
    names = [s.name for s in specs]
    if len(set(names)) != len(names):
        print(
            "cfabric-browser: two corpora have the same name; use name=... to"
            " tell them apart",
            file=sys.stderr,
        )
        return 2

    settings = load_settings()
    hide = [*((settings.get("library") or {}).get("hide") or ()), *args.hide]
    service = CorpusService(specs, hide=hide)
    for spec in specs:
        print(f"Loading {spec.name} from {spec.path} ...", flush=True)
        t = time.perf_counter()
        service.config(spec.name)
        print(f"  ready in {time.perf_counter() - t:.1f}s", flush=True)
    if not specs:
        print("No corpora given: open them in the browser.", flush=True)

    pool = SearchPool(
        [(s.name, str(s.path)) for s in specs],
        sort_key=lambda corpus: service.api(corpus).N.sortKeyTuple,
        workers=args.workers,
        max_results=args.max_results,
        timeout=args.timeout,
    )
    app = create_app(service, pool, library=not args.no_library)

    host = args.host
    port = args.port
    if port is None:
        if args.window:
            # a second window can run next to the first
            from cfabric_browser.window import free_port

            port = free_port(host, DEFAULT_PORT)
        else:
            port = DEFAULT_PORT
    url = f"http://{'localhost' if host in ('127.0.0.1', '0.0.0.0') else host}:{port}/"
    if host not in ("127.0.0.1", "localhost", "::1"):
        print(
            "Warning: the browser is reachable from other machines on this network.",
            file=sys.stderr,
        )
    if args.window:
        from cfabric_browser.window import run_window

        print(f"Context-Fabric browser at {url}, in its own window", flush=True)
        return run_window(app, host, port, url)

    print(f"Context-Fabric browser at {url}  (Ctrl+C to stop)", flush=True)
    if not args.no_open:
        threading.Timer(1.0, webbrowser.open, args=(url,)).start()

    uvicorn.run(app, host=host, port=port, log_level="warning")
    return 0


if __name__ == "__main__":
    sys.exit(main())
