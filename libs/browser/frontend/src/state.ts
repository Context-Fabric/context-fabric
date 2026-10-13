import { useCallback, useEffect, useMemo, useState } from "react";
import type { ViewSettings } from "./types";

// Everything that identifies what you are looking at lives in the URL, so a view
// can be bookmarked and shared. Display preferences are kept per browser.

export type View = "read" | "search";

/** What one pane shows. A workspace has one or more panes side by side. */
export interface Pane {
  corpus: string | null;
  view: View;
  ref: string | null; // passage being read
  query: string; // search template
  search: string | null; // id of the current search
  result: number | null; // expanded result
}

export interface Workspace {
  panes: Pane[];
  sync: boolean; // keep the passages of reading panes in step
}

export const MAX_PANES = 4;

export function emptyPane(corpus: string | null = null): Pane {
  return { corpus, view: "read", ref: null, query: "", search: null, result: null };
}

// URL keys per pane: c0, v0, ref0, ... ; links without an index are pane 0.
function parsePane(p: URLSearchParams, i: number): Pane | null {
  const key = (k: string) => p.get(`${k}${i}`) ?? (i === 0 ? p.get(k) : null);
  const corpus = key("c");
  const view = key("v");
  if (corpus === null && view === null && i > 0) return null;
  const result = key("r");
  return {
    corpus,
    view: view === "search" ? "search" : "read",
    ref: key("ref"),
    query: key("q") ?? "",
    search: key("s"),
    result: result === null ? null : Number(result),
  };
}

function parse(search: string = window.location.search): Workspace {
  const p = new URLSearchParams(search);
  const panes: Pane[] = [];
  for (let i = 0; i < MAX_PANES; i++) {
    const pane = parsePane(p, i);
    if (pane === null) break;
    panes.push(pane);
  }
  return { panes: panes.length ? panes : [emptyPane()], sync: p.get("sync") === "1" };
}

export function serialize(ws: Workspace): string {
  const p = new URLSearchParams();
  ws.panes.forEach((pane, i) => {
    if (pane.corpus) p.set(`c${i}`, pane.corpus);
    p.set(`v${i}`, pane.view);
    if (pane.ref) p.set(`ref${i}`, pane.ref);
    if (pane.query) p.set(`q${i}`, pane.query);
    if (pane.search) p.set(`s${i}`, pane.search);
    if (pane.result !== null) p.set(`r${i}`, String(pane.result));
  });
  if (ws.sync) p.set("sync", "1");
  return `?${p.toString()}`;
}

export interface WorkspaceActions {
  updatePane: (index: number, change: Partial<Pane>, push?: boolean) => void;
  /** Change several panes at once (e.g. when passages are kept in step). */
  updatePanes: (changes: Map<number, Partial<Pane>>, push?: boolean) => void;
  addPane: (pane: Pane) => void;
  removePane: (index: number) => void;
  setSync: (sync: boolean) => void;
  /** Replace the whole workspace, e.g. with a saved one (given as a URL query string). */
  load: (search: string) => void;
}

export function useWorkspace(): [Workspace, WorkspaceActions] {
  const [ws, setWs] = useState<Workspace>(parse);

  useEffect(() => {
    const onPop = () => setWs(parse());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const commit = useCallback((change: (ws: Workspace) => Workspace, push: boolean) => {
    setWs((current) => {
      const next = change(current);
      const url = serialize(next);
      if (url !== window.location.search) {
        if (push) window.history.pushState(null, "", url);
        else window.history.replaceState(null, "", url);
      }
      return next;
    });
  }, []);

  const actions = useMemo<WorkspaceActions>(
    () => ({
      updatePane: (index, change, push = false) =>
        commit(
          (w) => ({ ...w, panes: w.panes.map((p, i) => (i === index ? { ...p, ...change } : p)) }),
          push,
        ),
      updatePanes: (changes, push = false) =>
        commit((w) => ({ ...w, panes: w.panes.map((p, i) => ({ ...p, ...changes.get(i) })) }), push),
      addPane: (pane) =>
        commit((w) => (w.panes.length >= MAX_PANES ? w : { ...w, panes: [...w.panes, pane] }), true),
      removePane: (index) =>
        commit((w) => (w.panes.length <= 1 ? w : { ...w, panes: w.panes.filter((_, i) => i !== index) }), true),
      setSync: (sync) => commit((w) => ({ ...w, sync }), false),
      load: (search) => commit(() => parse(search), true),
    }),
    [commit],
  );

  return [ws, actions];
}

// per-browser preferences; storage may be unavailable (private windows)

const SETTINGS_KEY = "cfabric-browser/settings";

const DEFAULT_SETTINGS: ViewSettings = {
  fmt: null,
  hideTypes: true,
  standardFeatures: false,
  showNodes: false,
  showTypes: true,
};

function load<T>(key: string, fallback: T): T {
  try {
    const raw = window.localStorage.getItem(key);
    return raw ? { ...fallback, ...JSON.parse(raw) } : fallback;
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown): void {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // not persisted; the setting still applies to this session
  }
}

export function useSettings(): [ViewSettings, (change: Partial<ViewSettings>) => void] {
  const [settings, setSettings] = useState<ViewSettings>(() => load(SETTINGS_KEY, DEFAULT_SETTINGS));
  const update = useCallback((change: Partial<ViewSettings>) => {
    setSettings((current) => {
      const next = { ...current, ...change };
      save(SETTINGS_KEY, next);
      return next;
    });
  }, []);
  return [settings, update];
}

export type Theme = "light" | "dark";

export function useTheme(): [Theme, () => void] {
  const [theme, setTheme] = useState<Theme>(() => {
    try {
      const stored = window.localStorage.getItem("cfabric-browser/theme");
      if (stored === "light" || stored === "dark") return stored;
    } catch {
      // fall through to the system preference
    }
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  });

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const toggle = useCallback(() => {
    setTheme((t) => {
      const next = t === "dark" ? "light" : "dark";
      try {
        window.localStorage.setItem("cfabric-browser/theme", next);
      } catch {
        // not persisted
      }
      return next;
    });
  }, []);

  return [theme, toggle];
}
