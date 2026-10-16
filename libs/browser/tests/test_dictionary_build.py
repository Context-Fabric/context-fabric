"""Building the BDB file: Sefaria's walk, the join with Open Scriptures' ids,
the cleaning of the HTML (`dictionaries/sefaria.py`, `bdb.py`, `build.py`).
No network: Sefaria's answers come from the tests."""

import io
import json
import urllib.error
from pathlib import Path

import pytest
from cfabric_browser.dictionaries import bdb, build, sefaria

BDB_XML = """<?xml version="1.0" encoding="UTF-8"?>
<lexicon xmlns="http://openscriptures.github.com/morphhb/namespace">
  <part id="a" title="א" xml:lang="heb">
    <section id="a.ac">
      <entry id="a.ac.aa" type="root"><w>אָבַד</w> <pos>vb</pos>. <def>perish</def>
        <status p="2">base</status></entry>
      <entry id="a.ac.ab" cite="full"><w>אֹבֵד</w> <pos>n.m</pos>. <def>destruction</def>
        <status p="2">done</status></entry>
    </section>
  </part>
  <part id="xa" title="א" xml:lang="arc">
    <section id="xa.aa">
      <entry id="xa.aa.aa" mod="II"><w>אַב</w> <pos>n.m</pos>. <def>father</def></entry>
    </section>
  </part>
</lexicon>
"""


def test_read_entries(tmp_path: Path):
    path = tmp_path / "bdb.xml"
    path.write_text(BDB_XML, encoding="utf8")
    entries = bdb.read_entries(path)
    assert [e.id for e in entries] == ["a.ac.aa", "a.ac.ab", "xa.aa.aa"]
    first = entries[0]
    assert (first.headword, first.cons, first.pos, first.defs) == (
        "אָבַד",
        ">BD",
        "vb",
        ["perish"],
    )
    assert first.root and first.section == "a.ac" and not first.aramaic
    assert entries[2].aramaic and entries[2].homonym == "II"


def test_join_equal_and_differing_spellings():
    ours = [">BD", ">BD", ">BDH", "XYZ", ">BH"]
    theirs = [">BD", ">BD", ">BDH", "XJYZ", ">BH"]
    assert bdb.join(ours, theirs) == {0: 0, 1: 1, 2: 2, 3: 3, 4: 4}


def test_join_entry_only_on_one_side():
    ours = [">B", ">BB", ">BD", ">BDH"]
    theirs = [">B", ">BD", ">BDH"]
    assert bdb.join(ours, theirs) == {0: 0, 2: 1, 3: 2}


def test_join_two_entries_in_the_other_order():
    ours = [">DR", ">DRMLK", ">DRM", ">DRKWN", ">DR<J"]
    theirs = [">DR", ">DRMLK", ">DRKWN", ">DRM", ">DR<J"]
    assert bdb.join(ours, theirs) == {0: 0, 1: 1, 2: 3, 3: 2, 4: 4}


def test_headword_consonants():
    assert bdb.headword_consonants("מֶ֫לֶךְ²") == "MLK"
    assert bdb.headword_consonants("אֲבִי־עַלְבוֹן") == ">BJ<LBWN"


def test_osis():
    assert bdb.osis("II Samuel 12:4") == "2Sam.12.4"
    assert bdb.osis("Genesis 1") == "Gen.1"
    assert bdb.osis("Psalms 23:1-3") == "Ps.23.1"
    assert bdb.osis("The Book of Maccabees I (Kahana Translation) 1:1") is None


def test_clean():
    raw = (
        '<big><big><span dir="rtl">הָלַךְ</span></big></big><sub>1545</sub>'
        "  <strong>vb.</strong> go, &amp; come "
        '<a data-ref="Genesis 26:26" href="/Genesis.26.26">Gn 26:26</a>; '
        '<a data-ref="BDB, אבה" href="/BDB,_אבה", dir="rtl">II. אבה</a>'
        '<a data-ref="Mishnah Avot 1:1" href="/x">Ab</a><br>Qal'
    )
    c = bdb.clean(raw)
    assert c.html == (
        '<big><big><span dir="rtl">הָלַךְ</span></big></big><sub>1545</sub>'
        "  <strong>vb.</strong> go, &amp; come "
        '<a data-osis="Gen.26.26">Gn 26:26</a>; '
        '<a data-entry="BDB, אבה">II. אבה</a>'
        "<a>Ab</a><br>Qal"
    )
    assert c.refs == ["Gen.26.26"]
    assert c.entries == ["BDB, אבה"]
    assert c.unknown == ["Mishnah Avot 1:1"]


def test_clean_closes_what_is_open():
    assert bdb.clean("<em>open <strong>both").html == (
        "<em>open <strong>both</strong></em>"
    )


def test_clean_keeps_bdbs_less_than():
    raw = "for his error; <Th We Dr; but < Qr <em>x</em>"
    assert bdb.clean(raw).html == (
        "for his error; &lt;Th We Dr; but &lt; Qr <em>x</em>"
    )


def test_clean_refuses_unknown_tags():
    with pytest.raises(bdb.CleanError):
        bdb.clean('<img src="x.png">')
    with pytest.raises(bdb.CleanError):
        bdb.clean("<script>alert(1)</script>")


def test_plain():
    assert sefaria.plain("מֶ֫לֶךְ²") == "מֶ֫לֶךְ"
    assert sefaria.plain("אָב") == "אָב"


# Sefaria in miniature: two lexicons of a few entries
INDEX = {
    "BDB": {"schema": {"nodes": [{"lexiconName": "BDB Dictionary", "firstWord": "א"}]}},
    "BDB Aramaic": {
        "schema": {
            "nodes": [{"lexiconName": "BDB Aramaic Dictionary", "firstWord": "אָב"}]
        }
    },
}
TEXTS = {
    ("BDB", "א"): ("<em>Āleph</em>", None, "BDB, אָבַד"),
    ("BDB", "אָבַד"): (
        '<strong>vb.</strong> perish <a data-ref="BDB, אֹבֵד">אֹבֵד</a>',
        "BDB, א",
        "BDB, אֹבֵד",
    ),
    ("BDB", "אֹבֵד"): ("<strong>n.m.</strong> destruction", "BDB, אָבַד", None),
    ("BDB Aramaic", "אָב"): ("<strong>n.m.</strong> father", None, None),
}


def fake_sefaria(calls: list[str]):
    def download(url: str) -> bytes:
        calls.append(url)
        for book, answer in INDEX.items():
            if url == sefaria.index_url(book):
                return json.dumps(answer).encode()
        for (book, headword), (text, prev, nxt) in TEXTS.items():
            if url == sefaria.text_url(book, headword):
                answer = {"text": [text], "prev": prev, "next": nxt}
                return json.dumps(answer).encode()
        raise urllib.error.HTTPError(url, 404, "not found", None, None)  # type: ignore[arg-type]

    return download


def test_walk_and_cache(tmp_path: Path):
    calls: list[str] = []
    client = sefaria.Client(tmp_path, pause=0, download=fake_sefaria(calls))
    entries = sefaria.walk(client, "BDB Dictionary")
    assert [e["headword"] for e in entries] == ["א", "אָבַד", "אֹבֵד"]
    assert entries[1]["next_hw"] == "אֹבֵד"
    assert client.requests == 4  # the index and three entries
    # a second walk takes everything from the cache
    again = sefaria.Client(tmp_path, pause=0, download=fake_sefaria(calls))
    assert sefaria.walk(again, "BDB Dictionary") == entries
    assert again.requests == 0


def test_cache_with_a_wrong_checksum_is_fetched_again(tmp_path: Path):
    calls: list[str] = []
    client = sefaria.Client(tmp_path, pause=0, download=fake_sefaria(calls))
    url = sefaria.index_url("BDB")
    client.get(url)
    (tmp_path / client._name(url)).write_bytes(b"{}")
    again = sefaria.Client(tmp_path, pause=0, download=fake_sefaria(calls))
    assert again.get(url) == INDEX["BDB"]
    assert again.requests == 1


def test_retries_when_busy(tmp_path: Path):
    answers = [
        urllib.error.HTTPError("u", 429, "busy", {"Retry-After": "7"}, io.BytesIO()),  # type: ignore[arg-type]
        urllib.error.URLError("no connection"),
        b'{"ok": 1}',
    ]
    waits: list[float] = []

    def download(url: str) -> bytes:
        answer = answers.pop(0)
        if isinstance(answer, Exception):
            raise answer
        return answer

    client = sefaria.Client(tmp_path, pause=0, download=download, sleep=waits.append)
    assert client.get("https://example.org/x") == {"ok": 1}
    assert len(waits) == 2 and waits[0] > 6.9 and waits[1] > 9.9


def test_gives_up_on_other_errors(tmp_path: Path):
    def download(url: str) -> bytes:
        raise urllib.error.HTTPError(url, 403, "forbidden", None, None)  # type: ignore[arg-type]

    client = sefaria.Client(tmp_path, pause=0, download=download)
    with pytest.raises(sefaria.SefariaError):
        client.get("https://example.org/x")


def test_entries_join_and_cross_references(tmp_path: Path):
    path = tmp_path / "bdb.xml"
    path.write_text(BDB_XML, encoding="utf8")
    ours = bdb.read_entries(path)
    client = sefaria.Client(tmp_path / "cache", pause=0, download=fake_sefaria([]))
    entries, figures = build.entries(client, ours, {}, byHand={})
    byHead = {e["headword"]: e for e in entries}
    # א has no entry in the Open Scriptures file of this test
    assert byHead["א"]["id"] == "s.0"
    assert byHead["אָבַד"]["id"] == "a.ac.aa"
    assert byHead["אָבַד"]["defs"] == ["perish"]
    assert byHead["אָב"]["id"] == "xa.aa.aa"
    assert byHead["אָב"]["lexicon"] == "arc"
    # the reference to אֹבֵד became its id
    assert '<a data-entry="a.ac.ab">' in byHead["אָבַד"]["html"]
    assert figures["BDB"] == {
        "sefaria": 3,
        "openscriptures": 2,
        "joined": 2,
        "joined by hand": 0,
        "incomplete at Sefaria": [],
    }
    assert figures["unresolved cross references"] == 0


def test_entries_joined_by_hand(tmp_path: Path):
    # an entry spelled otherwise than Sefaria's and in another place, so the
    # join leaves it out
    path = tmp_path / "bdb.xml"
    path.write_text(
        BDB_XML.replace(
            "  </part>\n  <part",
            '  <section id="a.aa"><entry id="a.aa.aa"><w>אָלֶף</w></entry></section>'
            "\n  </part>\n  <part",
            1,
        ),
        encoding="utf8",
    )
    ours = bdb.read_entries(path)
    client = sefaria.Client(tmp_path / "cache", pause=0, download=fake_sefaria([]))
    entries, figures = build.entries(client, ours, {}, byHand={"BDB, א": "a.aa.aa"})
    assert entries[0]["id"] == "a.aa.aa" and entries[0]["headword"] == "א"
    assert figures["BDB"]["joined by hand"] == 1
    # a join by hand replaces the join's for both sides
    entries, _ = build.entries(client, ours, {}, byHand={"BDB, א": "a.ac.aa"})
    assert [e["id"] for e in entries[:3]] == ["a.ac.aa", "s.1", "a.ac.ab"]
    with pytest.raises(build.BuildError):
        build.entries(client, ours, {}, byHand={"BDB, אַיִן": "a.aa.aa"})
    with pytest.raises(build.BuildError):
        build.entries(
            client, ours, {}, byHand={"BDB, א": "a.aa.aa", "BDB, אָבַד": "a.aa.aa"}
        )


def test_cut_texts():
    assert build._cut("† [<span>אֶ֫שֶׁר</span>] <strong>n.</strong>[<strong>m.</strong>")
    assert build._cut("† II. [גְּדוּד] n.[m. or f.")
    assert not build._cut("† [אָבַד] vb. perish.")


def test_corrections_by_sefaria_reference():
    entries = [{"id": "s.4", "ref": "BDB, כְּרוּב"}, {"id": "k.da.ac", "ref": "x"}]
    fixed = {"Hebrew KRWB=/": ["BDB, כְּרוּב"], "Hebrew KRWB/": ["k.da.ac"]}
    assert build.resolved(fixed, entries) == {
        "Hebrew KRWB=/": ["s.4"],
        "Hebrew KRWB/": ["k.da.ac"],
    }
    with pytest.raises(build.BuildError):
        build.resolved({"Hebrew X/": ["BDB, אַיִן"]}, entries)


def test_missing_entries_come_from_open_scriptures(tmp_path: Path):
    path = tmp_path / "bdb.xml"
    path.write_text(BDB_XML, encoding="utf8")
    ours = bdb.read_entries(path)
    have = [{"id": "a.ac.aa"}]
    linked = {"Hebrew >BD[": {"entries": ["a.ac.aa", "a.ac.ab"]}}
    [extra] = build.missing(ours, have, linked, {})
    assert extra["id"] == "a.ac.ab" and extra["html"] == "" and extra["incomplete"]
    assert extra["defs"] == ["destruction"]
    with pytest.raises(build.BuildError):
        build.missing(ours, have, {"Hebrew X/": {"entries": ["z.zz.zz"]}}, {})


def test_joins_are_read():
    assert build.joins()["BDB, אֶלְקֹשִׁי"] == "a.du.ac"


def test_corrections_are_read():
    fixed = build.corrections()
    assert fixed["Aramaic W"] == ["xf.aa.aa"]


def test_abbreviations(tmp_path: Path):
    lines = ["NOTE: about citations"] + [f"A{i} = meaning {i}" for i in range(450)]

    def download(url: str) -> bytes:
        return json.dumps({"text": lines}).encode()

    client = sefaria.Client(tmp_path, pause=0, download=download)
    found = build.abbreviations(client)
    assert found["A3"] == "meaning 3"
    assert "NOTE: about citations" not in found
