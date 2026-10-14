"""Engine loaders and process-memory helpers shared by the RC baseline probes.

Every probe script in this directory runs one engine per process so that load
time and resident memory are measured in isolation. The scripts depend only on
the standard library, ``psutil`` (optional) and the engine package itself, so
they run unchanged under any of the three interpreters:

* ``tf``             Text-Fabric (``tf.fabric.Fabric``), the reference.
* ``cf_python``      Context-Fabric 0.5.8, the previous pure-Python core.
* ``cf_rust``        Context-Fabric 0.6.0rc1 (Rust/PyO3 core) via ``loadAll``,
                     which compiles/reuses ``<tf>/.cfr/<v>/corpus.cfr``.
* ``cf_rust_mapped`` Context-Fabric 0.6.0rc1 via ``openMapped(<cache>)`` on an
                     explicit compiled cache (the path ``perf_gate`` measures).

Run probes from a directory outside the repository (for example ``/tmp``):
importing ``cfabric`` from ``libs/core`` picks up the source overlay instead of
the installed package.
"""

from __future__ import annotations

import resource
import shutil
import sys
from pathlib import Path
from typing import Any

ENGINES = ("tf", "cf_python", "cf_rust", "cf_rust_mapped")

# Per-engine cache directory created next to the ``.tf`` sources. Deleting it
# forces the next load to rebuild the engine cache ("cold" load).
_CACHE_DIRS = {
    "tf": ".tf",
    "cf_python": ".cfm",
    "cf_rust": ".cfr",
}


def cache_dir(engine: str, tf_path: Path, mapped_cache: Path | None = None) -> Path | None:
    """Return the on-disk cache an engine builds for ``tf_path``.

    Args:
        engine: Engine name from ``ENGINES``.
        tf_path: Directory holding the corpus ``.tf`` files.
        mapped_cache: Explicit ``.cfr`` file used by ``cf_rust_mapped``.

    Returns:
        The cache path, or ``None`` when the engine keeps no cache.
    """
    if engine == "cf_rust_mapped":
        return mapped_cache
    name = _CACHE_DIRS.get(engine)
    return tf_path / name if name else None


def clear_cache(engine: str, tf_path: Path, mapped_cache: Path | None = None) -> bool:
    """Delete the engine cache so the next load is a cold (rebuild) load.

    Returns:
        True when something was deleted.
    """
    path = cache_dir(engine, tf_path, mapped_cache)
    if path is None or not path.exists():
        return False
    if path.is_dir():
        shutil.rmtree(path)
    else:
        path.unlink()
    return True


def load(engine: str, tf_path: Path, mapped_cache: Path | None = None) -> Any:
    """Load the full corpus with the requested engine and return its API object."""
    if engine == "tf":
        from tf.fabric import Fabric  # type: ignore[import-untyped]

        return Fabric(locations=str(tf_path), silent="deep").loadAll(silent="deep")
    if engine in ("cf_python", "cf_rust"):
        from cfabric import Fabric

        return Fabric(locations=str(tf_path), silent="deep").loadAll(silent="deep")
    if engine == "cf_rust_mapped":
        from cfabric import Fabric

        if mapped_cache is None:
            raise ValueError("cf_rust_mapped requires --mapped-cache")
        if not mapped_cache.exists():
            Fabric(locations=str(tf_path), silent="deep").compile(str(mapped_cache))
        return Fabric(locations=str(tf_path), silent="deep").openMapped(str(mapped_cache))
    raise ValueError(f"unknown engine {engine!r}")


def engine_version(engine: str) -> str:
    """Return the installed version string of the package behind ``engine``."""
    if engine == "tf":
        from tf.parameters import VERSION  # type: ignore[import-untyped]

        return str(VERSION)
    import cfabric

    return str(getattr(cfabric, "__version__", "unknown"))


def check_engine_identity(engine: str) -> None:
    """Fail fast when the interpreter has the wrong ``cfabric`` for the engine.

    ``cf_python`` must be the 0.5.x pure-Python package; ``cf_rust*`` must expose
    the compiled ``cfabric._core`` extension.
    """
    if engine == "tf":
        return
    import cfabric

    version = str(getattr(cfabric, "__version__", ""))
    is_rust = version.startswith("0.6")
    if engine == "cf_python" and is_rust:
        raise SystemExit(f"engine cf_python needs context-fabric 0.5.x, found {version}")
    if engine.startswith("cf_rust") and not is_rust:
        raise SystemExit(f"engine {engine} needs context-fabric 0.6.x, found {version}")


def memory_snapshot() -> dict[str, float | None]:
    """Current RSS, USS and peak RSS of this process in MB.

    ``peak_rss_mb`` comes from ``ru_maxrss`` (bytes on macOS, KiB on Linux) and
    is the high-water mark since process start. ``uss_mb`` (unique set size)
    excludes shared/clean file-backed pages, which matters for mmap engines.
    """
    maxrss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    peak = maxrss / (1024 * 1024) if sys.platform == "darwin" else maxrss / 1024
    snapshot: dict[str, float | None] = {"peak_rss_mb": round(peak, 1), "rss_mb": None, "uss_mb": None}
    try:
        import psutil

        process = psutil.Process()
        snapshot["rss_mb"] = round(process.memory_info().rss / (1024 * 1024), 1)
        try:
            snapshot["uss_mb"] = round(process.memory_full_info().uss / (1024 * 1024), 1)
        except Exception:  # noqa: BLE001 - USS is best effort (permission dependent)
            pass
    except ImportError:
        pass
    return snapshot


def normalize(value: Any) -> Any:
    """Convert engine return values into plain JSON-compatible Python values."""
    try:
        import numpy as np

        if isinstance(value, np.integer):
            return int(value)
        if isinstance(value, np.ndarray):
            return [normalize(item) for item in value.tolist()]
    except ImportError:
        pass
    if isinstance(value, bool) or value is None or isinstance(value, (str, float)):
        return value
    if isinstance(value, int):
        return int(value)
    if isinstance(value, (tuple, list)):
        return [normalize(item) for item in value]
    if isinstance(value, dict):
        return {str(key): normalize(item) for key, item in value.items()}
    if isinstance(value, (set, frozenset)):
        return sorted((normalize(item) for item in value), key=repr)
    if hasattr(value, "__iter__"):
        return [normalize(item) for item in value]
    return value
