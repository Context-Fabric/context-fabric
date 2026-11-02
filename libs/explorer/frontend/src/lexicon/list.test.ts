import { describe, expect, it } from "vitest";
import type { LexiconEntry } from "../types";
import { shownLexemes, type Shown } from "./list";

const lexeme = (lex: string, more: Partial<LexiconEntry>): LexiconEntry => ({
  node: 0,
  lex,
  letter: null,
  freq: 1,
  language: "Hebrew",
  ...more,
});

const ENTRIES = [
  lexeme(">BD[", { letter: "א", gloss: "perish", bdbShort: "perish", bdbOrder: 2, bdbLetter: "א", freq: 185 }),
  lexeme("MCPV/", { letter: "מ", gloss: "justice", bdbShort: "judgement", bdbOrder: 30, bdbLetter: "ש", freq: 425 }),
  lexeme("CPV[", { letter: "ש", gloss: "judge", bdbShort: "judge, govern", bdbOrder: 29, bdbLetter: "ש", freq: 203 }),
  lexeme(">CR/", { letter: "א", gloss: "happiness", freq: 45 }),
];
const ALL: Shown = { language: "Hebrew", letter: "א", sp: "", find: "", order: "alpha" };

describe("shownLexemes", () => {
  it("lists a letter by the lexeme's first letter", () => {
    expect(shownLexemes(ENTRIES, ALL).map((e) => e.lex)).toEqual([">BD[", ">CR/"]);
  });

  it("by root lists under the root's letter in BDB's order, unlinked ones last", () => {
    const shin = shownLexemes(ENTRIES, { ...ALL, letter: "ש", order: "root" });
    expect(shin.map((e) => e.lex)).toEqual(["CPV[", "MCPV/"]);
    const alef = shownLexemes(ENTRIES, { ...ALL, order: "root" });
    expect(alef.map((e) => e.lex)).toEqual([">BD[", ">CR/"]);
  });

  it("finds by gloss, by BDB's definition and by Hebrew", () => {
    expect(shownLexemes(ENTRIES, { ...ALL, find: "govern" }).map((e) => e.lex)).toEqual(["CPV["]);
    expect(shownLexemes(ENTRIES, { ...ALL, find: "judge" }).map((e) => e.lex)).toEqual(["MCPV/", "CPV["]);
  });

  it("orders by frequency", () => {
    expect(shownLexemes(ENTRIES, { ...ALL, order: "frequency" }).map((e) => e.lex)).toEqual([">BD[", ">CR/"]);
  });
});
