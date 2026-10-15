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
            "this computer to open. 'cfabric-browser add' lists the translations "
            "that can be added (such as the JPS 1917); 'cfabric-browser add ID' "
            "adds one and 'cfabric-browser remove ID' removes it. 'cfabric-browser "
            "download' lists the corpora that can be downloaded (the BHSA, the SP "
            "and the DSS), and 'cfabric-browser download bhsa sp dss' downloads "
            "them."
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
        "--only",
        action="append",
        default=[],
        metavar="ORG/REPO",
        help="limit the library to this corpus (can be repeated), e.g. for a test"
        " with a few corpora; see also library: only: in ~/.cfabric/browser.yaml",
    )
    parser.add_argument(
        "--no-library",
        action="store_true",
        help="do not let users open other corpora found on this computer",
    )
    parser.add_argument(
        "--ui",
        choices=[
            "panes",
            "classic",
            "parallel",
            "reader",
            "interlinear",
            "concordance",
        ],
        default=None,
        help="the interface to start in until the user chooses one: panes, parallel,"
        " reader, interlinear or concordance; see also ui: in ~/.cfabric/browser.yaml",
    )
    parser.add_argument("--version", action="version", version=__version__)
    return parser.parse_args(argv)


def download_command(names: list[str]) -> int:
    """`cfabric-browser download [NAME ...]`: download corpora without starting
    the browser; without names, list those that can be downloaded."""
    from cfabric_browser.corpora import SEARCH_ROOTS
    from cfabric_browser.downloads import (
        DOWNLOADS,
        DownloadError,
        find,
        get,
        is_present,
    )

    root = SEARCH_ROOTS[0]
    if not names:
        print("Corpora that can be downloaded (cfabric-browser download NAME ...):")
        for d in DOWNLOADS.values():
            here = any(is_present(d, r) for r in SEARCH_ROOTS)
            state = "on this computer" if here else f"{d.size / 1e6:.0f} MB"
            print(f"  {d.name:<6} {d.id} {d.version}: {d.title} ({state})")
        return 0
    wanted = [(name, find(name)) for name in names]
    unknown = [name for (name, d) in wanted if d is None]
    if unknown:
        known = ", ".join(d.name for d in DOWNLOADS.values())
        print(
            f"cfabric-browser: cannot download {', '.join(unknown)}; known: {known}",
            file=sys.stderr,
        )
        return 2

    status = 0
    for _, d in wanted:
        assert d is not None
        if any(is_present(d, r) for r in SEARCH_ROOTS):
            print(f"{d.id} {d.version} is already on this computer.")
            continue
        print(
            f"Downloading {d.id} {d.version} ({d.size / 1e6:.0f} MB); terms of use: {d.terms}"
        )

        def progress(step: str, done: int, total: int) -> None:
            if step == "download":
                print(
                    f"\r  {done / 1e6:.1f} of {total / 1e6:.1f} MB", end="", flush=True
                )
            elif done == total:
                print(f"\n  unpacked {total} files", flush=True)

        try:
            data = get(d, root=root, progress=progress)
        except (DownloadError, OSError) as e:
            print(f"\ncfabric-browser: {e}", file=sys.stderr)
            status = 1
            continue
        print(f"  done: {data}")
    return status


def translation_command(argv: list[str]) -> int:
    """`cfabric-browser add [ID]` and `cfabric-browser remove ID`: add or
    remove a translation without starting the browser."""
    from cfabric_browser.corpora import SEARCH_ROOTS
    from cfabric_browser.translations import TRANSLATIONS, BuildError, build
    from cfabric_browser.translations.manage import is_built, remove

    root = SEARCH_ROOTS[0]
    (command, *rest) = argv
    if not rest:
        if command == "remove":
            print("cfabric-browser: say which translation to remove", file=sys.stderr)
            return 2
        print("Translations that can be added (cfabric-browser add ID):")
        for t in TRANSLATIONS.values():
            state = "added" if is_built(t, root) else "not added"
            print(f"  {t.id:<10} {t.name} ({state})")
            print(f"  {'':<10} {t.licence_note}")
        return 0
    translation = TRANSLATIONS.get(rest[0].lower())
    if translation is None:
        known = ", ".join(TRANSLATIONS)
        print(
            f"cfabric-browser: no translation {rest[0]!r}; known: {known}",
            file=sys.stderr,
        )
        return 2

    if command == "remove":
        if not remove(translation, root):
            print(f"{translation.title} has not been added.")
            return 0
        print(f"Removed {translation.title}.")
        return 0

    print(f"Adding {translation.name}")
    print(f"  {translation.licence_note}")
    shown = {"step": ""}

    def progress(step: str, done: int, total: int) -> None:
        if step == "download":
            print(f"\r  downloading {done}/{total}", end="", flush=True)
        elif step != shown["step"]:
            print("\n  building ...", flush=True)
        shown["step"] = step

    try:
        data = build(translation, root=root, progress=progress)
    except (BuildError, OSError) as e:
        print(f"\ncfabric-browser: {e}", file=sys.stderr)
        return 1
    print(f"  done: {data}")
    print(f"Open it from the library, or: cfabric-browser bhsa {translation.id}")
    return 0


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if argv and argv[0] in ("add", "remove"):
        return translation_command(argv)
    if argv and argv[0] == "download":
        return download_command(argv[1:])
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
    from cfabric_browser.interfaces import read_ui_settings, ui_id
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

    # translations removed while they were open
    from cfabric_browser.corpora import SEARCH_ROOTS
    from cfabric_browser.translations.manage import remove_pending

    for removed in remove_pending(SEARCH_ROOTS[0]):
        print(f"Removed the translation {removed}.", flush=True)

    settings = load_settings()
    librarySettings = settings.get("library") or {}
    hide = [*(librarySettings.get("hide") or ()), *args.hide]
    only = [*(librarySettings.get("only") or ()), *args.only]
    service = CorpusService(specs, hide=hide, only=only)
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
    try:
        (default, hidden) = read_ui_settings(settings.get("ui"))
        ui = ui_id(args.ui) if args.ui else (default or "classic")
    except ValueError as e:
        print(f"cfabric-browser: {e} (ui: in browser.yaml)", file=sys.stderr)
        return 2
    app = create_app(
        service, pool, library=not args.no_library, ui=ui, hidden_uis=hidden
    )

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
