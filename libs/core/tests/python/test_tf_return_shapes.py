"""Return values *and shapes* (list vs tuple vs generator, int vs float, key
types) of core APIs match Text-Fabric 13.0.19 on ``mini_corpus``.

Expected values were captured from Text-Fabric on the same corpus; equality
below is type-strict where Text-Fabric's type differs from the obvious one.
"""

from __future__ import annotations

import types

import pytest

from conftest import LOAD_MODES, open_corpus


@pytest.fixture(params=LOAD_MODES)
def api(request, mini_corpus_path, tmp_path):
    return open_corpus([mini_corpus_path], request.param, tmp_path, features="")


def _exact(value, expected):
    """Equal, with the same container types all the way down."""
    assert type(value) is type(expected), (value, expected)
    if isinstance(expected, (list, tuple)):
        assert len(value) == len(expected), (value, expected)
        for item, expected_item in zip(value, expected):
            _exact(item, expected_item)
    elif isinstance(expected, dict):
        assert list(value) == list(expected), (value, expected)
        for key in expected:
            _exact(value[key], expected[key])
    else:
        assert value == expected


def test_locality_intersecting_is_a_list(api):
    _exact(api.L.i(6), [8, 1, 2, 3])
    _exact(api.L.i(1), ())
    _exact(api.L.i(99), ())


def test_sort_nodes_returns_a_list(api):
    _exact(api.N.sortNodes({5, 3, 1}), [1, 3, 5])
    _exact(api.N.sortNodes([8, 7, 6]), [8, 6, 7])


def test_walk_is_a_generator(api):
    walk = api.N.walk()
    assert isinstance(walk, types.GeneratorType) or iter(walk) is walk
    assert next(walk) == 8
    _exact(list(api.N.walk()), [8, 6, 1, 2, 3, 7, 4, 5])
    _exact(list(api.N.walk(nodes=[1, 6], events=True)), [(6, False), (1, None)])


def test_text_config_lists(api):
    _exact(api.T.sectionTypes, ["sentence", "phrase"])
    _exact(api.T.sectionFeats, ["sentence_id", "phrase_id"])
    _exact(api.T.structureTypes, [])
    _exact(api.T.structureFeats, [])
    assert api.T.defaultFormat == "text-orig-full"


def test_levels_slot_average_is_int(api):
    _exact(
        api.C.levels.data,
        (("sentence", 5.0, 8, 8), ("phrase", 2.5, 6, 7), ("word", 1, 1, 5)),
    )


def test_otype_interval_and_oslots_bounds(api):
    _exact(api.F.otype.sInterval("phrase"), (6, 7))
    _exact(api.F.otype.sInterval("word"), (1, 5))
    _exact(api.F.otype.sInterval("nope"), ())
    _exact(api.E.oslots.s(0), ())
    _exact(api.E.oslots.s(99), ())
    _exact(api.E.oslots.s(1), (1,))


def test_sections_keep_feature_value_types(api):
    # phrase_id is an int feature: TF keys sec1 by int, not str.
    _exact(api.C.sections.data["sec1"], {8: {1: 6, 2: 7}})
