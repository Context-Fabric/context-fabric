"""Linking BHSA lexemes to BDB through the OSHB (`dictionaries/link.py`)."""

from pathlib import Path

from cfabric_browser.dictionaries.link import (
    Segment,
    align,
    consonants,
    etcbc_consonants,
    lemma_id,
    read_aug_index,
    read_lexical_index,
    read_oshb,
    segments,
)

OSHB_BOOK = """<?xml version="1.0" encoding="UTF-8"?>
<osis xmlns="http://www.bibletechnologies.net/2003/OSIS/namespace">
  <osisText><div type="book" osisID="Gen"><chapter osisID="Gen.1">
    <verse osisID="Gen.1.1">
      <w lemma="b/7225" morph="HR/Ncfsa" id="a">בְּ/רֵאשִׁ֖ית</w>
      <w lemma="1254 a" morph="HVqp3ms" id="b">בָּרָ֣א</w>
      <w lemma="430" morph="HNcmpa" id="c">אֱלֹהִ֑ים</w>
    </verse>
    <verse osisID="Gen.1.2">
      <w type="x-ketiv" lemma="168" morph="HNcmsc/Sp3ms" id="d">אהל/ה</w><note type="variant"><catchWord>אהל/ה</catchWord><rdg type="x-qere"><w lemma="168" morph="HNcmsc/Sp3ms" id="e">אָֽהֳל/וֹ֙</w></rdg></note>
      <seg type="x-sof-pasuq">׃</seg>
    </verse>
  </chapter></div></osisText>
</osis>
"""

LEXICAL_INDEX = """<?xml version="1.0" encoding="UTF-8"?>
<index xmlns="http://openscriptures.github.com/morphhb/namespace">
  <part xml:lang="heb">
    <entry id="aaf">
      <w xlit="ʾābad">אָבַד</w> <pos>V</pos> <def>perish</def>
      <xref bdb="a.ac.aa" strong="6" twot="2"/>
      <etym root="אבד" type="main">aag, aah</etym>
    </entry>
    <entry id="aag">
      <w xlit="ʾōbēd">אֹבֵד</w> <pos>N</pos> <def>destruction</def>
      <xref bdb="a.ac.ab" strong="8" twot="2a"/>
      <etym type="sub">aaf</etym>
    </entry>
  </part>
  <part xml:lang="arc">
    <entry id="xaa">
      <w>אַב</w> <pos>N</pos> <def>father</def>
      <xref bdb="xa.aa.aa" strong="2"/>
    </entry>
  </part>
</index>
"""

AUG_INDEX = """<?xml version="1.0" encoding="UTF-8"?>
<index xmlns="http://openscriptures.github.com/morphhb/namespace">
  <w aug="6">aaf</w>
  <w aug="122a">aag</w>
</index>
"""


def test_consonants():
    assert consonants("בְּרֵאשִׁ֖ית") == "BR>CJT"
    assert consonants("מֶ֫לֶךְ") == "MLK"  # final forms
    assert consonants("עֵ֔ץ") == "<Y"
    assert consonants("שָׂרָה") == consonants("שָׁרָה") == "CRH"


def test_etcbc_consonants():
    assert etcbc_consonants("JFR>L") == "JCR>L"
    assert etcbc_consonants("CMJM") == "CMJM"
    assert etcbc_consonants("") == ""


def test_lemma_id():
    assert lemma_id("834 a") == "834a"
    assert lemma_id("1177+") == "1177"
    assert lemma_id("7225") == "7225"


def test_segments_prefix_and_main():
    assert segments("בְּ/רֵאשִׁ֖ית", "b/7225", "HR/Ncfsa") == [
        Segment("B", "b", "H"),
        Segment("R>CJT", "7225", "H"),
    ]


def test_segments_suffix_has_no_lemma():
    assert segments("צִוִּיתִ֨י/ךָ֙", "6680", "HVpp1cs/Sp2ms") == [
        Segment("YWJTJ", "6680", "H"),
        Segment("K", None, "H"),
    ]


def test_segments_article_in_a_vowel():
    # לַמֶּלֶךְ: the article has no letter, so it gets no part of the text
    assert segments("לַ/מֶּ֫לֶךְ", "l/d/4428", "HRd/Ncmsa") == [
        Segment("L", "l", "H"),
        Segment("MLK", "4428", "H"),
    ]


def test_segments_two_prefixes():
    assert segments("וְ/הָ/אָ֗רֶץ", "c/d/776", "HC/Td/Ncbsa") == [
        Segment("W", "c", "H"),
        Segment("H", "d", "H"),
        Segment(">RY", "776", "H"),
    ]


def test_segments_aramaic():
    assert segments("מַלְכָּא", "4430", "ANcmsd")[0].language == "A"


def test_segments_unequal_parts():
    # a morphology that does not match the text: the whole word, main lemma
    assert segments("בְּ/רֵאשִׁית", "b/7225", "HNcfsa") == [Segment("BR>CJT", "7225", "H")]


def test_align_prefix_words():
    words = [(1, "B"), (2, "R>CJT"), (3, "BR>"), (4, ">LHJM")]
    found = [
        Segment("B", "b", "H"),
        Segment("R>CJT", "7225", "H"),
        Segment("BR>", "1254a", "H"),
        Segment(">LHJM", "430", "H"),
    ]
    got = align(words, found)
    assert {n: s.lemma for n, s in got.items()} == {
        1: "b",
        2: "7225",
        3: "1254a",
        4: "430",
    }


def test_align_suffix_and_empty_article():
    # BHSA: L, H (article without letters), MLK; then a word with a suffix
    words = [(1, "L"), (2, ""), (3, "MLK"), (4, "<BWRK")]
    found = [
        Segment("L", "l", "H"),
        Segment("MLK", "4428", "H"),
        Segment("<BWR", "5668", "H"),
        Segment("K", None, "H"),
    ]
    got = align(words, found)
    assert {n: s.lemma for n, s in got.items()} == {1: "l", 3: "4428", 4: "5668"}


def test_align_small_differences():
    # a letter more in one of the texts (a mater lectionis) does not matter
    words = [(1, "DWD"), (2, "MLK")]
    found = [Segment("DWJD", "1732", "H"), Segment("MLK", "4428", "H")]
    got = align(words, found)
    assert got[1].lemma == "1732" and got[2].lemma == "4428"


def test_read_oshb(tmp_path: Path):
    path = tmp_path / "Gen.xml"
    path.write_text(OSHB_BOOK, encoding="utf8")
    verses = read_oshb(path)
    assert [s.lemma for s in verses[("Gen", 1, 1)]] == ["b", "7225", "1254a", "430"]
    # the ketiv, not the qere in the note
    assert verses[("Gen", 1, 2)] == [
        Segment(">HL", "168", "H"),
        Segment("H", None, "H"),
    ]


def test_read_indexes(tmp_path: Path):
    (tmp_path / "li.xml").write_text(LEXICAL_INDEX, encoding="utf8")
    (tmp_path / "aug.xml").write_text(AUG_INDEX, encoding="utf8")
    index = read_lexical_index(tmp_path / "li.xml")
    assert index["aaf"].bdb == "a.ac.aa"
    assert index["aaf"].strong == "6"
    assert index["aaf"].derived == ["aag", "aah"]
    assert index["aag"].root == "aaf"
    assert index["aag"].gloss == "destruction"
    assert index["xaa"].language == "arc"
    assert read_aug_index(tmp_path / "aug.xml") == {"6": "aaf", "122a": "aag"}
