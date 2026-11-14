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

An OSIS source (`format="osis"`, the Statenvertaling) is one file, pinned by
its checksum. `read_osis` reads its verses with their words (added words,
Strong's numbers) and notes, and `renumber_by_order` numbers them as the
BHSA's by their order in each book, after the `Split`s and `Join`s; then the
same checks and `write_corpus`.
"""

from __future__ import annotations

import html
import json
import re
import shutil
import urllib.parse
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from importlib import resources
from pathlib import Path
from typing import Any

import yaml

from cfabric_browser import __version__
from cfabric_browser.display.config import read_app_config
from cfabric_browser.jobs import Progress
from cfabric_browser.net import CORPUS_ROOT, read_url, sha256_bytes
from cfabric_browser.translations.catalog import Join, Split, Translation

CACHE = "~/.cfabric/translations"
ROOT = CORPUS_ROOT

# called with a step ("download", "build"), the number done and the total
Fetch = Callable[[str], bytes]


class BuildError(Exception):
    """The source is not what the builder expects; the message says where."""


@dataclass
class Word:
    """A word read from an OSIS source, with what follows it."""

    text: str
    trailer: str
    # added by the translators (in italics in the print)
    added: bool = False
    # Strong's numbers of the Hebrew it translates, such as "H7225"
    strong: str | None = None


@dataclass
class Verse:
    chapter: int
    verse: int
    text: str
    note: str | None = None
    # the source's own reference, where the BHSA divides differently
    printed: str | None = None
    # an OSIS source: the words, already divided, and the translators' notes
    words: list[Word] | None = None
    margin: str | None = None


def hebrew_verses() -> dict[str, list[int]]:
    """The number of verses per chapter of each book in the BHSA."""
    data = resources.files("cfabric_browser.translations").joinpath(
        "hebrew_verses.json"
    )
    return json.loads(data.read_text(encoding="utf-8"))


def fetch(
    translation: Translation,
    cache: str | Path = CACHE,
    progress: Progress | None = None,
    download: Fetch = read_url,
) -> dict[str, bytes]:
    """The source file of every book, keyed by the BHSA's book name.

    Files are kept in `<cache>/<id>/` with their checksums in
    `checksums.json`; a file whose checksum still matches is not downloaded
    again.
    """
    folder = Path(cache).expanduser() / translation.id
    folder.mkdir(parents=True, exist_ok=True)
    if translation.format == "osis":
        return {"": fetch_file(translation, folder, progress, download)}
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
            if sha256_bytes(data) != sums[name]:
                data = None
        if data is None:
            url = translation.url.format(path=urllib.parse.quote(path))
            try:
                data = download(url)
            except OSError as e:
                raise BuildError(f"Could not download {name} from {url}: {e}") from e
            target.write_bytes(data)
            sums[name] = sha256_bytes(data)
            sums_file.write_text(json.dumps(sums, indent=1), encoding="utf-8")
        files[name] = data
        if progress is not None:
            progress("download", i + 1, total)
    return files


def fetch_file(
    translation: Translation,
    folder: Path,
    progress: Progress | None,
    download: Fetch,
) -> bytes:
    """An OSIS source: one file, which must have the checksum in the
    catalogue; kept in `folder` and not downloaded again while it has."""
    target = folder / "source.xml"
    data: bytes | None = None
    if target.is_file():
        data = target.read_bytes()
        if sha256_bytes(data) != translation.checksum:
            data = None
    if data is None:
        try:
            data = download(translation.url)
        except OSError as e:
            raise BuildError(f"Could not download {translation.url}: {e}") from e
        if sha256_bytes(data) != translation.checksum:
            raise BuildError(
                f"The source at {translation.url} is not the version this "
                "program was made for (its checksum differs)"
            )
        target.write_bytes(data)
    if progress is not None:
        progress("download", 1, 1)
    return data


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


OSIS = "{http://www.bibletechnologies.net/2003/OSIS/namespace}"
# the last number in Strong's Hebrew dictionary; higher ones (8675 and up)
# are Strong's codes for verb forms, not words
LAST_STRONG = 8674
STRONG = re.compile(r"strong:H(\d+)")
NO_SPACE_BEFORE = ",;:.!?"


@dataclass
class SourceVerse:
    """A verse of an OSIS source, in the source's numbering."""

    chapter: int
    verse: int
    words: list[Word]
    margin: str | None


def note_text(note: Any) -> str:
    """A translators' note as text: its catchword, a dash, and the note."""
    head = ""
    parts = [note.text or ""]
    for child in note:
        if child.tag == OSIS + "catchWord" and not head:
            head = " ".join("".join(child.itertext()).split())
        else:
            parts.append("".join(child.itertext()))
        parts.append(child.tail or "")
    body = " ".join("".join(parts).split())
    # no space before punctuation, as in the verses (`Gen 12:5 .`)
    body = re.sub(r" ([" + re.escape(NO_SPACE_BEFORE) + r"])", r"\1", body)
    return f"{head} — {body}" if head else body


def osis_words(ref: str, verse: Any) -> tuple[list[Word], list[str]]:
    """The words of an OSIS verse, and its notes.

    Text in `<transChange type="added">` is added; `<w lemma="strong:H…">`
    gives Strong's numbers; `<divineName>` and `<title>` (the letters of an
    acrostic) are read as text. Any other element stops the build.
    """
    chars: list[tuple[str, bool, tuple[str, ...]]] = []
    notes: list[str] = []

    def put(text: str | None, added: bool, strong: tuple[str, ...]) -> None:
        for ch in text or "":
            if ch.isspace():
                if chars and chars[-1][0] != " ":
                    chars.append((" ", False, ()))
            else:
                # no space before punctuation (the source has one where an
                # empty <w> stands between a word and a comma)
                if ch in NO_SPACE_BEFORE and chars and chars[-1][0] == " ":
                    chars.pop()
                chars.append((ch, added, strong))

    def visit(element: Any, added: bool, strong: tuple[str, ...]) -> None:
        put(element.text, added, strong)
        for child in element:
            tag = child.tag.removeprefix(OSIS)
            if tag == "note":
                notes.append(note_text(child))
            elif tag == "w":
                numbers = tuple(
                    f"H{int(n)}"
                    for n in STRONG.findall(child.get("lemma", ""))
                    if int(n) <= LAST_STRONG
                )
                visit(child, added, numbers)
            elif tag == "transChange" and child.get("type") == "added":
                visit(child, True, strong)
            elif tag in ("divineName", "title"):
                visit(child, added, strong)
            else:
                raise BuildError(f"{ref}: unexpected markup <{tag}>")
            put(child.tail, added, strong)

    visit(verse, False, ())
    while chars and chars[-1][0] == " ":
        chars.pop()
    text = "".join(ch for (ch, _, _) in chars)
    if not text:
        raise BuildError(f"{ref}: the verse is empty")
    words: list[Word] = []
    at = 0
    for w, t in tokenize(text):
        span = chars[at : at + len(w)]
        if "".join(ch for (ch, _, _) in span) != w:
            raise BuildError(f"{ref}: cannot divide the verse into words")
        numbers: list[str] = []
        for _, _, strong in span:
            numbers.extend(n for n in strong if n not in numbers)
        added = any(a for (ch, a, _) in span if ch.isalnum())
        words.append(Word(w, t, added, " ".join(numbers) or None))
        at += len(w) + len(t)
    return (words, notes)


def read_osis(translation: Translation, data: bytes) -> dict[str, list[SourceVerse]]:
    """The verses of each book (keyed by the BHSA's name) of an OSIS source,
    after checking its title."""
    import xml.etree.ElementTree as ET

    try:
        root = ET.fromstring(data)
    except ET.ParseError as e:
        raise BuildError(f"The source is not readable OSIS ({e})") from e
    title = root.find(f"{OSIS}osisText/{OSIS}header/{OSIS}work/{OSIS}title")
    found = (title.text or "").strip() if title is not None else None
    if found != translation.source_title:
        raise BuildError(f"The source is '{found}', not '{translation.source_title}'")
    ids = {osis: name for (name, osis) in translation.books}
    books: dict[str, list[SourceVerse]] = {name: [] for name in ids.values()}
    for div in root.iter(OSIS + "div"):
        name = ids.get(div.get("osisID", ""))
        if name is None or div.get("type") != "book":
            continue
        for verse in div.iter(OSIS + "verse"):
            osisID = verse.get("osisID", "")
            parts = osisID.split(".")
            if len(parts) != 3 or not parts[1].isdigit() or not parts[2].isdigit():
                raise BuildError(f"{name}: a verse without a reference ({osisID!r})")
            (c, v) = (int(parts[1]), int(parts[2]))
            (words, notes) = osis_words(f"{name} {c}:{v}", verse)
            books[name].append(SourceVerse(c, v, words, " | ".join(notes) or None))
    for name, verses in books.items():
        if not verses:
            raise BuildError(f"{name} is missing from the source")
    return books


def spaced(words: list[Word]) -> list[Word]:
    """The words with a space after the last one, as between verses."""
    if not words or words[-1].trailer.endswith(" "):
        return words
    last = words[-1]
    return [*words[:-1], Word(last.text, last.trailer + " ", last.added, last.strong)]


def _cut_words(
    ref: str, words: list[Word], before: tuple[str, ...]
) -> list[list[Word]]:
    """The words of a verse cut before each text in `before`."""
    parts: list[list[Word]] = []
    rest = words
    for start in before:
        at = next(
            (
                i
                for i in range(1, len(rest))
                if "".join(w.text + w.trailer for w in rest[i:]).startswith(start)
            ),
            None,
        )
        if at is None:
            raise BuildError(f"{ref}: cannot divide the verse before {start!r}")
        parts.append(rest[:at])
        rest = rest[at:]
    parts.append(rest)
    return parts


def renumber_by_order(
    book: str,
    source: list[SourceVerse],
    expected: list[int],
    splits: tuple[Split, ...] = (),
    joins: tuple[Join, ...] = (),
) -> list[list[Verse]]:
    """Verses per chapter of an OSIS source, numbered as the BHSA's by their
    order in the book, after dividing (`Split`) and joining (`Join`) verses.

    A verse keeps the source's number in `printed` where it differs, and
    where it was divided or joined (`7:67-68`, `7:73-8:1`).
    """
    cut = {(s.chapter, s.verse): s for s in splits if s.book == book}
    joined = {(j.chapter, j.verse): j for j in joins if j.book == book}
    # (the source's number, words, notes, divided or joined)
    pieces: list[tuple[str, list[Word], list[str], bool]] = []
    i = 0
    while i < len(source):
        sv = source[i]
        here = f"{sv.chapter}:{sv.verse}"
        join = joined.pop((sv.chapter, sv.verse), None)
        split = cut.pop((sv.chapter, sv.verse), None)
        if join is not None:
            group = source[i : i + 1 + join.more]
            if len(group) != 1 + join.more:
                raise BuildError(f"{book} {here}: no verses after it to join")
            last = group[-1]
            ending = (
                f"{last.verse}"
                if last.chapter == sv.chapter
                else f"{last.chapter}:{last.verse}"
            )
            words = [w for g in group for w in spaced(g.words)]
            notes = [g.margin for g in group if g.margin]
            pieces.append((f"{here}-{ending}", words, notes, True))
            i += len(group)
            continue
        if split is not None:
            parts = _cut_words(f"{book} {here}", sv.words, split.before)
            for k, part in enumerate(parts):
                last_part = k == len(parts) - 1
                notes = [sv.margin] if sv.margin and last_part else []
                pieces.append((here, part, notes, True))
        else:
            pieces.append((here, sv.words, [sv.margin] if sv.margin else [], False))
        i += 1
    if cut or joined:
        missing = ", ".join(f"{c}:{v}" for (c, v) in [*cut, *joined])
        raise BuildError(f"{book}: no verse {missing} to divide or join")
    if len(pieces) != sum(expected):
        raise BuildError(
            f"{book}: {len(pieces)} verses after dividing and joining, "
            f"but the BHSA has {sum(expected)}"
        )

    result: list[list[Verse]] = []
    at = 0
    for c, count in enumerate(expected, start=1):
        verses = []
        for v in range(1, count + 1):
            (printed, words, notes, changed) = pieces[at]
            at += 1
            # a space between verses, for reading them as running text
            words = spaced(words)
            verses.append(
                Verse(
                    c,
                    v,
                    "",
                    printed=printed if changed or printed != f"{c}:{v}" else None,
                    words=words,
                    margin=" | ".join(notes) or None,
                )
            )
        result.append(verses)
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
            "margin",
            "added",
            "strong",
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
                if verse.words is not None:
                    for word in verse.words:
                        slot += 1
                        otype[slot] = "word"
                        feats["text"][slot] = word.text
                        feats["trailer"][slot] = word.trailer
                        feats["lower"][slot] = lower(word.text)
                        if word.added:
                            feats["added"][slot] = 1
                        if word.strong:
                            feats["strong"][slot] = word.strong
                    spans.append(
                        ("verse", (book, c, verse.verse), verseStart, slot, verse)
                    )
                    continue
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
                if verse.margin:
                    feats["margin"][node] = verse.margin

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
        "printed": f"the chapter and verse in {translation.printed_as}, where "
        "the BHSA divides the verses differently",
        "margin": "the translators' notes on the verse (kanttekeningen), each "
        "as its catchword, a dash and the note, separated by ' | '",
        "added": "1 on words the translators added (in italics in the print)",
        "strong": "the Strong's numbers of the Hebrew the word translates",
    }
    metaData: dict[str, dict[str, Any]] = {"": generic}
    for name, description in described.items():
        if name not in ("otype", "oslots") and not feats.get(name):
            continue
        isInt = name in ("chapter", "verse", "added")
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
    if translation.format == "osis":
        source = read_osis(translation, files[""])
        for name, _ in translation.books:
            if name not in expected:
                raise BuildError(f"{name}: not a book of the BHSA")
            divided = renumber_by_order(
                name,
                source[name],
                expected[name],
                translation.splits,
                translation.joins,
            )
            check_counts(name, divided, expected[name])
            books[name] = divided
        return books
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


def is_ours(repo: Path, translation: Translation) -> bool:
    """Whether `repo` was built as this translation (its app config says so)."""
    config = repo / "app" / "config.yaml"
    if not config.is_file():
        return False
    try:
        own = read_app_config(config).get("translation") or {}
    except Exception:  # noqa: BLE001 - not readable, so not ours to replace
        return False
    return own.get("id") == translation.id


def build(
    translation: Translation,
    root: str | Path = ROOT,
    cache: str | Path = CACHE,
    progress: Progress | None = None,
    download: Fetch = read_url,
) -> Path:
    """Download, check and build a translation; returns its data directory.

    The corpus is written next to the place it goes, then moved there, so an
    interrupted build leaves an earlier one intact.
    """
    files = fetch(translation, cache=cache, progress=progress, download=download)
    books = prepare(translation, files)
    if progress is not None:
        progress("build", 0, 1)
    checksum = (
        sha256_bytes(files[""])
        if translation.format == "osis"
        else sha256_bytes(
            "".join(
                f"{name}:{sha256_bytes(files[name])}\n" for name, _ in translation.books
            ).encode()
        )
    )

    org = Path(root).expanduser() / translation.org
    org.mkdir(parents=True, exist_ok=True)
    final = org / translation.repo
    building = org / f".{translation.repo}.building"
    old = org / f".{translation.repo}.old"
    # an earlier build that stopped between its two renames
    if old.exists() and not final.exists() and is_ours(old, translation):
        old.rename(final)
    if final.exists() and not is_ours(final, translation):
        raise BuildError(
            f"{final} exists but was not built as {translation.id}; move it away first"
        )
    for leftover in (building, old):
        if leftover.exists():
            shutil.rmtree(leftover)
    write_corpus(translation, books, building, checksum)
    if final.exists():
        final.rename(old)
    try:
        building.rename(final)
    except OSError:
        if old.exists() and not final.exists():
            old.rename(final)
        raise
    if old.exists():
        shutil.rmtree(old, ignore_errors=True)
    if progress is not None:
        progress("build", 1, 1)
    return final / "tf" / translation.version
