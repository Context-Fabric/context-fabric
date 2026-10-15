// The Concordance keeps its state in the URL:
// `/concordance/bhsa?q=word+lex%3DHLK%5B&s=<id>&sort=L1.text&f=vs:qal,book:Genesis&per=verse&sample=100&ctx=6&tab=lines`.

export type Tab = "lines" | "collocations" | "statistics";
export const TABS: Tab[] = ["lines", "collocations", "statistics"];

export interface Concordance {
  corpus: string | null;
  /** The search template, and the search (its id on the server). */
  q: string;
  s: string | null;
  /** `<position>.<feature>`: hit.text, L1.lex, R2.sp; null: in result order. */
  sort: string | null;
  /** Features of the hit's first word that results must have. */
  filters: [string, string][];
  per: "result" | "verse";
  sample: number | null;
  seed: number;
  /** Words of context on each side. */
  context: number;
  tab: Tab;
  /** Other corpora, carried along for the other interfaces. */
  texts: string[] | null;
}

export const PREFIX = "/concordance";
export const CONTEXTS = [3, 6, 10];

export function parseConcordance(location: { pathname: string; search: string }): Concordance {
  const [corpus] = location.pathname
    .slice(PREFIX.length)
    .split("/")
    .filter(Boolean)
    .map((p) => decodeURIComponent(p));
  const p = new URLSearchParams(location.search);
  const sample = Number(p.get("sample") ?? NaN);
  const seed = Number(p.get("seed") ?? NaN);
  const context = Number(p.get("ctx") ?? NaN);
  const tab = p.get("tab");
  const texts = p.get("texts");
  return {
    corpus: corpus ?? null,
    q: p.get("q") ?? "",
    s: p.get("s"),
    sort: p.get("sort"),
    filters: (p.get("f") ?? "")
      .split(",")
      .filter((f) => f.includes(":"))
      .map((f) => {
        const at = f.indexOf(":");
        return [f.slice(0, at), f.slice(at + 1)] as [string, string];
      }),
    per: p.get("per") === "verse" ? "verse" : "result",
    sample: Number.isInteger(sample) && sample > 0 ? sample : null,
    seed: Number.isInteger(seed) ? seed : 1,
    context: CONTEXTS.includes(context) ? context : 6,
    tab: TABS.includes(tab as Tab) ? (tab as Tab) : "lines",
    texts: texts === null ? null : texts.split(",").filter(Boolean),
  };
}

export function serializeConcordance(state: Concordance): string {
  const path = state.corpus ? `${PREFIX}/${encodeURIComponent(state.corpus)}` : PREFIX;
  const p = new URLSearchParams();
  if (state.q) p.set("q", state.q);
  if (state.s) p.set("s", state.s);
  if (state.sort) p.set("sort", state.sort);
  if (state.filters.length) p.set("f", filterParam(state.filters));
  if (state.per === "verse") p.set("per", "verse");
  if (state.sample !== null) p.set("sample", String(state.sample));
  if (state.sample !== null && state.seed !== 1) p.set("seed", String(state.seed));
  if (state.context !== 6) p.set("ctx", String(state.context));
  if (state.tab !== "lines") p.set("tab", state.tab);
  if (state.texts !== null) p.set("texts", state.texts.join(","));
  const search = p.toString().replace(/%2C/g, ",").replace(/%3A/g, ":");
  return search ? `${path}?${search}` : path;
}

/** The filters as the server takes them: `vs:qal,book:Genesis`. */
export function filterParam(filters: [string, string][]): string {
  return filters.map(([f, v]) => `${f}:${v}`).join(",");
}

/** Add a filter, or take it away when it is there already. */
export function toggleFilter(filters: [string, string][], feature: string, value: string): [string, string][] {
  const has = filters.some(([f, v]) => f === feature && v === value);
  if (has) return filters.filter(([f, v]) => !(f === feature && v === value));
  // one value per feature: a new value replaces the old one
  return [...filters.filter(([f]) => f !== feature), [feature, value]];
}

/** Lines as a TSV table: passage, context before, hit, context after. */
export function linesToTsv(
  lines: { ref: string | null; left: { text: string }[]; hit: { text: string }[]; right: { text: string }[] }[],
): string {
  const clean = (s: string) => s.replace(/[\t\n\r]+/g, " ").trim();
  const words = (ws: { text: string }[]) => clean(ws.map((w) => w.text).join(" "));
  const rows = lines.map((l) => [clean(l.ref ?? ""), words(l.left), words(l.hit), words(l.right)].join("\t"));
  return ["passage\tbefore\thit\tafter", ...rows].join("\n") + "\n";
}
