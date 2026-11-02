"""`python -m cfabric_browser`, and the entry point of the standalone program."""

import multiprocessing
import sys

from cfabric_browser.cli import main

if __name__ == "__main__":
    # In a frozen program (PyInstaller), each search worker starts the program
    # again; this turns that start into the worker instead of a second server.
    multiprocessing.freeze_support()
    sys.exit(main())
