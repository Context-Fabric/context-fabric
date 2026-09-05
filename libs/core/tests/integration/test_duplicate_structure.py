"""Loading a corpus whose otext declares a duplicated structure level.

Such a corpus is rejected by the `structure` pre-computation step. That
rejection must degrade to "no structural elements", not abort the load.
"""

import pytest
from cfabric.core.fabric import Fabric


@pytest.fixture(scope="module")
def duplicate_structure_path(fixtures_dir):
    """mini_corpus with `@structureTypes=sentence,phrase,phrase`."""
    return str(fixtures_dir / "duplicate_structure")


@pytest.fixture(scope="module")
def duplicate_structure_api(duplicate_structure_path, tmp_path_factory):
    CF = Fabric(locations=duplicate_structure_path, silent="deep")
    api = CF.loadAll(silent="deep")
    assert api is not False, "Failed to load duplicate_structure corpus"
    return api


class TestDuplicateStructureLevels:
    def test_corpus_still_loads(self, duplicate_structure_api):
        """Regression: this raised ValueError from Text.__init__.

        `structure` returned a 2-tuple on its failure paths while
        `Text.__init__` unpacks six values.
        """
        assert duplicate_structure_api is not False

    def test_text_api_reports_no_structure(self, duplicate_structure_api):
        T = duplicate_structure_api.T
        assert T.hdFromNd is None
        assert T.hdTop is None

    def test_structure_accessors_degrade_quietly(self, duplicate_structure_api):
        T = duplicate_structure_api.T
        assert T.structure(node=1) is None
        assert T.structurePretty(node=1) is None
        assert T.top() is None
        assert T.up(1) is None
        assert T.down(1) is None
        assert T.headingFromNode(1) is None
        assert T.nodeFromHeading((("sentence", "s1"),)) is None

    def test_ordinary_text_access_is_unaffected(self, duplicate_structure_api):
        """A broken structure config must not disturb the rest of the API."""
        F = duplicate_structure_api.F
        assert F.otype.v(1) == "word"
        assert F.word.v(1) == "hello"
