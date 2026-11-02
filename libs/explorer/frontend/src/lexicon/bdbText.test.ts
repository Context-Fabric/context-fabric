import { describe, expect, it } from "vitest";
import { abbreviationPattern, linkNote, preview, sefariaUrl, splitAbbreviations, stems } from "./bdbText";

describe("sefariaUrl", () => {
  it("links to the entry", () => {
    expect(sefariaUrl("BDB, אב")).toBe("https://www.sefaria.org/BDB%2C_%D7%90%D7%91");
  });
});

describe("splitAbbreviations", () => {
  const abbreviations = { We: "Wellhausen", Dr: "Driver", "Thes.": "Thesaurus", G: "one letter" };
  const pattern = abbreviationPattern(abbreviations);

  it("finds whole abbreviations, the longest first", () => {
    expect(splitAbbreviations("so We Dr; Thes. al.", pattern, abbreviations)).toEqual([
      "so ",
      { abbr: "We", meaning: "Wellhausen" },
      " ",
      { abbr: "Dr", meaning: "Driver" },
      "; ",
      { abbr: "Thes.", meaning: "Thesaurus" },
      " al.",
    ]);
  });

  it("leaves parts of words and one-letter ones alone", () => {
    expect(splitAbbreviations("Weird Drink G", pattern, abbreviations)).toEqual(["Weird Drink G"]);
  });

  it("works without abbreviations", () => {
    expect(splitAbbreviations("text", abbreviationPattern({}), {})).toEqual(["text"]);
  });
});

describe("linkNote", () => {
  it("says how a link was made when it matters", () => {
    expect(linkNote({ entries: [], how: "manual" })).toBe("Linked by hand.");
    expect(linkNote({ entries: [], how: "cited" })).toMatch(/verses BDB cites/);
    expect(linkNote({ entries: [], how: "oshb", share: 0.75 })).toMatch(/75%/);
    expect(linkNote({ entries: [], how: "oshb", share: 0.99 })).toBeNull();
  });
});

describe("stems", () => {
  it("divides an entry where its stems begin", () => {
    expect(stems("vb. go<br><strong>Qal</strong> Pf.<br/>Hiph. <br>")).toEqual(["vb. go", "<strong>Qal</strong> Pf.", "Hiph. "]);
  });

  it("previews a part by its first words", () => {
    expect(preview("<strong>Qal</strong>  Pf. הָלַךְ Gn 2:14")).toBe("Qal Pf. הָלַךְ Gn 2:14");
    expect(preview("a".repeat(100), 10)).toBe("aaaaaaaaaa…");
    expect(preview("Pi. (chiefly poet. &amp; late) &lt;Th")).toBe("Pi. (chiefly poet. & late) <Th");
  });
});
