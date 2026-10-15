import { describe, expect, it } from "vitest";
import {
  commonToParallel,
  commonToWorkspace,
  EMPTY,
  mergeCommon,
  parallelToCommon,
  splitRef,
  workspaceToCommon,
  type Common,
} from "./common";
import { emptyPane, type Workspace } from "./state";
import { canHide, uiOfUrl } from "./ui";

const reading: Common = {
  ...EMPTY,
  corpora: ["bhsa", "sp", "dss"],
  ref: "Genesis 1",
  verse: 3,
  diff: true,
};

const refs = { bhsa: "Genesis 1", sp: "Genesis 1", dss: null };

describe("splitRef", () => {
  it("splits off the verse", () => {
    expect(splitRef("Genesis 1:3")).toEqual({ chapter: "Genesis 1", verse: 3 });
    expect(splitRef("1_Samuel 3")).toEqual({ chapter: "1_Samuel 3", verse: null });
  });
});

describe("panes", () => {
  const ws: Workspace = {
    panes: [
      { ...emptyPane("bhsa"), ref: "Genesis 1:3" },
      { ...emptyPane("sp"), ref: "Genesis 1:3" },
      { ...emptyPane("dss"), by: "books", ref: "Genesis 1:3" },
    ],
    sync: true,
    diff: true,
  };

  it("describe what they show", () => {
    expect(workspaceToCommon(ws, "Genesis 1:3")).toEqual({
      corpora: ["bhsa", "sp", "dss"],
      ref: "Genesis 1",
      verse: 3,
      search: null,
      searching: false,
      lexicon: null,
      diff: true,
    });
  });

  it("describe a search pane as the search", () => {
    const searching = { ...ws, panes: [...ws.panes, { ...emptyPane("bhsa"), view: "search" as const, query: "word", search: "s1" }] };
    const common = workspaceToCommon(searching, "Genesis 1:3");
    expect(common.search).toEqual({ corpus: "bhsa", q: "word", s: "s1" });
    expect(common.searching).toBe(true);
    // a corpus in two panes counts once
    expect(common.corpora).toEqual(["bhsa", "sp", "dss"]);
  });

  it("are made from a description, kept in step", () => {
    const made = commonToWorkspace(reading, refs, ["dss"]);
    expect(made.sync).toBe(true);
    expect(made.diff).toBe(true);
    expect(made.panes.map((p) => [p.corpus, p.view, p.by, p.ref])).toEqual([
      // one pane with the passage; synchronised reading fills in the others
      ["bhsa", "read", "sections", "Genesis 1"],
      ["sp", "read", "sections", null],
      ["dss", "read", "books", null],
    ]);
  });

  it("show the search in its corpus' pane, and keep the passage in another", () => {
    const common = { ...reading, search: { corpus: "bhsa", q: "word", s: "s1" }, searching: true };
    const made = commonToWorkspace(common, refs, ["dss"]);
    expect(made.panes.map((p) => [p.corpus, p.view, p.ref])).toEqual([
      ["bhsa", "search", null],
      ["sp", "read", "Genesis 1"],
      ["dss", "read", null],
    ]);
    expect(made.panes[0].query).toBe("word");
  });

  it("have at most four", () => {
    const many = { ...reading, corpora: ["a", "b", "c", "d", "e"] };
    expect(commonToWorkspace(many, {}, []).panes).toHaveLength(4);
  });
});

describe("parallel", () => {
  it("goes there and back", () => {
    const common: Common = {
      ...reading,
      search: { corpus: "bhsa", q: "word", s: "s1" },
      searching: false,
      lexicon: { corpus: "bhsa", lex: 7 },
    };
    expect(mergeCommon(null, parallelToCommon(commonToParallel(common)))).toEqual(common);
  });

  it("does not show the lexicon while searching", () => {
    const common = { ...reading, search: { corpus: "bhsa", q: "w", s: null }, searching: true, lexicon: { corpus: "bhsa", lex: 7 } };
    const parallel = commonToParallel(common);
    expect(parallel.searching).toBe(true);
    expect(parallel.lexicon).toBeNull();
  });
});

describe("keeping what an interface cannot show", () => {
  it("keeps the search when the next description has none", () => {
    const kept = { ...reading, search: { corpus: "bhsa", q: "word", s: "s1" } };
    const merged = mergeCommon(kept, { ref: "Exodus 2", verse: null, searching: false });
    expect(merged.search).toEqual(kept.search);
    expect(merged.ref).toBe("Exodus 2");
    expect(merged.verse).toBeNull();
  });
});

describe("interfaces", () => {
  it("are found by their URL", () => {
    const at = (pathname: string, search = "") => uiOfUrl({ pathname, search });
    expect(at("/", "?c0=bhsa")).toBe("classic");
    expect(at("/Genesis/1", "?texts=bhsa")).toBe("parallel");
    expect(at("/", "?ui=parallel")).toBe("parallel");
    expect(at("/read/bhsa/Genesis/1")).toBe("reader");
    expect(at("/interlinear")).toBe("interlinear");
    expect(at("/concordance/bhsa", "?q=word")).toBe("concordance");
    // a book that starts like a prefix is still a passage
    expect(at("/readings/1")).toBe("parallel");
    expect(at("/")).toBeNull();
  });

  it("can be hidden, but not the one in use, nor both Panes and Parallel", () => {
    expect(canHide("reader", [], "parallel")).toBe(true);
    expect(canHide("parallel", [], "parallel")).toBe(false);
    expect(canHide("classic", [], "parallel")).toBe(true);
    expect(canHide("classic", ["parallel"], "reader")).toBe(false);
  });
});
