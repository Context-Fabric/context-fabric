"""Bad search templates raise ``cfabric.SearchTemplateError`` with
Text-Fabric's messages (``tf/search/syntax.py``, ``semantics.py``,
``graph.py``), and ``S.study`` records them on ``S.exe`` instead.

Expected messages were captured from Text-Fabric 13.0.19 on ``mini_corpus``
(``S.exe.badSyntax`` / ``badSemantics``), except where a comment notes how
Context-Fabric differs. ``tests/parity`` compares them with a live
Text-Fabric.
"""

from __future__ import annotations

import pytest

import cfabric
from cfabric import SearchTemplateError

VALID_TYPES = "Valid object types are: sentence, phrase, word"

SYNTAX = {
    "word number>abc": [(0, 'Limit is non numeric "abc"')],
    "word pos=": [(0, 'Unrecognized feature condition "pos="')],
    "x:word\ny:word\nx <> y": [(2, 'Unrecognized feature condition ""')],
    "a$:word": [(0, 'Illegal name: "a$"')],
    "/with/\nword": [(0, "Quantifier: No preceding tokens")],
    "word\n/have/\n/-/": [
        (1, 'Quantifier: Can not start with "/have/:"'),
        (2, 'Quantifier: Can not start with "/-/:"'),
    ],
    "word\n  /with/\n  word": [
        (1, "Quantifier: Does not immediately follow an atom at the same level")
    ],
    "word\n/without/\n  phrase\n/or/\n  phrase\n/-/": [
        (3, 'Quantifier: "/or/" can not follow "/without/" on line 1')
    ],
    "word\n/with/\n  phrase\n /or/\n  phrase\n/-/": [
        (3, 'Quantifier "/or/" has not same indentation as "/with/" on line 1'),
        (5, 'Quantifier "/-/" has not same indentation as "/or/" on line 3'),
    ],
    "word\n/where/\n  phrase\n/-/": [
        (3, 'Quantifier: "/-/" : premature end of "/where/" on line 1')
    ],
    "word\n/where/\n  phrase\n/have/\n  phrase": [(3, 'Quantifier: Unterminated "/have/"')],
}
"""Templates with syntax errors (``S.exe.badSyntax``)."""

SEMANTICS = {
    "  word": [(0, "Unexpected indent: 2, expected 0")],
    "sentence\n  phrase\n    word\n   word": [(3, "Unexpected indent: 3, expected one of 0, 2")],
    "<\nword": [(0, "Lonely relation: not allowed at outermost level")],
    "sentence\n  <\n  word": [(1, "Lonely relation: not allowed as first child")],
    "x:word\nx < z": [(1, 'Relation with undefined name: "z"')],
    "wordd": [(0, 'Unknown object type: "wordd"'), (None, VALID_TYPES)],
    "word nosuch=1": [(None, 'Missing feature "nosuch" in line(s) 0')],
    "word number=abc": [
        (None, 'Feature "number" has wrong values:'),
        (None, '    "abc" is not a number: line(s) 0'),
    ],
    # Text-Fabric prints the address of its comparison function for `<3`.
    "word pos<3": [
        (None, 'Feature "pos" has wrong values:'),
        (None, '    "<3" is not a number: line(s) 0'),
    ],
    # Text-Fabric reports line(s) 0 here: it looks up the edge's index among
    # the atoms' lines. Context-Fabric reports the line of the relation.
    "word\n-parent=3> word": [
        (None, 'Feature "parent" cannot have values:'),
        (None, "    \"(True, frozenset({'3'}))\" superfluous: line(s) 1"),
    ],
    "x:word\ny:word\nx .number<pos. y": [
        (None, 'Feature "pos" has wrong type:'),
        (None, '    "str" is the wrong type: line(s) 2'),
    ],
}
"""Templates with grammar or validation errors (``S.exe.badSemantics``)."""

PLAN = {
    "pos=noun": ["Search without instructions. Tell me what to look for."],
    "x:word\ny:word": [
        "More than one connected components (2):",
        "Either run the subqueries one by one, or connect the components by a relation",
    ],
}
"""Templates Text-Fabric cannot plan: reported, but on neither ``S.exe`` list."""


def _text(messages):
    return "\n".join(text if line is None else f"line {line}: {text}" for line, text in messages)


def test_the_error_is_a_value_error_exported_by_cfabric():
    assert issubclass(SearchTemplateError, ValueError)
    assert cfabric.SearchTemplateError is cfabric._core.SearchTemplateError
    assert "SearchTemplateError" in cfabric.__all__


@pytest.mark.parametrize("template", [*SYNTAX, *SEMANTICS])
def test_search_raises_text_fabric_messages(loaded_api, template):
    expected = SYNTAX.get(template) or SEMANTICS[template]
    with pytest.raises(SearchTemplateError) as raised:
        loaded_api.S.search(template)
    assert str(raised.value) == _text(expected)


@pytest.mark.parametrize("template", [*SYNTAX, *SEMANTICS])
def test_study_records_the_messages_and_fetch_and_count_raise(loaded_api, template, capsys):
    search = loaded_api.S
    assert search.study(template) is None
    assert search.exe.good is False
    assert search.exe.badSyntax == SYNTAX.get(template, [])
    assert search.exe.badSemantics == SEMANTICS.get(template, [])
    with pytest.raises(SearchTemplateError):
        search.fetch()
    with pytest.raises(SearchTemplateError):
        search.count()
    search.showPlan()
    assert capsys.readouterr().out == ""


def test_bad_regular_expressions_name_the_pattern(loaded_api):
    # The reason is the regex engine's (Rust's), not Python's.
    with pytest.raises(SearchTemplateError, match=r'^line 0: Wrong regular expression "\[": "'):
        loaded_api.S.search("word pos~[")


def test_unknown_relations_list_the_allowed_ones(loaded_api):
    with pytest.raises(SearchTemplateError) as raised:
        loaded_api.S.search("word\n<x: word")
    lines = str(raised.value).split("\n")
    assert lines[:3] == ['line 1: Unknown relation: "<x:"', "Allowed relations:", "Supported relations:"]
    loaded_api.S.study("word\n<x: word")
    assert loaded_api.S.exe.badSemantics[0] == (1, 'Unknown relation: "<x:"')


@pytest.mark.parametrize("template", PLAN)
def test_unplannable_templates_raise_without_exe_messages(loaded_api, template):
    with pytest.raises(SearchTemplateError) as raised:
        loaded_api.S.search(template)
    assert str(raised.value) == "\n".join(PLAN[template])
    loaded_api.S.study(template)
    assert loaded_api.S.exe.good is False
    assert loaded_api.S.exe.badSyntax == []
    assert loaded_api.S.exe.badSemantics == []


@pytest.mark.parametrize(
    ("template", "header"),
    [
        ("word\n/without/\n  wordd\n/-/", (1, "Error under /without/:")),
        ("word\n/with/\n  phrase\n/or/\n  wordd\n/-/", (3, "Error under /or/:")),
        ("word\n/where/\n  phrase\n/have/\n  wordd\n/-/", (2, "Error under /have/:")),
    ],
)
def test_quantifier_sub_templates_are_checked(loaded_api, template, header):
    # Text-Fabric reports these while running the quantifier and goes on
    # without its results; Context-Fabric rejects the template up front.
    with pytest.raises(SearchTemplateError) as raised:
        loaded_api.S.search(template)
    wordd_line = template.split("\n").index("  wordd")
    assert str(raised.value) == _text(
        [header, (wordd_line, 'Unknown object type: "wordd"'), (None, VALID_TYPES)]
    )


def test_custom_sets_extend_the_valid_types(loaded_api):
    with pytest.raises(SearchTemplateError) as raised:
        loaded_api.S.search("nouns", sets={"verbs": [1]})
    assert str(raised.value) == _text(
        [
            (0, 'Unknown object type: "nouns"'),
            (None, VALID_TYPES),
            (None, "Or choose a custom set from: verbs"),
        ]
    )


def test_a_good_study_after_a_bad_one(loaded_api):
    loaded_api.S.study("wordd")
    loaded_api.S.study("word")
    assert loaded_api.S.exe.good is True
    assert loaded_api.S.exe.badSyntax == [] and loaded_api.S.exe.badSemantics == []
    assert tuple(loaded_api.S.fetch()) == ((1,), (2,), (3,), (4,), (5,))
