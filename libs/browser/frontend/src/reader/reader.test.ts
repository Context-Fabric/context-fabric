import { describe, expect, it } from "vitest";
import { commonToReader, EMPTY, mergeCommon, readerToCommon } from "../common";
import { mergeNotes } from "../shared/notes";
import type { RenderTree } from "../types";
import { textOf, wordsOf } from "./tree";
import { closesParagraph, parseReader, serializeReader, type Reader } from "./url";

const reader: Reader = {
  corpus: "bhsa",
  ref: "1_Samuel 3",
  verse: 4,
  panel: "parallels",
  mode: "running",
  texts: ["sp", "dss"],
  diff: true,
};

describe("the Reader's URL", () => {
  it("goes there and back", () => {
    const url = serializeReader(reader);
    expect(url).toBe("/read/bhsa/1_Samuel/3?v=4&panel=parallels&mode=running&texts=sp,dss&diff=1");
    const [pathname, search] = url.split("?");
    expect(parseReader({ pathname, search: `?${search}` })).toEqual(reader);
  });

  it("starts empty", () => {
    expect(parseReader({ pathname: "/read", search: "" })).toEqual({
      corpus: null,
      ref: null,
      verse: null,
      panel: null,
      mode: "verses",
      texts: null,
      diff: false,
    });
    // an unknown panel is no panel
    expect(parseReader({ pathname: "/read/sp/Genesis/1", search: "?panel=maps" }).panel).toBeNull();
  });
});

describe("switching to and from the Reader", () => {
  it("describes what it shows, leaving the search alone", () => {
    const common = readerToCommon(reader, "1_Samuel 3");
    expect(common).toEqual({
      corpora: ["bhsa", "sp", "dss"],
      ref: "1_Samuel 3",
      verse: 4,
      searching: false,
      diff: true,
    });
    const kept = { ...EMPTY, search: { corpus: "bhsa", q: "word", s: null } };
    expect(mergeCommon(kept, common).search).toEqual(kept.search);
  });

  it("reads the first corpus, with the others as parallels", () => {
    const common = { ...EMPTY, corpora: ["sp", "bhsa"], ref: "Genesis 1", verse: 2 };
    expect(commonToReader(common, "sp", "Genesis 1")).toEqual({
      corpus: "sp",
      ref: "Genesis 1",
      verse: 2,
      panel: null,
      mode: "verses",
      texts: ["bhsa"],
      diff: false,
    });
  });
});

describe("paragraphs", () => {
  it("end with petuhah or setumah", () => {
    expect(closesParagraph(" לֵאמֹֽר׃ ס ")).toBe(true);
    expect(closesParagraph("יֹ֥ום אֶחָֽד׃ פ")).toBe(true);
    expect(closesParagraph("עֲבָדִֽ֑ים׃ ")).toBe(false);
    // a word ending in samekh is no setumah
    expect(closesParagraph("וַיָּנָס")).toBe(false);
  });
});

describe("render trees", () => {
  const frag = (node: number, type: string, text?: string, children?: RenderTree["fragments"]) => ({
    node,
    type,
    range: [node, node] as [number, number],
    start: "node" as const,
    end: "node" as const,
    text,
    children,
  });
  // a verse with two words; the second is cut into two fragments by a phrase boundary
  const tree = {
    node: 100,
    type: "verse",
    mode: "plain",
    fmt: "text-orig-full",
    direction: "rtl",
    heading: "Genesis 1:1",
    fragments: [
      frag(90, "phrase", undefined, [frag(1, "word", "בְּ"), frag(2, "word", "רֵאשִׁית ")]),
      frag(91, "phrase", undefined, [frag(2, "word", "")]),
    ],
  } as unknown as RenderTree;

  it("give the words in order, each once", () => {
    expect(wordsOf(tree, "word")).toEqual([1, 2]);
  });

  it("give the text as shown", () => {
    expect(textOf(tree)).toBe("בְּרֵאשִׁית ");
  });
});

describe("notes", () => {
  it("merge, the newer note winning", () => {
    const mine = { bhsa: { "Genesis 1:1": { text: "old", starred: false, updated: "2026-01-01" } } };
    const theirs = {
      bhsa: {
        "Genesis 1:1": { text: "new", starred: true, updated: "2026-02-01" },
        "Genesis 1:2": { text: "other", starred: false, updated: "2026-01-05" },
      },
    };
    const merged = mergeNotes(mine, theirs);
    expect(merged.bhsa["Genesis 1:1"].text).toBe("new");
    expect(Object.keys(merged.bhsa)).toEqual(["Genesis 1:1", "Genesis 1:2"]);
    expect(mergeNotes(theirs, mine).bhsa["Genesis 1:1"].text).toBe("new");
  });
});
