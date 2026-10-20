import { useCallback, useEffect, useMemo, useState } from "react";
import { parseInterlinear, serializeInterlinear, type Interlinear, type Line } from "./url";

export interface InterlinearActions {
  /** Show a chapter (and a verse in it). */
  open: (ref: string, push?: boolean, verse?: number | null) => void;
  setCorpus: (corpus: string, ref: string | null) => void;
  setLines: (lines: Line[]) => void;
  /** Change the other settings (under, structure, practice, colour). */
  set: (change: Partial<Pick<Interlinear, "under" | "structure" | "hide" | "colour">>) => void;
}

export function useInterlinear(): [Interlinear, InterlinearActions] {
  const [state, setState] = useState<Interlinear>(() => parseInterlinear(window.location));

  useEffect(() => {
    const onPop = () => setState(parseInterlinear(window.location));
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const commit = useCallback((change: (s: Interlinear) => Interlinear, push: boolean) => {
    setState((current) => {
      const next = change(current);
      const url = serializeInterlinear(next);
      if (url !== window.location.pathname + window.location.search) {
        if (push) window.history.pushState(null, "", url);
        else window.history.replaceState(null, "", url);
      }
      return next;
    });
  }, []);

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
