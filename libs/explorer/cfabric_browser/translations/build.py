"""Building a translation into a corpus.

The steps, each of which stops the build with a `BuildError` when the source
is not what is expected:

1.  `fetch`: download the books, or take them from the cache when their
    checksums still match.
2.  `read_book`: check the title and licence the source gives, and get the
    verses per chapter.
3.  `clean`: remove the source's markup. A note in `<small>` (a verse repeated
    at the end of a book, as read in the synagogue) is kept apart; any other
    markup stops the build, so that nothing slips in unseen.
4.  `renumber`: divide verses as the BHSA does (`Split`), and check the
    number of chapters and verses against the BHSA's.
5.  `write_corpus`: words as slots, with `text`, `trailer` and `lower`;
    books, chapters and verses as sections; metadata in every feature and in
    `app/config.yaml`.

The same source gives the same corpus: only the date each feature file was
written (`@dateWritten`) differs between builds.
"""

from __future__ import annotations

import hashlib
import html
import json
import re
import shutil
import ssl
import urllib.parse
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import Any

import yaml

from cfabric_browser import __version__
from cfabric_browser.translations.catalog import Split, Translation

CACHE = "~/.cfabric/translations"
ROOT = "~/text-fabric-data/github"

# called with a step ("download", "build"), the number done and the total
Progress = Callable[[str, int, int], None]
Fetch = Callable[[str], bytes]


class BuildError(Exception):
    """The source is not what the builder expects; the message says where."""


@dataclass
class Verse:
    chapter: int
    verse: int
    text: str
    note: str | None = None
    # the source's own reference, where the BHSA divides differently
    printed: str | None = None


def hebrew_verses() -> dict[str, list[int]]:
    """The number of verses per chapter of each book in the BHSA."""
    data = resources.files("cfabric_browser.translations").joinpath(
        "hebrew_verses.json"
    )
    return json.loads(data.read_text(encoding="utf-8"))


def _ssl_context() -> ssl.SSLContext:
    """The system's certificates, or certifi's where it is installed: the
    standalone program on macOS has none of its own."""
    try:
        import certifi
    except ImportError:
        return ssl.create_default_context()
    return ssl.create_default_context(cafile=certifi.where())


def _download(url: str) -> bytes:
    request = urllib.request.Request(
        url, headers={"User-Agent": f"cfabric-browser/{__version__}"}
    )
    with urllib.request.urlopen(
        request, timeout=60, context=_ssl_context()
    ) as response:
        return response.read()


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fetch(
    translation: Translation,
    cache: str | Path = CACHE,
    progress: Progress | None = None,
    download: Fetch = _download,
) -> dict[str, bytes]:
    """The source file of every book, keyed by the BHSA's book name.

    Files are kept in `<cache>/<id>/` with their checksums in
    `checksums.json`; a file whose checksum still matches is not downloaded
    again.
    """
    folder = Path(cache).expanduser() / translation.id
    folder.mkdir(parents=True, exist_ok=True)
    sums_file = folder / "checksums.json"
    try:
        sums: dict[str, str] = json.loads(sums_file.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        sums = {}

    files: dict[str, bytes] = {}
    total = len(translation.books)
    for i, (name, path) in enumerate(translation.books):
        target = folder / f"{name}.json"
        data: bytes | None = None
        if target.is_file() and name in sums:
            data = target.read_bytes()
            if _sha256(data) != sums[name]:
                data = None
        if data is None:
            url = translation.url.format(path=urllib.parse.quote(path))
            try:
                data = download(url)
            except OSError as e:
                raise BuildError(f"Could not download {name} from {url}: {e}") from e
            target.write_bytes(data)
            sums[name] = _sha256(data)
            sums_file.write_text(json.dumps(sums, indent=1), encoding="utf-8")
        files[name] = data
        if progress is not None:
            progress("download", i + 1, total)
    return files


def read_book(translation: Translation, name: str, data: bytes) -> list[list[str]]:
    """The verses per chapter of a book's source file, after checking that the
    file is the expected version under the expected licence."""
    try:
        source = json.loads(data.decode("utf-8"))
    except ValueError as e:
        raise BuildError(f"{name}: the source is not readable JSON ({e})") from e
    title = source.get("versionTitle")
    if title != translation.source_title:
        raise BuildError(
            f"{name}: the source is '{title}', not '{translation.source_title}'"
        )
    licence = source.get("license")
    if licence != translation.source_licence:
        raise BuildError(
            f"{name}: the source's licence is '{licence}', "
            f"not '{translation.source_licence}'"
        )
    text = source.get("text")
    if not isinstance(text, list) or not all(
        isinstance(ch, list) and all(isinstance(v, str) for v in ch) for ch in text
    ):
        raise BuildError(f"{name}: the source has no chapters with verses")
    return text


NOTE = re.compile(r"\s*<br\s*/?>\s*<small>\s*\[?(.*?)\]?\s*</small>\s*$", re.DOTALL)
TAG = re.compile(r"<[^>]*>")


def clean(ref: str, raw: str) -> tuple[str, str | None]:
    """A verse's text without markup, and its note in `<small>`, if any."""
    note = None
    match = NOTE.search(raw)
    if match:
        note = " ".join(html.unescape(match.group(1)).split())
        raw = raw[: match.start()]
    tag = TAG.search(raw)
    if tag:
        raise BuildError(f"{ref}: unexpected markup {tag.group(0)!r}")
    text = " ".join(html.unescape(raw).split())
    if not text:
        raise BuildError(f"{ref}: the verse is empty")
    return (text, note)


def _cut(ref: str, text: str, before: tuple[str, ...]) -> list[str]:
    parts = []
    rest = text
    for start in before:
        at = rest.find(start)
        if at <= 0:
            raise BuildError(f"{ref}: cannot divide the verse before {start!r}")
        parts.append(rest[:at].rstrip())
        rest = rest[at:]
    parts.append(rest)
    return parts


def renumber(
    book: str,
    chapters: list[list[tuple[str, str | None]]],
    splits: tuple[Split, ...] = (),
) -> list[list[Verse]]:
    """Verses per chapter, divided as in the BHSA.

    `chapters` holds (text, note) per verse in the source's numbering.
    """
    mine = {(s.chapter, s.verse): s for s in splits if s.book == book}
    result: list[list[Verse]] = []
    for c, verses in enumerate(chapters, start=1):
        current: list[Verse] = []
        for v, (text, note) in enumerate(verses, start=1):
            printed = f"{c}:{v}"
            split = mine.pop((c, v), None)
            if split is None:
                current.append(Verse(c, 0, text, note, printed))
                continue
            parts = _cut(f"{book} {printed}", text, split.before)
            pieces = [Verse(c, 0, part, None, printed) for part in parts]
            pieces[-1].note = note
            if split.to_previous_chapter:
                if not result:
                    raise BuildError(f"{book} {printed}: there is no previous chapter")
                first = pieces.pop(0)
                first.chapter = c - 1
                result[-1].append(first)
            current.extend(pieces)
        result.append(current)
    if mine:
        missing = ", ".join(f"{c}:{v}" for (c, v) in mine)
        raise BuildError(f"{book}: no verse {missing} to divide")

    divided = {(s.chapter, s.verse) for s in splits if s.book == book}
    for verses in result:
        for v, verse in enumerate(verses, start=1):
            verse.verse = v
            own = f"{verse.chapter}:{v}"
            source = tuple(int(x) for x in verse.printed.split(":"))  # type: ignore[union-attr]
            if verse.printed == own and source not in divided:
                verse.printed = None
    return result


def check_counts(book: str, chapters: list[list[Verse]], expected: list[int]) -> None:
    """Stop when the chapters or verses differ from the BHSA's."""
    counts = [len(verses) for verses in chapters]
    if len(counts) != len(expected):
        raise BuildError(
            f"{book}: {len(counts)} chapters, but the BHSA has {len(expected)}"
        )
    wrong = [
        f"{c} has {n} verses, not {e}"
        for (c, (n, e)) in enumerate(zip(counts, expected), start=1)
        if n != e
    ]
    if wrong:
        raise BuildError(f"{book}: chapter " + "; ".join(wrong))


WORD = re.compile(r"([^\s—]+)([\s—]*)")
LAST_LETTER = re.compile(r"^(.*[^\W_])(.*)$", re.DOTALL)


def tokenize(text: str) -> list[tuple[str, str]]:
    """Words with what follows them (punctuation and space), so that joining
    them gives the text again. A word keeps punctuation before it (`‘Speak`,
    `[into`); punctuation after it goes into its trailer."""
    words: list[tuple[str, str]] = []
    prefix = ""
    for match in WORD.finditer(text):
        (chunk, space) = match.groups()
        letters = LAST_LETTER.match(chunk)
        if letters is None:
            # punctuation on its own: with the word before it, else the next
            if words:
                (w, t) = words[-1]
                words[-1] = (w, t + chunk + space)
            else:
                prefix += chunk + space
            continue
        words.append((prefix + letters.group(1), letters.group(2) + space))
        prefix = ""
    if prefix:
        if not words:
            raise ValueError(f"no words in {text!r}")
        (w, t) = words[-1]
        words[-1] = (w, t + prefix)
    return words


def lower(word: str) -> str:
    """A word for searching: lower case, without punctuation (`‘Speak` →
    `speak`), keeping apostrophes and hyphens inside it (`jacob's`,
    `man-servant`)."""
    plain = re.sub(r"[^\w'-]", "", word.replace("’", "'")).strip("'-")
    return plain.lower()


def write_corpus(
    translation: Translation,
    books: dict[str, list[list[Verse]]],
    repo: Path,
    checksum: str,
) -> Path:
    """Write the corpus in Text-Fabric layout under `repo`; returns the data
    directory (`<repo>/tf/<version>`)."""
    from cfabric import Fabric

    otype: dict[int, str] = {}
    oslots: dict[int, set[int]] = {}
    feats: dict[str, dict[int, Any]] = {
        name: {}
        for name in (
            "text",
            "trailer",
            "lower",
            "book",
            "chapter",
            "verse",
            "note",
            "printed",
        )
    }

    slot = 0
    spans: list[tuple[str, Any, int, int, Verse | None]] = []
    for book, chapters in books.items():
        bookStart = slot + 1
        for c, verses in enumerate(chapters, start=1):
            chapterStart = slot + 1
            for verse in verses:
                verseStart = slot + 1
                words = tokenize(verse.text)
                for i, (w, t) in enumerate(words):
                    slot += 1
                    otype[slot] = "word"
                    feats["text"][slot] = w
                    # a space between verses, for reading them as running text
                    feats["trailer"][slot] = (
                        t if (i < len(words) - 1 or t.endswith(" ")) else t + " "
                    )
                    feats["lower"][slot] = lower(w)
                spans.append(("verse", (book, c, verse.verse), verseStart, slot, verse))
            spans.append(("chapter", (book, c), chapterStart, slot, None))
        spans.append(("book", (book,), bookStart, slot, None))

    maxSlot = slot
    node = maxSlot
    for kind in ("book", "chapter", "verse"):
        for k, ref, first, last, verse in spans:
            if k != kind:
                continue
            node += 1
            otype[node] = kind
            oslots[node] = set(range(first, last + 1))
            feats["book"][node] = ref[0]
            if kind != "book":
                feats["chapter"][node] = ref[1]
            if kind == "verse":
                feats["verse"][node] = ref[2]
                assert verse is not None
                if verse.note:
                    feats["note"][node] = verse.note
                if verse.printed:
                    feats["printed"][node] = verse.printed

    generic = {
        "name": translation.title,
        "title": translation.cite,
        "language": translation.language,
        "licence": translation.licence,
        "source": translation.source,
        "sourceChecksum": checksum,
        "createdBy": f"cfabric-browser {__version__}",
    }
    described = {
        "otype": "the type of a node: book, chapter, verse or word",
        "oslots": "the words of a book, chapter or verse",
        "text": "the word as printed, with any punctuation before it",
        "trailer": "punctuation and space after the word",
        "lower": "the word in lower case without punctuation, for searching",
        "book": "the book, by its English name in the BHSA",
        "chapter": "the chapter, as in the BHSA",
        "verse": "the verse, as in the BHSA",
        "note": "a note printed after the verse in small type, such as the verse "
        "repeated at the end of Malachi",
        "printed": "the chapter and verse in the 1917 print, where the BHSA "
        "divides the verses differently",
    }
    metaData: dict[str, dict[str, Any]] = {"": generic}
    for name, description in described.items():
        isInt = name in ("chapter", "verse")
        metaData[name] = {
            "description": description,
            "valueType": "int" if isInt else "str",
        }
    metaData["otext"] = {
        "fmt:text-orig-full": "{text}{trailer}",
        "sectionTypes": "book,chapter,verse",
        "sectionFeatures": "book,chapter,verse",
    }

    data = repo / "tf" / translation.version
    data.mkdir(parents=True, exist_ok=True)
    CF = Fabric(locations=str(data), silent="deep")
    good = CF.save(
        nodeFeatures={"otype": otype, **{k: v for k, v in feats.items() if v}},
        edgeFeatures={"oslots": oslots},
        metaData=metaData,
        location=str(data),
        module="",
        silent="deep",
    )
    if not good:
        raise BuildError("Could not write the corpus")

    app = repo / "app"
    app.mkdir(parents=True, exist_ok=True)
    config = {
        "apiVersion": 3,
        "writing": translation.language,
        "provenanceSpec": {
            "corpus": f"{translation.title} = {translation.name}",
            "org": translation.org,
            "repo": translation.repo,
            "version": translation.version,
        },
        "translation": {
            "id": translation.id,
            "licence": translation.licence,
            "licenceNote": translation.licence_note,
            "source": translation.source,
            "cite": translation.cite,
            "builder": f"cfabric-browser {__version__}",
            "sourceChecksum": checksum,
        },
    }
    (app / "config.yaml").write_text(
        yaml.safe_dump(config, sort_keys=False, allow_unicode=True), encoding="utf-8"
    )
    return data


def prepare(
    translation: Translation, files: dict[str, bytes]
) -> dict[str, list[list[Verse]]]:
    """Check and clean the source files and divide them as in the BHSA."""
    expected = hebrew_verses()
    books: dict[str, list[list[Verse]]] = {}
    for name, _ in translation.books:
        if name not in files:
            raise BuildError(f"{name} is missing from the source")
        chapters = read_book(translation, name, files[name])
        cleaned = [
            [clean(f"{name} {c}:{v}", raw) for v, raw in enumerate(verses, start=1)]
            for c, verses in enumerate(chapters, start=1)
        ]
        divided = renumber(name, cleaned, translation.splits)
        if name in expected:
            check_counts(name, divided, expected[name])
        books[name] = divided
    return books


def build(
    translation: Translation,
    root: str | Path = ROOT,
    cache: str | Path = CACHE,
    progress: Progress | None = None,
    download: Fetch = _download,
) -> Path:
    """Download, check and build a translation; returns its data directory.

    The corpus is written next to the place it goes, then moved there, so an
    interrupted build leaves an earlier one intact.
    """
    files = fetch(translation, cache=cache, progress=progress, download=download)
    books = prepare(translation, files)
    if progress is not None:
        progress("build", 0, 1)
    checksum = _sha256(
        "".join(
            f"{name}:{_sha256(files[name])}\n" for name, _ in translation.books
        ).encode()
    )

    org = Path(root).expanduser() / translation.org
    org.mkdir(parents=True, exist_ok=True)
    final = org / translation.repo
    building = org / f".{translation.repo}.building"
    old = org / f".{translation.repo}.old"
    for leftover in (building, old):
        if leftover.exists():
            shutil.rmtree(leftover)
    write_corpus(translation, books, building, checksum)
    if final.exists():
        final.rename(old)
    building.rename(final)
    if old.exists():
        shutil.rmtree(old, ignore_errors=True)
    if progress is not None:
        progress("build", 1, 1)
    return final / "tf" / translation.version
