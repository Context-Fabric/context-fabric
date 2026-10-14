"""Entry point of the standalone program: the browser in a window of its own."""

import multiprocessing
import sys

from cfabric_browser.cli import main

if __name__ == "__main__":
    # In a frozen program, each search worker starts the program again; this
    # turns that start into the worker instead of a second window.
    multiprocessing.freeze_support()
    sys.exit(main(["--window", *sys.argv[1:]]))
