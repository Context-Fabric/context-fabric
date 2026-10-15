import { useCallback, useEffect, useMemo, useState } from "react";

// The parallel interface keeps its state in the URL, like Parabible:
// `/Genesis/1?texts=bhsa,sp,dss&diff=1`. The passage is a chapter in the
// numbering of the reference corpus (the BHSA); `texts` are the columns.

export interface Parallel {
  /** The chapter, as the reference corpus names it (`Genesis 1`, `1_Samuel 3`). */
  ref: string | null;
  /** The corpora shown, one column each, in this order. Null: not chosen yet. */
  texts: string[] | null;
  diff: boolean;
  /** The verse to show (scrolled to and marked), e.g. after opening a word's passage. */
  verse: number | null;
  /** The corpus whose lexicon is shown instead of the chapter, and the lexeme in it. */
  lexicon: string | null;
  lex: number | null;
  /** Searching instead of reading: in corpus `sc`, with template `q`; `s` is the search. */
  searching: boolean;
  sc: string | null;
  q: string;
  s: string | null;
}

export type SearchState = Pick<Parallel, "searching" | "sc" | "q" | "s">;

export function parseParallel(location: Location = window.location): Parallel {
  const parts = location.pathname
    .split("/")
    .filter(Boolean)
    .map((p) => decodeURIComponent(p));
  // spaces in book names are underscores in the corpus (1 Samuel is 1_Samuel)
  const ref = parts.length ? parts.map((p) => p.replace(/ /g, "_")).join(" ") : null;
  const p = new URLSearchParams(location.search);
  const texts = p.get("texts");
  const number = (key: string) => {
    const value = Number(p.get(key) ?? NaN);
    return Number.isInteger(value) ? value : null;
  };
  return {
    ref,
    texts: texts === null ? null : texts.split(",").filter(Boolean),
    diff: p.get("diff") === "1",
    verse: number("v"),
    lexicon: p.get("lexicon"),
    lex: number("lx"),
    searching: p.get("search") === "1",
    sc: p.get("sc"),
    q: p.get("q") ?? "",
    s: p.get("s"),
  };
}

export function serializeParallel(state: Parallel): string {
  const path = state.ref
    ? "/" +
      state.ref
        .split(" ")
        .map((p) => encodeURIComponent(p))
        .join("/")
    : "/";
  const p = new URLSearchParams();
  if (state.texts !== null) p.set("texts", state.texts.join(","));
  if (state.diff) p.set("diff", "1");
  if (state.verse !== null) p.set("v", String(state.verse));
  if (state.lexicon) {
    p.set("lexicon", state.lexicon);
    if (state.lex !== null) p.set("lx", String(state.lex));
  }
  if (state.searching) p.set("search", "1");
  if (state.sc) p.set("sc", state.sc);
  if (state.q) p.set("q", state.q);
  if (state.s) p.set("s", state.s);
  // keep the commas readable
  const search = p.toString().replace(/%2C/g, ",");
  // at `/`, the query string tells the parallel interface from the classic one
  if (path === "/") return `/?${search || "ui=parallel"}`;
  return search ? `${path}?${search}` : path;
}

export interface ParallelActions {
  /** Read a chapter, optionally at a verse; this closes the lexicon. */
  open: (ref: string, push?: boolean, verse?: number | null) => void;
  /** Show a corpus' lexicon (null: back to the chapter), at a lexeme. */
  setLexicon: (lexicon: string | null, lex?: number | null) => void;
  /** Change the search (and show it, unless `searching` says otherwise). */
  setSearch: (change: Partial<SearchState>, push?: boolean) => void;
  setTexts: (texts: string[]) => void;
  setDiff: (diff: boolean) => void;
}

export function useParallel(): [Parallel, ParallelActions] {
  const [state, setState] = useState<Parallel>(() => parseParallel());

  useEffect(() => {
    const onPop = () => setState(parseParallel());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const commit = useCallback((change: (s: Parallel) => Parallel, push: boolean) => {
    setState((current) => {
      const next = change(current);
      const url = serializeParallel(next);
      if (url !== window.location.pathname + window.location.search) {
        if (push) window.history.pushState(null, "", url);
        else window.history.replaceState(null, "", url);
      }
      return next;
    });
  }, []);

  const actions = useMemo<ParallelActions>(
    () => ({
      open: (ref, push = true, verse = null) =>
        commit((s) => ({ ...s, ref, verse, lexicon: null, lex: null, searching: false }), push),
      setLexicon: (lexicon, lex = null) =>
        commit((s) => ({ ...s, lexicon, lex: lexicon ? lex : null, searching: false }), true),
      setSearch: (change, push = true) => commit((s) => ({ ...s, searching: true, lexicon: null, ...change }), push),
      setTexts: (texts) => commit((s) => ({ ...s, texts }), true),
      setDiff: (diff) => commit((s) => ({ ...s, diff }), false),
    }),
    [commit],
  );

  return [state, actions];
}
