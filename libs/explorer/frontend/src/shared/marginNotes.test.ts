import { describe, expect, it } from "vitest";
import { marginNotes } from "./marginNotes";

describe("marginNotes", () => {
  it("splits the notes and their catchwords", () => {
    expect(marginNotes("beginne — Van den tijd. | schiep — Scheppen, is | Without a catchword")).toEqual([
      { catchword: "beginne", text: "Van den tijd." },
      { catchword: "schiep", text: "Scheppen, is" },
      { catchword: null, text: "Without a catchword" },
    ]);
  });
  it("keeps a dash inside a note", () => {
    expect(marginNotes("hemel — of — hemelen")).toEqual([{ catchword: "hemel", text: "of — hemelen" }]);
  });
});
