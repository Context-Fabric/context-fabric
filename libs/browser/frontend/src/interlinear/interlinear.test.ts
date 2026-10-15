import { describe, expect, it } from "vitest";
import { commonToInterlinear, EMPTY, interlinearToCommon } from "../common";
import { DEFAULT_LINES, parseInterlinear, runs, serializeInterlinear, type Interlinear } from "./url";

const state: Interlinear = {
  corpus: "bhsa",
  ref: "Genesis 1",
  verse: 3,
  lines: ["word", "gloss", "f:vt"],
  under: "sp",
  structure: true,
  hide: "gloss",
  colour: "stem",
  texts: ["dss"],
};

describe("the Interlinear's URL", () => {
  it("goes there and back", () => {
    const url = serializeInterlinear(state);
    expect(url).toBe(
      "/interlinear/bhsa/Genesis/1?v=3&lines=word,gloss,f:vt&under=sp&struct=1&hide=gloss&colour=stem&texts=dss",
    );
    const [pathname, search] = url.split("?");
    expect(parseInterlinear({ pathname, search: `?${search}` })).toEqual(state);
  });

  it("leaves out the default lines, and ignores unknown ones", () => {
    const plain = {
      ...state,
      lines: DEFAULT_LINES,
      under: null,
      structure: false,
      hide: null,
      colour: null,
      texts: null,
      verse: null,
    };
    expect(serializeInterlinear(plain)).toBe("/interlinear/bhsa/Genesis/1");
    expect(
      parseInterlinear({ pathname: "/interlinear/bhsa/Genesis/1", search: "?lines=word,colour,f:" }).lines,
    ).toEqual(["word"]);
  });
});

describe("groups of words", () => {
  it("are runs of the same phrase or clause", () => {
    expect(runs([0, 0, 1, 1, 1, 0])).toEqual([
      [0, 0, 1],
      [1, 2, 4],
      [0, 5, 5],
    ]);
    expect(runs([])).toEqual([]);
  });
});

describe("switching to and from the Interlinear", () => {
  it("carries the other corpora along", () => {
    expect(interlinearToCommon(state, "Genesis 1")).toEqual({
      corpora: ["bhsa", "sp", "dss"],
      ref: "Genesis 1",
      verse: 3,
      searching: false,
    });
  });

  it("shows the corpus it is given, with the default lines", () => {
    const common = { ...EMPTY, corpora: ["sp", "bhsa", "dss"], ref: "Exodus 20", verse: 8 };
    const made = commonToInterlinear(common, "bhsa", "Exodus 20");
    expect(made.corpus).toBe("bhsa");
    expect(made.lines).toEqual(DEFAULT_LINES);
    expect(made.texts).toEqual(["sp", "dss"]);
    expect(made.verse).toBe(8);
  });
});
