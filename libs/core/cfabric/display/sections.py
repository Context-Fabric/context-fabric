"""Human-readable section and structure references.

Section references are built from the section levels of the corpus, joined by
the separators of its display configuration: `Genesis 1:1` for the BHSA,
`P005381 1:1` for a cuneiform tablet, `S1 1` for a corpus with two levels.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from cfabric.core.api import Api
    from cfabric.display.config import DisplayConfig


def section_ref(
    api: Api,
    node: int,
    sep1: str = " ",
    sep2: str = ":",
    lang: str = "en",
    last_slot: bool = False,
    fillup: bool = False,
    level: int | None = None,
) -> str:
    """The section reference of a node, e.g. `Genesis 1:1`.

    Parameters
    ----------
    api: Api
        The loaded corpus.
    node: int
        Any node; its first (or last) slot determines the section.
    sep1, sep2: str
        Separators between section levels 0 and 1, and 1 and 2.
    lang: str
        Language of section names that have translations (e.g. book names).
    last_slot: bool
        Take the section of the last slot instead of the first.
    fillup: bool
        See `cfabric.navigation.text.Text.sectionFromNode`.
    level: int | None
        Restrict to this section level.

    Returns
    -------
    str
        The reference, or the empty string if the corpus has no sections.
    """
    T = api.T
    if not T.sectionTypes:
        return ""
    section = T.sectionFromNode(
        int(node), lang=lang, lastSlot=last_slot, fillup=fillup, level=level
    )
    if not section:
        return ""
    seps = ("", sep1, sep2)
    return "".join(
        "" if part is None else f"{seps[i]}{part}" for (i, part) in enumerate(section)
    )


def section_ref_for(
    api: Api, config: DisplayConfig, node: int, lang: str = "en", **kwargs
) -> str:
    """`section_ref` with the separators of a display configuration."""
    return section_ref(
        api,
        node,
        sep1=config.section_sep1,
        sep2=config.section_sep2,
        lang=lang,
        **kwargs,
    )


def node_from_section_ref(
    api: Api,
    ref: str,
    sep1: str = " ",
    sep2: str = ":",
    lang: str = "en",
) -> int:
    """The section node for a reference like `Genesis 1:1`, `Genesis 1` or `Genesis`.

    Section names may contain `sep1` themselves (e.g. `Song of Songs 1:1`):
    the reference is split at the last occurrence.

    Raises
    ------
    ValueError
        If the reference is malformed or does not exist in the corpus.
    """
    T = api.T
    sectionTypes = T.sectionTypes
    if not sectionTypes:
        raise ValueError("This corpus has no sections")

    ref = ref.strip()
    if not ref:
        raise ValueError("Empty section reference")

    # Candidate readings: split into levels at the last `sep1`, and, as a
    # fallback, the whole reference as a level-0 name (which may contain `sep1`).
    candidates: list[list[str]] = []
    if len(sectionTypes) > 1:
        head, sep, tail = ref.rpartition(sep1)
        if sep and head:
            sub = tail.split(sep2) if len(sectionTypes) > 2 else [tail]
            candidates.append([head, *sub])
    candidates.append([ref])

    error = f'Not a valid passage: "{ref}"'
    dataTypes = T.sectionFeatureTypes
    for parts in candidates:
        if len(parts) > len(sectionTypes):
            continue
        typed: list[str | int] = []
        for i, part in enumerate(parts):
            if dataTypes[i] == "int":
                try:
                    typed.append(int(part))
                except ValueError:
                    error = f'{sectionTypes[i]} "{part}" is not a number'
                    break
            else:
                typed.append(part)
        else:
            node = T.nodeFromSection(tuple(typed), lang=lang)
            if node is not None:
                return int(node)
    raise ValueError(error)


def structure_ref(api: Api, node: int) -> str:
    """The structure heading of a node, e.g. `chapter 3, section 2`."""
    T = api.T
    if not getattr(T, "structureTypes", None):
        return ""
    heading = T.headingFromNode(int(node))
    if not heading:
        return ""
    return ", ".join(f"{part[0]} {part[1]}" for part in heading if part is not None)
