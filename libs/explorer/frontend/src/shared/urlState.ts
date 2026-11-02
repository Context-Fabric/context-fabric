import { useCallback, useEffect, useRef, useState } from "react";

// The state of an interface, all of it in the URL: read from it at the start and on
// Back/Forward, and written to it with each change (a step in the history, or not).

/** Where the browser is: path and query. */
export function here(): string {
  return window.location.pathname + window.location.search;
}

/** The state `parse` reads from the URL, and `commit` to change it: `serialize`
 * gives its URL, compared with `current()` to see whether the URL changes. The
 * functions must be stable (module-level). */
export function useUrlState<T>(
  parse: () => T,
  serialize: (state: T) => string,
  current: () => string = here,
): [T, (change: (state: T) => T, push: boolean) => void] {
  const [state, setState] = useState<T>(parse);
  // the latest state, also for changes made before React renders again
  const latest = useRef(state);

  useEffect(() => {
    const onPop = () => {
      latest.current = parse();
      setState(latest.current);
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, [parse]);

  const commit = useCallback(
    (change: (state: T) => T, push: boolean) => {
      const next = change(latest.current);
      latest.current = next;
      const url = serialize(next);
      if (url !== current()) {
        if (push) window.history.pushState(null, "", url);
        else window.history.replaceState(null, "", url);
      }
      setState(next);
    },
    [serialize, current],
  );

  return [state, commit];
}
