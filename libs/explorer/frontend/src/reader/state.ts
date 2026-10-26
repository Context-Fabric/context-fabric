import { useMemo } from "react";
import { parseReader, serializeReader, type Mode, type Panel, type Reader } from "./url";
import { useUrlState } from "../shared/urlState";

const readReader = () => parseReader(window.location);

export interface ReaderActions {
  /** Read a chapter (and select a verse in it). */
  open: (ref: string, push?: boolean, verse?: number | null) => void;
  /** The chapter scrolled into view: only the URL follows, nothing is reloaded. */
  scrolled: (ref: string) => void;
  /** Select a verse of the chapter in the URL, and open the panel if it is closed. */
  select: (verse: number | null, ref?: string) => void;
  setPanel: (panel: Panel | null) => void;
  setMode: (mode: Mode) => void;
  setCorpus: (corpus: string, ref: string | null) => void;
  setTexts: (texts: string[]) => void;
  setDiff: (diff: boolean) => void;
}

export function useReader(): [Reader, ReaderActions] {
  const [state, commit] = useUrlState(readReader, serializeReader);

  const actions = useMemo<ReaderActions>(
    () => ({
      open: (ref, push = true, verse = null) => commit((s) => ({ ...s, ref, verse }), push),
      scrolled: (ref) => commit((s) => (s.ref === ref ? s : { ...s, ref, verse: null }), false),
      select: (verse, ref) =>
        commit(
          (s) => ({ ...s, ref: ref ?? s.ref, verse, panel: verse === null ? s.panel : (s.panel ?? "parallels") }),
          false,
        ),
      setPanel: (panel) => commit((s) => ({ ...s, panel }), false),
      setMode: (mode) => commit((s) => ({ ...s, mode }), false),
      setCorpus: (corpus, ref) =>
        commit((s) => ({ ...s, corpus, ref, verse: null, texts: s.texts?.filter((t) => t !== corpus) ?? null }), true),
      setTexts: (texts) => commit((s) => ({ ...s, texts }), false),
      setDiff: (diff) => commit((s) => ({ ...s, diff }), false),
    }),
    [commit],
  );

  return [state, actions];
}
