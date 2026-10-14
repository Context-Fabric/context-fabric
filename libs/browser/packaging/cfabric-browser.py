"""Entry point of the standalone program: the browser in a window of its own."""

import multiprocessing
import sys
import traceback
from pathlib import Path

from cfabric_browser.cli import main

LOG = Path.home() / ".cfabric" / "logs" / "cfabric-browser.log"

# The test version offers only these corpora (--only); empty for all of them.
CORPORA = ["ETCBC/bhsa", "DT-UCPH/sp", "ETCBC/dss"]


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
    try:
        only = [arg for corpus in CORPORA for arg in ("--only", corpus)]
        status = main(["--window", *only, *sys.argv[1:]])
    except SystemExit as e:  # argparse
        status = e.code if isinstance(e.code, int) else 1
    except Exception:  # noqa: BLE001 - into the log, then the error message
        traceback.print_exc()
        status = 1
    if status and logged:
        show_error(f"Context-Fabric could not start.\n\nSee {LOG} for details.")
    sys.exit(status)
