"""
# Guidance for searching

*   User guide to search: `tf.about.searchusage`
*   Search API: `tf.search.search`.
*   Implementation of search: `tf.about.searchdesign`
"""

from cfabric.search.cancel import CancelToken, SearchCancelled, SearchTimeout

__all__ = ["CancelToken", "SearchCancelled", "SearchTimeout"]
