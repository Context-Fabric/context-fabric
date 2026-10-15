"""Downloads a corpus from the Hugging Face Hub."""

from __future__ import annotations

from pathlib import Path

from cfabric.downloader.registry import _resolve_corpus_id


def download(
    corpus_id: str,
    *,
    revision: str | None = None,
    force: bool = False,
) -> Path:
    """Download a corpus from the Hugging Face Hub.

    Args:
        corpus_id: A short name from the registry (see `list_corpora`) or a
            full Hugging Face dataset repo ID (e.g. ``'etcbc/cfabric-bhsa'``).
        revision: Version to download (tag, branch, or commit hash); the
            latest when ``None``.
        force: Download again even when the corpus is already cached.

    Returns:
        The local directory of the downloaded corpus (in the Hugging Face cache).

    Raises:
        ValueError: If `corpus_id` is neither a registered name nor a repo ID.
        ImportError: If the ``download`` extra (``huggingface_hub``) is not installed.

    Example:
        >>> import cfabric
        >>> path = cfabric.download('researcher/cfabric-my-corpus')
        >>> api = cfabric.Fabric(locations=path).loadAll()
        >>> path = cfabric.download('researcher/cfabric-my-corpus', revision='v2023.1')
    """
    repo_id = _resolve_corpus_id(corpus_id)
    try:
        from huggingface_hub import snapshot_download
    except ImportError as error:
        raise ImportError(
            "cfabric.download needs the huggingface_hub package; install it with "
            "pip install 'context-fabric[download]'"
        ) from error

    local_path = snapshot_download(
        repo_id=repo_id,
        repo_type="dataset",
        revision=revision,
        force_download=force,
    )
    return Path(local_path)
