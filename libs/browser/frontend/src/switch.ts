import { serializeParallel, type Parallel } from "./parallel/state";
import { emptyPane, MAX_PANES, serialize, type Workspace } from "./state";
import { alignRef, chapterOf, navigate, openReference, rememberUi } from "./ui";

/** From the classic interface to the parallel one, with the same corpora and passage. */
export async function switchToParallel(ws: Workspace): Promise<void> {
  const texts = [...new Set(ws.panes.map((p) => p.corpus).filter((c): c is string => Boolean(c)))];
  const source = ws.panes.find((p) => p.view === "read" && p.corpus && p.ref);
  let ref: string | null = null;
  if (source) {
    const reference = await openReference();
    ref = reference ? await alignRef(source.corpus!, source.ref!, reference) : source.ref;
  }
  const verse = ref?.includes(":") ? Number(ref.split(":")[1]) : NaN;
  const lexicon = ws.panes.find((p) => p.view === "lexicon" && p.corpus);
  const state: Parallel = {
    ref: ref ? chapterOf(ref) : null,
    texts,
    diff: ws.diff,
    verse: Number.isInteger(verse) ? verse : null,
    lexicon: lexicon?.corpus ?? null,
    lex: lexicon?.lex ?? null,
    searching: false,
    sc: null,
    q: "",
    s: null,
  };
  // a search pane carries over as the search
  const search = ws.panes.find((p) => p.view === "search" && p.corpus);
  if (search) Object.assign(state, { searching: true, sc: search.corpus, q: search.query, s: search.search });
  rememberUi("parallel");
  navigate(serializeParallel(state));
}

/** From the parallel interface to the classic one: a pane per text, kept in step. */
export async function switchToClassic(
  state: Parallel,
  reference: string | null,
  /** The texts read by biblical book (DSS), as in the parallel columns. */
  byBook: string[] = [],
): Promise<void> {
  const texts = (state.texts ?? []).slice(0, MAX_PANES);
  const panes = texts.map((t) => ({
    ...emptyPane(t),
    by: byBook.includes(t) ? ("books" as const) : ("sections" as const),
  }));
  if (state.ref && panes.length) {
    // synchronised reading fills in the other panes from one that has a passage
    const at = reference ? texts.indexOf(reference) : 0;
    if (at >= 0) panes[at].ref = state.ref;
    else panes[0].ref = await alignRef(reference!, state.ref, texts[0]);
  }
  if (state.lexicon) {
    const at = texts.indexOf(state.lexicon);
    if (at >= 0) panes[at] = { ...panes[at], view: "lexicon", lex: state.lex };
  }
  if (state.searching && state.sc) {
    const at = texts.indexOf(state.sc);
    const pane = { ...emptyPane(state.sc), view: "search" as const, query: state.q, search: state.s };
    if (at >= 0) panes[at] = pane;
    else panes.unshift(pane);
    // the passage stays with a reading pane
    const reading = panes.findIndex((p) => p.view === "read");
    if (state.ref && reading >= 0 && !panes.some((p) => p.view === "read" && p.ref)) {
      panes[reading].ref = await alignRef(reference ?? texts[0], state.ref, panes[reading].corpus!);
    }
  }
  const ws: Workspace = {
    panes: panes.length ? panes.slice(0, MAX_PANES) : [emptyPane()],
    sync: panes.length > 1,
    diff: state.diff && panes.length > 1,
  };
  rememberUi("classic");
  navigate(`/${serialize(ws)}`);
}
