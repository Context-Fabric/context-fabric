"""Corpus downloads from the Hugging Face Hub.

Requires the ``download`` extra (``pip install 'context-fabric[download]'``)::

    >>> import cfabric
    >>> path = cfabric.download("researcher/cfabric-my-corpus")
    >>> api = cfabric.Fabric(locations=path).loadAll()
"""

from cfabric.downloader.download import download
from cfabric.downloader.registry import CORPUS_REGISTRY, list_corpora

__all__ = ["download", "list_corpora", "CORPUS_REGISTRY"]
