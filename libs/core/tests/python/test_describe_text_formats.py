"""``describe_text_formats``: original-script / transliteration pairs from ``otext``.

The MCP ``get_text_formats`` tool shows these pairs so agents can learn how a
corpus encodes its text before writing lexical searches.
"""

from __future__ import annotations

import pytest

from cfabric import Fabric
from cfabric.describe import describe_text_formats


def _load(corpus):
    return Fabric(locations=str(corpus), silent="deep").loadAll(silent="deep")


@pytest.mark.parametrize("marker", ["trans", "translit"])
def test_pairs_original_and_transliterated_formats(mini_corpus_with_otext, marker):
    api = _load(mini_corpus_with_otext(f"@fmt:text-{marker}-full={{word}}"))

    info = describe_text_formats(api)

    assert [fmt.name for fmt in info.formats] == ["text-full"]
    pair = info.formats[0]
    assert pair.original_spec == "{word}"
    assert pair.transliteration_spec == "{word}"
    assert pair.samples


def test_corpus_without_a_transliteration_has_no_pairs(mini_corpus_with_otext):
    api = _load(mini_corpus_with_otext())

    info = describe_text_formats(api)

    assert info.formats == []
    assert "no orig/trans pairs" in info.description


@pytest.mark.parametrize(
    ("corpus", "expected"),
    [
        ("bhsa", {"lex-full", "lex-plain", "text-full", "text-full-ketiv", "text-plain"}),
        ("n1904", {"lex-plain", "text-plain"}),
    ],
)
def test_real_corpora_report_their_pairs(corpus_tf_dir, corpus, expected):
    info = describe_text_formats(_load(corpus_tf_dir(corpus)))

    assert {fmt.name for fmt in info.formats} == expected
    assert all(fmt.samples for fmt in info.formats)
