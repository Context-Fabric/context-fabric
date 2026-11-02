import { describe, expect, it } from "vitest";
import { commonToConcordance, concordanceToCommon, EMPTY, mergeCommon } from "../common";
import { linesToTsv, parseConcordance, serializeConcordance, toggleFilter, type Concordance } from "./url";

const state: Concordance = {
  corpus: "bhsa",
  q: "word lex=HLK[",
  s: "abc",
  ask: '["all cases of HLK[","Claude","2026-10-16T12:00:00+00:00","word lex=HLK["]',
  sort: "L1.text",
  filters: [
    ["vs", "hif"],
    ["book", "Genesis"],
  ],
  per: "verse",
  sample: 100,
  seed: 3,
  context: 10,
  tab: "collocations",
  texts: ["sp"],
};

describe("the Concordance's URL", () => {
  it("goes there and back", () => {
    const url = serializeConcordance(state);
    const [pathname, search] = url.split("?");
    expect(pathname).toBe("/concordance/bhsa");
    expect(parseConcordance({ pathname, search: `?${search}` })).toEqual(state);
  });

  it("leaves out the defaults", () => {
    const plain = {
      ...state,
      s: null,
      ask: null,
      sort: null,
      filters: [],
      per: "result" as const,
      sample: null,
      seed: 1,
      context: 6,
      tab: "lines" as const,
      texts: null,
    };
    expect(serializeConcordance(plain)).toBe("/concordance/bhsa?q=word+lex%3DHLK%5B");
  });

  it("keeps a colon in a filter value", () => {
    const parsed = parseConcordance({ pathname: "/concordance/sp", search: "?f=book:2:Kings,nu:sg" });
    expect(parsed.filters).toEqual([
      ["book", "2:Kings"],
      ["nu", "sg"],
    ]);
  });
});

describe("filters", () => {
  it("are added, replaced per feature, and taken away", () => {
    let filters = toggleFilter([], "vs", "qal");
    expect(filters).toEqual([["vs", "qal"]]);
    filters = toggleFilter(filters, "vs", "hif");
    expect(filters).toEqual([["vs", "hif"]]);
    filters = toggleFilter(filters, "book", "Genesis");
    expect(toggleFilter(filters, "vs", "hif")).toEqual([["book", "Genesis"]]);
  });
});

describe("downloading lines", () => {
  it("gives a table with passage, before, hit and after", () => {
    const tsv = linesToTsv([
      {
        ref: "Genesis 2:14",
        left: [{ text: "הוּא" }, { text: "הַ" }],
        hit: [{ text: "הֹלֵךְ" }],
        right: [{ text: "קִדְמַת\t" }],
      },
    ]);
    expect(tsv).toBe("passage\tbefore\thit\tafter\nGenesis 2:14\tהוּא הַ\tהֹלֵךְ\tקִדְמַת\n");
  });
});

describe("switching to and from the Concordance", () => {
  it("describes its search", () => {
    expect(concordanceToCommon(state)).toEqual({
      corpora: ["bhsa", "sp"],
      search: { corpus: "bhsa", q: "word lex=HLK[", s: "abc", ask: state.ask },
      searching: true,
    });
  });

  it("shows the search it is given, in its corpus", () => {
    const common = mergeCommon(null, {
      ...EMPTY,
      corpora: ["bhsa", "sp"],
      search: { corpus: "sp", q: "word", s: null },
    });
    const made = commonToConcordance(common, "bhsa");
    expect([made.corpus, made.q, made.s, made.texts]).toEqual(["sp", "word", null, ["bhsa"]]);
    // without a search: an empty one in the corpus given
    expect(commonToConcordance(EMPTY, "bhsa").corpus).toBe("bhsa");
  });
});
