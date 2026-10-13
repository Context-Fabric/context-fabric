import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import { DisplayProvider } from "./display/Display";
import { Inspector } from "./Inspector";
import { ReadView } from "./read/ReadView";
import { SearchView } from "./search/SearchView";
import { useLocation, useSettings, useTheme, type View } from "./state";
import type { CorpusCard, ViewSettings } from "./types";

function Mark() {
  // the Context-Fabric woven mark
  return <img src="/favicon.svg" alt="" className="h-5 w-5" />;
}

function DisplayOptions({
  corpus,
  settings,
  onChange,
}: {
  corpus: CorpusCard;
  settings: ViewSettings;
  onChange: (change: Partial<ViewSettings>) => void;
}) {
  const [open, setOpen] = useState(false);
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (panel.current && !panel.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const formats = corpus.display.textFormats;
  const toggles: [keyof ViewSettings, string][] = [
    ["showTypes", "Show object types"],
    ["standardFeatures", "Show the corpus' standard features"],
    ["hideTypes", `Hide ${corpus.display.hiddenTypes.join(", ") || "minor types"}`],
    ["showNodes", "Show node numbers"],
  ];

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="rounded px-2 py-1 text-[14px] hover:bg-woad-soft aria-expanded:bg-woad-soft sm:px-2.5"
      >
        Display
      </button>
      {open ? (
        <div className="absolute end-0 z-20 mt-1 flex w-72 flex-col gap-3 rounded border border-rule-strong bg-raised p-4 text-[14px] shadow-[0_8px_24px_-12px_rgba(31,43,77,0.35)]">
          <label className="flex flex-col gap-1">
            <span className="text-[13px] text-stone">Text format</span>
            <select
              value={settings.fmt ?? corpus.display.defaultFormat}
              onChange={(e) => onChange({ fmt: e.target.value === corpus.display.defaultFormat ? null : e.target.value })}
              className="rounded border border-rule-strong bg-vellum px-2 py-1"
            >
              {formats.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </select>
          </label>
          {toggles.map(([key, label]) => (
            <label key={key} className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={Boolean(settings[key])}
                onChange={(e) => onChange({ [key]: e.target.checked })}
                className="accent-[var(--woad)]"
              />
              {label}
            </label>
          ))}
        </div>
      ) : null}
    </div>
  );
}

export function App() {
  const [loc, setLoc] = useLocation();
  const [settings, setSettings] = useSettings();
  const [theme, toggleTheme] = useTheme();
  const [inspected, setInspected] = useState<number | null>(null);

  const corpora = useQuery({ queryKey: ["corpora"], queryFn: api.corpora });
  const corpusName = loc.corpus ?? corpora.data?.corpora[0]?.name ?? null;
  const corpus = useQuery({
    queryKey: ["corpus", corpusName],
    queryFn: () => api.corpus(corpusName!),
    enabled: corpusName !== null,
  });

  useEffect(() => {
    if (!loc.corpus && corpusName) setLoc({ corpus: corpusName });
  }, [loc.corpus, corpusName, setLoc]);

  useEffect(() => {
    const title = corpus.data ? `${corpus.data.name}: Context-Fabric` : "Context-Fabric Browser";
    document.title = title;
  }, [corpus.data]);

  const openPassage = useCallback(
    (ref: string) => setLoc({ view: "read", ref }, true),
    [setLoc],
  );
  const setView = (view: View) => setLoc({ view }, true);

  if (corpora.error || corpus.error) {
    return (
      <main className="p-10">
        <h1 className="mb-2 text-[20px] font-semibold">The browser cannot reach its server</h1>
        <p className="text-stone">
          Check that <code className="font-mono">cfabric-browser</code> is still running, then reload this page.
        </p>
      </main>
    );
  }
  if (!corpus.data) {
    return <main className="p-10 text-stone">Opening the corpus…</main>;
  }

  const card = corpus.data;
  return (
    <DisplayProvider config={card.display} settings={settings} onInspect={setInspected}>
      <div className="flex h-full flex-col">
        <header className="flex items-center gap-2 border-b border-rule px-3 py-2 sm:gap-4 sm:px-4">
          <div className="flex items-center gap-2" title="Context-Fabric">
            <Mark />
            <span className="hidden text-[15px] font-semibold tracking-tight sm:inline">Context-Fabric</span>
          </div>
          {corpora.data && corpora.data.corpora.length > 1 ? (
            <select
              value={card.name}
              onChange={(e) => setLoc({ corpus: e.target.value, ref: null, search: null, result: null }, true)}
              className="rounded border border-rule-strong bg-raised px-2 py-1 text-[14px]"
              aria-label="Corpus"
            >
              {corpora.data.corpora.map((c) => (
                <option key={c.name} value={c.name}>
                  {c.name}
                </option>
              ))}
            </select>
          ) : (
            <span className="text-[14px] text-stone" title={card.title}>
              {card.name} <span className="hidden tabular-nums sm:inline">{card.version}</span>
            </span>
          )}
          <nav className="flex gap-1" aria-label="Views">
            {(["read", "search"] as const).map((v) => (
              <button
                key={v}
                type="button"
                aria-current={loc.view === v ? "page" : undefined}
                onClick={() => setView(v)}
                className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft aria-[current=page]:bg-ink aria-[current=page]:text-vellum sm:px-3"
              >
                {v === "read" ? "Read" : "Search"}
              </button>
            ))}
          </nav>
          <div className="ms-auto flex items-center">
            <DisplayOptions corpus={card} settings={settings} onChange={setSettings} />
            <button
              type="button"
              onClick={toggleTheme}
              className="rounded px-2 py-1 text-[14px] hover:bg-woad-soft sm:px-2.5"
              aria-label={theme === "dark" ? "Use light colours" : "Use dark colours"}
            >
              {theme === "dark" ? "Light" : "Dark"}
            </button>
          </div>
        </header>
        <div className="flex min-h-0 flex-1">
          <main className="min-h-0 min-w-0 flex-1">
            {loc.view === "read" ? (
              <ReadView corpus={card} passageRef={loc.ref} settings={settings} onOpen={openPassage} />
            ) : (
              <SearchView
                corpus={card}
                template={loc.query}
                searchId={loc.search}
                expanded={loc.result}
                settings={settings}
                onTemplate={(query) => setLoc({ query })}
                onSearch={(search) => setLoc({ search, result: null }, true)}
                onExpand={(result) => setLoc({ result })}
                onOpenPassage={openPassage}
              />
            )}
          </main>
          {inspected !== null ? (
            <div className="fixed inset-0 z-30 md:static md:z-auto">
              <Inspector
                corpus={card}
                node={inspected}
                onInspect={setInspected}
                onClose={() => setInspected(null)}
                onOpenPassage={(ref) => {
                  setInspected(null);
                  openPassage(ref);
                }}
              />
            </div>
          ) : null}
        </div>
      </div>
    </DisplayProvider>
  );
}
