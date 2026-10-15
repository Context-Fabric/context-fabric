import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import { DisplayProvider, displayRef } from "../display/Display";
import { HoverCard, useHover } from "../HoverCard";
import { Library } from "../Library";
import { Inspector } from "../Inspector";
import { LexiconView } from "../lexicon/LexiconView";
import { useDifferences, type PaneDiff } from "../differences";
import { DiffNote } from "../read/BookView";
import { emptyPane, useSettings, useTheme } from "../state";
import { switchToClassic } from "../switch";
import type { CorpusCard } from "../types";
import { alignRef, chapterOf, REFERENCE_IDS } from "../ui";
import { UiSwitch } from "../UiSwitch";
import { ParallelSearch } from "./Search";
import { useParallel } from "./state";
import { termFrom, type Term } from "./terms";
import { settingsFor, Table } from "./Table";

// The parallel interface, after Parabible's layout: a slim top bar with the
// chapter, a drawer with the books, and a column per corpus.

function Mark() {
  return <img src="/favicon.svg" alt="" className="h-5 w-5" />;
}

/** The open corpora and their cards. */
function useCards() {
  const corpora = useQuery({ queryKey: ["corpora"], queryFn: api.corpora });
  const names = corpora.data?.corpora.map((c) => c.name) ?? [];
  const cards = useQuery({
    queryKey: ["cards", ...names],
    queryFn: async () => {
      const entries = await Promise.allSettled(names.map((n) => api.corpus(n)));
      const result: Record<string, CorpusCard> = {};
      entries.forEach((e, i) => {
        if (e.status === "fulfilled") result[names[i]] = e.value;
      });
      return result;
    },
    enabled: names.length > 0,
    placeholderData: (previous) => previous,
  });
  return {
    corpora,
    names,
    cards: cards.data ?? {},
    ready: corpora.data !== undefined && (!names.length || !!cards.data),
  };
}

/** The corpus whose verse numbering the rows follow: the BHSA, opened in the
 * background when it is on this computer but not open. Otherwise null. */
function useReference(cards: Record<string, CorpusCard>, ready: boolean): { name: string | null; settled: boolean } {
  const client = useQueryClient();
  const open = Object.values(cards).find((c) => REFERENCE_IDS.includes(c.id?.toLowerCase() ?? ""));
  const library = useQuery({ queryKey: ["library"], queryFn: api.library, enabled: ready && !open });
  const tried = useRef(false);
  const [failed, setFailed] = useState(false);
  const entry = library.data?.corpora.find((c) => REFERENCE_IDS.includes(c.id.toLowerCase()));

  useEffect(() => {
    if (!ready || open || !entry || tried.current) return;
    tried.current = true;
    api
      .openCorpus(entry.id)
      .then(() => client.invalidateQueries({ queryKey: ["corpora"] }))
      .catch(() => setFailed(true));
  }, [ready, open, entry, client]);

  if (open) return { name: open.name, settled: true };
  const settled =
    ready && (library.isError || (library.data !== undefined && (!entry || !library.data.enabled)) || failed);
  return { name: null, settled };
}

/** Books, and the chapters of the book opened in the list. */
function Navigate({
  corpus,
  current,
  onOpen,
  onClose,
}: {
  corpus: string;
  current: string | null;
  onOpen: (ref: string) => void;
  onClose: () => void;
}) {
  const panel = useRef<HTMLElement>(null);
  const currentBook = current ? current.split(" ")[0] : null;
  const [expanded, setExpanded] = useState<string | null>(currentBook);
  const books = useQuery({ queryKey: ["sections", corpus, null], queryFn: () => api.sections(corpus) });
  const chapters = useQuery({
    queryKey: ["sections", corpus, expanded],
    queryFn: () => api.sections(corpus, expanded!),
    enabled: expanded !== null,
  });

  useEffect(() => {
    panel.current?.querySelector<HTMLElement>("[aria-current]")?.scrollIntoView({ block: "center" });
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, books.data]);

  return (
    <>
      <div className="fixed inset-0 z-20 bg-[rgba(21,23,29,0.25)]" onClick={onClose} aria-hidden />
      <nav
        ref={panel}
        aria-label="Books"
        className="fixed inset-y-0 start-0 z-30 flex w-[17rem] max-w-[85vw] flex-col border-e border-rule bg-raised shadow-lg"
      >
        <header className="flex items-center border-b border-rule px-4 py-2.5">
          <h2 className="text-[15px] font-semibold">Navigate</h2>
          <button
            type="button"
            onClick={onClose}
            className="ms-auto rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          >
            Close
          </button>
        </header>
        <ol className="min-h-0 flex-1 overflow-y-auto py-1">
          {(books.data?.children ?? []).map((b) => {
            const isOpen = expanded === b.ref;
            return (
              <li key={b.node}>
                <button
                  type="button"
                  aria-expanded={isOpen}
                  aria-current={b.ref === currentBook ? "true" : undefined}
                  onClick={() => setExpanded(isOpen ? null : b.ref)}
                  className="w-full px-4 py-1.5 text-left text-[14px] hover:bg-woad-soft aria-[current=true]:border-s-2 aria-[current=true]:border-woad aria-[current=true]:font-semibold"
                >
                  {displayRef(b.label)}
                </button>
                {isOpen ? (
                  <ol className="grid grid-cols-[repeat(auto-fill,minmax(2.4rem,1fr))] gap-1 px-4 pb-2 pt-1">
                    {(chapters.data?.children ?? []).map((c) => (
                      <li key={c.node}>
                        <button
                          type="button"
                          onClick={() => onOpen(c.ref)}
                          aria-current={c.ref === current ? "page" : undefined}
                          className="w-full rounded py-1 text-center text-[13px] tabular-nums hover:bg-woad-soft aria-[current=page]:bg-woad aria-[current=page]:text-vellum"
                        >
                          {c.label}
                        </button>
                      </li>
                    ))}
                    {chapters.isLoading ? <li className="col-span-full text-[13px] text-stone">…</li> : null}
                  </ol>
                ) : null}
              </li>
            );
          })}
        </ol>
      </nav>
    </>
  );
}

/** The columns shown: on or off per open corpus, in an order of choice, and more from the library. */
function Texts({
  names,
  cards,
  texts,
  onChange,
  onAdd,
}: {
  names: string[];
  cards: Record<string, CorpusCard>;
  texts: string[];
  onChange: (texts: string[]) => void;
  onAdd: () => void;
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

  // the shown texts in their order, then the others
  const listed = [...texts.filter((t) => names.includes(t)), ...names.filter((n) => !texts.includes(n))];
  const move = (name: string, by: number) => {
    const i = texts.indexOf(name);
    const j = i + by;
    if (i < 0 || j < 0 || j >= texts.length) return;
    const next = [...texts];
    [next[i], next[j]] = [next[j], next[i]];
    onChange(next);
  };

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft aria-expanded:bg-woad-soft"
      >
        Texts
      </button>
      {open ? (
        <div className="absolute end-0 top-full z-30 mt-1 w-[19rem] rounded border border-rule-strong bg-raised py-1.5 shadow-lg">
          <ul>
            {listed.map((name) => {
              const shown = texts.includes(name);
              const i = texts.indexOf(name);
              return (
                <li key={name} className="flex items-center gap-2 px-3 py-1 text-[14px]">
                  <label className="flex min-w-0 flex-1 items-center gap-2">
                    <input
                      type="checkbox"
                      checked={shown}
                      onChange={(e) => onChange(e.target.checked ? [...texts, name] : texts.filter((t) => t !== name))}
                      className="accent-[var(--woad)]"
                    />
                    <span className="truncate" title={cards[name]?.title}>
                      {name} <span className="text-[12px] text-stone">{cards[name]?.version}</span>
                    </span>
                  </label>
                  {shown ? (
                    <span className="flex text-[12px] text-stone">
                      <button
                        type="button"
                        disabled={i === 0}
                        onClick={() => move(name, -1)}
                        className="rounded px-1 hover:bg-woad-soft disabled:opacity-30"
                        aria-label={`Move ${name} to the left`}
                      >
                        ←
                      </button>
                      <button
                        type="button"
                        disabled={i === texts.length - 1}
                        onClick={() => move(name, 1)}
                        className="rounded px-1 hover:bg-woad-soft disabled:opacity-30"
                        aria-label={`Move ${name} to the right`}
                      >
                        →
                      </button>
                    </span>
                  ) : null}
                </li>
              );
            })}
          </ul>
          <div className="mt-1 border-t border-rule px-3 pt-1.5">
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                onAdd();
              }}
              className="text-[14px] text-woad hover:underline"
            >
              Add a text…
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}

/** The first chapter of the first book, to start with. */
async function firstChapter(corpus: string): Promise<string | null> {
  const books = await api.sections(corpus);
  const first = books.children[0];
  if (!first) return null;
  const chapters = await api.sections(corpus, first.ref);
  return chapters.children[0]?.ref ?? first.ref;
}

export function ParallelApp() {
  const [state, actions] = useParallel();
  const [settings] = useSettings();
  const [theme, toggleTheme] = useTheme();
  const [navOpen, setNavOpen] = useState(false);
  const closeNav = useCallback(() => setNavOpen(false), []);
  const [library, setLibrary] = useState(false);
  const { corpora, names, cards, ready } = useCards();
  const reference = useReference(cards, ready);
  // the word shown in the panel on the left
  const [inspected, setInspected] = useState<{ corpus: string; node: number } | null>(null);
  const [lexiconHovered, onLexiconHover] = useHover();
  // search terms made from words; they last while the page is open
  const [terms, setTerms] = useState<Term[]>([]);

  const texts = (state.texts ?? []).filter((t) => names.includes(t));
  // the numbering the rows follow: the BHSA, or else the first column's
  const lead = reference.name ?? texts[0] ?? names[0] ?? null;

  // a first visit shows every open corpus
  useEffect(() => {
    if (state.texts === null && ready && reference.settled && names.length) {
      actions.setTexts(names);
    }
  }, [state.texts, ready, reference.settled, names, actions]);

  // started without a chapter: the first one
  useEffect(() => {
    if (state.ref || !lead || !reference.settled) return;
    let current = true;
    firstChapter(lead).then((ref) => current && ref && actions.open(ref, false));
    return () => {
      current = false;
    };
  }, [state.ref, lead, reference.settled, actions]);

  // started without corpora: offer the library straight away, once
  const noCorpora = corpora.data !== undefined && names.length === 0;
  const offered = useRef(false);
  useEffect(() => {
    if (noCorpora && !offered.current) {
      offered.current = true;
      setLibrary(true);
    }
  }, [noCorpora]);

  // the chapter in all shown corpora, verse by verse
  const chapter = useQuery({
    queryKey: ["parallel", lead, state.ref, texts.join(","), settings.fmt, settings.hideTypes],
    queryFn: () =>
      api.parallel(lead!, state.ref!, texts, { mode: "plain", fmt: settings.fmt, hide_types: settings.hideTypes }),
    enabled: lead !== null && state.ref !== null && ready,
    placeholderData: (previous) => previous,
  });
  // differences: the columns as reading panes, the first one compared with the others
  const comparable = (chapter.data?.columns ?? []).filter((c) => c.ref !== null && cards[c.name]);
  const diffList = useDifferences(
    comparable.map((c) => ({ ...emptyPane(c.name), ref: c.ref, by: c.kind === "witnesses" ? "books" : "sections" })),
    state.diff,
  );
  const diffs: Record<string, PaneDiff | null> = {};
  comparable.forEach((c, i) => (diffs[c.name] = diffList[i]));
  const baseDiff = comparable.length ? diffs[comparable[0].name] : null;

  const prev = chapter.data?.prev ?? null;
  const next = chapter.data?.next ?? null;

  useEffect(() => {
    document.title = state.ref ? `${displayRef(state.ref)}: Context-Fabric` : "Context-Fabric Browser";
  }, [state.ref]);

  /** Read the chapter of a passage of `corpus` (`Genesis 1:3`, a verse of a lexeme), at its verse. */
  const openPassage = async (corpus: string, ref: string) => {
    if (!lead) return;
    const aligned = await alignRef(corpus, ref, lead);
    if (!aligned) return;
    const verse = Number(aligned.split(":")[1]);
    actions.open(chapterOf(aligned), true, Number.isInteger(verse) ? verse : null);
  };

  // the corpus whose lexicon the top bar offers (BHSA)
  const lexiconCorpus = [...texts, ...names].find((n) => cards[n]?.lexicon) ?? null;
  const lexiconCard = state.lexicon ? cards[state.lexicon] : undefined;

  // ← and → go to the previous and next chapter
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (state.lexicon) return;
      if (e.altKey || e.ctrlKey || e.metaKey || target.closest("input, textarea, select, [contenteditable]")) return;
      if (e.key === "ArrowLeft" && prev) actions.open(prev.ref);
      if (e.key === "ArrowRight" && next) actions.open(next.ref);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [prev, next, actions, state.lexicon]);

  if (corpora.error) {
    return (
      <main className="p-10">
        <h1 className="mb-2 text-[20px] font-semibold">The browser server cannot be reached</h1>
        <p className="text-stone">
          Check that <code className="font-mono">cfabric-browser</code> is still running, then reload this page.
        </p>
      </main>
    );
  }

  const shown = texts.map((t) => cards[t]).filter((c): c is CorpusCard => c !== undefined);
  const arrow = "rounded px-2 py-1 text-[16px] leading-none hover:bg-woad-soft disabled:opacity-30";

  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-1 border-b border-rule bg-raised px-2 py-1.5 sm:gap-2 sm:px-3">
        <span className="me-1 flex items-center gap-2" title="Context-Fabric">
          <Mark />
        </span>
        <button
          type="button"
          disabled={!prev}
          onClick={() => prev && actions.open(prev.ref)}
          className={arrow}
          aria-label="Previous chapter"
          title={prev ? `Previous: ${displayRef(prev.ref)}` : undefined}
        >
          ‹
        </button>
        <button
          type="button"
          onClick={() => setNavOpen(true)}
          disabled={!lead}
          className="rounded px-2 py-1 text-[15px] font-semibold hover:bg-woad-soft"
          aria-label="Choose a book and chapter"
        >
          {state.ref ? displayRef(state.ref) : "…"}
        </button>
        <button
          type="button"
          disabled={!next}
          onClick={() => next && actions.open(next.ref)}
          className={arrow}
          aria-label="Next chapter"
          title={next ? `Next: ${displayRef(next.ref)}` : undefined}
        >
          ›
        </button>
        <div className="ms-auto flex items-center gap-1 sm:gap-2">
          <button
            type="button"
            aria-pressed={state.searching}
            disabled={!texts.length}
            onClick={() => (state.searching ? actions.setSearch({ searching: false }) : actions.setSearch({}))}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft disabled:opacity-40 aria-pressed:bg-ink aria-pressed:text-vellum"
            title="Search with terms made from words, or with a template"
          >
            Search{terms.length ? ` (${terms.length})` : ""}
          </button>
          <button
            type="button"
            aria-pressed={state.diff}
            disabled={texts.length < 2}
            onClick={() => actions.setDiff(!state.diff)}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft disabled:opacity-40 aria-pressed:bg-ink aria-pressed:text-vellum"
            title="Mark the words where the texts differ from the first column, compared on their consonants"
          >
            Differences
          </button>
          {lexiconCorpus ? (
            <button
              type="button"
              aria-pressed={state.lexicon !== null}
              onClick={() => actions.setLexicon(state.lexicon ? null : lexiconCorpus)}
              className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft aria-pressed:bg-ink aria-pressed:text-vellum"
              title={`The lexicon of ${lexiconCorpus}`}
            >
              Lexicon
            </button>
          ) : null}
          <Texts names={names} cards={cards} texts={texts} onChange={actions.setTexts} onAdd={() => setLibrary(true)} />
          <button
            type="button"
            onClick={toggleTheme}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft"
            aria-label={theme === "dark" ? "Use light colours" : "Use dark colours"}
          >
            {theme === "dark" ? "Light" : "Dark"}
          </button>
          <UiSwitch
            current="parallel"
            onSwitch={() =>
              switchToClassic(
                state,
                reference.name,
                texts.filter((t) => cards[t]?.bookView),
              )
            }
          />
        </div>
      </header>

      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        {inspected && cards[inspected.corpus] ? (
          <div className="max-h-[45vh] min-h-0 border-b border-rule md:max-h-none md:border-b-0">
            <Inspector
              corpus={cards[inspected.corpus]}
              node={inspected.node}
              side="start"
              onInspect={(node) => setInspected({ corpus: inspected.corpus, node })}
              onClose={() => setInspected(null)}
              onOpenPassage={(ref) => void openPassage(inspected.corpus, ref)}
              onLexicon={(lex) => actions.setLexicon(inspected.corpus, lex)}
              onSearchTerm={(info) => {
                setTerms([...terms, termFrom(inspected.corpus, info)]);
                actions.setSearch({ sc: inspected.corpus });
              }}
            />
          </div>
        ) : null}
        <main className="min-h-0 min-w-0 flex-1 overflow-y-auto">
          {state.searching && texts.length ? (
            <ParallelSearch
              cards={cards}
              texts={texts}
              search={state}
              terms={terms}
              settings={settings}
              onTerms={setTerms}
              onSearch={actions.setSearch}
              onOpenPassage={(corpus, ref) => void openPassage(corpus, ref)}
              onInspect={(corpus, node) => setInspected({ corpus, node })}
            />
          ) : lexiconCard ? (
            <div className="flex h-full min-h-0 flex-col">
              <p className="border-b border-rule px-4 py-1.5 text-[13px]">
                <button type="button" onClick={() => actions.setLexicon(null)} className="text-woad hover:underline">
                  ← Back to {state.ref ? displayRef(state.ref) : "the text"}
                </button>
              </p>
              <div className="min-h-0 flex-1">
                <DisplayProvider
                  config={lexiconCard.display}
                  settings={settingsFor(lexiconCard, settings)}
                  onInspect={(node) => setInspected({ corpus: lexiconCard.name, node })}
                  onHover={onLexiconHover}
                >
                  <LexiconView
                    corpus={lexiconCard}
                    lex={state.lex}
                    settings={settingsFor(lexiconCard, settings)}
                    onLex={(lex) => actions.setLexicon(lexiconCard.name, lex)}
                    onOpenPassage={(ref) => void openPassage(lexiconCard.name, ref)}
                    onSearch={(q) => actions.setSearch({ sc: lexiconCard.name, q, s: null })}
                  />
                  {lexiconHovered ? <HoverCard corpus={lexiconCard} hovered={lexiconHovered} /> : null}
                </DisplayProvider>
              </div>
            </div>
          ) : noCorpora ? (
            <div className="p-10">
              <h1 className="mb-2 text-[20px] font-semibold">No corpus is open</h1>
              <p className="mb-4 max-w-prose text-stone">Add a text from the corpora on this computer.</p>
              <button
                type="button"
                onClick={() => setLibrary(true)}
                className="rounded bg-ink px-3 py-1.5 text-[14px] text-vellum hover:opacity-90"
              >
                Add a text
              </button>
            </div>
          ) : !ready || !state.ref || !lead ? (
            <p className="p-10 text-stone">Opening…</p>
          ) : !shown.length ? (
            <p className="p-10 text-stone">No text is shown. Choose one under Texts.</p>
          ) : chapter.error ? (
            <p className="p-10 text-madder">{String((chapter.error as Error).message)}</p>
          ) : chapter.data ? (
            <div className={chapter.isFetching ? "opacity-70" : ""}>
              {state.diff && baseDiff ? (
                <div className="mx-auto max-w-[60rem] px-5 pt-2">
                  <DiffNote message={`Compared with the first column, ${comparable[0].name}, on consonants`} />
                </div>
              ) : null}
              <Table
                data={chapter.data}
                cards={cards}
                settings={settings}
                verse={state.verse}
                diffs={state.diff ? diffs : {}}
                onRemove={(name) => actions.setTexts(texts.filter((t) => t !== name))}
                onInspect={(corpus, node) => setInspected({ corpus, node })}
              />
            </div>
          ) : (
            <p className="p-10 text-stone">Loading…</p>
          )}
        </main>
      </div>

      {navOpen && lead ? (
        <Navigate
          corpus={lead}
          current={state.ref}
          onOpen={(ref) => {
            setNavOpen(false);
            actions.open(ref);
          }}
          onClose={closeNav}
        />
      ) : null}
      {library ? (
        <Library
          onOpen={(name) => {
            setLibrary(false);
            if (!texts.includes(name)) actions.setTexts([...texts, name]);
          }}
          onClose={() => setLibrary(false)}
        />
      ) : null}
    </div>
  );
}
