// The Reader keeps its state in the URL:
// `/read/bhsa/Genesis/1?v=3&panel=parallels&mode=running&texts=sp,dss&diff=1`.
// The passage is a chapter of the corpus being read, in its own numbering.

export type Panel = "parallels" | "words" | "lexicon" | "search" | "notes";
export const PANELS: Panel[] = ["parallels", "words", "lexicon", "search", "notes"];

export type Mode = "verses" | "running";

export interface Reader {
  /** The corpus being read. */
  corpus: string | null;
  /** The chapter (`Genesis 1`), as that corpus names it. */
  ref: string | null;
  verse: number | null;
  /** The tab of the resource panel, or null when it is closed. */
  panel: Panel | null;
  mode: Mode;
  /** The corpora shown under *Parallels*. Null: not chosen yet. */
  texts: string[] | null;
  diff: boolean;
}

export const PREFIX = "/read";

export function parseReader(location: { pathname: string; search: string }): Reader {
  const parts = location.pathname
    .slice(PREFIX.length)
    .split("/")
    .filter(Boolean)
    .map((p) => decodeURIComponent(p));
  const [corpus, ...rest] = parts;
  // spaces in book names are underscores in the corpus (1 Samuel is 1_Samuel)
  const ref = rest.length ? rest.map((p) => p.replace(/ /g, "_")).join(" ") : null;
  const p = new URLSearchParams(location.search);
  const verse = Number(p.get("v") ?? NaN);
  const panel = p.get("panel");
  const texts = p.get("texts");
  return {
    corpus: corpus ?? null,
    ref,
    verse: Number.isInteger(verse) ? verse : null,
    panel: PANELS.includes(panel as Panel) ? (panel as Panel) : null,
    mode: p.get("mode") === "running" ? "running" : "verses",
    texts: texts === null ? null : texts.split(",").filter(Boolean),
    diff: p.get("diff") === "1",
  };
}

export function serializeReader(state: Reader): string {
  const parts = [PREFIX];
  if (state.corpus) {
    parts.push(encodeURIComponent(state.corpus));
    if (state.ref) parts.push(...state.ref.split(" ").map((p) => encodeURIComponent(p)));
  }
  const p = new URLSearchParams();
  if (state.verse !== null) p.set("v", String(state.verse));
  if (state.panel) p.set("panel", state.panel);
  if (state.mode === "running") p.set("mode", "running");
  if (state.texts !== null) p.set("texts", state.texts.join(","));
  if (state.diff) p.set("diff", "1");
  const search = p.toString().replace(/%2C/g, ",");
  return search ? `${parts.join("/")}?${search}` : parts.join("/");
}

/** Whether a verse closes a paragraph: its text ends with the petuhah (פ) or
 * setumah (ס) mark, as in the BHSA. */
export function closesParagraph(text: string): boolean {
  return /\s[פס]\s*$/.test(text);
}
