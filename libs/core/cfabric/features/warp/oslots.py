"""
# Access to `oslots` feature data.

In general, features are stored as dictionaries, but this specific feature
has an optimised representation. Since it is a large feature and present
in any CF dataset, this pays off.

Supports two backends:
- Dict-based tuple format (.tf loading): data = (slots_tuple, maxSlot, maxNode)
- CSR array format (.cfm loading): data = CSRArray instance

"""

from __future__ import annotations

from collections.abc import Iterator
from typing import TYPE_CHECKING

from cfabric.storage.csr import CSRArray

if TYPE_CHECKING:
    from cfabric.core.api import Api


class OslotsFeature:
    def __init__(
        self,
        api: Api,
        metaData: dict[str, str],
        data: tuple[tuple[tuple[int, ...], ...], int, int] | CSRArray,
        maxSlot: int | None = None,
        maxNode: int | None = None,
    ) -> None:
        """Initialize OslotsFeature with either dict-based or CSR backend.

        Parameters
        ----------
        api : object
            The API object
        metaData : dict
            Feature metadata
        data : tuple or CSRArray
            Either:
            - Dict-based (.tf): tuple (slots_tuple, maxSlot, maxNode)
            - CSR: CSRArray instance containing slot data for non-slot nodes
        maxSlot : int, optional
            When using CSR backend, the maximum slot node number.
            Required when data is a CSRArray.
        maxNode : int, optional
            When using CSR backend, the maximum node number.
            Required when data is a CSRArray.
        """
        self.api: Api = api
        self.meta: dict[str, str] = metaData
        """Metadata of the feature.

        This is the information found in the lines starting with `@`
        in the `.tf` feature file.
        """

        # Detect backend type based on data format
        self._is_mmap: bool = isinstance(data, CSRArray)

        self._data: tuple[tuple[int, ...], ...] | CSRArray
        self.maxSlot: int | None
        self.maxNode: int | None

        if self._is_mmap:
            # CSR backend
            self._data = data
            self.maxSlot = maxSlot
            self.maxNode = maxNode
        else:
            # Dict-based tuple format (.tf loading)
            assert isinstance(data, tuple)
            self._data = data[0]
            self.maxSlot = data[1]
            self.maxNode = data[2]

    @property
    def data(self) -> tuple[tuple[int, ...], ...] | CSRArray:
        """Access to raw slots data.

        For dict-based backend (.tf), returns the slots tuple.
        For CSR backend (.cfm), returns the CSRArray.
        """
        return self._data

    def items(self) -> Iterator[tuple[int, tuple[int, ...]]]:
        """A generator that yields the non-slot nodes with their slots."""

        maxSlot = self.maxSlot
        maxNode = self.maxNode

        assert maxSlot is not None
        assert maxNode is not None

        shift = maxSlot + 1

        if self._is_mmap:
            # CSR backend: use get_as_tuple for API compatibility
            data = self._data
            assert isinstance(data, CSRArray)
            for n in range(maxSlot + 1, maxNode + 1):
                yield (n, data.get_as_tuple(n - shift))
        else:
            # Dict-based backend (.tf): direct tuple access
            data = self._data
            for n in range(maxSlot + 1, maxNode + 1):
                yield (n, data[n - shift])

    def s(self, n: int) -> tuple[int, ...]:
        """Get the slots of a (non-slot) node.

        Parameters
        ----------
        node: integer
            The node whose slots must be retrieved.

        Returns
        -------
        tuple
            The slot nodes of the node in question, in canonical order.
            (`cfabric.nodes`)

            For slot nodes `n` it is the tuple `(n,)`.

            All non-slot nodes are linked to at least one slot.
        """

        if n == 0:
            return ()
        if self.maxSlot is not None and n < self.maxSlot + 1:
            return (n,)
        if self.maxSlot is None:
            return ()
        m = n - self.maxSlot
        if m <= len(self._data):
            if self._is_mmap:
                assert isinstance(self._data, CSRArray)
                return self._data.get_as_tuple(m - 1)
            else:
                return self._data[m - 1]
        return ()
