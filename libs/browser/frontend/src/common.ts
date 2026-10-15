import type { Parallel } from "./parallel/state";
import { emptyPane, MAX_PANES, type Pane, type Workspace } from "./state";

// What is being looked at, apart from how an interface shows it. Switching goes
// from one interface to this description and on to the next, so that five
// interfaces need two conversions each instead of one per pair. What an
// interface cannot show stays in the description (kept for the session), so
// that switching back does not lose it.
//
// Passages are chapters in the numbering of the reference corpus (the BHSA),
// or of the first corpus when the reference corpus is not open.

export interface Common {
  /** The corpora being read, in order. */
  corpora: string[];
  /** The chapter (`Genesis 1`), and the verse in it. */
  ref: string | null;
  verse: number | null;
  /** The search, and whether it is what is shown. */
  search: { corpus: string; q: string; s: string | null } | null;
  searching: boolean;
  /** The lexicon shown (corpus, lexeme), if it is. */
  lexicon: { corpus: string; lex: number | null } | null;
  diff: boolean;
}

export const EMPTY: Common = {
  corpora: [],
  ref: null,
  verse: null,
  search: null,
  searching: false,
  lexicon: null,
  diff: false,
};

/** The description an interface gives, on top of what was kept from before. Only
 * what the interface says replaces what was kept. */
export function mergeCommon(kept: Common | null, current: Partial<Common>): Common {
  const merged = { ...EMPTY, ...kept };
  for (const [key, value] of Object.entries(current)) {
    if (value !== undefined) (merged as Record<string, unknown>)[key] = value;
  }
  return merged;
}

/** `Genesis 1:3` → `Genesis 1` and 3; `Genesis 1` → `Genesis 1` and null. */
export function splitRef(ref: string): { chapter: string; verse: number | null } {
  const [chapter, verse] = ref.split(":");
  const number = Number(verse);
  return { chapter, verse: verse !== undefined && Number.isInteger(number) ? number : null };
}

// Panes

/**
 * The panes as a description. `aligned` is the passage of the first reading pane
 * with a passage, in the reference corpus' numbering (the caller aligns it).
 */
export function workspaceToCommon(ws: Workspace, aligned: string | null): Partial<Common> {
  const corpora = [...new Set(ws.panes.map((p) => p.corpus).filter((c): c is string => Boolean(c)))];
  const search = ws.panes.find((p) => p.view === "search" && p.corpus);
  const lexicon = ws.panes.find((p) => p.view === "lexicon" && p.corpus);
  const where = aligned ? splitRef(aligned) : null;
  return {
    corpora,
    ref: where?.chapter ?? null,
    verse: where?.verse ?? null,
    search: search ? { corpus: search.corpus!, q: search.query, s: search.search } : null,
    searching: search !== undefined,
    lexicon: lexicon ? { corpus: lexicon.corpus!, lex: lexicon.lex } : null,
    diff: ws.diff,
  };
}

/**
 * A description as panes: a pane per corpus (at most four), kept in step.
 * `refs` gives the chapter per corpus in its own numbering (the caller aligns
 * them); `byBook` the corpora read by biblical book (the DSS).
 */
export function commonToWorkspace(c: Common, refs: Record<string, string | null>, byBook: string[]): Workspace {
  const corpora = c.corpora.slice(0, MAX_PANES);
  let panes: Pane[] = corpora.map((corpus) => ({
    ...emptyPane(corpus),
    by: byBook.includes(corpus) ? "books" : "sections",
  }));
  // synchronised reading fills in the other panes from the first that has a passage
  const withRef = (pane: Pane) => pane.by === "sections" && c.ref !== null && refs[pane.corpus!];
  const reading = panes.findIndex(withRef);
  if (reading >= 0) panes[reading].ref = refs[panes[reading].corpus!];

  const replace = (corpus: string, pane: Pane) => {
    const at = panes.findIndex((p) => p.corpus === corpus);
    if (at >= 0) panes[at] = pane;
    else panes = [pane, ...panes];
  };
  if (c.lexicon) replace(c.lexicon.corpus, { ...emptyPane(c.lexicon.corpus), view: "lexicon", lex: c.lexicon.lex });
  if (c.searching && c.search) {
    replace(c.search.corpus, {
      ...emptyPane(c.search.corpus),
      view: "search",
      query: c.search.q,
      search: c.search.s,
    });
  }
  // the passage stays with a reading pane when its pane became a search or the lexicon
  if (c.ref && !panes.some((p) => p.view === "read" && p.ref)) {
    const other = panes.findIndex((p) => p.view === "read" && withRef(p));
    if (other >= 0) panes[other] = { ...panes[other], ref: refs[panes[other].corpus!] };
  }
  panes = panes.slice(0, MAX_PANES);
  return {
    panes: panes.length ? panes : [emptyPane()],
    sync: panes.length > 1,
    diff: c.diff && panes.length > 1,
  };
}

// Parallel

export function parallelToCommon(p: Parallel): Partial<Common> {
  return {
    corpora: p.texts ?? undefined,
    ref: p.ref,
    verse: p.verse,
    search: p.sc ? { corpus: p.sc, q: p.q, s: p.s } : undefined,
    searching: p.searching,
    lexicon: p.lexicon ? { corpus: p.lexicon, lex: p.lex } : null,
    diff: p.diff,
  };
}

export function commonToParallel(c: Common): Parallel {
  return {
    ref: c.ref,
    texts: c.corpora.length ? c.corpora : null,
    diff: c.diff,
    verse: c.verse,
    lexicon: c.searching ? null : (c.lexicon?.corpus ?? null),
    lex: c.searching ? null : (c.lexicon?.lex ?? null),
    searching: c.searching,
    sc: c.search?.corpus ?? null,
    q: c.search?.q ?? "",
    s: c.search?.s ?? null,
  };
}
