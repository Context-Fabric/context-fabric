"""MCP tool implementations for Context-Fabric.

Tools are functions that agents can execute to interact with corpora.
"""

from __future__ import annotations

import logging
import re
from collections import defaultdict
from typing import Any

from cfabric.results import NodeInfo, FeatureInfo, CorpusInfo
from cfabric.describe import (
    describe_corpus_overview,
    describe_feature as core_describe_feature,
    describe_text_formats,
    list_features as core_list_features,
)

from cfabric_mcp.cache import get_cache
from cfabric_mcp.corpus_manager import corpus_manager

logger = logging.getLogger("cfabric_mcp.tools")

# Cache for text formats (never expires - otext metadata is immutable)
_text_formats_cache: dict[str, dict[str, Any]] = {}

# Max limits for token management
MAX_SEARCH_LIMIT = 100  # Max results per page for search
MAX_PASSAGES_LIMIT = 100  # Max sections per get_passages call

# Transport mode - set by server at startup
_transport: str = "stdio"


def set_transport(transport: str) -> None:
    """Set the transport mode (called by server at startup)."""
    global _transport
    _transport = transport


# ============================================================================
# Corpus Management Tools
# ============================================================================


def list_loaded_corpora() -> dict[str, Any]:
    """List all currently loaded corpora.

    Returns:
        Dictionary with 'corpora' list and 'current' corpus name.
    """
    corpora = corpus_manager.list_corpora()
    logger.debug("list_corpora: %d corpora loaded", len(corpora))
    return {
        "corpora": corpora,
        "current": corpus_manager.current,
    }


def get_corpus_info(corpus: str | None = None) -> dict[str, Any]:
    """Get detailed information about a loaded corpus.

    Args:
        corpus: Corpus name (defaults to current corpus)

    Returns:
        Detailed corpus information including node types, features, and section structure.
    """
    api = corpus_manager.get_api(corpus)
    name = corpus or corpus_manager.current
    CF, _ = corpus_manager.get(corpus)
    path = CF.locations[-1] if CF.locations else ""
    logger.debug("corpus_info: fetching info for '%s'", name)
    return CorpusInfo.from_api(api, name or "", str(path)).to_dict()


# ============================================================================
# Search Tools
# ============================================================================


SEARCH_SYNTAX_SECTIONS = {
    "basics": """## Basic Syntax

### Node Patterns
```
node_type                    # Match any node of this type
node_type feature=value      # Match with exact feature value
node_type feature~regex      # Match with regex pattern
node_type feature#regex      # Match with case-insensitive regex
node_type feature<value      # Less than (numeric)
node_type feature>value      # Greater than (numeric)
```

### Multiple Conditions
```
word sp=verb tense=past      # AND: both conditions must match
```

### Variables (Capturing)
```
word sp=verb                 # Anonymous match
w:word sp=verb               # Named match (capture as 'w')
```""",

    "structure": """## Structure (Indentation)

Indentation defines containment:
```
clause                       # Find a clause
  phrase function=subject    # containing a subject phrase
    word sp=noun             # containing a noun
```""",

    "relations": """## Relations

### Default Relations
- Indented items are contained by their parent (`:` relation)
- Items at same level follow each other in order

### Explicit Relations
```
clause
  word sp=verb
  < word sp=noun             # noun comes BEFORE verb
  > word sp=adj              # adjective comes AFTER verb
  <: word sp=prep            # preposition immediately before verb
  :> word sp=adv             # adverb immediately after verb
```

### Relation Operators
- `<` - comes before (canonical node ordering)
- `>` - comes after (canonical node ordering)
- `<:` - immediately before (adjacent)
- `:>` - immediately after (adjacent)
- `<<` - completely before (slot ordering)
- `>>` - completely after (slot ordering)
- `[[` - left embeds right
- `]]` - left embedded in right
- `=:` - start at same slot
- `:=` - end at same slot
- `::` - start and end at same slot (co-extensive)
- `==` - occupy same slots""",

    "quantifiers": """## Quantifiers

Quantifiers filter parent nodes based on what they contain. All blocks end with `/-/`.

### /without/ - Exclusion
Find nodes that do NOT contain a matching pattern:
```
phrase
/without/
  word sp=verb
/-/
```
Returns phrases that don't contain any verbs.

### /where/ + /have/ - Required Conditions (AND)
Find nodes containing ALL specified patterns:
```
clause
/where/
  word sp=verb
/have/
  word sp=subs
/-/
```
Returns clauses containing both a verb AND a noun.

### /with/ + /or/ - Alternatives (OR)
Find nodes matching ANY of the alternative patterns:
```
phrase
/with/
  word sp=verb
/or/
  word sp=nmpr
/-/
```
Returns phrases containing a verb OR a proper name.

### Rules
- Quantifier keywords (`/where/`, `/have/`, etc.) must be on their own line
- Templates inside quantifiers follow normal search syntax
- All quantifier blocks must terminate with `/-/`
""",

    "examples": """## Examples

### Find all verbs:
```
word sp=verb
```

### Find verbs with their objects:
```
clause
  phrase function=predicate
    word sp=verb
  phrase function=object
```

### Find adjacent words:
```
word
<: word                      # Two adjacent words
```

### Find a word and its containing clause:
```
clause
  word lex=king
```

### Find clauses without verbs:
```
clause
/without/
  word sp=verb
/-/
```""",
}


def search(
    template: str,
    return_type: str = "results",
    aggregate_features: list[str] | None = None,
    group_by_section: bool = False,
    top_n: int = 50,
    limit: int = 100,
    max_override: bool = False,
    corpus: str | None = None,
) -> dict[str, Any]:
    """Search for patterns in the corpus.

    Args:
        template: Search template (see search_syntax_guide for syntax)
        return_type: What to return - "results", "count", "statistics", or "passages"
        aggregate_features: For statistics: which features to aggregate (default: all low-cardinality)
        group_by_section: For statistics: include distribution by section (book/chapter)
        top_n: For statistics: max values per feature distribution (default 50)
        limit: For results/passages: page size (default 100)
        max_override: Bypass limit cap. May produce large responses - use judiciously.
        corpus: Corpus name (defaults to current corpus)

    Returns:
        Search results formatted according to return_type.
    """
    # Enforce max limit for token management (unless overridden)
    if not max_override:
        limit = min(limit, MAX_SEARCH_LIMIT)

    template_preview = template.strip().replace("\n", " ")[:80]
    logger.info("search: executing query (return_type=%s): %s...", return_type, template_preview)

    corpus_name = corpus or corpus_manager.current or ""
    api = corpus_manager.get_api(corpus)
    S = api.S

    # First, validate the template
    try:
        S.study(template)
    except Exception as e:
        logger.error("search: template study failed: %s", e)
        return {"error": f"Invalid search template: {e}", "template": template}

    exe = S.exe
    if exe and not exe.good:
        # Return detailed errors from both syntax and semantics checks
        errors = []
        for ln, msg in getattr(exe, "badSyntax", []):
            errors.append(f"Line {ln}: {msg}" if ln is not None else msg)
        for ln, msg in getattr(exe, "badSemantics", []):
            errors.append(f"Line {ln}: {msg}" if ln is not None else msg)
        return {
            "error": "Invalid search template",
            "errors": errors,
            "template": template,
        }

    # Use cache for search execution
    cache = get_cache()

    def execute_search() -> list[tuple[int, ...]]:
        try:
            results = S.search(template)
            if results is None:
                return []
            if isinstance(results, tuple):
                return list(results)
            return list(results)
        except Exception as e:
            logger.error("search: execution failed: %s", e)
            return []

    cached = cache.get_or_execute(corpus_name, template, execute_search)
    results = cached.results  # Already sorted by cache
    total_count = len(results)

    if total_count == 0:
        # Return appropriate empty response for each return_type
        if return_type == "count":
            return {"total_count": 0, "template": template}
        elif return_type == "statistics":
            return {"total_count": 0, "template": template, "nodes": {}}
        elif return_type == "passages":
            return {"total_count": 0, "template": template, "passages": [], "has_more": False}
        else:  # "results"
            return {"total_count": 0, "template": template, "results": []}

    # Handle different return types
    if return_type == "count":
        return {"total_count": total_count, "template": template}

    elif return_type == "statistics":
        return _format_statistics(
            api, results, template, aggregate_features, group_by_section, top_n
        )

    elif return_type == "passages":
        return _format_passages(api, results, template, limit)

    else:  # "results" (default)
        return _format_results(api, results, template, cached.cursor_id, limit)


def _format_results(
    api: Any,
    results: list[tuple[int, ...]],
    template: str,
    cursor_id: str,
    limit: int,
) -> dict[str, Any]:
    """Format search results as paginated node info."""
    total_count = len(results)
    page = results[:limit]
    has_more = len(results) > limit

    # Convert to NodeInfo
    result_list = []
    for tup in page:
        result_list.append([NodeInfo.from_api(api, n).to_dict() for n in tup])

    cache = get_cache()
    cached = cache.get_by_cursor(cursor_id)
    expires_at = cached.expires_at if cached else None

    return {
        "results": result_list,
        "total_count": total_count,
        "template": template,
        "cursor": {
            "id": cursor_id,
            "offset": 0,
            "limit": limit,
            "has_more": has_more,
            "expires_at": expires_at,
        },
    }


def _format_statistics(
    api: Any,
    results: list[tuple[int, ...]],
    template: str,
    aggregate_features: list[str] | None,
    group_by_section: bool,
    top_n: int,
) -> dict[str, Any]:
    """Format search results as statistics with feature distributions."""
    total_count = len(results)
    F = api.F
    T = api.T

    # Determine which features to aggregate
    if aggregate_features is None:
        # Auto-select: all node features (caller can filter)
        aggregate_features = list(api.Fall(warp=False))

    # Group results by atom position and node type
    # Each result tuple has nodes at different positions
    if not results:
        return {"total_count": 0, "template": template, "nodes": {}}

    # Analyze first result to get structure
    first_result = results[0]
    num_atoms = len(first_result)

    # Build per-atom statistics
    nodes_stats: dict[str, dict[str, Any]] = {}

    for atom_idx in range(num_atoms):
        # Collect all nodes at this position
        atom_nodes = [r[atom_idx] for r in results if atom_idx < len(r)]
        if not atom_nodes:
            continue

        # Get node type from first node
        first_node = atom_nodes[0]
        node_type = F.otype.v(first_node)

        # Build atom key (use template line or type)
        atom_key = f"{atom_idx}_{node_type}"

        # Aggregate features for these nodes
        distributions: dict[str, list[tuple[Any, int]]] = {}

        for feat_name in aggregate_features:
            fobj = api.Fs(feat_name, warn=False)
            if not fobj:
                continue

            # Count feature values
            value_counts: dict[Any, int] = defaultdict(int)
            for node in atom_nodes:
                val = fobj.v(node)
                if val is not None:
                    value_counts[val] += 1

            if value_counts:
                # Sort by frequency, take top_n
                sorted_vals = sorted(
                    value_counts.items(), key=lambda x: -x[1]
                )[:top_n]
                distributions[feat_name] = sorted_vals

        nodes_stats[atom_key] = {
            "type": node_type,
            "count": len(atom_nodes),
            "distributions": {
                k: [{"value": v, "count": c} for v, c in vals]
                for k, vals in distributions.items()
            },
        }

    result: dict[str, Any] = {
        "total_count": total_count,
        "template": template,
        "nodes": nodes_stats,
    }

    # Add section distribution if requested
    if group_by_section:
        section_counts: dict[str, int] = defaultdict(int)
        for r in results:
            if r:
                try:
                    section_tuple = T.sectionFromNode(r[0])
                    if section_tuple and len(section_tuple) > 0:
                        book = str(section_tuple[0])
                        section_counts[book] += 1
                except Exception:
                    pass

        result["section_distribution"] = {
            "book": [
                {"value": v, "count": c}
                for v, c in sorted(section_counts.items(), key=lambda x: -x[1])[:top_n]
            ]
        }

    return result


def _format_passages(
    api: Any,
    results: list[tuple[int, ...]],
    template: str,
    limit: int,
) -> dict[str, Any]:
    """Format search results as readable passages."""
    total_count = len(results)

    passages = []
    for r in results[:limit]:
        if not r:
            continue
        node = r[0]
        info = NodeInfo.from_api(api, node)
        passages.append({
            "reference": info.section_ref,
            "text": info.text,
            "node": info.node,
            "type": info.otype,
        })

    return {
        "total_count": total_count,
        "template": template,
        "passages": passages,
        "has_more": total_count > limit,
    }


def search_continue(
    cursor_id: str,
    offset: int = 0,
    limit: int = 100,
    max_override: bool = False,
) -> dict[str, Any]:
    """Continue a paginated search using a cursor ID.

    Args:
        cursor_id: The cursor ID from a previous search
        offset: Number of results to skip
        limit: Maximum number of results to return (default 100)
        max_override: Bypass limit cap. May produce large responses - use judiciously.

    Returns:
        Next page of search results.
    """
    # Enforce max limit for token management (unless overridden)
    if not max_override:
        limit = min(limit, MAX_SEARCH_LIMIT)

    cache = get_cache()
    result = cache.get_page(cursor_id, offset, limit)

    if result is None:
        return {
            "error": "Cursor not found or expired",
            "cursor_id": cursor_id,
        }

    page_results, has_more, total_count = result

    # Get the cached entry to access the corpus/api
    cached = cache.get_by_cursor(cursor_id)
    if not cached:
        return {
            "error": "Cursor not found or expired",
            "cursor_id": cursor_id,
        }

    # Get API for formatting
    api = corpus_manager.get_api(cached.corpus or None)

    # Format results
    result_list = []
    for tup in page_results:
        result_list.append([NodeInfo.from_api(api, n).to_dict() for n in tup])

    return {
        "results": result_list,
        "total_count": total_count,
        "template": cached.template,
        "cursor": {
            "id": cursor_id,
            "offset": offset,
            "limit": limit,
            "has_more": has_more,
            "expires_at": cached.expires_at,
        },
    }


def search_csv(
    template: str,
    file_path: str,
    limit: int = 10000,
    delimiter: str = ",",
    corpus: str | None = None,
) -> dict[str, Any]:
    """Export search results to a CSV file.

    Args:
        template: Search template (same syntax as search())
        file_path: Absolute path to write CSV file
        limit: Max rows to export (default 10000)
        delimiter: Field separator (default ",", use "\\t" for TSV)
        corpus: Corpus name (defaults to current)

    Returns:
        Dictionary with file_path, total_count, and rows_written.
    """
    import csv

    # Check transport - file writes only work with stdio
    if _transport != "stdio":
        return {
            "error": "This tool writes files to the server's local filesystem and is not "
            "available over HTTP/SSE transport. Use search() with a limit parameter instead."
        }

    template_preview = template.strip().replace("\n", " ")[:80]
    logger.info("search_csv: exporting to %s (limit=%d): %s...", file_path, limit, template_preview)

    corpus_name = corpus or corpus_manager.current or ""
    api = corpus_manager.get_api(corpus)
    S = api.S

    # Validate template
    try:
        S.study(template)
    except Exception as e:
        logger.error("search_csv: template study failed: %s", e)
        return {"error": f"Invalid search template: {e}", "template": template}

    exe = S.exe
    if exe and not exe.good:
        errors = []
        for ln, msg in getattr(exe, "badSyntax", []):
            errors.append(f"Line {ln}: {msg}" if ln is not None else msg)
        for ln, msg in getattr(exe, "badSemantics", []):
            errors.append(f"Line {ln}: {msg}" if ln is not None else msg)
        return {"error": "Invalid search template", "errors": errors, "template": template}

    # Execute search (uses cache)
    cache = get_cache()

    def execute_search() -> list[tuple[int, ...]]:
        try:
            results = S.search(template)
            if results is None:
                return []
            if isinstance(results, tuple):
                return list(results)
            return list(results)
        except Exception as e:
            logger.error("search_csv: execution failed: %s", e)
            return []

    cached = cache.get_or_execute(corpus_name, template, execute_search)
    results = cached.results
    total_count = len(results)

    if total_count == 0:
        # Write empty file
        with open(file_path, "w", newline="") as f:
            pass
        return {"file_path": file_path, "total_count": 0, "rows_written": 0}

    # Determine columns from first result
    num_nodes = len(results[0])
    base_fields = ["node", "otype", "text", "section_ref"]
    header = []
    for i in range(num_nodes):
        for field in base_fields:
            header.append(f"node{i}_{field}")

    # Write CSV
    rows_written = 0
    with open(file_path, "w", newline="") as f:
        writer = csv.writer(f, delimiter=delimiter)
        writer.writerow(header)

        for tup in results[:limit]:
            row = []
            for node in tup:
                info = NodeInfo.from_api(api, node)
                row.extend([info.node, info.otype, info.text, info.section_ref])
            writer.writerow(row)
            rows_written += 1

    logger.info("search_csv: wrote %d rows to %s", rows_written, file_path)
    return {
        "file_path": file_path,
        "total_count": total_count,
        "rows_written": rows_written,
    }


def search_syntax_guide(section: str | None = None) -> dict[str, Any]:
    """Get documentation on search template syntax.

    By default returns a summary with available sections.
    Call with section='relations' (or basics/structure/quantifiers/examples)
    to get detailed info on a specific topic.

    Args:
        section: Specific section to retrieve (basics, structure, relations,
                 quantifiers, examples). Default: returns summary.

    Returns:
        Summary with section list (default) or specific section content.
    """
    available_sections = list(SEARCH_SYNTAX_SECTIONS.keys())

    if section is None:
        # Return summary with section list
        return {
            "summary": "Templates: node_type feature=value. Indentation=containment. Relations: < > <: >: for ordering.",
            "sections": available_sections,
            "hint": "Call with section='relations' to get detailed info on a specific section",
        }

    if section not in SEARCH_SYNTAX_SECTIONS:
        return {
            "error": f"Unknown section: '{section}'",
            "available_sections": available_sections,
        }

    return {
        "section": section,
        "content": SEARCH_SYNTAX_SECTIONS[section],
    }


# ============================================================================
# Corpus Discovery Tools
# ============================================================================


def describe_corpus(corpus: str | None = None) -> dict[str, Any]:
    """Get corpus structure overview.

    Returns node types, section structure, and feature counts.
    Use list_features() to browse features, describe_feature() for details,
    and get_text_formats() for text encoding samples.

    Args:
        corpus: Corpus name (defaults to current corpus)

    Returns:
        Corpus overview with node types, sections, and feature counts.
    """
    api = corpus_manager.get_api(corpus)
    name = corpus or corpus_manager.current
    overview = describe_corpus_overview(api, name or "")
    return overview.to_dict()


def list_features(
    kind: str = "all",
    node_types: list[str] | None = None,
    corpus: str | None = None,
) -> dict[str, Any]:
    """List all features with optional filtering.

    Returns a lightweight catalog of features for discovery.
    Use node_types to filter by object type. For full details including
    sample values, use describe_feature().

    Args:
        kind: Filter by feature kind - "all", "node", or "edge"
        node_types: Filter to features that apply to these object types (e.g., ["word"])
        corpus: Corpus name (defaults to current corpus)

    Returns:
        List of features with name, kind, value_type, and description.
    """
    api = corpus_manager.get_api(corpus)
    entries = core_list_features(api, kind, node_types)
    result: dict[str, Any] = {"features": [e.to_dict() for e in entries]}
    if node_types:
        result["node_types"] = node_types
    return result


def get_text_formats(corpus: str | None = None) -> dict[str, Any]:
    """Get text format samples showing how text is encoded in this corpus.

    Returns original script and transliteration pairs with diverse samples.
    Use this when you need to understand text encoding for search queries.
    Results are cached per corpus.

    Args:
        corpus: Corpus name (defaults to current corpus)

    Returns:
        Text format information with samples.
    """
    corpus_name = corpus or corpus_manager.current or ""

    # Check cache
    if corpus_name in _text_formats_cache:
        return _text_formats_cache[corpus_name]

    api = corpus_manager.get_api(corpus)
    text_info = describe_text_formats(api)
    result = text_info.to_dict()

    # Cache the result
    _text_formats_cache[corpus_name] = result
    return result


def describe_features(
    features: str | list[str],
    sample_limit: int = 20,
    corpus: str | None = None,
) -> dict[str, Any]:
    """Get detailed info about one or more features.

    Returns sample values (by frequency) and statistics for each feature.
    Use after describe_corpus() to explore specific features.

    Args:
        features: Feature name or list of feature names (e.g., "sp" or ["sp", "vt", "vs"])
        sample_limit: Max sample values to return per feature (default 20)
        corpus: Corpus name (defaults to current corpus)

    Returns:
        For single feature: Feature details dict
        For multiple features: Dict keyed by feature name with details for each
    """
    # Normalize to list
    if isinstance(features, str):
        feature_list = [features]
        single_mode = True
    else:
        feature_list = features
        single_mode = False

    api = corpus_manager.get_api(corpus)
    results = {}

    for feature in feature_list:
        desc = core_describe_feature(api, feature, sample_limit)
        results[feature] = desc.to_dict()

    # Return single result directly for backward compatibility
    if single_mode:
        return results[feature_list[0]]

    return {"features": results}


# Backward compatibility alias
describe_feature = describe_features


def get_passages(
    sections: list[list[str | int]],
    limit: int = 50,
    lang: str = "en",
    max_override: bool = False,
    corpus: str | None = None,
) -> dict[str, Any]:
    """Get passages by section references.

    Args:
        sections: List of section references, e.g., [['Genesis', 1, 1], ['Exodus', 2, 3]]
        limit: Maximum sections to return (default 50)
        lang: ISO 639 language code for section names (e.g., 'en' for English book names)
        max_override: Bypass limit cap. May produce large responses - use judiciously.
        corpus: Corpus name (defaults to current corpus)

    Returns:
        List of passages with text and node information.
    """
    # Enforce max limit for token management (unless overridden)
    if not max_override:
        limit = min(limit, MAX_PASSAGES_LIMIT)
    sections = sections[:limit]

    api = corpus_manager.get_api(corpus)
    T = api.T

    passages = []
    for section in sections:
        try:
            node = T.nodeFromSection(tuple(section), lang=lang)
            if node is not None:
                info = NodeInfo.from_api(api, node)
                passages.append(info.to_dict())
            else:
                passages.append({
                    "error": f"Section not found: {section}",
                    "section": section,
                })
        except Exception as e:
            passages.append({
                "error": f"Failed to find section: {e}",
                "section": section,
            })

    return {
        "passages": passages,
        "total": len(passages),
        "found": len([p for p in passages if "error" not in p]),
    }


def get_node_features(
    nodes: list[int],
    features: list[str],
    corpus: str | None = None,
) -> dict[str, Any]:
    """Get feature values for a list of nodes.

    Args:
        nodes: List of node IDs
        features: List of feature names to retrieve
        corpus: Corpus name (defaults to current corpus)

    Returns:
        Feature values for each node.
    """
    api = corpus_manager.get_api(corpus)
    F = api.F

    results = []
    for node in nodes:
        node_data: dict[str, Any] = {
            "node": int(node),
            "type": F.otype.v(node),
        }

        for fname in features:
            fobj = api.Fs(fname, warn=False)
            if fobj:
                val = fobj.v(node)
                node_data[fname] = val

        results.append(node_data)

    return {
        "nodes": results,
        "features_requested": features,
        "total": len(results),
    }

