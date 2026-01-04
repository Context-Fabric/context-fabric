"""
String pool management for string-valued features.

This module provides efficient storage for string-valued features using
integer indices into a shared string pool. This approach minimizes memory
usage when many nodes share the same string values.
"""

import numpy as np
from typing import Optional

# Sentinel for missing string index
MISSING_STR_INDEX = 0xFFFFFFFF
NODE_DTYPE = 'uint32'


class StringPool:
    """
    Efficient string storage with integer indices.

    Uses numpy object arrays which support copy-on-write sharing.

    Attributes
    ----------
    strings : np.ndarray
        Array of unique strings (dtype=object)
    indices : np.ndarray
        Per-node index into strings array (dtype=uint32)
        MISSING_STR_INDEX indicates no value
    """

    def __init__(self, strings: np.ndarray, indices: np.ndarray):
        """
        Initialize a StringPool.

        Parameters
        ----------
        strings : np.ndarray
            Array of unique strings (dtype=object)
        indices : np.ndarray
            Per-node index into strings array (dtype=uint32)
        """
        self.strings = strings
        self.indices = indices

    def get(self, node: int) -> Optional[str]:
        """
        Get string value for node (1-indexed).

        Parameters
        ----------
        node : int
            Node number (1-indexed)

        Returns
        -------
        Optional[str]
            String value or None if missing
        """
        idx = self.indices[node - 1]
        if idx == MISSING_STR_INDEX:
            return None
        return self.strings[idx]

    def __getitem__(self, node: int) -> Optional[str]:
        """
        Get string value for node using bracket notation.

        Parameters
        ----------
        node : int
            Node number (1-indexed)

        Returns
        -------
        Optional[str]
            String value or None if missing
        """
        return self.get(node)

    def __len__(self) -> int:
        """
        Number of nodes tracked.

        Returns
        -------
        int
            Length of indices array
        """
        return len(self.indices)

    def items(self):
        """
        Iterate over (node, value) pairs efficiently using numpy.

        Only yields nodes that have values (skips MISSING entries).

        Yields
        ------
        tuple
            (node, string_value) pairs
        """
        # Use numpy to find all nodes with values (vectorized, fast)
        mask = self.indices != MISSING_STR_INDEX
        valid_indices = np.where(mask)[0]

        for idx in valid_indices:
            node = idx + 1  # Convert 0-indexed to 1-indexed
            string_idx = self.indices[idx]
            yield (node, self.strings[string_idx])

    def to_dict(self) -> dict:
        """
        Convert to dict efficiently.

        Returns
        -------
        dict
            Mapping from node to string value
        """
        return dict(self.items())

    @classmethod
    def from_dict(cls, data: dict, max_node: int) -> 'StringPool':
        """
        Build string pool from node->string dict.

        Parameters
        ----------
        data : dict
            Mapping from node (int) to string value
        max_node : int
            Maximum node number in corpus

        Returns
        -------
        StringPool
            New StringPool instance
        """
        # Build unique string list
        unique_strings = sorted(set(data.values()))
        string_to_idx = {s: i for i, s in enumerate(unique_strings)}

        # Build index array
        indices = np.full(max_node, MISSING_STR_INDEX, dtype=NODE_DTYPE)
        for node, value in data.items():
            indices[node - 1] = string_to_idx[value]

        strings = np.array(unique_strings, dtype=object)
        return cls(strings, indices)

    def save(self, path_prefix: str) -> None:
        """
        Save to {path_prefix}_strings.npy and {path_prefix}_idx.npy.

        Parameters
        ----------
        path_prefix : str
            Path prefix for output files
        """
        np.save(f"{path_prefix}_strings.npy", self.strings, allow_pickle=True)
        np.save(f"{path_prefix}_idx.npy", self.indices)

    @classmethod
    def load(cls, path_prefix: str, mmap_mode: str = 'r') -> 'StringPool':
        """
        Load from files.

        Parameters
        ----------
        path_prefix : str
            Path prefix for input files
        mmap_mode : str, optional
            Memory-map mode for indices array (default: 'r')

        Returns
        -------
        StringPool
            Loaded StringPool instance
        """
        # Note: object arrays can't be mmap'd, but they're typically small
        strings = np.load(f"{path_prefix}_strings.npy", allow_pickle=True)
        indices = np.load(f"{path_prefix}_idx.npy", mmap_mode=mmap_mode)
        return cls(strings, indices)


class IntFeatureArray:
    """
    Integer feature storage.

    Dense array with sentinel for missing values.

    Attributes
    ----------
    values : np.ndarray
        Array of integer values (dtype=int32)
        MISSING (-1) indicates no value
    """

    MISSING = -1

    def __init__(self, values: np.ndarray):
        """
        Initialize an IntFeatureArray.

        Parameters
        ----------
        values : np.ndarray
            Array of integer values (dtype=int32)
        """
        self.values = values

    def get(self, node: int) -> Optional[int]:
        """
        Get int value for node (1-indexed).

        Parameters
        ----------
        node : int
            Node number (1-indexed)

        Returns
        -------
        Optional[int]
            Integer value or None if missing
        """
        val = self.values[node - 1]
        if val == self.MISSING:
            return None
        return int(val)

    def __getitem__(self, node: int) -> Optional[int]:
        """
        Get int value for node using bracket notation.

        Parameters
        ----------
        node : int
            Node number (1-indexed)

        Returns
        -------
        Optional[int]
            Integer value or None if missing
        """
        return self.get(node)

    def __len__(self) -> int:
        """
        Number of nodes tracked.

        Returns
        -------
        int
            Length of values array
        """
        return len(self.values)

    def items(self):
        """
        Iterate over (node, value) pairs efficiently using numpy.

        Only yields nodes that have values (skips MISSING entries).

        Yields
        ------
        tuple
            (node, int_value) pairs
        """
        # Use numpy to find all nodes with values (vectorized, fast)
        mask = self.values != self.MISSING
        valid_indices = np.where(mask)[0]

        for idx in valid_indices:
            node = idx + 1  # Convert 0-indexed to 1-indexed
            yield (node, int(self.values[idx]))

    def to_dict(self) -> dict:
        """
        Convert to dict efficiently.

        Returns
        -------
        dict
            Mapping from node to int value
        """
        return dict(self.items())

    @classmethod
    def from_dict(cls, data: dict, max_node: int) -> 'IntFeatureArray':
        """
        Build from node->int dict.

        Parameters
        ----------
        data : dict
            Mapping from node (int) to integer value (or None for missing)
        max_node : int
            Maximum node number in corpus

        Returns
        -------
        IntFeatureArray
            New IntFeatureArray instance
        """
        values = np.full(max_node, cls.MISSING, dtype='int32')
        for node, value in data.items():
            # None values stay as MISSING sentinel
            if value is not None:
                values[node - 1] = value
        return cls(values)

    def save(self, path: str) -> None:
        """
        Save to .npy file.

        Parameters
        ----------
        path : str
            Output file path
        """
        np.save(path, self.values)

    @classmethod
    def load(cls, path: str, mmap_mode: str = 'r') -> 'IntFeatureArray':
        """
        Load from .npy file.

        Parameters
        ----------
        path : str
            Input file path
        mmap_mode : str, optional
            Memory-map mode (default: 'r')

        Returns
        -------
        IntFeatureArray
            Loaded IntFeatureArray instance
        """
        values = np.load(path, mmap_mode=mmap_mode)
        return cls(values)
