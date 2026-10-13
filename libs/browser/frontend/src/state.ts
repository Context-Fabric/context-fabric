import { useCallback, useEffect, useState } from "react";
import type { ViewSettings } from "./types";

// Everything that identifies what you are looking at lives in the URL, so a view
// can be bookmarked and shared. Display preferences are kept per browser.

export type View = "read" | "search";

export interface Location {
  corpus: string | null;
  view: View;
  ref: string | null; // passage being read
  query: string; // search template
  search: string | null; // id of the current search
  result: number | null; // expanded result
}

function parse(): Location {
  const p = new URLSearchParams(window.location.search);
  const result = p.get("r");
  return {
    corpus: p.get("c"),
    view: p.get("v") === "search" ? "search" : "read",
    ref: p.get("ref"),
    query: p.get("q") ?? "",
    search: p.get("s"),
    result: result === null ? null : Number(result),
  };
}

function serialize(loc: Location): string {
  const p = new URLSearchParams();
  if (loc.corpus) p.set("c", loc.corpus);
  p.set("v", loc.view);
  if (loc.ref) p.set("ref", loc.ref);
  if (loc.query) p.set("q", loc.query);
  if (loc.search) p.set("s", loc.search);
  if (loc.result !== null) p.set("r", String(loc.result));
  return `?${p.toString()}`;
}

export function useLocation(): [Location, (change: Partial<Location>, push?: boolean) => void] {
  const [loc, setLoc] = useState<Location>(parse);

  useEffect(() => {
    const onPop = () => setLoc(parse());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const update = useCallback((change: Partial<Location>, push = false) => {
    setLoc((current) => {
      const next = { ...current, ...change };
      const url = serialize(next);
      if (url !== window.location.search) {
        if (push) window.history.pushState(null, "", url);
        else window.history.replaceState(null, "", url);
      }
      return next;
    });
  }, []);

  return [loc, update];
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
