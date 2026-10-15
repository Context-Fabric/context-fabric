import { api } from "./api";

// Two interfaces on the same server: the classic one (panes side by side, its
// state in the query string of `/`) and the parallel one (a column per corpus,
// a row per verse, at `/<book>/<chapter>`). Switching keeps the passage, the
// corpora and the settings, and the last choice is remembered per browser.

export type Ui = "classic" | "parallel";

const KEY = "cfabric-browser/ui";

/** The interface a URL belongs to, or null for the bare start page. */
export function uiOfUrl(location: Location = window.location): Ui | null {
  if (location.pathname !== "/" && location.pathname !== "/index.html") return "parallel";
  const p = new URLSearchParams(location.search);
  if (p.get("ui") === "parallel" || p.has("texts")) return "parallel";
  if (location.search.length > 1) return "classic";
  return null;
}

export function rememberedUi(): Ui | null {
  try {
    const stored = window.localStorage.getItem(KEY);
    return stored === "classic" || stored === "parallel" ? stored : null;
  } catch {
    return null;
  }
}

export function rememberUi(ui: Ui): void {
  try {
    window.localStorage.setItem(KEY, ui);
  } catch {
    // not remembered; the switch still works now
  }
}

/** Go to a URL of either interface, without reloading the page. */
export function navigate(url: string): void {
  window.history.pushState(null, "", url);
  window.dispatchEvent(new PopStateEvent("popstate"));
}

/** The ids (`org/repo`) of the corpus whose verse numbering the rows follow. */
export const REFERENCE_IDS = ["etcbc/bhsa"];

/** The open corpus whose verse numbering the parallel rows follow (the BHSA), if any. */
export async function openReference(): Promise<string | null> {
  const { corpora } = await api.corpora();
  const cards = await Promise.allSettled(corpora.map((c) => api.corpus(c.name)));
  for (const card of cards) {
    if (card.status === "fulfilled" && REFERENCE_IDS.includes(card.value.id?.toLowerCase() ?? "")) {
      return card.value.name;
    }
  }
  return null;
}

/** The chapter of a passage: `Genesis 1:3` becomes `Genesis 1`. */
export function chapterOf(ref: string): string {
  return ref.split(":")[0];
}

/** `ref` of corpus `from` in corpus `to`, or null when `to` has no counterpart. */
export async function alignRef(from: string, ref: string, to: string): Promise<string | null> {
  if (from === to) return ref;
  try {
    const aligned = await api.align(to, from, ref);
    return aligned.node === null ? null : aligned.ref;
  } catch {
    return null;
  }
}
