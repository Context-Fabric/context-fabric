"""Translations that can be added to the browser, such as the JPS 1917.

A translation is downloaded once, checked and built into an ordinary corpus
in Text-Fabric layout, so that every interface can read, align and search it
like the Hebrew texts. See `catalog.py` for what is known about each source
and `build.py` for the steps.
"""

from cfabric_browser.translations.build import BuildError, build
from cfabric_browser.translations.catalog import TRANSLATIONS, Split, Translation

__all__ = ["TRANSLATIONS", "BuildError", "Split", "Translation", "build"]
