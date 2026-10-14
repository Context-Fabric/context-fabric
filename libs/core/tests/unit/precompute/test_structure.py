"""Tests for the `structure` pre-computation step.

The failure paths of `structure` must keep the arity of the success path:
`Text.__init__` unpacks the result into six variables unconditionally.
"""

import pytest
from cfabric.precompute.prepare import NO_STRUCTURE, structure


def _noop(*args, **kwargs):
    return None


def _run(structureTypes, nsFeats):
    """Call `structure` far enough to reach its validation branches."""
    return structure(
        _noop,                          # info
        _noop,                          # error
        ({}, 0, 0, "word"),             # otype
        ({},),                          # oslots
        {"structureTypes": structureTypes},
        (),                             # rank
        (),                             # levUp
        *({} for _ in range(nsFeats)),  # sFeats
    )


class TestStructureFailureArity:
    """A rejected structure configuration still returns six values."""

    def test_duplicate_levels_returns_six_values(self):
        result = _run("book,card,card", 3)
        assert len(result) == 6
        assert result == NO_STRUCTURE

    def test_level_feature_mismatch_returns_six_values(self):
        result = _run("book,card", 1)
        assert len(result) == 6
        assert result == NO_STRUCTURE

    @pytest.mark.parametrize(
        "structureTypes,nsFeats",
        [("book,card,card", 3), ("book,card", 1)],
    )
    def test_failure_result_unpacks_like_the_consumer(self, structureTypes, nsFeats):
        """Text.__init__ unpacks six names; this must not raise ValueError."""
        (hdFromNd, ndFromHd, hdMult, hdTop, hdUp, hdDown) = _run(
            structureTypes, nsFeats
        )
        # `None` is the sentinel the Text API tests for.
        assert hdFromNd is None
        assert ndFromHd is None
        assert hdMult is None
        assert hdTop is None
        assert hdUp is None
        assert hdDown is None

    def test_failure_is_reported_to_the_error_channel(self):
        messages = []
        structure(
            _noop,
            lambda msg, *a, **k: messages.append(msg),
            ({}, 0, 0, "word"),
            ({},),
            {"structureTypes": "book,card,card"},
            (),
            (),
            {}, {}, {},
        )
        assert any("duplicate structure levels" in m for m in messages)
