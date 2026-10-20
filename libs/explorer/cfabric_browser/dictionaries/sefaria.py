"""Getting BDB from Sefaria, entry by entry.

Sefaria's *BDB Dictionary* and *BDB Aramaic Dictionary* (public domain, from a
scan in the National Library of Israel) are not in its export of texts; the
API gives one headword at a time. `walk` goes through a lexicon from its first
headword along `next_hw`, as BDB orders its entries.

`/api/texts/BDB, <headword>` gives the article as one piece of HTML, the
stems divided by `<br>`, and the next headword (`BDB, מֶ֫לֶךְ²`: homonyms are
numbered with superscripts). One request per entry, about 10,000 in all.
(`/api/words/<word>` has the senses as a tree, with Strong's numbers, but
finds only about 60% of the headwords and none with a number, so it would
take more requests and still need this one.)

Requests are made one at a time with a pause between them, and are retried
after a while when Sefaria is busy. Every answer is kept in a cache with its
checksum, so an interrupted walk goes on where it stopped and a second walk
makes no requests at all.
"""

from __future__ import annotations

import hashlib
import json
import time
import urllib.error
import urllib.parse
import urllib.request
from collections.abc import Callable
from pathlib import Path
from typing import Any

from cfabric_browser import __version__
from cfabric_browser.translations.build import _ssl_context

SITE = "https://www.sefaria.org"
CACHE = "~/.cfabric/dictionaries/cache/sefaria"
AGENT = (
    f"cfabric-browser/{__version__} "
    "(BDB for its lexicon; https://github.com/Context-Fabric/context-fabric)"
)

# lexicon: the title of its book (for references and the text API)
LEXICONS = {"BDB Dictionary": "BDB", "BDB Aramaic Dictionary": "BDB Aramaic"}

# BDB's homonyms are numbered with superscripts in Sefaria's headwords
SUPERSCRIPTS = "¹²³⁴⁵⁶⁷⁸⁹⁰"

Download = Callable[[str], bytes]
# called with the number of entries so far and the last headword
Progress = Callable[[int, str], None]


class SefariaError(Exception):
    """Sefaria didn't give what was asked for; the message says what."""


def plain(headword: str) -> str:
    """A headword without its homonym number."""
    return headword.rstrip(SUPERSCRIPTS + " ")


def text_url(book: str, headword: str) -> str:
    quoted = urllib.parse.quote(f"{book}, {headword}", safe="")
    return f"{SITE}/api/texts/{quoted}?context=0&commentary=0&pad=0"


def index_url(book: str) -> str:
    return f"{SITE}/api/v2/index/{urllib.parse.quote(book, safe='')}"


def _download(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(request, timeout=60, context=_ssl_context()) as r:
        data: bytes = r.read()
    return data


class Client:
    """Requests to Sefaria with a cache, a pause between requests, and
    retries."""

    def __init__(
        self,
        cache: str | Path = CACHE,
        pause: float = 1.0,
        download: Download = _download,
        sleep: Callable[[float], None] = time.sleep,
        retries: int = 6,
    ) -> None:
        self.folder = Path(cache).expanduser()
        self.folder.mkdir(parents=True, exist_ok=True)
        self.pause = pause
        self.download = download
        self.sleep = sleep
        self.retries = retries
        self.requests = 0  # made over the network
        self._last = 0.0
        self._sums: dict[str, str] = {}
        sums = self.folder / "checksums.tsv"
        if sums.is_file():
            for line in sums.read_text(encoding="utf-8").splitlines():
                name, _, digest = line.partition("\t")
                self._sums[name] = digest

    def _name(self, url: str) -> str:
        return hashlib.sha1(url.encode("utf-8")).hexdigest() + ".json"

    def cached(self, url: str) -> bytes | None:
        name = self._name(url)
        path = self.folder / name
        if name in self._sums and path.is_file():
            data = path.read_bytes()
            if hashlib.sha256(data).hexdigest() == self._sums[name]:
                return data
        return None

    def get(self, url: str) -> Any:
        """The JSON at `url`, from the cache or from Sefaria."""
        data = self.cached(url)
        if data is None:
            data = self._fetch(url)
            name = self._name(url)
            (self.folder / name).write_bytes(data)
            digest = hashlib.sha256(data).hexdigest()
            self._sums[name] = digest
            with open(self.folder / "checksums.tsv", "a", encoding="utf-8") as f:
                f.write(f"{name}\t{digest}\n")
        try:
            return json.loads(data)
        except ValueError as e:
            raise SefariaError(f"Not JSON from {url}") from e

    def _fetch(self, url: str) -> bytes:
        wait = 0.0
        for attempt in range(self.retries + 1):
            gap = max(self.pause, wait) - (time.monotonic() - self._last)
            if gap > 0:
                self.sleep(gap)
            self._last = time.monotonic()
            try:
                data = self.download(url)
                self.requests += 1
                return data
            except urllib.error.HTTPError as e:
                if e.code not in (429, 500, 502, 503, 504):
                    raise SefariaError(f"{url}: HTTP {e.code}") from e
                after = e.headers.get("Retry-After") if e.headers else None
                wait = float(after) if after and after.isdigit() else 0.0
                problem: Exception = e
            except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
                problem = e
            # wait longer after each failure: 5 s, 10 s, 20 s ... (at most 5 min)
            wait = max(wait, min(300.0, 5.0 * 2**attempt))
        raise SefariaError(f"{url}: {problem}")


def first_headword(client: Client, lexicon: str) -> str:
    index = client.get(index_url(LEXICONS[lexicon]))
    for node in index.get("schema", {}).get("nodes", []):
        if node.get("lexiconName") == lexicon and node.get("firstWord"):
            return str(node["firstWord"])
    raise SefariaError(f"No first word for {lexicon}")


def entry(client: Client, lexicon: str, headword: str) -> dict[str, Any]:
    """One entry of a lexicon, by its headword as Sefaria writes it."""
    book = LEXICONS[lexicon]
    text = client.get(text_url(book, headword))
    if not isinstance(text, dict) or "error" in text or "text" not in text:
        raise SefariaError(f"{lexicon} has no entry {headword}: {text}")
    lines = text["text"]

    def after(ref: str | None) -> str | None:
        prefix = f"{book}, "
        return ref[len(prefix) :] if ref and ref.startswith(prefix) else None

    return {
        "lexicon": lexicon,
        "headword": headword,
        "html": "".join(lines) if isinstance(lines, list) else str(lines),
        "prev_hw": after(text.get("prev")),
        "next_hw": after(text.get("next")),
    }


def walk(
    client: Client,
    lexicon: str,
    progress: Progress | None = None,
    limit: int | None = None,
) -> list[dict[str, Any]]:
    """All entries of a lexicon, in BDB's order."""
    entries: list[dict[str, Any]] = []
    seen: set[str] = set()
    headword: str | None = first_headword(client, lexicon)
    while headword and headword not in seen:
        if limit is not None and len(entries) >= limit:
            break
        seen.add(headword)
        e = entry(client, lexicon, headword)
        entries.append(e)
        if progress is not None:
            progress(len(entries), headword)
        headword = e.get("next_hw")
    return entries


def main() -> None:
    """Walk both lexicons into the cache and write them to `bdb-sefaria.json`
    in the cache's folder (run by hand once, for the build)."""
    import argparse
    import sys

    parser = argparse.ArgumentParser(description=main.__doc__)
    parser.add_argument("--cache", default=CACHE)
    parser.add_argument("--pause", type=float, default=1.0)
    parser.add_argument("--limit", type=int, default=None)
    args = parser.parse_args()
    client = Client(args.cache, pause=args.pause)
    start = time.monotonic()

    def report(n: int, headword: str) -> None:
        if n % 100 == 0:
            minutes = (time.monotonic() - start) / 60
            print(
                f"{n} entries, {client.requests} requests, {minutes:.1f} min",
                file=sys.stderr,
                flush=True,
            )

    result = {}
    for lexicon in LEXICONS:
        print(f"{lexicon} ...", file=sys.stderr, flush=True)
        result[lexicon] = walk(client, lexicon, report, args.limit)
        print(f"{lexicon}: {len(result[lexicon])} entries", file=sys.stderr)
    out = client.folder / "bdb-sefaria.json"
    out.write_text(json.dumps(result, ensure_ascii=False), encoding="utf-8")
    print(f"{out}, {client.requests} requests", file=sys.stderr)


if __name__ == "__main__":
    main()
