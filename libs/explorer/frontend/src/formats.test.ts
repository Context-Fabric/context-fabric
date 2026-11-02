import { describe, expect, it } from "vitest";
import { formatOptions, shownFormat } from "./formats";

describe("formatOptions", () => {
  it("names the BHSA's two formats", () => {
    expect(formatOptions(["text-orig-full", "text-orig-plain"])).toEqual([
      { value: "text-orig-full", label: "Vocalized" },
      { value: "text-orig-plain", label: "Unvocalized" },
    ]);
  });

  it("offers nothing when there is no choice", () => {
    expect(formatOptions(["text-orig-full"])).toEqual([]);
    expect(formatOptions([])).toEqual([]);
  });
});

describe("shownFormat", () => {
  const formats = ["text-orig-full", "text-orig-plain"];

  it("keeps a choice the corpus offers", () => {
    expect(shownFormat("text-orig-plain", formats, "text-orig-full")).toBe("text-orig-plain");
  });

  it("gives the default for a format no longer offered, or none chosen", () => {
    expect(shownFormat("text-trans-full", formats, "text-orig-full")).toBe("text-orig-full");
    expect(shownFormat(null, formats, "text-orig-full")).toBe("text-orig-full");
  });
});
