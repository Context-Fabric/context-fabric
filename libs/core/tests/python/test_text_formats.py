"""Validate T.formats / T.languages / T.structureInfo against text-fabric.

These tests load BHSA (see ``bhsa_api`` in ``tests/conftest.py``) and compare
the text-metadata surface against text-fabric's golden values for the same
dataset. They also include a (non-gating) verse-text render timing probe.

Skipped automatically when the BHSA corpus is unavailable.
"""

from __future__ import annotations

import time

import pytest

# TF 13.0.19 golden values for the BHSA tf dataset shipped in this repo
# (libs/benchmarks/.corpora/bhsa/tf), captured directly from text-fabric.
TF_FORMATS = {
    "lex-default": "word",
    "lex-orig-full": "word",
    "lex-orig-plain": "word",
    "lex-trans-full": "word",
    "lex-trans-plain": "word",
    "text-orig-full": "word",
    "text-orig-full-ketiv": "word",
    "text-orig-plain": "word",
    "text-trans-full": "word",
    "text-trans-full-ketiv": "word",
    "text-trans-plain": "word",
}

TF_LANGUAGES = {
    "": {"language": "default", "languageEnglish": "default"},
    "am": {"language": "ኣማርኛ", "languageEnglish": "amharic"},
    "ar": {"language": "العَرَبِية", "languageEnglish": "arabic"},
    "bn": {"language": "বাংলা", "languageEnglish": "bengali"},
    "da": {"language": "Dansk", "languageEnglish": "danish"},
    "de": {"language": "Deutsch", "languageEnglish": "german"},
    "el": {"language": "Ελληνικά", "languageEnglish": "greek"},
    "en": {"language": "English", "languageEnglish": "english"},
    "es": {"language": "Español", "languageEnglish": "spanish"},
    "fa": {"language": "فارسی", "languageEnglish": "farsi"},
    "fr": {"language": "Français", "languageEnglish": "french"},
    "he": {"language": "עברית", "languageEnglish": "hebrew"},
    "hi": {"language": "हिन्दी", "languageEnglish": "hindi"},
    "id": {"language": "Bahasa Indonesia", "languageEnglish": "indonesian"},
    "ja": {"language": "日本語", "languageEnglish": "japanese"},
    "ko": {"language": "한국어", "languageEnglish": "korean"},
    "la": {"language": "Latina", "languageEnglish": "latin"},
    "nl": {"language": "Nederlands", "languageEnglish": "dutch"},
    "pa": {"language": "ਪੰਜਾਬੀ", "languageEnglish": "punjabi"},
    "pt": {"language": "Português", "languageEnglish": "portuguese"},
    "ru": {"language": "Русский", "languageEnglish": "russian"},
    "sw": {"language": "Kiswahili", "languageEnglish": "swahili"},
    "syc": {"language": "ܠܫܢܐ ܣܘܪܝܝܐ", "languageEnglish": "syriac"},
    "tr": {"language": "Türkçe", "languageEnglish": "turkish"},
    "ur": {"language": "اُردُو", "languageEnglish": "urdu"},
    "yo": {"language": "èdè Yorùbá", "languageEnglish": "yoruba"},
    "zh": {"language": "中文", "languageEnglish": "chinese"},
}


@pytest.fixture(scope="module")
def api(bhsa_api):
    return bhsa_api


def test_formats_match_tf(api):
    formats = api.T.formats
    assert isinstance(formats, dict)
    assert formats == TF_FORMATS


def test_languages_match_tf(api):
    languages = api.T.languages
    assert isinstance(languages, dict)
    assert languages == TF_LANGUAGES


def test_structure_info_no_structure(api, capsys):
    # BHSA configures no structure sections, so TF prints
    # "No structural elements configured" and returns None.
    assert api.T.structureInfo() is None
    assert "No structural elements configured" in capsys.readouterr().out


def test_verse_text_render_timing(api):
    """Non-gating timing probe for per-verse text rendering."""
    T = api.T
    F = api.F
    verses = [n for n in F.otype.s("verse")][:200]
    assert verses, "expected verse nodes"

    # warm caches
    for v in verses[:10]:
        T.text(v)

    start = time.perf_counter()
    for v in verses:
        T.text(v)
    elapsed = time.perf_counter() - start
    per_call_us = elapsed / len(verses) * 1e6
    print(
        f"\n[verse-text] {len(verses)} verses in {elapsed * 1e3:.2f} ms "
        f"=> {per_call_us:.1f} µs/verse"
    )
