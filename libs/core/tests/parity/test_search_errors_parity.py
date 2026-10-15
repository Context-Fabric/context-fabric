"""Search template errors, the fail limit and result types against a live
Text-Fabric.

For every bad template Text-Fabric prints its messages and returns no
results; Context-Fabric raises ``SearchTemplateError`` with the same
messages. Known differences are normalized away in ``_normalize`` and listed
there.
"""

from __future__ import annotations

import contextlib
import io
import os
import re
import warnings
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest
from cfabric import Fabric, SearchTemplateError

pytestmark = pytest.mark.skipif(
    os.environ.get("CONTEXT_FABRIC_RUN_PARITY") != "1",
    reason="set CONTEXT_FABRIC_RUN_PARITY=1 to run Text-Fabric oracle parity tests",
)

MINI_CORPUS = Path(__file__).resolve().parents[1] / "fixtures" / "mini_corpus"

BAD_TEMPLATES = [
    # tf/search/syntax.py
    "word number>abc",
    "word pos~[",
    "word pos=",
    "x:word\ny:word\nx <> y",
    "a$:word",
    "/with/\nword",
    "word\n/have/\n/-/",
    "word\n  /with/\n  word",
    "word\n/without/\n  phrase\n/or/\n  phrase\n/-/",
    "word\n/with/\n  phrase\n /or/\n  phrase\n/-/",
    "word\n/where/\n  phrase\n/-/",
    "word\n/where/\n  phrase\n/have/\n  phrase",
    # tf/search/semantics.py::_grammar
    "  word",
    "sentence\n  phrase\n    word\n   word",
    "<\nword",
    "sentence\n  <\n  word",
    "x:word\nx < z",
    # tf/search/semantics.py::_validation
    "wordd",
    "word\n< nosuch",
    "word nosuch=1",
    "word number=abc",
    "word number~abc",
    "word pos<3",
    "word\n-parent=3> word",
    "word\n<nosuch- word",
    "x:word\ny:word\nx .number<pos. y",
    "x:word\ny:word\nx .pos~a~number. y",
    "word\n<x: word",
    # tf/search/graph.py::connectedness
    "pos=noun",
    "x:word\ny:word",
    # quantifier sub-templates (tf/search/spin.py)
    "word\n/without/\n  wordd\n/-/",
    "phrase\n/with/\n  word\n/or/\n  wordd\n/-/",
    "phrase\n/where/\n  word\n/have/\n  wordd\n/-/",
]

PAIRS = "a:.\nb:.\na # b"
"""56 results on mini_corpus, over its fail limit of 4 * 8 = 32."""


@pytest.fixture(scope="module")
def pair():
    tf = pytest.importorskip("tf.fabric").Fabric
    tf_api = tf(locations=str(MINI_CORPUS), silent="deep").loadAll(silent="deep")
    cf_api = Fabric(locations=str(MINI_CORPUS), silent="deep").loadAll(silent="deep")
    return tf_api, cf_api


def _printed(work) -> tuple[Any, str]:
    """The value of ``work()`` and what it printed."""
    output = io.StringIO()
    with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
        value = work()
    return value, output.getvalue()


def _normalize(line: str) -> str:
    """A message without the parts in which Context-Fabric knowingly differs:

    * the reason for a bad regular expression comes from Rust's regex engine;
    * a `<n` / `>n` condition prints as such, where TF prints the address of
      its comparison function;
    * for conditions on edge features and for feature comparison relations,
      TF reports the line of an atom by the relation's index;
    * TF pads the line number of an `Error under` header, and timestamps
      planning errors.
    """
    line = line.strip()
    line = re.sub(r"^\d+\.\d+s ", "", line)
    line = re.sub(r"^line\s+(\d+):", r"line \1:", line)
    line = re.sub(r'(Wrong regular expression "[^"]*"): .*$', r"\1", line)
    line = re.sub(r'"(<function .*>|[<>]-?\d+)" is not a number', '"LIMIT" is not a number', line)
    line = re.sub(r"(superfluous:|is the wrong type:|in) line\(s\) .*$", r"\1", line)
    return line


@pytest.mark.parametrize("template", BAD_TEMPLATES)
def test_bad_templates_report_text_fabric_messages(pair, template):
    tf_api, cf_api = pair
    tf_results, tf_output = _printed(lambda: tf_api.S.search(template))
    if not tf_api.S.exe.good:
        assert list(tf_results) == []
    _, cf_legend = _printed(cf_api.S.relationsLegend)
    with pytest.raises(SearchTemplateError) as raised:
        cf_api.S.search(template)
    tf_lines = {_normalize(line) for line in tf_output.splitlines()}
    for line in str(raised.value).splitlines():
        if line in cf_legend.splitlines():
            # The relations legend after "Allowed relations:" is Context-Fabric's.
            continue
        assert _normalize(line) in tf_lines, (line, tf_output)


@pytest.mark.parametrize("template", BAD_TEMPLATES)
def test_study_records_text_fabric_exe_lists(pair, template):
    tf_api, cf_api = pair
    _printed(lambda: tf_api.S.study(template))
    cf_api.S.study(template)
    if tf_api.S.exe.good:
        # Quantifier sub-template errors: TF goes on without the quantifier's
        # results; Context-Fabric rejects the template.
        assert "/" in template
        assert cf_api.S.exe.good is False
        return
    assert cf_api.S.exe.good is False
    for kind in ("badSyntax", "badSemantics"):
        tf_messages = getattr(tf_api.S.exe, kind, None) or []
        cf_messages = getattr(cf_api.S.exe, kind)
        assert [
            (line, _normalize(text)) for line, text in cf_messages if not text.startswith("Allowed")
        ] == [
            (line, _normalize(text)) for line, text in tf_messages if not text.startswith("Allowed")
        ]


def test_the_cut_off_warning_and_count_match_text_fabric(pair):
    tf_api, cf_api = pair
    tf_rows, tf_output = _printed(lambda: list(tf_api.S.search(PAIRS)))
    message = re.search(r"cut off at \d+ results\. There are more \.\.\.", tf_output).group(0)
    with pytest.warns(UserWarning) as caught:
        cf_rows = list(cf_api.S.search(PAIRS))
    assert [str(warning.message) for warning in caught] == [message]
    assert len(cf_rows) == len(tf_rows) == 32

    tf_rows, tf_output = _printed(lambda: tf_api.S.search(PAIRS, limit=0))
    assert message in tf_output
    with pytest.warns(UserWarning, match=re.escape(message)):
        assert len(cf_api.S.search(PAIRS, limit=0)) == len(tf_rows)

    _printed(lambda: tf_api.S.study(PAIRS))
    _, tf_output = _printed(tf_api.S.count)
    assert message in tf_output
    cf_api.S.study(PAIRS)
    with pytest.warns(UserWarning, match=re.escape(message)):
        assert cf_api.S.count() == 32

    with warnings.catch_warnings():
        warnings.simplefilter("error")
        _, tf_output = _printed(lambda: tf_api.S.search(PAIRS, limit=40))
        assert "cut off" not in tf_output
        cf_api.S.search(PAIRS, limit=40)


@pytest.mark.parametrize(
    ("limit", "shallow"),
    [(None, False), (3, False), (0, False), (-1, False), (None, True), (2, True), (None, 2)],
)
def test_result_types_match_text_fabric(pair, limit, shallow):
    tf_api, cf_api = pair
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        tf_result, _ = _printed(lambda: tf_api.S.search(PAIRS, limit=limit, shallow=shallow))
        cf_result = cf_api.S.search(PAIRS, limit=limit, shallow=shallow)
        if type(tf_result).__name__ == "generator":
            # A lazy iterator in place of Text-Fabric's generator.
            assert isinstance(cf_result, Iterator)
            tf_result, _ = _printed(lambda: list(tf_result))
            cf_result = list(cf_result)
            assert sorted(cf_result) == sorted(tf_result)
            return
        assert type(cf_result) is type(tf_result)
        assert len(cf_result) == len(tf_result)
        if isinstance(tf_result, set):
            assert cf_result == tf_result
