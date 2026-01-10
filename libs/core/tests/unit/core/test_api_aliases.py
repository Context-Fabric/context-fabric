"""Test that both api.CF and api.TF aliases work identically."""

import pytest
from unittest.mock import MagicMock


def test_api_cf_and_tf_are_same_object():
    """Verify api.CF and api.TF reference the same Fabric instance."""
    from cfabric.core.api import Api

    mock_fabric = MagicMock()
    mock_fabric.featuresIgnored = set()

    api = Api(mock_fabric)

    assert api.CF is api.TF
    assert api.CF is mock_fabric


def test_api_cf_in_api_refs():
    """Verify CF is documented in API_REFS."""
    from cfabric.core.api import API_REFS

    assert "CF" in API_REFS
    assert "TF" in API_REFS
    assert API_REFS["CF"] == API_REFS["TF"]


def test_api_attributes_set_on_cf():
    """Verify that attributes set via CF are accessible."""
    from cfabric.core.api import Api

    mock_fabric = MagicMock()
    mock_fabric.featuresIgnored = set()
    mock_fabric.features = {}

    api = Api(mock_fabric)

    # Access through CF
    assert api.CF.features == {}
    # Also accessible through TF alias
    assert api.TF.features == {}
