import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../api";
import { DisplayProvider, displayRef, Plain } from "../display/Display";
import { Library } from "../Library";
import { useSettings, useTheme } from "../state";
import { switchToClassic } from "../switch";
import type { CorpusCard, ViewSettings } from "../types";
import { REFERENCE_IDS } from "../ui";
import { UiSwitch } from "../UiSwitch";
import { useParallel } from "./state";

// The parallel interface, after Parabible's layout: a slim top bar with the
// chapter, a drawer with the books, and a column per corpus.

function Mark() {
  return <img src="/favicon.svg" alt="" className="h-5 w-5" />;
}

/** The text formats a corpus has; another corpus' format falls back to its default. */
function settingsFor(card: CorpusCard, settings: ViewSettings): ViewSettings {
  return settings.fmt && !card.display.textFormats.includes(settings.fmt) ? { ...settings, fmt: null } : settings;
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
  return { corpora, names, cards: cards.data ?? {}, ready: corpora.data !== undefined && (!names.length || !!cards.data) };
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
  const settled = ready && (library.isError || (library.data !== undefined && (!entry || !library.data.enabled)) || failed);
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
                      onChange={(e) =>
                        onChange(e.target.checked ? [...texts, name] : texts.filter((t) => t !== name))
                      }
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

/** One corpus' text of the chapter, verse by verse. (Rows aligned across columns come next.) */
function Column({
  card,
  reference,
  chapter,
  settings,
  onRemove,
}: {
  card: CorpusCard;
  reference: string;
  chapter: string;
  settings: ViewSettings;
  onRemove: () => void;
}) {
  const aligned = useQuery({
    queryKey: ["align", card.name, reference, chapter],
    queryFn: () =>
      card.name === reference
        ? Promise.resolve({ node: 0, ref: chapter, exact: true })
        : api.align(card.name, reference, chapter),
    enabled: !card.bookView,
  });
  const ref = aligned.data?.node === null ? null : (aligned.data?.ref ?? null);
  const own = settingsFor(card, settings);
  const passage = useQuery({
    queryKey: ["passage", card.name, ref, "plain", own.fmt, own.hideTypes],
    queryFn: () => api.passage(card.name, ref!, { mode: "plain", fmt: own.fmt, hide_types: own.hideTypes }),
    enabled: ref !== null,
    placeholderData: (previous) => previous,
  });

  return (
    <section className="min-w-0" aria-label={card.name}>
      <header className="sticky top-0 z-10 flex items-center border-b border-rule bg-vellum px-2 py-1 text-[13px]">
        <span className="flex-1 text-center font-semibold" title={card.title}>
          {card.name}
        </span>
        <button
          type="button"
          onClick={onRemove}
          className="rounded px-1.5 text-stone hover:bg-woad-soft hover:text-ink"
          aria-label={`Remove ${card.name}`}
          title="Remove this text"
        >
          –
        </button>
      </header>
      <div className={`px-2 py-2 ${passage.isFetching ? "opacity-70" : ""}`}>
        {card.bookView ? (
          <p className="text-[13px] text-stone">The scrolls will be shown here verse by verse.</p>
        ) : aligned.data?.node === null ? (
          <p className="text-[13px] text-stone">Not in {card.name}.</p>
        ) : passage.error ? (
          <p className="text-[13px] text-madder">{String((passage.error as Error).message)}</p>
        ) : passage.data ? (
          <DisplayProvider config={card.display} settings={own}>
            {passage.data.trees.map((tree) => (
              <p key={tree.node} dir={tree.direction} className="py-1">
                <sup dir="ltr" className="me-1 text-[11px] font-semibold tabular-nums text-madder">
                  {tree.heading.split(/[ :.]/).pop()}
                </sup>
                <Plain tree={tree} />
              </p>
            ))}
          </DisplayProvider>
        ) : (
          <p className="text-[13px] text-stone">…</p>
        )}
      </div>
    </section>
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

  // the chapter in the lead corpus, for its neighbours
  const leadCard = lead ? cards[lead] : undefined;
  const leadSettings = leadCard ? settingsFor(leadCard, settings) : settings;
  const leadPassage = useQuery({
    queryKey: ["passage", lead, state.ref, "plain", leadSettings.fmt, leadSettings.hideTypes],
    queryFn: () =>
      api.passage(lead!, state.ref!, { mode: "plain", fmt: leadSettings.fmt, hide_types: leadSettings.hideTypes }),
    enabled: lead !== null && state.ref !== null && leadCard !== undefined,
    placeholderData: (previous) => previous,
  });
  const prev = leadPassage.data?.prev ?? null;
  const next = leadPassage.data?.next ?? null;

  useEffect(() => {
    document.title = state.ref ? `${displayRef(state.ref)}: Context-Fabric` : "Context-Fabric Browser";
  }, [state.ref]);

  // ← and → go to the previous and next chapter
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (e.altKey || e.ctrlKey || e.metaKey || target.closest("input, textarea, select, [contenteditable]")) return;
      if (e.key === "ArrowLeft" && prev) actions.open(prev.ref);
      if (e.key === "ArrowRight" && next) actions.open(next.ref);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [prev, next, actions]);

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
          <Texts
            names={names}
            cards={cards}
            texts={texts}
            onChange={actions.setTexts}
            onAdd={() => setLibrary(true)}
          />
          <button
            type="button"
            onClick={toggleTheme}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft"
            aria-label={theme === "dark" ? "Use light colours" : "Use dark colours"}
          >
            {theme === "dark" ? "Light" : "Dark"}
          </button>
          <UiSwitch current="parallel" onSwitch={() => switchToClassic(state, reference.name)} />
        </div>
      </header>

      <main className="min-h-0 flex-1 overflow-y-auto">
        {noCorpora ? (
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
        ) : (
          <div
            className="mx-auto grid gap-x-4 px-3 py-2"
            style={{
              gridTemplateColumns: `repeat(${shown.length}, minmax(0, 1fr))`,
              maxWidth: `${Math.min(shown.length * 26, 110)}rem`,
            }}
          >
            {shown.map((card) => (
              <Column
                key={card.name}
                card={card}
                reference={lead}
                chapter={state.ref!}
                settings={settings}
                onRemove={() => actions.setTexts(texts.filter((t) => t !== card.name))}
              />
            ))}
          </div>
        )}
      </main>

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
