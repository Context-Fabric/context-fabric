"""Entry point of the standalone program: the browser in a window of its own."""

import multiprocessing
import os
import sys
import traceback
from pathlib import Path

from cfabric_browser.cli import main

LOG = Path.home() / ".cfabric" / "logs" / "cfabric-browser.log"

# The test version offers only these corpora (--only); empty for all of them.
# Context-Fabric/jps1917 is the translation that can be added from the library.
CORPORA = ["ETCBC/bhsa", "DT-UCPH/sp", "ETCBC/dss", "Context-Fabric/jps1917"]


def log_to_file() -> bool:
    """Without a console, send output and errors to a log file."""
    if sys.stdout is not None and sys.stderr is not None:
        return False
    try:
        LOG.parent.mkdir(parents=True, exist_ok=True)
        # the program starts the log afresh; its search workers add to it
        worker = "--multiprocessing-fork" in sys.argv
        # open for the whole run, so no context manager
        log = open(LOG, "a" if worker else "w", encoding="utf-8", buffering=1)  # noqa: SIM115
    except OSError:
        return False
    sys.stdout = sys.stderr = log
    return True


def unblock_libraries() -> None:
    """Take Windows' download mark off the program's own libraries.

    Unpacking a downloaded zip with Explorer marks every file as coming from the
    internet (an alternate data stream `Zone.Identifier`), and .NET Framework
    then refuses to load the assemblies the window needs (pythonnet's
    `Python.Runtime.dll`). Removing the mark is what *Unblock* in a file's
    properties does; only the libraries inside the program's folder are touched.
    """
    if sys.platform != "win32" or not getattr(sys, "frozen", False):
        return
    internal = Path(sys.executable).parent / "_internal"
    for library in internal.rglob("*.dll"):
        try:
            os.remove(f"{library}:Zone.Identifier")
        except OSError:
            pass  # not marked, or not ours to change


def show_error(message: str) -> None:
    """Tell the user that the program could not start (there is no console)."""
    if sys.platform == "win32":
        import ctypes

        ctypes.windll.user32.MessageBoxW(None, message, "Context-Fabric", 0x10)


if __name__ == "__main__":
    logged = log_to_file()
    # In a frozen program, each search worker starts the program again; this
    # turns that start into the worker instead of a second window.
    multiprocessing.freeze_support()
    unblock_libraries()
    try:
        only = [arg for corpus in CORPORA for arg in ("--only", corpus)]
        # the test version starts in the parallel interface; the switch in its
        # top bar goes to the panes, and the window remembers the choice
        # and offers to download the corpora that are missing
        status = main(
            ["--window", "--ui", "parallel", "--ask-downloads", *only, *sys.argv[1:]]
        )
    except SystemExit as e:  # argparse
        status = e.code if isinstance(e.code, int) else 1
    except Exception:  # noqa: BLE001 - into the log, then the error message
        traceback.print_exc()
        status = 1
    if status and logged:
        show_error(f"Context-Fabric could not start.\n\nSee {LOG} for details.")
    sys.exit(status)
