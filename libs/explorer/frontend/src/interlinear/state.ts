import { useMemo } from "react";
import { parseInterlinear, serializeInterlinear, type Interlinear, type Line } from "./url";
import { useUrlState } from "../shared/urlState";

const readInterlinear = () => parseInterlinear(window.location);

export interface InterlinearActions {
  /** Show a chapter (and a verse in it). */
  open: (ref: string, push?: boolean, verse?: number | null) => void;
  setCorpus: (corpus: string, ref: string | null) => void;
  setLines: (lines: Line[]) => void;
  /** Change the other settings (under, structure, practice, colour). */
  set: (change: Partial<Pick<Interlinear, "under" | "structure" | "hide" | "colour">>) => void;
}

export function useInterlinear(): [Interlinear, InterlinearActions] {
  const [state, commit] = useUrlState(readInterlinear, serializeInterlinear);

  const actions = useMemo<InterlinearActions>(
    () => ({
      open: (ref, push = true, verse = null) => commit((s) => ({ ...s, ref, verse }), push),
      setCorpus: (corpus, ref) =>
        commit((s) => ({ ...s, corpus, ref, verse: null, under: s.under === corpus ? null : s.under }), true),
      setLines: (lines) =>
        commit((s) => ({ ...s, lines, hide: s.hide && lines.includes(s.hide) ? s.hide : null }), false),
      set: (change) => commit((s) => ({ ...s, ...change }), false),
    }),
    [commit],
  );

  return [state, actions];
}
