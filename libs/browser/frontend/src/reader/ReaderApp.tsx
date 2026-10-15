import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import { splitRef } from "../common";
import { useDifferences, type PaneDiff } from "../differences";
import { DisplayProvider, displayRef, Plain } from "../display/Display";
import { DisplayOptions } from "../DisplayOptions";
import { HoverCard, useHover } from "../HoverCard";
import { InterfaceMenu } from "../InterfaceMenu";
import { Library } from "../Library";
import { settingsFor } from "../parallel/Table";
import { firstChapter, useCards, useReference } from "../shared/corpora";
import { Mark } from "../shared/Mark";
import { Navigate } from "../shared/Navigate";
import { emptyPane, useSettings, useTheme } from "../state";
import { describeReader, switchInterface } from "../switch";
import type { CorpusCard, DiffKind, RenderTree, ViewSettings } from "../types";
import { alignRef, navigate } from "../ui";
import { Workspaces } from "../Workspaces";
import { ReaderPanel } from "./Panel";
import { useReader } from "./state";
import { textOf } from "./tree";
import { closesParagraph, type Mode } from "./url";

// The Reader, after Sefaria: one text in a quiet column, chapter after chapter
// as one scrolls; clicking a verse opens a panel about it.

const MAX_CHAPTERS = 12; // chapters kept below each other while scrolling on

function passageKey(corpus: string, ref: string, settings: ViewSettings) {
  return ["passage", corpus, ref, "plain", settings.fmt, settings.hideTypes];
}

function usePassage(card: CorpusCard | undefined, ref: string | null, settings: ViewSettings) {
  return useQuery({
    queryKey: passageKey(card?.name ?? "", ref ?? "", settings),
    queryFn: () => api.passage(card!.name, ref!, { mode: "plain", fmt: settings.fmt, hide_types: settings.hideTypes }),
    enabled: card !== undefined && ref !== null,
    staleTime: Infinity,
  });
}

/** The number of a verse from its heading (`Genesis 1:3` → 3). */
function verseNumber(tree: RenderTree): number | null {
  return splitRef(tree.heading).verse;
}

interface ChapterProps {
  card: CorpusCard;
  chapter: string;
  settings: ViewSettings;
  mode: Mode;
  selected: string | null;
  marks: Map<number, DiffKind> | undefined;
  onSelect: (ref: string) => void;
  onWord: (ref: string, node: number) => void;
  onHover: (node: number | null, target?: Element) => void;
  onNext: (chapter: string, next: string | null) => void;
}

/** One chapter: its heading and its verses, line by line or as running text. */
function Chapter({ card, chapter, settings, mode, selected, marks, onSelect, onWord, onHover, onNext }: ChapterProps) {
  const { data, error } = usePassage(card, chapter, settings);
  useEffect(() => {
    if (data) onNext(chapter, data.next?.ref ?? null);
  }, [data, chapter, onNext]);

  if (error) return <p className="py-6 text-madder">{String((error as Error).message)}</p>;
  if (!data) return <p className="py-6 text-stone">…</p>;

  const verse = (tree: RenderTree, inline: boolean) => {
    const isSelected = tree.heading === selected;
    const props = {
      "data-verse": tree.heading,
      onClick: () => onSelect(tree.heading),
      className: `cursor-pointer rounded transition-colors ${isSelected ? "bg-gold-leaf/60" : "hover:bg-woad-soft/50"}`,
    };
    const number = verseNumber(tree);
    const content = (
      <DisplayProvider
        config={card.display}
        settings={settings}
        onInspect={(node) => onWord(tree.heading, node)}
        onHover={onHover}
        marks={marks}
      >
        <Plain tree={tree} />
      </DisplayProvider>
    );
    if (inline) {
      return (
        <span key={tree.node} {...props}>
          <sup dir="ltr" className="mx-0.5 text-[11px] font-semibold tabular-nums text-madder">
            {number}
          </sup>
          {content}{" "}
        </span>
      );
    }
    return (
      <p key={tree.node} {...props} dir={tree.direction} className={`${props.className} flex gap-4 px-2 py-1.5`}>
        <span dir="ltr" className="w-7 shrink-0 pt-1 text-end text-[12px] tabular-nums text-stone">
          {number}
        </span>
        <span className="flex-1">{content}</span>
      </p>
    );
  };

  // running text: verses run on; a verse that ends with petuhah or setumah closes a paragraph
  const paragraphs: RenderTree[][] = [[]];
  for (const tree of data.trees) {
    paragraphs[paragraphs.length - 1].push(tree);
    if (closesParagraph(textOf(tree))) paragraphs.push([]);
  }

  return (
    <section aria-label={displayRef(data.ref)} className="pb-6">
      <h2
        data-chapter={chapter}
        className="sticky top-0 z-10 bg-vellum/95 py-2 text-center text-[15px] font-semibold text-stone"
      >
        {displayRef(data.ref)}
      </h2>
      {mode === "verses" ? (
        <div className="flex flex-col">{data.trees.map((t) => verse(t, false))}</div>
      ) : (
        paragraphs
          .filter((p) => p.length)
          .map((p) => (
            <p key={p[0].node} dir={p[0].direction} className="mb-4 px-2 leading-[2.1]">
              {p.map((t) => verse(t, true))}
            </p>
          ))
      )}
    </section>
  );
}

export function ReaderApp() {
  const [state, actions] = useReader();
  const [settings, setSettings] = useSettings();
  const [theme, toggleTheme] = useTheme();
  const { corpora, names, cards, ready } = useCards();
  const reference = useReference(cards, ready);
  const [navOpen, setNavOpen] = useState(false);
  const closeNav = useCallback(() => setNavOpen(false), []);
  const [library, setLibrary] = useState(false);
  // the word shown in full in the panel, of the corpus read or of a parallel
  const [word, setWord] = useState<{ corpus: string; node: number } | null>(null);
  const [hovered, onHover] = useHover();

  const corpus = state.corpus && cards[state.corpus] ? state.corpus : null;
  const card = corpus ? cards[corpus] : undefined;
  const own = card ? settingsFor(card, settings) : settings;
  const others = (state.texts ?? names.filter((n) => n !== corpus)).filter((n) => n !== corpus && cards[n]);

  // started without a corpus or chapter: the BHSA (or the first corpus), at its first chapter
  useEffect(() => {
    if (state.corpus || !ready || !reference.settled) return;
    const start = reference.name ?? names[0];
    if (start) actions.setCorpus(start, null);
  }, [state.corpus, ready, reference.settled, reference.name, names, actions]);
  useEffect(() => {
    if (!corpus || state.ref) return;
    let current = true;
    firstChapter(corpus).then((ref) => current && ref && actions.open(ref, false));
    return () => {
      current = false;
    };
  }, [corpus, state.ref, actions]);

  // the chapters below each other: from the one opened, more as one scrolls on
  const [chain, setChain] = useState<string[]>([]);
  const [nextOf, setNextOf] = useState<Record<string, string | null>>({});
  const scrolledTo = useRef<string | null>(null);
  useEffect(() => {
    if (!state.ref) return;
    if (scrolledTo.current === state.ref) return;
    setChain([state.ref]);
    scrollerRef.current?.scrollTo({ top: 0 });
  }, [state.ref, corpus]);
  const onNext = useCallback((chapter: string, next: string | null) => {
    setNextOf((m) => (m[chapter] === next ? m : { ...m, [chapter]: next }));
  }, []);

  const scrollerRef = useRef<HTMLElement>(null);
  const sentinel = useRef<HTMLDivElement>(null);
  const last = chain[chain.length - 1];
  useEffect(() => {
    const el = sentinel.current;
    if (!el || !last) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const next = nextOf[last];
        if (entries.some((e) => e.isIntersecting) && next && chain.length < MAX_CHAPTERS) {
          setChain((c) => (c.includes(next) ? c : [...c, next]));
        }
      },
      { root: scrollerRef.current, rootMargin: "600px" },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, [last, nextOf, chain.length]);

  // the URL follows the chapter at the top of the window
  useEffect(() => {
    const root = scrollerRef.current;
    if (!root || chain.length < 2) return;
    const onScroll = () => {
      const headings = [...root.querySelectorAll<HTMLElement>("[data-chapter]")];
      const top = root.getBoundingClientRect().top;
      let current = chain[0];
      for (const h of headings) if (h.getBoundingClientRect().top - top < 80) current = h.dataset.chapter!;
      if (current !== scrolledTo.current && current !== state.ref) {
        scrolledTo.current = current;
        actions.scrolled(current);
      }
    };
    root.addEventListener("scroll", onScroll, { passive: true });
    return () => root.removeEventListener("scroll", onScroll);
  }, [chain, state.ref, actions]);

  // the selected verse: from the URL (a verse of its chapter), or clicked anywhere in the chain
  const [selected, setSelected] = useState<string | null>(null);
  // a verse that came with the URL (a link, the lexicon, another interface) is scrolled to once shown
  const pending = useRef<string | null>(state.ref && state.verse !== null ? `${state.ref}:${state.verse}` : null);
  useEffect(() => {
    if (state.ref && state.verse !== null) setSelected(`${state.ref}:${state.verse}`);
  }, [state.ref, state.verse]);
  useEffect(() => {
    if (!pending.current) return;
    const el = scrollerRef.current?.querySelector(`[data-verse="${CSS.escape(pending.current)}"]`);
    if (el) {
      el.scrollIntoView({ block: "center" });
      pending.current = null;
    }
  });
  const selectedChapter = selected ? splitRef(selected).chapter : state.ref;
  const selectedVerse = selected ? splitRef(selected).verse : null;
  const select = (ref: string) => {
    setSelected(ref);
    const { chapter, verse } = splitRef(ref);
    // the URL names the chapter of the selected verse
    scrolledTo.current = chapter;
    actions.select(verse, chapter);
  };

  // parallels and differences for the selected verse's chapter
  const lead = corpus;
  const chapterTexts = corpus ? [corpus, ...others] : [];
  const parallel = useQuery({
    queryKey: ["parallel", lead, selectedChapter, chapterTexts.join(","), settings.fmt, settings.hideTypes],
    queryFn: () =>
      api.parallel(lead!, selectedChapter!, chapterTexts, {
        mode: "plain",
        fmt: settings.fmt,
        hide_types: settings.hideTypes,
      }),
    enabled: lead !== null && selectedChapter !== null && state.panel === "parallels",
    placeholderData: (previous) => previous,
  });
  const comparable = (parallel.data?.columns ?? []).filter((c) => c.ref !== null && cards[c.name]);
  const diffList = useDifferences(
    comparable.map((c) => ({ ...emptyPane(c.name), ref: c.ref, by: c.kind === "witnesses" ? "books" : "sections" })),
    state.diff,
  );
  const diffs: Record<string, PaneDiff | null> = {};
  comparable.forEach((c, i) => (diffs[c.name] = diffList[i]));

  // the selected verse's render tree, for its words
  const selectedPassage = usePassage(card, selectedChapter, own);
  const tree = selectedPassage.data?.trees.find((t) => t.heading === selected);

  // the chapter's neighbours
  const current = usePassage(card, state.ref, own);
  const prev = current.data?.prev ?? null;
  const next = current.data?.next ?? null;
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

  useEffect(() => {
    document.title = state.ref ? `${displayRef(state.ref)} · ${corpus}: Context-Fabric` : "Context-Fabric Browser";
  }, [state.ref, corpus]);

  /** What the Reader shows, for switching. */
  const describe = useMemo(() => () => describeReader(state), [state]);

  /** Read a passage of any corpus here: aligned to the corpus being read. */
  const openPassage = async (from: string, ref: string) => {
    if (!corpus) return;
    const aligned = await alignRef(from, ref, corpus);
    if (!aligned) return;
    const { chapter, verse } = splitRef(aligned);
    scrolledTo.current = null;
    pending.current = aligned;
    actions.open(chapter, true, verse);
  };

  const toParallel = async (change: Parameters<typeof switchInterface>[1]) => {
    const base = await describeReader(state);
    await switchInterface("parallel", { ...base, ...change });
  };

  const noCorpora = corpora.data !== undefined && names.length === 0;
  const arrow = "rounded px-2 py-1 text-[16px] leading-none hover:bg-woad-soft disabled:opacity-30";

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-wrap items-center gap-x-1 gap-y-1 border-b border-rule bg-raised px-2 py-1.5 sm:gap-x-2 sm:px-3">
        <span className="me-1" title="Context-Fabric">
          <Mark />
        </span>
        <button
          type="button"
          disabled={!prev}
          onClick={() => prev && actions.open(prev.ref)}
          className={arrow}
          aria-label="Previous chapter"
        >
          ‹
        </button>
        <button
          type="button"
          onClick={() => setNavOpen(true)}
          disabled={!corpus}
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
        >
          ›
        </button>
        <label className="ms-1 flex items-center gap-1 text-[13px]">
          <span className="sr-only">Text</span>
          <select
            value={corpus ?? ""}
            onChange={async (e) => {
              const target = e.target.value;
              const aligned = corpus && state.ref ? await alignRef(corpus, state.ref, target) : null;
              actions.setCorpus(target, aligned ? splitRef(aligned).chapter : null);
            }}
            className="rounded border border-rule-strong bg-raised px-1.5 py-0.5"
            title="The text being read"
          >
            {names.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <div className="flex rounded border border-rule-strong text-[13px]" role="group" aria-label="Reading mode">
          {(["verses", "running"] as const).map((m) => (
            <button
              key={m}
              type="button"
              aria-pressed={state.mode === m}
              onClick={() => actions.setMode(m)}
              className="px-2.5 py-0.5 aria-pressed:bg-woad aria-pressed:text-vellum"
              title={m === "verses" ? "A line per verse" : "Verses run on, in paragraphs"}
            >
              {m === "verses" ? "Verses" : "Running text"}
            </button>
          ))}
        </div>
        <div className="ms-auto flex flex-wrap items-center justify-end gap-1 sm:gap-2">
          <button
            type="button"
            aria-pressed={state.panel !== null}
            onClick={() => actions.setPanel(state.panel ? null : "parallels")}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft aria-pressed:bg-ink aria-pressed:text-vellum"
          >
            Panel
          </button>
          <DisplayOptions
            settings={settings}
            formats={card?.display.textFormats ?? []}
            defaultFormat={card?.display.defaultFormat ?? null}
            onChange={setSettings}
          />
          <Workspaces corpora={names} current="reader" onLoad={navigate} />
          <button
            type="button"
            onClick={toggleTheme}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft"
            aria-label={theme === "dark" ? "Use light colours" : "Use dark colours"}
          >
            {theme === "dark" ? "Light" : "Dark"}
          </button>
          <InterfaceMenu current="reader" describe={describe} />
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        <main ref={scrollerRef} className="min-h-0 min-w-0 flex-1 overflow-y-auto">
          {noCorpora ? (
            <div className="p-10">
              <h1 className="mb-2 text-[20px] font-semibold">No corpus is open</h1>
              <button
                type="button"
                onClick={() => setLibrary(true)}
                className="rounded bg-ink px-3 py-1.5 text-[14px] text-vellum hover:opacity-90"
              >
                Open a corpus
              </button>
            </div>
          ) : !card || !state.ref ? (
            <p className="p-10 text-stone">Opening…</p>
          ) : (
            <div className={`mx-auto max-w-[44rem] px-4 pb-[40vh] text-[1.15em] ${state.panel ? "md:pb-24" : ""}`}>
              {chain.map((chapter) => (
                <Chapter
                  key={chapter}
                  card={card}
                  chapter={chapter}
                  settings={own}
                  mode={state.mode}
                  selected={selected}
                  marks={state.diff ? diffs[card.name]?.marks : undefined}
                  onSelect={select}
                  onWord={(ref, node) => {
                    select(ref);
                    setWord({ corpus: card.name, node });
                    actions.setPanel("words");
                  }}
                  onHover={onHover}
                  onNext={onNext}
                />
              ))}
              <div ref={sentinel} aria-hidden className="h-4" />
              {chain.length >= MAX_CHAPTERS && nextOf[last] ? (
                <p className="py-4 text-center">
                  <button
                    type="button"
                    onClick={() => actions.open(nextOf[last]!)}
                    className="text-[14px] text-woad hover:underline"
                  >
                    Go on with {displayRef(nextOf[last]!)}
                  </button>
                </p>
              ) : null}
            </div>
          )}
          {hovered && card ? (
            <HoverCard
              corpus={card}
              hovered={hovered}
              note={state.diff ? diffs[card.name]?.titles?.get(hovered.node) : undefined}
            />
          ) : null}
        </main>
        {state.panel && card ? (
          <ReaderPanel
            card={card}
            cards={cards}
            names={names}
            others={others}
            verseRef={selected}
            verse={selectedVerse}
            tree={tree}
            chapter={parallel.data}
            diffs={diffs}
            diff={state.diff}
            settings={settings}
            panel={state.panel}
            word={word}
            onTab={(p) => {
              setWord(null);
              actions.setPanel(p);
            }}
            onClose={() => actions.setPanel(null)}
            onWord={setWord}
            onTexts={actions.setTexts}
            onAddText={() => setLibrary(true)}
            onDiff={actions.setDiff}
            onInspect={(from, node) => {
              setWord({ corpus: from, node });
              actions.setPanel("words");
            }}
            onOpenPassage={(from, ref) => void openPassage(from, ref)}
            onLexicon={(lex) => void toParallel({ lexicon: { corpus: card.name, lex }, searching: false })}
            onSearch={(q) => void toParallel({ search: { corpus: card.name, q, s: null }, searching: true })}
          />
        ) : null}
      </div>

      {navOpen && corpus ? (
        <Navigate
          corpus={corpus}
          current={state.ref}
          onOpen={(ref) => {
            setNavOpen(false);
            scrolledTo.current = null;
            actions.open(ref);
          }}
          onClose={closeNav}
        />
      ) : null}
      {library ? (
        <Library
          onOpen={(name) => {
            setLibrary(false);
            if (!others.includes(name) && name !== corpus) actions.setTexts([...others, name]);
          }}
          onClose={() => setLibrary(false)}
        />
      ) : null}
    </div>
  );
}
