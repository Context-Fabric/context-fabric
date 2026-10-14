"""Regression contracts for composed TF topology when .cfm caching is present."""

from __future__ import annotations

import shutil
from pathlib import Path

from cfabric.core.fabric import Fabric


def _feature_only_module(tmp_path: Path) -> Path:
    module = tmp_path / "module"
    module.mkdir()
    (module / "module_label.tf").write_text(
        "@node\n"
        "@valueType=str\n"
        "@description=feature-only composition cache contract\n"
        "\n"
        "one\n"
        "two\n"
        "three\n"
        "four\n"
        "five\n",
        encoding="utf-8",
    )
    return module


def _copy_base(fixtures_dir: Path, tmp_path: Path) -> Path:
    base = tmp_path / "base"
    shutil.copytree(fixtures_dir / "mini_corpus", base)
    shutil.rmtree(base / ".cfm", ignore_errors=True)
    return base


def test_base_cfm_cache_does_not_hide_second_location_feature(fixtures_dir, tmp_path):
    base = _copy_base(fixtures_dir, tmp_path)

    base_fabric = Fabric(locations=str(base), silent="deep")
    base_api = base_fabric.loadAll(silent="deep")
    assert base_api is not False
    assert base_api.F.otype.maxSlot == 5
    assert base_api.F.otype.maxNode == 8
    assert (base / ".cfm").is_dir(), "base-only load must create the cache precondition"

    module = _feature_only_module(tmp_path)
    composed = Fabric(locations=[str(base), str(module)], silent="deep")
    assert tuple(Path(location).resolve() for location in composed.locations) == (
        base.resolve(),
        module.resolve(),
    )
    api = composed.loadAll(silent="deep")

    assert api is not False
    assert api.F.otype.maxSlot == 5
    assert api.F.otype.maxNode == 8
    assert "module_label" in api.Fall()
    assert api.Fs("module_label", warn=False).v(1) == "one"
    assert api.Fs("module_label", warn=False).v(5) == "five"
    assert (base / ".cfm").is_dir(), "composition must not delete the reusable base cache"


def test_composed_load_does_not_emit_single_location_cfm_cache(fixtures_dir, tmp_path):
    base = _copy_base(fixtures_dir, tmp_path)
    module = _feature_only_module(tmp_path)

    composed = Fabric(locations=[str(base), str(module)], silent="deep")
    api = composed.loadAll(silent="deep")

    assert api is not False
    assert "module_label" in api.Fall()
    assert not (base / ".cfm").exists()
    assert not (module / ".cfm").exists()


def test_base_module_cfm_cache_does_not_hide_overlay_module(fixtures_dir, tmp_path):
    root = tmp_path / "root"
    root.mkdir()
    base = _copy_base(fixtures_dir, root)

    base_fabric = Fabric(locations=str(base), silent="deep")
    base_api = base_fabric.loadAll(silent="deep")
    assert base_api is not False
    assert (base / ".cfm").is_dir(), "base module must create the cache precondition"

    module = _feature_only_module(root)
    composed = Fabric(locations=str(root), modules=["base", "module"], silent="deep")
    assert tuple(composed.modules) == ("base", "module")
    api = composed.loadAll(silent="deep")

    assert api is not False
    assert api.F.otype.maxSlot == 5
    assert api.F.otype.maxNode == 8
    assert "module_label" in api.Fall()
    assert api.Fs("module_label", warn=False).v(3) == "three"
    assert (base / ".cfm").is_dir(), "composition must retain the reusable base cache"
    assert not (module / ".cfm").exists()
