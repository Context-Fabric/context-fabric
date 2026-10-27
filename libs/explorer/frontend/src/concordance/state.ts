import { useMemo } from "react";
import { parseConcordance, serializeConcordance, type Concordance } from "./url";
import { useUrlState } from "../shared/urlState";

const readConcordance = () => parseConcordance(window.location);

export interface ConcordanceActions {
  /** Change the state; `push` makes it a step in the browser's history. */
  set: (change: Partial<Concordance>, push?: boolean) => void;
}

export function useConcordance(): [Concordance, ConcordanceActions] {
  const [state, commit] = useUrlState(readConcordance, serializeConcordance);

  const actions = useMemo<ConcordanceActions>(
    () => ({ set: (change, push = false) => commit((s) => ({ ...s, ...change }), push) }),
    [commit],
  );
  return [state, actions];
}
