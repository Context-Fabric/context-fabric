"""The text formats the browser offers: the BHSA's vocalized and unvocalized
text, and only the full text of the other corpora.

A corpus defines more formats (transliterations, lexemes, the ketiv, the
DSS' word-level and source formats); the browser leaves them out of its
display options, and a request for one of them (an old link, a saved
workspace) gets the corpus' default. The full list stays in the corpus'
`DisplayConfig`, because the comparison and the Concordance choose their
consonants from it.
"""

from __future__ import annotations

from dataclasses import replace
from typing import Any

from cfabric_browser.display import DisplayConfig, RenderOptions

# vocalized (vowels, accents, the qere) and unvocalized (consonants, the ketiv)
FORMATS = ("text-orig-full", "text-orig-plain")


def shown(config: DisplayConfig) -> list[str]:
    """The formats of a corpus the browser offers, in the order of `FORMATS`."""
    return [f for f in FORMATS if f in config.text_formats]


def default(config: DisplayConfig) -> str:
    """The format a corpus is shown in when none is chosen."""
    offered = shown(config)
    if config.default_format in offered or not offered:
        return str(config.default_format)
    return offered[0]


def chosen(config: DisplayConfig, fmt: str | None) -> str | None:
    """The format to use for a request: `fmt` if the browser offers it for
    this corpus, else None (the default)."""
    return fmt if fmt in shown(config) else None


def options(config: DisplayConfig, opts: RenderOptions) -> RenderOptions:
    """Display options with a format the browser offers for this corpus."""
    fmt = chosen(config, opts.fmt)
    return opts if fmt == opts.fmt else replace(opts, fmt=fmt)


def display(config: DisplayConfig) -> dict[str, Any]:
    """`config.to_dict()` with only the formats offered."""
    d: dict[str, Any] = config.to_dict()
    d["textFormats"] = shown(config)
    d["defaultFormat"] = default(config)
    return d
