"""
# Mappings from nodes to values.

Every node feature is logically a mapping from nodes to values,
string or integer.

A feature object gives you methods that you can pass a node and that returns
its value for that node.

It is easiest to think of all node features as a dictionary keyed by nodes.

However, some features have an optimised representation, and do not have
a dictionary underneath.

But you can still iterate over the data of a feature as if it were a
dictionary: `tf.core.nodefeature.NodeFeature.items`
"""

import collections

from .strings import StringPool, IntFeatureArray


class NodeFeatures:
    pass


class NodeFeature:
    """Provides access to (node) feature data.

    For feature `fff` it is the result of `F.fff` or `Fs('fff')`.

    Supports both legacy dict-based storage and new mmap-based backends
    (StringPool for string features, IntFeatureArray for int features).
    """

    def __init__(self, api, metaData, data):
        self.api = api
        self.meta = metaData
        """Metadata of the feature.

        This is the information found in the lines starting with `@`
        in the `.tf` feature file.
        """

        self._data = data
        self._is_mmap = isinstance(data, (StringPool, IntFeatureArray))
        self._cached_data = None  # Cache for materialized dict

    @property
    def data(self):
        """Get data as dict (for backward compatibility).

        Note: For mmap backends, this materializes the data into memory.
        Use v() for efficient single lookups.

        Returns
        -------
        dict
            The feature data as a dictionary mapping nodes to values.
        """
        if self._is_mmap:
            return self._materialize()
        return self._data

    def _materialize(self):
        """Convert mmap storage to dict (cached).

        Returns
        -------
        dict
            Dictionary mapping nodes to their values.
        """
        if self._cached_data is not None:
            return self._cached_data

        # Use efficient items() from StringPool/IntFeatureArray
        # which uses numpy vectorized operations to find non-missing values
        self._cached_data = dict(self._data.items())
        return self._cached_data

    def items(self):
        """A generator that yields the items of the feature, seen as a mapping.

        It does not yield entries for nodes without values,
        so this gives you a rather efficient way to iterate over
        just the feature data, instead of over all nodes.

        If you need this repeatedly, or you need the whole dictionary,
        you can store the result as follows:

           data = dict(F.fff.items())

        """
        # Both dict and mmap backends (StringPool/IntFeatureArray) have items()
        return self._data.items()

    def v(self, n):
        """Get the value of a feature for a node.

        Parameters
        ----------
        n: integer
            The node in question

        Returns
        -------
        integer | string | None
            The value of the feature for that node, if it is defined, else `None`.
        """
        if self._is_mmap:
            return self._data.get(n)

        if n in self._data:
            return self._data[n]
        return None

    def s(self, val):
        """Query all nodes having a specified feature value.

        This is an other way to walk through nodes than using
        `tf.core.nodes.Nodes.walk`.

        Parameters
        ----------
        value: integer | string
            The feature value that all resulting nodes have.

        Returns
        -------
        tuple of integer
            All nodes that have this value for this feature,
            sorted in the canonical order.
            (`tf.core.nodes`)
        """

        Crank = self.api.C.rank.data

        if self._is_mmap:
            # For mmap, we need to scan all nodes
            matches = []
            max_node = len(self._data)
            for n in range(1, max_node + 1):
                if self.v(n) == val:
                    matches.append(n)
            return tuple(sorted(matches, key=lambda n: Crank[n - 1]))
        else:
            return tuple(
                sorted(
                    [n for n in self._data if self._data[n] == val],
                    key=lambda n: Crank[n - 1],
                )
            )

    def freqList(self, nodeTypes=None):
        """Frequency list of the values of this feature.

        Inspect the values of this feature and see how often they occur.

        Parameters
        ----------
        nodeTypes: set of string, optional None
            If you pass a set of node types, only the values for nodes
            within those types will be counted.

        Returns
        -------
        tuple of 2-tuple
            A tuple of `(value, frequency)`, items, ordered by `frequency`,
            highest frequencies first.

        """

        fql = collections.Counter()
        fOtype = self.api.F.otype.v if nodeTypes else None

        if self._is_mmap:
            max_node = len(self._data)
            for n in range(1, max_node + 1):
                val = self.v(n)
                if val is not None:
                    if nodeTypes is None or fOtype(n) in nodeTypes:
                        fql[val] += 1
        else:
            if nodeTypes is None:
                for n in self._data:
                    fql[self._data[n]] += 1
            else:
                for n in self._data:
                    if fOtype(n) in nodeTypes:
                        fql[self._data[n]] += 1

        return tuple(sorted(fql.items(), key=lambda x: (-x[1], x[0])))
