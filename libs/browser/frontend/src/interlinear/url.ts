// The Interlinear keeps its state in the URL:
// `/interlinear/bhsa/Genesis/1?lines=word,translit,gloss,morph&under=sp&struct=1&hide=gloss&colour=pos&v=3`.
// The passage is a chapter of the corpus shown, in its own numbering.

/** The lines under each word: built-in ones, and `f:<feature>` for any feature. */
export type Line = "word" | "translit" | "lexeme" | "gloss" | "bdb" | "pos" | "morph" | `f:${string}`;

export const BUILT_IN: { line: Line; name: string }[] = [
  { line: "word", name: "Word" },
  { line: "translit", name: "Transliteration (of the lexeme)" },
  { line: "lexeme", name: "Lexeme" },
  { line: "gloss", name: "Gloss" },
  { line: "bdb", name: "BDB (its short definition)" },
  { line: "pos", name: "Part of speech" },
  { line: "morph", name: "Morphology" },
];

export const DEFAULT_LINES: Line[] = ["word", "translit", "gloss", "morph"];

export type Colour = "pos" | "stem";

export interface Interlinear {
  corpus: string | null;
  ref: string | null;
  verse: number | null;
  lines: Line[];
  /** Another corpus shown word by word under the words. */
  under: string | null;
  /** Phrases and clauses marked, with their function and type. */
  structure: boolean;
  /** A line hidden to practise: shown per word on a click. */
  hide: Line | null;
  colour: Colour | null;
  /** Other corpora, carried along for the other interfaces. */
  texts: string[] | null;
}

export const PREFIX = "/interlinear";

function isLine(value: string): value is Line {
  return BUILT_IN.some((b) => b.line === value) || (value.startsWith("f:") && value.length > 2);
}

export function parseInterlinear(location: { pathname: string; search: string }): Interlinear {
  const parts = location.pathname
    .slice(PREFIX.length)
    .split("/")
    .filter(Boolean)
    .map((p) => decodeURIComponent(p));
  const [corpus, ...rest] = parts;
  const p = new URLSearchParams(location.search);
  const verse = Number(p.get("v") ?? NaN);
  const lines = p.get("lines");
  const hide = p.get("hide");
  const colour = p.get("colour");
  const texts = p.get("texts");
  return {
    corpus: corpus ?? null,
    ref: rest.length ? rest.map((r) => r.replace(/ /g, "_")).join(" ") : null,
    verse: Number.isInteger(verse) ? verse : null,
    lines: lines === null ? DEFAULT_LINES : lines.split(",").filter(isLine),
    under: p.get("under"),
    structure: p.get("struct") === "1",
    hide: hide && isLine(hide) ? hide : null,
    colour: colour === "pos" || colour === "stem" ? colour : null,
    texts: texts === null ? null : texts.split(",").filter(Boolean),
  };
}

export function serializeInterlinear(state: Interlinear): string {
  const parts = [PREFIX];
  if (state.corpus) {
    parts.push(encodeURIComponent(state.corpus));
    if (state.ref) parts.push(...state.ref.split(" ").map((r) => encodeURIComponent(r)));
  }
  const p = new URLSearchParams();
  if (state.verse !== null) p.set("v", String(state.verse));
  if (state.lines.join(",") !== DEFAULT_LINES.join(",")) p.set("lines", state.lines.join(","));
  if (state.under) p.set("under", state.under);
  if (state.structure) p.set("struct", "1");
  if (state.hide) p.set("hide", state.hide);
  if (state.colour) p.set("colour", state.colour);
  if (state.texts !== null) p.set("texts", state.texts.join(","));
  const search = p.toString().replace(/%2C/g, ",").replace(/%3A/g, ":");
  return search ? `${parts.join("/")}?${search}` : parts.join("/");
}

/** Consecutive runs of the same group (phrase or clause) index: [group, from, to]. */
export function runs(groups: number[]): [number, number, number][] {
  const result: [number, number, number][] = [];
  groups.forEach((g, i) => {
    const last = result[result.length - 1];
    if (last && last[0] === g) last[2] = i;
    else result.push([g, i, i]);
  });
  return result;
}
