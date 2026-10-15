"""Adding and removing translations, from the library or the command line.

A translation is added by building it (`build.py`) into the first corpus root
(`~/text-fabric-data/github`), after which the library lists it like any other
corpus. Removing deletes only a directory the builder made (its
`app/config.yaml` names the translation). A corpus that is open cannot be
deleted while the browser runs (on Windows its files are in use), so its
removal waits until the next start (`remove_pending`).
"""

from __future__ import annotations

import json
import logging
import shutil
from pathlib import Path
from typing import Any

from cfabric.display.config import read_app_config

from cfabric_browser.jobs import BackgroundJobs
from cfabric_browser.translations.build import CACHE, BuildError, build
from cfabric_browser.translations.catalog import TRANSLATIONS, Translation

logger = logging.getLogger(__name__)

PENDING = "~/.cfabric/translations/remove.json"


def repo_of(translation: Translation, root: str | Path) -> Path:
    return Path(root).expanduser() / translation.org / translation.repo


def is_built(translation: Translation, root: str | Path) -> bool:
    """Whether the translation has been built under `root` (by this builder)."""
    repo = repo_of(translation, root)
    data = repo / "tf" / translation.version
    return (data / "otype.tf").is_file() and _is_ours(repo, translation)


def _is_ours(repo: Path, translation: Translation) -> bool:
    config = repo / "app" / "config.yaml"
    if not config.is_file():
        return False
    try:
        own = read_app_config(config).get("translation") or {}
    except Exception:  # noqa: BLE001 - not readable, so not ours to delete
        return False
    return own.get("id") == translation.id


def _pending(path: str | Path) -> list[str]:
    try:
        ids = json.loads(Path(path).expanduser().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return []
    return [i for i in ids if isinstance(i, str)] if isinstance(ids, list) else []


def _write_pending(path: str | Path, ids: list[str]) -> None:
    target = Path(path).expanduser()
    if ids:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(sorted(set(ids))), encoding="utf-8")
    elif target.exists():
        target.unlink()


def is_pending(translation: Translation, pending: str | Path = PENDING) -> bool:
    return translation.id in _pending(pending)


def remove(translation: Translation, root: str | Path) -> bool:
    """Delete a built translation; False if there is none (made by the builder)."""
    repo = repo_of(translation, root)
    if not _is_ours(repo, translation):
        return False
    shutil.rmtree(repo)
    return True


def schedule_removal(translation: Translation, pending: str | Path = PENDING) -> None:
    _write_pending(pending, [*_pending(pending), translation.id])


def cancel_removal(translation: Translation, pending: str | Path = PENDING) -> None:
    _write_pending(pending, [i for i in _pending(pending) if i != translation.id])


def remove_pending(root: str | Path, pending: str | Path = PENDING) -> list[str]:
    """Remove the translations whose removal waited for a restart; returns their
    ids. Run at startup, before any corpus is opened."""
    done = []
    left = []
    for tid in _pending(pending):
        translation = TRANSLATIONS.get(tid)
        if translation is None:
            continue
        try:
            remove(translation, root)
            done.append(tid)
        except OSError as e:
            logger.warning("Could not remove %s: %s", tid, e)
            left.append(tid)
    _write_pending(pending, left)
    return done


class Jobs(BackgroundJobs):
    """Builds running in the background, one at a time per translation, with
    their progress for the library to show."""

    def __init__(
        self,
        root: str | Path,
        cache: str | Path = CACHE,
        pending: str | Path = PENDING,
        **build_options: Any,
    ) -> None:
        super().__init__(expected=(BuildError,))
        self.root = root
        self.cache = cache
        self.pending = pending  # removals waiting for a restart
        self.build_options = build_options  # e.g. download=, for tests

    def start(self, translation: Translation) -> dict[str, Any]:
        return self.run(
            translation.id,
            lambda progress: build(
                translation,
                root=self.root,
                cache=self.cache,
                progress=progress,
                **self.build_options,
            ),
        )
