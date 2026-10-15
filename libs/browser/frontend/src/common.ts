import type { Parallel } from "./parallel/state";
import type { Concordance } from "./concordance/url";
import { DEFAULT_LINES, type Interlinear } from "./interlinear/url";
import type { Reader } from "./reader/url";
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
    // panes without a passage say nothing about it: the one kept from before stays
    ref: where?.chapter,
    verse: where ? where.verse : undefined,
    // likewise without a search pane: the search kept from before stays, though not shown
    search: search ? { corpus: search.corpus!, q: search.query, s: search.search } : undefined,
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
    // a pane of its own when the pane it would take holds the only passage
    const only = at === reading && !panes.some((p, i) => i !== at && withRef(p));
    if (at >= 0 && !(only && panes.length < MAX_PANES)) panes[at] = pane;
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

// Reader

/**
 * What the Reader shows. `aligned` is its chapter in the reference corpus'
 * numbering (the caller aligns it). The Reader shows no search or lexicon, so
 * those are left as they were. Without chosen parallels it shows all `open`
 * corpora (the same holds for the Interlinear and the Concordance).
 */
export function readerToCommon(r: Reader, aligned: string | null, open: string[] = []): Partial<Common> {
  return {
    corpora: r.corpus ? [...new Set([r.corpus, ...(r.texts ?? open)])] : undefined,
    ref: aligned,
    verse: r.verse,
    searching: false,
    diff: r.diff,
  };
}

/**
 * A description in the Reader: the first corpus is read, the others are its
 * parallels. `ref` is the chapter in that corpus' own numbering (the caller
 * aligns it).
 */
export function commonToReader(c: Common, corpus: string | null, ref: string | null): Reader {
  return {
    corpus,
    ref,
    verse: c.verse,
    panel: null,
    mode: "verses",
    texts: corpus ? c.corpora.filter((t) => t !== corpus) : null,
    diff: c.diff,
  };
}

// Interlinear

/** What the Interlinear shows. `aligned` is its chapter in the reference corpus'
 * numbering (the caller aligns it). It shows no search or lexicon. */
export function interlinearToCommon(s: Interlinear, aligned: string | null, open: string[] = []): Partial<Common> {
  const corpora = s.corpus ? [s.corpus, ...(s.under ? [s.under] : []), ...(s.texts ?? open)] : [];
  return {
    corpora: corpora.length ? [...new Set(corpora)] : undefined,
    ref: aligned,
    verse: s.verse,
    searching: false,
  };
}

/** A description in the Interlinear: `corpus` (the BHSA, when open) is shown;
 * the other corpora are carried along. `ref` is in that corpus' numbering. */
export function commonToInterlinear(c: Common, corpus: string | null, ref: string | null): Interlinear {
  return {
    corpus,
    ref,
    verse: c.verse,
    lines: DEFAULT_LINES,
    under: null,
    structure: false,
    hide: null,
    colour: null,
    texts: corpus ? c.corpora.filter((t) => t !== corpus) : null,
  };
}

// Concordance

/** What the Concordance shows: a search. */
export function concordanceToCommon(s: Concordance, open: string[] = []): Partial<Common> {
  const corpora = s.corpus ? [s.corpus, ...(s.texts ?? open)] : [];
  return {
    corpora: corpora.length ? [...new Set(corpora)] : undefined,
    search: s.corpus ? { corpus: s.corpus, q: s.q, s: s.s } : undefined,
    // an empty concordance doesn't open the search elsewhere
    searching: s.corpus !== null && s.q.trim() !== "",
  };
}

/** A description in the Concordance: its search, else an empty search in `corpus`. */
export function commonToConcordance(c: Common, corpus: string | null): Concordance {
  const searchCorpus = c.search?.corpus ?? corpus;
  return {
    corpus: searchCorpus,
    q: c.search?.q ?? "",
    s: c.search?.s ?? null,
    sort: null,
    filters: [],
    per: "result",
    sample: null,
    seed: 1,
    context: 6,
    tab: "lines",
    texts: searchCorpus ? c.corpora.filter((t) => t !== searchCorpus) : null,
  };
}
