"""Rich result types for Context Fabric API.

These classes wrap raw node IDs with contextual information,
making them suitable for serialization and agent interaction via MCP.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass, field
from typing import Any

# Maximum slots for text extraction - nodes larger than this skip textification
MAX_TEXT_SLOTS = 100


@dataclass
class NodeInfo:
    """Rich representation of a single node.

    Attributes:
        node: The integer node ID
        otype: The node type (e.g., 'word', 'verse', 'chapter')
        text: Text representation of the node
        section_ref: Human-readable section reference (e.g., 'Genesis 1:1')
        slots: Tuple of slot node IDs this node spans (for non-slot nodes)
        features: Dict of feature values for this node (optional, populated on demand)
    """

    node: int
    otype: str
    text: str = ""
    section_ref: str = ""
    slots: tuple[int, ...] | None = None
    features: dict[str, str | int] | None = None

    def to_dict(self) -> dict[str, Any]:
        """Convert to JSON-serializable dictionary."""
        result: dict[str, Any] = {"node": self.node, "otype": self.otype, "text": self.text}
        if self.section_ref:
            result["section_ref"] = self.section_ref
        if self.slots:
            result["slots"] = list(self.slots)
        if self.features:
            result["features"] = self.features
        return result

    @staticmethod
    def _format_section_ref(section_tuple: tuple, section_types: tuple) -> str:
        """Format section tuple as human-readable reference.

        Examples:
            ('Genesis', 1, 1) -> 'Genesis 1:1'
            ('Genesis', 1) -> 'Genesis 1'
            ('Genesis',) -> 'Genesis'
        """
        if not section_tuple:
            return ""

        parts = [str(val) for val in section_tuple if val is not None]

        if len(parts) >= 3:
            # Book Chapter:Verse format
            return f"{parts[0]} {parts[1]}:{parts[2]}"
        elif len(parts) == 2:
            # Book Chapter format
            return f"{parts[0]} {parts[1]}"
        elif len(parts) == 1:
            return parts[0]
        return ""

    def to_json(self) -> str:
        """Serialize to JSON string."""
        return json.dumps(self.to_dict())

    @classmethod
    def from_api(
        cls,
        api: Any,
        node: int,
        include_text: bool = True,
        include_section: bool = True,
        include_slots: bool = False,
        include_features: list[str] | None = None,
    ) -> NodeInfo:
        """Create NodeInfo from API and node ID.

        Parameters
        ----------
        api: Api
            The Context Fabric API object
        node: int
            The node ID
        include_text: bool
            Whether to include text representation
        include_section: bool
            Whether to include section reference
        include_slots: bool
            Whether to include slot node IDs
        include_features: list[str] | None
            Feature names to include values for
        """
        F = api.F
        T = api.T
        E = api.E

        otype = F.otype.v(node)

        # Get text representation
        text = ""
        if include_text:
            try:
                # Check slot count to avoid textifying large nodes (books, chapters)
                slot_type = F.otype.slotType
                if otype == slot_type:
                    # Slot nodes always get text (they are the text)
                    text = T.text(node) or ""
                else:
                    # Non-slot nodes: check size first
                    slots = E.oslots.s(node)
                    slot_count = len(slots) if slots else 0
                    if slot_count <= MAX_TEXT_SLOTS:
                        text = T.text(node) or ""
                    else:
                        text = f"[{slot_count} slots - text omitted]"
            except Exception:
                text = ""

        # Get section reference
        section_ref = ""
        if include_section:
            try:
                section_tuple = T.sectionFromNode(node)
                if section_tuple:
                    section_ref = cls._format_section_ref(section_tuple, T.sectionTypes)
            except Exception:
                section_ref = ""

        # Get slots
        slots = None
        if include_slots:
            try:
                slot_type = F.otype.slotType
                if otype != slot_type:
                    raw_slots = E.oslots.s(node)
                    slots = tuple(int(s) for s in raw_slots) if raw_slots else None
            except Exception:
                slots = None

        # Get feature values
        features = None
        if include_features:
            features = {}
            for fname in include_features:
                fobj = api.Fs(fname, warn=False)
                if fobj:
                    val = fobj.v(node)
                    if val is not None:
                        features[fname] = val

        return cls(
            node=int(node),  # Convert numpy types to Python int
            otype=otype,
            text=text,
            section_ref=section_ref,
            slots=slots,
            features=features,
        )


@dataclass
class FeatureInfo:
    """Metadata about a feature.

    Attributes:
        name: Feature name
        kind: 'node' or 'edge'
        value_type: 'str', 'int', or '' for edges without values
        description: Feature description from metadata
        has_values: For edge features, whether edges have values
    """

    name: str
    kind: str
    value_type: str = ""
    description: str = ""
    has_values: bool | None = None

    def to_dict(self) -> dict[str, Any]:
        """Convert to JSON-serializable dictionary."""
        result: dict[str, Any] = {
            "name": self.name,
            "kind": self.kind,
            "value_type": self.value_type,
            "description": self.description,
        }
        if self.has_values is not None:
            result["has_values"] = self.has_values
        return result

    def to_json(self) -> str:
        """Serialize to JSON string."""
        return json.dumps(self.to_dict())

    @classmethod
    def from_api(
        cls,
        api: Any,
        name: str,
        kind: str,
    ) -> FeatureInfo | None:
        """Create FeatureInfo from API.

        Parameters
        ----------
        api: Api
            The Context Fabric API object
        name: str
            Feature name
        kind: str
            'node' or 'edge'
        """
        if kind == "node":
            fobj = api.Fs(name, warn=False)
        elif kind == "edge":
            fobj = api.Es(name, warn=False)
        else:
            fobj = None
        if not fobj:
            return None

        has_values = None
        if kind == "edge":
            has_values = bool(fobj.doValues)

        return cls(
            name=name,
            kind=kind,
            value_type=getattr(fobj, "valueType", None) or "",
            description=getattr(fobj, "description", None) or "",
            has_values=has_values,
        )


@dataclass
class CorpusInfo:
    """Information about a loaded corpus.

    Attributes:
        name: Corpus name (derived from path or user-specified)
        path: Path to the corpus
        node_types: List of node types with counts and ranges
        node_features: List of node feature names
        edge_features: List of edge feature names
        slot_type: The slot type name
        max_slot: Maximum slot node ID
        max_node: Maximum node ID
        section_types: Section type hierarchy (e.g., ['book', 'chapter', 'verse'])
    """

    name: str
    path: str
    node_types: list[dict[str, Any]] = field(default_factory=list)
    node_features: list[str] = field(default_factory=list)
    edge_features: list[str] = field(default_factory=list)
    slot_type: str = ""
    max_slot: int = 0
    max_node: int = 0
    section_types: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        """Convert to JSON-serializable dictionary."""
        return asdict(self)

    def to_json(self) -> str:
        """Serialize to JSON string."""
        return json.dumps(self.to_dict())

    @classmethod
    def from_api(cls, api: Any, name: str, path: str) -> CorpusInfo:
        """Create CorpusInfo from API.

        Parameters
        ----------
        api: Api
            The Context Fabric API object
        name: str
            Corpus name
        path: str
            Path to the corpus
        """
        F = api.F
        T = api.T
        C = api.C

        # Get node types with counts
        node_types = []
        for level_info in C.levels.data:
            ntype, avg_slots, min_node, max_node = level_info
            count = int(max_node) - int(min_node) + 1  # Convert numpy types
            node_types.append(
                {
                    "type": ntype,
                    "count": count,
                    "avg_slots": float(avg_slots),
                    "min_node": int(min_node),
                    "max_node": int(max_node),
                }
            )

        # Get features (excluding warp features)
        node_features = api.Fall(warp=False)
        edge_features = api.Eall(warp=False)

        # Get section types
        section_types = []
        try:
            section_types = list(T.sectionTypes)
        except Exception:
            pass

        return cls(
            name=name,
            path=path,
            node_types=node_types,
            node_features=node_features,
            edge_features=edge_features,
            slot_type=F.otype.slotType,
            max_slot=int(F.otype.maxSlot),
            max_node=int(F.otype.maxNode),
            section_types=section_types,
        )


__all__ = [
    "NodeInfo",
    "NodeList",
    "SearchResult",
    "FeatureInfo",
    "CorpusInfo",
]
