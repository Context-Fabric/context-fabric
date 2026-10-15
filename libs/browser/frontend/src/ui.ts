import { api } from "./api";

// Five interfaces on the same server, each with its own URLs:
//
//   Panes        /?c0=bhsa&ref0=...        (its id is "classic")
//   Parallel     /Genesis/1?texts=...      (or / with texts= or ui=parallel)
//   Reader       /read/...
//   Interlinear  /interlinear/...
//   Concordance  /concordance/...
//
// Switching keeps the passage, the corpora and the settings (see common.ts),
// and the last choice is remembered per browser.

export type Ui = "classic" | "parallel" | "reader" | "interlinear" | "concordance";

export interface UiInfo {
  id: Ui;
  name: string;
  about: string;
  /** The URL prefix of the interface; Panes and Parallel have none. */
  prefix: string | null;
  /** Built and offered in the menu. */
  ready: boolean;
}

export const INTERFACES: UiInfo[] = [
  {
    id: "classic",
    name: "Panes",
    about: "Panes side by side, each with its own corpus, passage and view",
    prefix: null,
    ready: true,
  },
  {
    id: "parallel",
    name: "Parallel",
    about: "A column per corpus and a row per verse, after Parabible",
    prefix: null,
    ready: true,
  },
  {
    id: "reader",
    name: "Reader",
    about: "One text to read, with parallels, words and notes on request",
    prefix: "/read",
    ready: true,
  },
  {
    id: "interlinear",
    name: "Interlinear",
    about: "Word by word, with transliteration, gloss and morphology",
    prefix: "/interlinear",
    ready: true,
  },
  {
    id: "concordance",
    name: "Concordance",
    about: "Search results in context, sorted and counted",
    prefix: "/concordance",
    ready: false,
  },
];

export const UI_IDS = INTERFACES.map((i) => i.id);

export function isUi(value: unknown): value is Ui {
  return typeof value === "string" && (UI_IDS as string[]).includes(value);
}

const KEY = "cfabric-browser/ui";
const HIDDEN_KEY = "cfabric-browser/hidden-uis";

/** The interface a URL belongs to, or null for the bare start page. */
export function uiOfUrl(location: { pathname: string; search: string } = window.location): Ui | null {
  const path = location.pathname;
  for (const ui of INTERFACES) {
    if (ui.prefix && (path === ui.prefix || path.startsWith(`${ui.prefix}/`))) return ui.id;
  }
  if (path !== "/" && path !== "/index.html") return "parallel";
  const p = new URLSearchParams(location.search);
  if (p.get("ui") === "parallel" || p.has("texts")) return "parallel";
  if (location.search.length > 1) return "classic";
  return null;
}

export function rememberedUi(): Ui | null {
  try {
    const stored = window.localStorage.getItem(KEY);
    return isUi(stored) ? stored : null;
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

/** The interfaces this browser leaves out of the menu (null: not chosen here yet). */
export function hiddenUis(): Ui[] | null {
  try {
    const stored = window.localStorage.getItem(HIDDEN_KEY);
    if (stored === null) return null;
    const list = JSON.parse(stored);
    return Array.isArray(list) ? list.filter(isUi) : null;
  } catch {
    return null;
  }
}

export function rememberHiddenUis(hidden: Ui[]): void {
  try {
    window.localStorage.setItem(HIDDEN_KEY, JSON.stringify(hidden));
  } catch {
    // not remembered; the menu still changes now
  }
}

/**
 * Which interfaces may be hidden, given those hidden already: never the one in
 * use, and at least one of Panes and Parallel stays.
 */
export function canHide(ui: Ui, hidden: Ui[], current: Ui): boolean {
  if (ui === current) return false;
  if (ui === "classic" || ui === "parallel") {
    const other = ui === "classic" ? "parallel" : "classic";
    return !hidden.includes(other);
  }
  return true;
}

/** Go to a URL of any interface, without reloading the page. */
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
