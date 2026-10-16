"""Building the BDB dictionary file (`bdb.json.gz`), once, for publishing.

The steps, each of which stops the build with a `BuildError` when a source is
not what is expected:

1.  `fetch_sources`: Open Scriptures' `HebrewLexicon` (BDB's ids and short
    definitions, the lexical index, the augmented Strong's index) and the
    OSHB's books, at pinned commits, the lexicon files by checksum.
2.  Sefaria's two lexicons, walked entry by entry (`sefaria.walk`; from its
    cache after the first time), and its list of abbreviations.
3.  `join` Sefaria's entries to Open Scriptures' ids, then the joins of
    `joins.yaml`; `clean` their HTML; references to other entries become
    their ids.
4.  Link the corpus' lexemes: align its words with the OSHB per verse, count
    the BDB entries their lemmas lead to; where another entry cites the
    lexeme's verses clearly better (`by_citations`), take that one; then
    apply `corrections.yaml`.
5.  `check`, and write the file with its metadata.
"""

from __future__ import annotations

import datetime
import gzip
import hashlib
import html
import json
import re
from collections import Counter
from collections.abc import Callable
from importlib import resources
from pathlib import Path
from typing import Any

import yaml

from cfabric_browser import __version__
from cfabric_browser.books import canonical_book
from cfabric_browser.dictionaries import bdb, link, sefaria

CACHE = "~/.cfabric/dictionaries/cache"
BHSA = "~/text-fabric-data/github/ETCBC/bhsa/tf/2021"

LEXICON_COMMIT = "21c9add13bc727d3a951361778e97e3ff7afd1ce"  # 2019-09-02
LEXICON_URL = (
    "https://raw.githubusercontent.com/openscriptures/HebrewLexicon/"
    f"{LEXICON_COMMIT}/{{name}}"
)
LEXICON_FILES = {
    "BrownDriverBriggs.xml": (
        "2b52658a4323d91674cda4090ab8b3ebddfff640f4f18143c28300e80b2c38f8"
    ),
    "LexicalIndex.xml": (
        "8f7a605c58899d2f44430149c143c00903976e1e91232476677972a69e5bc85f"
    ),
    "AugIndex.xml": "e7217ca8ff8ff3f21f9cf1bbe87411adf55f6aa88bcf5ed9ddc886cc6b160c5d",
}
OSHB_COMMIT = "3d15126fb1ef74867fc1434be1942e837932691f"  # 2024-08-27, WLC 4.20
OSHB_URL = (
    "https://raw.githubusercontent.com/openscriptures/morphhb/"
    f"{OSHB_COMMIT}/wlc/{{name}}.xml"
)
# the books in OSIS ids, as the OSHB names its files
OSHB_BOOKS = (
    "Gen",
    "Exod",
    "Lev",
    "Num",
    "Deut",
    "Josh",
    "Judg",
    "Ruth",
    "1Sam",
    "2Sam",
    "1Kgs",
    "2Kgs",
    "1Chr",
    "2Chr",
    "Ezra",
    "Neh",
    "Esth",
    "Job",
    "Ps",
    "Prov",
    "Eccl",
    "Song",
    "Isa",
    "Jer",
    "Lam",
    "Ezek",
    "Dan",
    "Hos",
    "Joel",
    "Amos",
    "Obad",
    "Jonah",
    "Mic",
    "Nah",
    "Hab",
    "Zeph",
    "Hag",
    "Zech",
    "Mal",
)

# a lexeme links to the entries that at least this share of its aligned
# occurrences lead to (the first one is the main entry)
SHARE = 0.1
# lexemes this frequent must be linked to an entry with text
FREQUENT = 10
# 22 of Sefaria's entries stop after the gender, as `[אֶ֫שֶׁר, or אָשָׁר] n.[m.`
CUT = re.compile(r"\[(?:m|f)\.\??(?: or f\.)?$")


def _cut(raw: str) -> bool:
    """Whether Sefaria's text of an entry stops short."""
    return CUT.search(re.sub(r"<[^>]+>", "", raw).strip()) is not None


# another entry replaces the linked one when it cites at least this many of
# the lexeme's verses (all of them if it has fewer) and scores at least this
# many times as high
CITED = 2
BETTER = 2.0

Progress = Callable[[str, int, int], None]


class BuildError(Exception):
    """A source is not what the builder expects; the message says where."""


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fetch_sources(
    cache: str | Path = CACHE, download: sefaria.Download = sefaria._download
) -> tuple[dict[str, Path], dict[str, str]]:
    """The local paths of the Open Scriptures files, and the checksums of the
    OSHB's books (for the metadata)."""
    folder = Path(cache).expanduser() / "openscriptures"
    paths: dict[str, Path] = {}
    sums: dict[str, str] = {}
    wanted = [
        (name, LEXICON_URL.format(name=name), LEXICON_COMMIT, sha)
        for (name, sha) in LEXICON_FILES.items()
    ] + [(f"{b}.xml", OSHB_URL.format(name=b), OSHB_COMMIT, None) for b in OSHB_BOOKS]
    for name, url, commit, sha in wanted:
        target = folder / commit[:7] / name
        data = target.read_bytes() if target.is_file() else None
        if data is None or (sha is not None and _sha256(data) != sha):
            try:
                data = download(url)
            except OSError as e:
                raise BuildError(f"Could not download {url}: {e}") from e
            if sha is not None and _sha256(data) != sha:
                raise BuildError(f"{url} has another checksum than expected")
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        paths[name] = target
        if sha is None:
            sums[name] = _sha256(data)
    return paths, sums


def _package_yaml(name: str) -> dict[Any, Any]:
    text = (
        resources.files("cfabric_browser.dictionaries")
        .joinpath(name)
        .read_text(encoding="utf-8")
    )
    return yaml.safe_load(text) or {}


def corrections() -> dict[str, list[str]]:
    """Links made by hand: `<language> <lex>` → entry ids (the first is the
    main entry), from `corrections.yaml`."""
    data = _package_yaml("corrections.yaml")
    return {str(k): [str(i) for i in v["entries"]] for (k, v) in data.items()}


def joins() -> dict[str, str]:
    """Joins made by hand: Sefaria's reference (`BDB, אֶלְקֹשִׁי`) → Open
    Scriptures' id, from `joins.yaml`."""
    return {str(k): str(v) for (k, v) in _package_yaml("joins.yaml").items()}


def abbreviations(client: sefaria.Client) -> dict[str, str]:
    """BDB's abbreviations (`AE`: `Aben Ezra`), from Sefaria."""
    text = client.get(sefaria.text_url("BDB", "Abbrevations"))
    lines = text.get("text", []) if isinstance(text, dict) else []
    found = {}
    for line in lines:
        plainLine = re.sub(r"<[^>]+>", "", line).strip()
        m = re.fullmatch(r"(\S.{0,30}?) = (.+)", plainLine)
        if m:
            found[m.group(1).strip()] = m.group(2).strip()
    if len(found) < 400:
        raise BuildError(f"Only {len(found)} abbreviations from Sefaria")
    return found


def entries(
    client: sefaria.Client,
    ours: list[bdb.Entry],
    index: dict[str, link.IndexEntry],
    progress: Progress | None = None,
    byHand: dict[str, str] | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """BDB's entries with Sefaria's text joined to Open Scriptures' ids, in
    BDB's order (Hebrew, then Aramaic), and figures about the join. `byHand`
    (Sefaria's reference → id) replaces what the join did with either side:
    for entries it leaves out, and for neighbours with the same consonants
    (a homonym, a root, a cross reference) that it takes for each other."""
    byHand = joins() if byHand is None else byHand
    if len(set(byHand.values())) < len(byHand):
        raise BuildError("joins.yaml: an id is joined twice")
    numbers = _numbers(index)

    result: list[dict[str, Any]] = []
    figures: dict[str, Any] = {}
    for lexicon, book in sefaria.LEXICONS.items():
        aramaic = book != "BDB"

        def report(n: int, headword: str, step: str = f"sefaria {book}") -> None:
            if progress is not None:
                progress(step, n, 0)

        theirs = sefaria.walk(client, lexicon, report)
        mine = [e for e in ours if e.aramaic == aramaic]
        pairs = bdb.join(
            [e.cons for e in mine],
            [bdb.headword_consonants(e["headword"]) for e in theirs],
        )
        byTheirs = {j: mine[i] for (i, j) in pairs.items()}
        joined = len(pairs)
        refs = {f"{book}, {e['headword']}": j for (j, e) in enumerate(theirs)}
        positions = {e.id: i for (i, e) in enumerate(mine)}
        for ref, oid in byHand.items():
            if not ref.startswith(f"{book}, "):
                continue
            j, i = refs.get(ref), positions.get(oid)
            if j is None or i is None:
                raise BuildError(f"joins.yaml: no entry {ref if j is None else oid}")
            if j in byTheirs:
                del pairs[positions[byTheirs.pop(j).id]]
            if i in pairs:
                del byTheirs[pairs.pop(i)]
            byTheirs[j] = mine[i]
            pairs[i] = j
        incomplete = [f"{book}, {e['headword']}" for e in theirs if _cut(e["html"])]
        figures[book] = {
            "sefaria": len(theirs),
            "openscriptures": len(mine),
            "joined": joined,
            "joined by hand": sum(1 for r in byHand if r.startswith(f"{book}, ")),
            "incomplete at Sefaria": incomplete,
        }
        for j, e in enumerate(theirs):
            try:
                cleaned = bdb.clean(e["html"])
            except bdb.CleanError as x:
                raise BuildError(f"{book}, {e['headword']}: {x}") from x
            os = byTheirs.get(j)
            strong, twot = numbers.get(os.id, (set(), set())) if os else (set(), set())
            result.append(
                {
                    "id": os.id if os else f"{'x' if aramaic else ''}s.{j}",
                    "lexicon": "arc" if aramaic else "heb",
                    "headword": e["headword"],
                    "ref": f"{book}, {e['headword']}",
                    "html": cleaned.html,
                    "incomplete": bool(_cut(e["html"])),
                    "refs": len(cleaned.refs),
                    "unknownRefs": cleaned.unknown,
                    "pos": os.pos if os else None,
                    "defs": os.defs if os else [],
                    "root": os.root if os else False,
                    "section": os.section if os else None,
                    "strong": sorted(strong, key=_number),
                    "twot": sorted(twot),
                }
            )
    # references to other entries: by Sefaria's reference to its id
    ids = {e["ref"]: e["id"] for e in result}
    unresolved = 0
    for entry in result:

        def resolve(m: re.Match[str]) -> str:
            nonlocal unresolved
            target = ids.get(html.unescape(m.group(1)))
            if target is None:
                unresolved += 1
                return "<a>"
            return f'<a data-entry="{target}">'

        entry["html"] = re.sub(r'<a data-entry="([^"]*)">', resolve, entry["html"])
    figures["unresolved cross references"] = unresolved
    return result, figures


def _numbers(
    index: dict[str, link.IndexEntry],
) -> dict[str, tuple[set[str], set[str]]]:
    """Per entry id its Strong's and TWOT numbers."""
    numbers: dict[str, tuple[set[str], set[str]]] = {}
    for ix in index.values():
        if ix.bdb:
            strongs, twots = numbers.setdefault(ix.bdb, (set(), set()))
            if ix.strong:
                strongs.add(ix.strong)
            if ix.twot:
                twots.add(ix.twot)
    return numbers


def missing(
    ours: list[bdb.Entry],
    entries: list[dict[str, Any]],
    linked: dict[str, dict[str, Any]],
    index: dict[str, link.IndexEntry],
) -> list[dict[str, Any]]:
    """Entries that lexemes link to and Sefaria lacks (its text has gaps):
    Open Scriptures' headword, part of speech and short definitions, without
    text."""
    have = {e["id"] for e in entries}
    wanted = {i for v in linked.values() for i in v["entries"]} - have
    numbers = _numbers(index)
    result = []
    for os in ours:
        if os.id in wanted:
            strong, twot = numbers.get(os.id, (set(), set()))
            result.append(
                {
                    "id": os.id,
                    "lexicon": "arc" if os.aramaic else "heb",
                    "headword": os.headword,
                    "ref": None,
                    "html": "",
                    "incomplete": True,
                    "refs": 0,
                    "unknownRefs": [],
                    "pos": os.pos,
                    "defs": os.defs,
                    "root": os.root,
                    "section": os.section,
                    "strong": sorted(strong, key=_number),
                    "twot": sorted(twot),
                }
            )
    unknown = wanted - {e["id"] for e in result}
    if unknown:
        raise BuildError(f"links to entries that don't exist: {sorted(unknown)}")
    return result


def resolved(
    fixed: dict[str, list[str]], entries: list[dict[str, Any]]
) -> dict[str, list[str]]:
    """Corrections with Sefaria's references (`BDB, כְּרוּב`, for entries
    without an Open Scriptures id) replaced by the entries' ids."""
    ids = {e["ref"]: e["id"] for e in entries}
    result = {}
    for key, targets in fixed.items():
        result[key] = []
        for t in targets:
            if ", " in t and t not in ids:
                raise BuildError(f"corrections.yaml: no entry {t}")
            result[key].append(ids.get(t, t))
    return result


def _number(strong: str) -> tuple[int, str]:
    digits = re.match(r"\d+", strong)
    return (int(digits.group()) if digits else 0, strong)


def is_cross_reference(html_: str) -> bool:
    """Whether an entry only refers to another (`יסך v. סוּךְ`): short, with
    `v.`, BDB's "see"."""
    text = re.sub(r"<[^>]+>", "", html_)
    return len(text) < 150 and re.search(r"\bv\.\s", text) is not None


def is_root(html_: str) -> bool:
    """Whether an entry is a root that only introduces the words after it
    (`קעקע (√ of foll.; …)`)."""
    return "√ of foll" in re.sub(r"<[^>]+>", "", html_)[:80]


def is_verb(entry: dict[str, Any]) -> bool:
    """Whether an entry is a verb: by Open Scriptures' part of speech, else
    by `vb.` at the start of Sefaria's text."""
    if entry.get("pos"):
        return str(entry["pos"]).startswith("vb")
    return re.search(r"\bvb\.", re.sub(r"<[^>]+>", "", entry["html"])[:80]) is not None


def cited_verses(entries: list[dict[str, Any]]) -> dict[str, set[str]]:
    """Per entry the verses its text cites (OSIS, as `Gen.3.6`)."""
    return {
        e["id"]: {
            r
            for r in re.findall(r'data-osis="([^"]+)"', e["html"])
            if r.count(".") == 2
        }
        for e in entries
    }


def citation_score(cited: set[str], verses: set[str]) -> tuple[float, int]:
    """How well an entry's cited verses fit a lexeme's verses: the share of
    the cited verses with the lexeme times the share of the lexeme's verses
    cited (a root entry cites hundreds of verses, so it shouldn't win by
    chance), and the number of verses both have."""
    hits = len(cited & verses)
    if not hits:
        return (0.0, 0)
    return (hits / len(cited) * hits / len(verses), hits)


def by_citations(
    current: list[str],
    candidates: list[str],
    verses: set[str],
    cited: dict[str, set[str]],
) -> list[str] | None:
    """The entries of a lexeme when a candidate other than the linked entry
    cites its verses clearly better (at least `CITED` of them, or all if the
    lexeme has fewer, and `BETTER` times the score), else None. The chosen entry comes first; of the others
    only those that cite one of its verses stay."""
    main = current[0]
    mine = citation_score(cited.get(main, set()), verses)[0]
    best, bestScore = None, (0.0, 0)
    for c in candidates:
        if c == main:
            continue
        score = citation_score(cited.get(c, set()), verses)
        if score > bestScore:
            best, bestScore = c, score
    enough = min(CITED, len(verses))
    if best is None or bestScore[1] < enough or bestScore[0] < BETTER * mine:
        return None
    rest = [
        c
        for c in current
        if c != best and citation_score(cited.get(c, set()), verses)[1]
    ]
    return [best, *rest]


def links(
    api: Any,
    verses: dict[tuple[str, int, int], list[link.Segment]],
    aug: dict[str, str],
    index: dict[str, link.IndexEntry],
    fixed: dict[str, list[str]],
    entries: list[dict[str, Any]] | None = None,
) -> dict[str, dict[str, Any]]:
    """Per lexeme (`<language> <lex>`) the BDB entries it links to, with how
    the link was made: `oshb`, `cited` (another entry cites its verses
    better, `by_citations`; among the entries its lemmas lead to and those
    with its consonants, in its language, verbs for verbs and other words
    for other words) or `manual`."""
    F, L, T = api.F, api.L, api.T
    counts = link.lemma_counts(api, verses)
    cited = cited_verses(entries or [])
    language = {e["id"]: e["lexicon"] for e in entries or []}
    verb = {e["id"]: is_verb(e) for e in entries or []}
    # an entry that only refers to another or introduces a root may stay,
    # but doesn't replace another
    minor = {
        e["id"]
        for e in entries or []
        if is_cross_reference(e["html"]) or is_root(e["html"])
    }
    byConsonants: dict[tuple[str, str], list[str]] = {}
    for e in entries or []:
        cons = bdb.headword_consonants(e["headword"])
        byConsonants.setdefault((e["lexicon"], cons), []).append(e["id"])
    result: dict[str, dict[str, Any]] = {}
    for node in F.otype.s("lex"):
        node = int(node)
        key = f"{F.language.v(node)} {F.lex.v(node)}"
        found = link.entry_counts(counts.get(node, Counter()), aug, index)
        total = sum(n for (_, n) in found)
        if key in fixed:
            result[key] = {"entries": fixed[key], "how": "manual"}
            continue
        if not found:
            continue
        result[key] = {
            "entries": [e for (e, n) in found if n >= SHARE * total],
            "share": round(found[0][1] / total, 3),
            "how": "oshb",
        }
        if not cited:
            continue
        lang = "arc" if F.language.v(node) == "Aramaic" else "heb"
        cons = link.etcbc_consonants(F.lex.v(node) or "")
        candidates = [
            c
            for c in [e for (e, _) in found] + byConsonants.get((lang, cons), [])
            if language.get(c) == lang
            and verb[c] == (F.sp.v(node) == "verb")
            and c not in minor
        ]
        lexVerses = set()
        for w in L.d(node, "word"):
            book, chapter, verse = T.sectionFromNode(w, lang="en")
            lexVerses.add(f"{canonical_book(book)}.{chapter}.{verse}")
        better = by_citations(result[key]["entries"], candidates, lexVerses, cited)
        if better is not None:
            result[key]["was"] = result[key]["entries"][0]
            result[key]["entries"] = better
            result[key]["how"] = "cited"
    return result


def check(
    api: Any,
    entries: list[dict[str, Any]],
    linked: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    """Stop the build if a frequent lexeme has no entry with text; figures
    for the metadata."""
    F = api.F
    withText = {e["id"] for e in entries if e["html"]}
    missing = []
    unlinked = []
    for node in F.otype.s("lex"):
        node = int(node)
        key = f"{F.language.v(node)} {F.lex.v(node)}"
        freq = int(F.freq_lex.v(node) or 0)
        found = linked.get(key)
        if found is None:
            unlinked.append(key)
            if freq >= FREQUENT:
                missing.append(f"{key} (not linked, {freq}×)")
        elif found["entries"][0] not in withText and freq >= FREQUENT:
            missing.append(f"{key} ({found['entries'][0]} has no text, {freq}×)")
    if missing:
        raise BuildError(
            f"{len(missing)} frequent lexemes without an entry: "
            + ", ".join(missing[:20])
        )
    textless = sorted(
        {i for v in linked.values() for i in v["entries"] if i not in withText}
    )
    return {
        "lexemes": len(F.otype.s("lex")),
        "linked": len(linked),
        "linked by citations": sum(1 for v in linked.values() if v["how"] == "cited"),
        "unlinked": unlinked,
        "entries without text that lexemes link to": textless,
        "references": sum(e["refs"] for e in entries),
        "references not understood": sum(len(e["unknownRefs"]) for e in entries),
    }


def build(
    out: str | Path,
    corpus: str | Path = BHSA,
    cache: str | Path = CACHE,
    client: sefaria.Client | None = None,
    progress: Progress | None = None,
) -> dict[str, Any]:
    """Build the file; gives its metadata."""
    from cfabric import Fabric

    paths, sums = fetch_sources(cache)
    if client is None:
        client = sefaria.Client(Path(cache).expanduser() / "sefaria")
    api = Fabric(locations=str(Path(corpus).expanduser()), silent="deep").load(
        "g_cons lex language freq_lex sp book@en", silent="deep"
    )
    if not api:
        raise BuildError(f"Could not load the corpus in {corpus}")
    index = link.read_lexical_index(paths["LexicalIndex.xml"])
    aug = link.read_aug_index(paths["AugIndex.xml"])
    ours = bdb.read_entries(paths["BrownDriverBriggs.xml"])
    verses: dict[tuple[str, int, int], list[link.Segment]] = {}
    for b in OSHB_BOOKS:
        verses.update(link.read_oshb(paths[f"{b}.xml"]))

    allEntries, figures = entries(client, ours, index, progress)
    linked = links(
        api, verses, aug, index, resolved(corrections(), allEntries), allEntries
    )
    allEntries += missing(ours, allEntries, linked, index)
    figures.update(check(api, allEntries, linked))
    for e in allEntries:
        del e["unknownRefs"]

    meta = {
        "title": "A Hebrew and English Lexicon of the Old Testament",
        "short": "BDB",
        "authors": "Francis Brown, S. R. Driver, Charles A. Briggs",
        "edition": "Oxford, 1906",
        "licence": "Public domain",
        "sources": [
            {
                "what": "the text of the entries",
                "name": "Sefaria, BDB Dictionary and BDB Aramaic Dictionary",
                "url": "https://www.sefaria.org/BDB",
                "licence": "Public Domain (Sefaria's version metadata), "
                "from a scan in the National Library of Israel",
            },
            {
                "what": "ids, parts of speech and short definitions of the "
                "entries; Strong's and TWOT numbers",
                "name": "Open Scriptures, HebrewLexicon",
                "url": "https://github.com/openscriptures/HebrewLexicon",
                "commit": LEXICON_COMMIT,
                "licence": "CC BY 4.0, Open Scriptures Hebrew Bible Project",
                "checksums": LEXICON_FILES,
            },
            {
                "what": "the lemmas that link the corpus' words to the entries",
                "name": "Open Scriptures Hebrew Bible (morphhb)",
                "url": "https://github.com/openscriptures/morphhb",
                "commit": OSHB_COMMIT,
                "licence": "CC BY 4.0",
                "checksums": sums,
            },
            {
                "what": "the lexemes linked",
                "name": "BHSA (ETCBC)",
                "path": str(corpus),
                "licence": "CC BY-NC 4.0",
            },
        ],
        "built": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
        "builder": f"cfabric-browser {__version__}",
        "figures": figures,
    }
    data = {
        "meta": meta,
        "entries": allEntries,
        "links": linked,
        "abbreviations": abbreviations(client),
    }
    target = Path(out).expanduser()
    target.parent.mkdir(parents=True, exist_ok=True)
    raw = json.dumps(data, ensure_ascii=False, separators=(",", ":")).encode()
    target.write_bytes(gzip.compress(raw, mtime=0))
    return meta


def main() -> None:
    """Build `bdb.json.gz` (run by hand, for publishing)."""
    import argparse
    import sys

    parser = argparse.ArgumentParser(description=main.__doc__)
    parser.add_argument("out")
    parser.add_argument("--corpus", default=BHSA)
    parser.add_argument("--cache", default=CACHE)
    args = parser.parse_args()

    def report(step: str, n: int, total: int) -> None:
        if n % 500 == 0:
            print(f"{step}: {n}", file=sys.stderr, flush=True)

    meta = build(args.out, args.corpus, args.cache, progress=report)
    print(json.dumps(meta["figures"], ensure_ascii=False, indent=1)[:4000])


if __name__ == "__main__":
    main()
