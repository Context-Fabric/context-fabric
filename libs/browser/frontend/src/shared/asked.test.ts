import { describe, expect, it } from "vitest";
import type { Proposal } from "../types";
import { askedOf, decodeAsked, describeAsked, encodeAsked, trialCount, waiting } from "./asked";

const proposal = (more: Partial<Proposal>): Proposal => ({
  id: "p1",
  corpus: "bhsa",
  template: "word lex=>LHJM/",
  request: "all cases of >LHJM/",
  explanation: "",
  model: "Claude",
  created: "2026-10-16T12:00:00+00:00",
  count: 2601,
  atLeast: false,
  state: "new",
  ...more,
});

describe("waiting", () => {
  it("keeps new proposals for the corpora shown", () => {
    const list = [proposal({ id: "a" }), proposal({ id: "b", state: "run" }), proposal({ id: "c", corpus: "sp" })];
    expect(waiting(list, ["bhsa"]).map((p) => p.id)).toEqual(["a"]);
    expect(waiting(list, ["bhsa", "sp"]).map((p) => p.id)).toEqual(["a", "c"]);
  });
});

describe("provenance", () => {
  it("goes into the URL and back", () => {
    const asked = askedOf(proposal({}));
    expect(decodeAsked(encodeAsked(asked))).toEqual(asked);
    expect(encodeAsked(null)).toBeNull();
    expect(decodeAsked("not json")).toBeNull();
    expect(decodeAsked(null)).toBeNull();
  });

  it("is described in words", () => {
    expect(describeAsked(askedOf(proposal({})))).toBe(
      "Proposed by Claude, 16 October 2026, for: “all cases of >LHJM/”",
    );
    expect(describeAsked(askedOf(proposal({ model: null })))).toMatch(/^Proposed by a language model/);
    const asked = askedOf(proposal({}));
    expect(describeAsked(asked, "word lex=>LHJM/")).not.toMatch(/edited/);
    expect(describeAsked(asked, "word lex=>LHJM/ nu=pl")).toMatch(/\(edited\)$/);
  });
});

describe("trialCount", () => {
  it("says how many the trial found", () => {
    expect(trialCount(proposal({}))).toBe("2,601 results");
    expect(trialCount(proposal({ count: 1 }))).toBe("1 result");
    expect(trialCount(proposal({ count: 10000, atLeast: true }))).toBe("at least 10,000 results");
    expect(trialCount(proposal({ count: null }))).toBeNull();
  });
});
