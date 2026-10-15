import { useCallback, useEffect, useMemo, useState } from "react";
import { parseConcordance, serializeConcordance, type Concordance } from "./url";

export interface ConcordanceActions {
  /** Change the state; `push` makes it a step in the browser's history. */
  set: (change: Partial<Concordance>, push?: boolean) => void;
}

export function useConcordance(): [Concordance, ConcordanceActions] {
  const [state, setState] = useState<Concordance>(() => parseConcordance(window.location));

  useEffect(() => {
    const onPop = () => setState(parseConcordance(window.location));
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const commit = useCallback((change: (s: Concordance) => Concordance, push: boolean) => {
    setState((current) => {
      const next = change(current);
      const url = serializeConcordance(next);
      if (url !== window.location.pathname + window.location.search) {
        if (push) window.history.pushState(null, "", url);
        else window.history.replaceState(null, "", url);
      }
      return next;
    });
  }, []);

  const actions = useMemo<ConcordanceActions>(
    () => ({ set: (change, push = false) => commit((s) => ({ ...s, ...change }), push) }),
    [commit],
  );
  return [state, actions];
}
