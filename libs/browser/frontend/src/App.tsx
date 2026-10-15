import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import { DisplayProvider, displayRef } from "./display/Display";
import { useDifferences, type PaneDiff } from "./differences";
import { HoverCard, useHover } from "./HoverCard";
import { Inspector } from "./Inspector";
import { LexiconView } from "./lexicon/LexiconView";
import { Library } from "./Library";
import { Workspaces } from "./Workspaces";
import { BookView } from "./read/BookView";
import { ReadView } from "./read/ReadView";
import { SearchView } from "./search/SearchView";
import { switchToParallel } from "./switch";
import { UiSwitch } from "./UiSwitch";
import {
  emptyPane,
  MAX_PANES,
  useSettings,
  useTheme,
  useWorkspace,
  type Pane,
  type View,
} from "./state";
import type { CorpusCard, ReadBy, ViewSettings } from "./types";

const VIEW_NAMES: Record<View, string> = { read: "Read", search: "Search", lexicon: "Lexicon" };

/** The views a corpus offers: the lexicon only where it has one (BHSA). */
function viewsOf(card: CorpusCard | undefined): View[] {
  return card?.lexicon ? ["read", "search", "lexicon"] : ["read", "search"];
}

function Mark() {
  // the Context-Fabric woven mark
  return <img src="/favicon.svg" alt="" className="h-5 w-5" />;
}

function DisplayOptions({
  settings,
  formats,
  defaultFormat,
  onChange,
}: {
  settings: ViewSettings;
  formats: string[];
  defaultFormat: string | null;
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

  const toggles: [keyof ViewSettings, string][] = [
    ["showTypes", "Show object types"],
    ["standardFeatures", "Show each corpus' standard features"],
    ["hideTypes", "Hide minor object types"],
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
          {formats.length ? (
            <label className="flex flex-col gap-1">
              <span className="text-[13px] text-stone">Text format</span>
              <select
                value={settings.fmt ?? defaultFormat ?? ""}
                onChange={(e) => onChange({ fmt: e.target.value === defaultFormat ? null : e.target.value })}
                className="rounded border border-rule-strong bg-vellum px-2 py-1"
              >
                {formats.map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
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

interface PaneProps {
  index: number;
  pane: Pane;
  card: CorpusCard | undefined;
  cardError: unknown;
  corpora: string[];
  paneCount: number;
  settings: ViewSettings;
  note: string | null;
  diff: PaneDiff | null;
  onChange: (change: Partial<Pane>, push?: boolean) => void;
  onOpenPassage: (ref: string) => void;
  onBy: (by: ReadBy) => void;
  onOpenPane: (pane: Pane) => void;
  onClose: () => void;
  onInspect: (corpus: string, node: number) => void;
}

function PaneView({
  index,
  pane,
  card,
  cardError,
  corpora,
  paneCount,
  settings,
  note,
  diff,
  onChange,
  onOpenPassage,
  onBy,
  onOpenPane,
  onClose,
  onInspect,
}: PaneProps) {
  const compact = paneCount > 1;
  const setView = (view: View) => onChange({ view }, true);
  // a corpus without a lexicon (after switching corpus) is read instead
  const view: View = pane.view === "lexicon" && card && !card.lexicon ? "read" : pane.view;
  const [hovered, onHover] = useHover();
  // the word under the card goes when the pane shows something else
  useEffect(() => {
    onHover(null);
  }, [onHover, pane.corpus, view, pane.ref, pane.by, pane.lex, pane.search, pane.result]);
  // a corpus' own default format applies unless the reader chose one it has
  const paneSettings =
    card && settings.fmt && !card.display.textFormats.includes(settings.fmt) ? { ...settings, fmt: null } : settings;

  return (
    <section
      className="flex min-h-0 min-w-0 flex-1 flex-col border-rule [&+&]:border-t md:[&+&]:border-s md:[&+&]:border-t-0"
      aria-label={`Pane ${index + 1}${card ? `: ${card.name}` : ""}`}
    >
      {paneCount > 1 || corpora.length > 1 ? (
        <div className="flex flex-wrap items-center gap-2 border-b border-rule bg-raised px-3 py-1.5">
          <select
            value={pane.corpus ?? ""}
            onChange={(e) => onChange({ corpus: e.target.value, ref: null, by: "sections", search: null, result: null }, true)}
            className="rounded border border-rule-strong bg-vellum px-2 py-0.5 text-[14px] font-semibold"
            aria-label="Corpus"
          >
            {corpora.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
          <div className="flex gap-0.5" role="group" aria-label="View">
            {viewsOf(card).map((v) => (
              <button
                key={v}
                type="button"
                aria-pressed={view === v}
                onClick={() => setView(v)}
                className="rounded px-2.5 py-0.5 text-[14px] hover:bg-woad-soft aria-pressed:bg-ink aria-pressed:text-vellum"
              >
                {VIEW_NAMES[v]}
              </button>
            ))}
          </div>
          {paneCount > 1 ? (
            <button
              type="button"
              onClick={onClose}
              className="ms-auto rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
              aria-label={`Close pane ${index + 1}`}
            >
              Close
            </button>
          ) : null}
        </div>
      ) : null}
      <div className="min-h-0 flex-1">
        {cardError ? (
          <p className="p-6 text-madder">
            Could not open {pane.corpus}: {String((cardError as Error).message)}
          </p>
        ) : !card ? (
          <p className="p-6 text-stone">
            Opening {pane.corpus}… A corpus opened for the first time is prepared once, which can take a minute.
          </p>
        ) : (
          <DisplayProvider
            config={card.display}
            settings={paneSettings}
            onInspect={(n) => onInspect(card.name, n)}
            onHover={onHover}
            marks={view === "read" ? diff?.marks : undefined}
            markTitles={view === "read" ? diff?.titles : undefined}
          >
            {view === "read" && pane.by === "books" && card.bookView ? (
              <BookView
                corpus={card}
                passageRef={pane.ref}
                settings={paneSettings}
                onOpen={onOpenPassage}
                onBy={onBy}
                onOpenScroll={(ref) => onChange({ by: "sections", ref }, true)}
                compact={compact}
                note={note}
                diff={diff?.message ?? null}
                places={diff?.places}
              />
            ) : view === "read" ? (
              <ReadView
                corpus={card}
                passageRef={pane.ref}
                settings={paneSettings}
                onOpen={onOpenPassage}
                compact={compact}
                note={note}
                diff={diff?.message ?? null}
                onBy={onBy}
              />
            ) : view === "lexicon" ? (
              <LexiconView
                corpus={card}
                lex={pane.lex}
                settings={paneSettings}
                onLex={(lex) => onChange({ lex }, true)}
                // read the verse here; with sync, the other reading panes follow
                onOpenPassage={onOpenPassage}
                onSearch={async (query) => {
                  // start the search, so that its results are there straight away
                  try {
                    const started = await api.search(card.name, query);
                    onChange({ view: "search", query, search: started.id, result: null }, true);
                  } catch {
                    onChange({ view: "search", query, search: null, result: null }, true);
                  }
                }}
                compact={compact}
              />
            ) : (
              <SearchView
                corpus={card}
                template={pane.query}
                searchId={pane.search}
                expanded={pane.result}
                settings={paneSettings}
                onTemplate={(query) => onChange({ query })}
                onSearch={(search) => onChange({ search, result: null }, true)}
                onExpand={(result) => onChange({ result })}
                onOpenPassage={(ref) => onChange({ view: "read", ref }, true)}
                otherCorpora={corpora.filter((c) => c !== card.name)}
                onOpenPane={onOpenPane}
                compact={compact}
              />
            )}
            {hovered ? (
              <HoverCard
                corpus={card}
                hovered={hovered}
                note={view === "read" ? diff?.titles?.get(hovered.node) : undefined}
              />
            ) : null}
          </DisplayProvider>
        )}
      </div>
    </section>
  );
}

export function App() {
  const [ws, actions] = useWorkspace();
  const [settings, setSettings] = useSettings();
  const [theme, toggleTheme] = useTheme();
  const [inspected, setInspected] = useState<{ corpus: string; node: number } | null>(null);
  const [library, setLibrary] = useState(false);
  const [notes, setNotes] = useState<Record<number, string | null>>({});
  const differences = useDifferences(ws.panes, ws.diff);

  const corporaQuery = useQuery({ queryKey: ["corpora"], queryFn: api.corpora });
  const corpora = corporaQuery.data?.corpora.map((c) => c.name) ?? [];
  const firstCorpus = corpora[0] ?? null;

  // panes without a corpus get the first one
  useEffect(() => {
    if (!firstCorpus) return;
    const missing = new Map<number, Partial<Pane>>();
    ws.panes.forEach((p, i) => {
      if (!p.corpus) missing.set(i, { corpus: firstCorpus });
    });
    if (missing.size) actions.updatePanes(missing);
  }, [ws.panes, firstCorpus, actions]);

  // started without corpora: offer the library straight away, once
  const noCorpora = corporaQuery.data !== undefined && corpora.length === 0;
  const offeredLibrary = useRef(false);
  useEffect(() => {
    if (noCorpora && !offeredLibrary.current) {
      offeredLibrary.current = true;
      setLibrary(true);
    }
  }, [noCorpora]);

  const names = [...new Set(ws.panes.map((p) => p.corpus).filter((c): c is string => Boolean(c)))];
  const cards = useQuery({
    queryKey: ["cards", ...names],
    queryFn: async () => {
      const entries = await Promise.allSettled(names.map((n) => api.corpus(n)));
      const result: Record<string, { card?: CorpusCard; error?: unknown }> = {};
      entries.forEach((e, i) => {
        result[names[i]] = e.status === "fulfilled" ? { card: e.value } : { error: e.reason };
      });
      return result;
    },
    enabled: names.length > 0,
    placeholderData: (previous) => previous,
  });
  const cardOf = (name: string | null) => (name ? cards.data?.[name] : undefined);
  const nameList = names.join(" + ");

  useEffect(() => {
    document.title = nameList ? `${nameList}: Context-Fabric` : "Context-Fabric Browser";
  }, [nameList]);

  // With synchronised reading, reading panes without a passage (a short link,
  // a new pane) open the passage of the first pane that has one.
  useEffect(() => {
    if (!ws.sync) return;
    const source = ws.panes.find((p) => p.view === "read" && p.corpus && p.ref);
    const missing = ws.panes
      .map((p, i) => (p.view === "read" && p.corpus && !p.ref ? i : -1))
      .filter((i) => i >= 0);
    if (!source || !missing.length) return;
    let current = true;
    (async () => {
      const changes = new Map<number, Partial<Pane>>();
      const newNotes: Record<number, string | null> = {};
      await Promise.all(
        missing.map(async (i) => {
          const p = ws.panes[i];
          try {
            const target = await api.align(p.corpus!, source.corpus!, source.ref!, p.by);
            changes.set(i, { ref: target.ref });
            newNotes[i] = target.exact
              ? null
              : `${displayRef(source.ref!)} is not in ${p.corpus}; showing ${displayRef(target.ref)}.`;
          } catch {
            newNotes[i] = `${p.corpus} has no counterpart of ${displayRef(source.ref!)}.`;
          }
        }),
      );
      if (!current) return;
      if (changes.size) actions.updatePanes(changes);
      setNotes((n) => ({ ...n, ...newNotes }));
    })();
    return () => {
      current = false;
    };
  }, [ws.panes, ws.sync, actions]);

  /** Open a passage in a pane; with synchronised reading, the other reading panes follow. */
  const openPassage = useCallback(
    async (index: number, ref: string) => {
      const source = ws.panes[index];
      if (!ws.sync || !source.corpus) {
        setNotes((n) => ({ ...n, [index]: null }));
        actions.updatePane(index, { view: "read", ref }, true);
        return;
      }
      const sourceCorpus = source.corpus;
      const changes = new Map<number, Partial<Pane>>([[index, { view: "read", ref }]]);
      const newNotes: Record<number, string | null> = { [index]: null };
      await Promise.all(
        ws.panes.map(async (p, i) => {
          if (i === index || p.view !== "read" || !p.corpus) return;
          try {
            const target = await api.align(p.corpus, sourceCorpus, ref, p.by);
            changes.set(i, { ref: target.ref });
            newNotes[i] = target.exact
              ? null
              : `${displayRef(ref)} is not in ${p.corpus}; showing ${displayRef(target.ref)}.`;
          } catch {
            newNotes[i] = `${p.corpus} has no counterpart of ${displayRef(ref)}.`;
          }
        }),
      );
      actions.updatePanes(changes, true);
      setNotes((n) => ({ ...n, ...newNotes }));
    },
    [ws.panes, ws.sync, actions],
  );

  /** Read a pane by its own sections or by biblical book; with sync, at the passage of the other panes. */
  const setBy = async (index: number, by: ReadBy) => {
    const pane = ws.panes[index];
    const other = ws.panes.find((p, i) => i !== index && p.view === "read" && p.corpus && p.ref);
    let ref: string | null = null;
    let note: string | null = null;
    if (ws.sync && other && pane.corpus) {
      try {
        const target = await api.align(pane.corpus, other.corpus!, other.ref!, by);
        ref = target.ref;
        note = target.exact ? null : `${displayRef(other.ref!)} is not in ${pane.corpus}; showing ${displayRef(target.ref)}.`;
      } catch {
        note = `${pane.corpus} has no counterpart of ${displayRef(other.ref!)}.`;
      }
    }
    setNotes((n) => ({ ...n, [index]: note }));
    actions.updatePane(index, { by, ref }, true);
  };

  const addPane = (pane: Pane) => {
    if (ws.panes.length < MAX_PANES) actions.addPane(pane);
  };

  if (corporaQuery.error) {
    return (
      <main className="p-10">
        <h1 className="mb-2 text-[20px] font-semibold">The browser cannot reach its server</h1>
        <p className="text-stone">
          Check that <code className="font-mono">cfabric-browser</code> is still running, then reload this page.
        </p>
      </main>
    );
  }
  if (!corporaQuery.data) return <main className="p-10 text-stone">Opening…</main>;

  const firstCard = cardOf(ws.panes[0]?.corpus ?? null)?.card;
  const inspectedCard = inspected ? cardOf(inspected.corpus)?.card : undefined;
  const single = ws.panes.length === 1 && corpora.length === 1;

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-wrap items-center gap-x-2 gap-y-1 border-b border-rule px-3 py-2 sm:gap-x-4 sm:px-4">
        <div className="flex items-center gap-2" title="Context-Fabric">
          <Mark />
          <span className="hidden text-[15px] font-semibold tracking-tight sm:inline">Context-Fabric</span>
        </div>
        {single && firstCard ? (
          <>
            <span className="text-[14px] text-stone" title={firstCard.title}>
              {firstCard.name} <span className="hidden tabular-nums sm:inline">{firstCard.version}</span>
            </span>
            <nav className="flex gap-1" aria-label="Views">
              {viewsOf(firstCard).map((v) => (
                <button
                  key={v}
                  type="button"
                  aria-current={ws.panes[0].view === v ? "page" : undefined}
                  onClick={() => actions.updatePane(0, { view: v }, true)}
                  className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft aria-[current=page]:bg-ink aria-[current=page]:text-vellum sm:px-3"
                >
                  {VIEW_NAMES[v]}
                </button>
              ))}
            </nav>
          </>
        ) : null}
        <div className="ms-auto flex flex-wrap items-center justify-end gap-1 whitespace-nowrap">
          {ws.panes.length > 1 ? (
            <label
              className="me-1 flex items-center gap-1.5 text-[14px]"
              title="Opening a passage in one pane opens the same passage in the others"
            >
              <input
                type="checkbox"
                checked={ws.sync}
                onChange={(e) => actions.setSync(e.target.checked)}
                className="accent-[var(--woad)]"
              />
              Sync passages
            </label>
          ) : null}
          {ws.panes.length > 1 ? (
            <label
              className="me-1 flex items-center gap-1.5 text-[14px]"
              title="Mark the words where the texts differ from the first pane, compared on their consonants"
            >
              <input
                type="checkbox"
                checked={ws.diff}
                onChange={(e) => actions.setDiff(e.target.checked)}
                className="accent-[var(--woad)]"
              />
              Show differences
            </label>
          ) : null}
          <button
            type="button"
            disabled={noCorpora || ws.panes.length >= MAX_PANES}
            onClick={() => {
              const last = ws.panes[ws.panes.length - 1];
              const other = corpora.find((c) => !ws.panes.some((p) => p.corpus === c)) ?? last.corpus;
              addPane({ ...emptyPane(other), ref: last.view === "read" && other === last.corpus ? last.ref : null });
            }}
            className="rounded px-2 py-1 text-[14px] hover:bg-woad-soft disabled:opacity-40 sm:px-2.5"
            title="Show another corpus, or another passage, side by side"
          >
            Add a pane
          </button>
          <button
            type="button"
            onClick={() => setLibrary(true)}
            className="rounded px-2 py-1 text-[14px] hover:bg-woad-soft sm:px-2.5"
          >
            Open a corpus
          </button>
          <Workspaces corpora={corpora} onLoad={actions.load} />
          <DisplayOptions
            settings={settings}
            formats={firstCard?.display.textFormats ?? []}
            defaultFormat={firstCard?.display.defaultFormat ?? null}
            onChange={setSettings}
          />
          <UiSwitch current="classic" onSwitch={() => switchToParallel(ws)} />
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
        <main className="flex min-h-0 min-w-0 flex-1 flex-col md:flex-row">
          {noCorpora ? (
            <div className="p-10">
              <h1 className="mb-2 text-[20px] font-semibold">No corpus is open</h1>
              <p className="mb-4 max-w-prose text-stone">
                Open one of the corpora on this computer to read, search and compare it.
              </p>
              <button
                type="button"
                onClick={() => setLibrary(true)}
                className="rounded bg-ink px-3 py-1.5 text-[14px] text-vellum hover:opacity-90"
              >
                Open a corpus
              </button>
            </div>
          ) : null}
          {noCorpora ? null : ws.panes.map((pane, i) => {
            const entry = cardOf(pane.corpus);
            return (
              <PaneView
                key={i}
                index={i}
                pane={pane}
                card={entry?.card}
                cardError={entry?.error}
                corpora={corpora}
                paneCount={ws.panes.length}
                settings={settings}
                note={notes[i] ?? null}
                diff={differences[i]}
                onChange={(change, push) => actions.updatePane(i, change, push)}
                onOpenPassage={(ref) => openPassage(i, ref)}
                onBy={(by) => setBy(i, by)}
                onOpenPane={addPane}
                onClose={() => actions.removePane(i)}
                onInspect={(corpus, node) => setInspected({ corpus, node })}
              />
            );
          })}
        </main>
        {inspected && inspectedCard ? (
          <div className="fixed inset-0 z-30 md:static md:z-auto">
            <Inspector
              corpus={inspectedCard}
              node={inspected.node}
              onInspect={(node) => setInspected({ corpus: inspected.corpus, node })}
              onClose={() => setInspected(null)}
              onOpenPassage={(ref) => {
                const i = ws.panes.findIndex((p) => p.corpus === inspected.corpus);
                setInspected(null);
                if (i >= 0) openPassage(i, ref);
              }}
            />
          </div>
        ) : null}
      </div>
      {library ? (
        <Library
          onClose={() => setLibrary(false)}
          onOpen={(corpus) => {
            setLibrary(false);
            if (ws.panes.some((p) => p.corpus === corpus)) return;
            // a pane still without a corpus (the browser started empty) gets it
            const empty = ws.panes.findIndex((p) => !p.corpus);
            if (empty >= 0) actions.updatePane(empty, { corpus }, true);
            else addPane(emptyPane(corpus));
          }}
        />
      ) : null}
    </div>
  );
}
