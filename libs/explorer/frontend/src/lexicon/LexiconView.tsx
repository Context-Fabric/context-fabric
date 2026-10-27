import { useQuery } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useMemo, useRef, useState } from "react";
import { api } from "../api";
import { displayRef, Plain, scriptClass } from "../display/Display";
import type { CorpusCard, Lexeme, LexiconEntry, ViewSettings } from "../types";
import { BdbSection } from "./Bdb";
import { letterOf, shownLexemes, type Order } from "./list";
import { keepTab, keptTab, type Tab } from "./tab";

// The lexicon of a corpus with lexeme nodes (BHSA), after SHEBANQ's Words page:
// per language and first letter a list of lexemes with their glosses; a lexeme
// shows every verse it occurs in, with its words marked.

const LETTERS = "אבגדהוזחטיכלמנסעפצקרשת".split("");
const PAGE = 50;

/** Parts of speech of the BHSA, in words. */
const PARTS: Record<string, string> = {
  subs: "noun",
  nmpr: "proper noun",
  verb: "verb",
  adjv: "adjective",
  advb: "adverb",
  prep: "preposition",
  conj: "conjunction",
  prps: "personal pronoun",
  prde: "demonstrative pronoun",
  prin: "interrogative pronoun",
  intj: "interjection",
  nega: "negation",
  inrg: "interrogative",
  art: "article",
};

const part = (sp: string | null | undefined) => (sp ? (PARTS[sp] ?? sp) : "");

function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}

interface Props {
  corpus: CorpusCard;
  lex: number | null;
  settings: ViewSettings;
  onLex: (node: number | null) => void;
  onOpenPassage: (ref: string) => void;
  /** Search for the words of a lexeme, in the search view. */
  onSearch: (template: string) => void;
  /** In a narrow pane the list and a lexeme are shown one at a time. */
  compact?: boolean;
}

export function LexiconView({ corpus, lex, settings, onLex, onOpenPassage, onSearch, compact = false }: Props) {
  const { data, error } = useQuery({
    queryKey: ["lexicon", corpus.name],
    queryFn: () => api.lexicon(corpus.name),
    staleTime: Infinity,
  });

  if (error) return <p className="p-6 text-madder">Could not load the lexicon: {String((error as Error).message)}</p>;
  if (!data) return <p className="p-6 text-stone">Loading the lexicon…</p>;

  const detail =
    lex !== null ? (
      <LexemeDetail
        corpus={corpus}
        node={lex}
        settings={settings}
        onBack={compact ? () => onLex(null) : undefined}
        onLex={onLex}
        onOpenPassage={onOpenPassage}
        onSearch={onSearch}
      />
    ) : (
      <Intro corpus={corpus} count={data.entries.length} />
    );

  if (compact && lex !== null) return <div className="h-full min-h-0">{detail}</div>;
  return (
    // one row of the pane's height: the list and the verses scroll, not the page
    <div
      className={`grid h-full min-h-0 grid-cols-1 grid-rows-[minmax(0,1fr)] ${
        compact ? "" : "md:grid-cols-[minmax(22rem,30rem)_minmax(0,1fr)]"
      }`}
    >
      <LexiconList corpus={corpus} entries={data.entries} selected={lex} onSelect={onLex} />
      {compact ? null : <section className="min-h-0 min-w-0 border-rule md:border-s">{detail}</section>}
    </div>
  );
}

function Intro({ corpus, count }: { corpus: CorpusCard; count: number }) {
  return (
    <div className="max-w-[38rem] p-8 text-[15px] leading-relaxed">
      <h1 className="mb-3 text-[22px] font-semibold">Lexicon of {corpus.title.split(" = ").pop()}</h1>
      <p className="mb-3">
        {formatCount(count)} lexemes, by language and first letter. Choose one to see every verse it occurs in, with the
        word marked.
      </p>
      <p className="text-stone">
        Find a lexeme in all letters at once by its Hebrew, its transliteration, its gloss or, when BDB has been added,
        BDB's definition: <em>create</em> finds ברא.
      </p>
    </div>
  );
}

interface ListProps {
  corpus: CorpusCard;
  entries: LexiconEntry[];
  selected: number | null;
  onSelect: (node: number) => void;
}

function LexiconList({ corpus, entries, selected, onSelect }: ListProps) {
  const languages = useMemo(() => {
    const counts = new Map<string, number>();
    for (const e of entries) counts.set(e.language ?? "", (counts.get(e.language ?? "") ?? 0) + 1);
    // Hebrew first, then the others (Aramaic)
    return [...counts.entries()].sort(([a], [b]) => (a === "Hebrew" ? -1 : b === "Hebrew" ? 1 : a.localeCompare(b)));
  }, [entries]);
  const selectedEntry = entries.find((e) => e.node === selected);
  const [language, setLanguage] = useState(selectedEntry?.language ?? languages[0]?.[0] ?? "");
  const [letter, setLetter] = useState(selectedEntry?.letter ?? "א");
  const [filter, setFilter] = useState("");
  const [sp, setSp] = useState("");
  const [order, setOrder] = useState<Order>("alpha");
  // ordering by root needs BDB
  const hasBdb = useMemo(() => entries.some((e) => e.bdbOrder !== undefined), [entries]);
  const shownOrder: Order = order === "root" && !hasBdb ? "alpha" : order;

  const letters = useMemo(
    () => new Set(entries.filter((e) => (e.language ?? "") === language).map((e) => letterOf(e, shownOrder))),
    [entries, language, shownOrder],
  );
  const parts = useMemo(() => [...new Set(entries.map((e) => e.sp).filter(Boolean))] as string[], [entries]);

  const needle = filter.trim().toLowerCase();
  const shown = useMemo(
    () => shownLexemes(entries, { language, letter, sp, find: filter, order: shownOrder }),
    [entries, language, letter, sp, filter, shownOrder],
  );

  const script = scriptClass(corpus.display, "rtl");
  return (
    <aside className="flex min-h-0 flex-col overflow-hidden border-rule" aria-label="Lexicon">
      <div className="flex flex-col gap-2 border-b border-rule p-3">
        {languages.length > 1 ? (
          <div className="flex gap-0.5 text-[13px]" role="group" aria-label="Language">
            {languages.map(([name, count]) => (
              <button
                key={name}
                type="button"
                aria-pressed={language === name}
                onClick={() => setLanguage(name)}
                className="rounded px-2.5 py-0.5 hover:bg-woad-soft aria-pressed:bg-ink aria-pressed:text-vellum"
              >
                {name || "Other"} <span className="tabular-nums opacity-70">{formatCount(count)}</span>
              </button>
            ))}
          </div>
        ) : null}
        <div dir="rtl" className={`flex flex-wrap gap-0.5 ${script}`} role="group" aria-label="First letter">
          {LETTERS.map((l) => (
            <button
              key={l}
              type="button"
              disabled={!letters.has(l)}
              aria-pressed={!needle && letter === l}
              onClick={() => {
                setLetter(l);
                setFilter("");
              }}
              className="min-w-[1.9rem] rounded px-1 py-0.5 text-[18px] leading-tight hover:bg-woad-soft disabled:opacity-30 aria-pressed:bg-ink aria-pressed:text-vellum"
            >
              {l}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Find: ברא, BR>[, create"
            aria-label="Find a lexeme"
            className="min-w-0 flex-1 rounded border border-rule-strong bg-vellum px-2 py-1 text-[14px]"
          />
          <select
            value={sp}
            onChange={(e) => setSp(e.target.value)}
            aria-label="Part of speech"
            className="rounded border border-rule-strong bg-vellum px-1.5 py-1 text-[13px]"
          >
            <option value="">All parts of speech</option>
            {parts.map((p) => (
              <option key={p} value={p}>
                {part(p)}
              </option>
            ))}
          </select>
          <select
            value={shownOrder}
            onChange={(e) => setOrder(e.target.value as Order)}
            aria-label="Order"
            className="rounded border border-rule-strong bg-vellum px-1.5 py-1 text-[13px]"
          >
            <option value="alpha">Alphabetical</option>
            <option value="frequency">Most frequent first</option>
            {hasBdb ? <option value="root">By root (BDB)</option> : null}
          </select>
        </div>
        <p className="text-[12px] text-stone" role="status">
          {formatCount(shown.length)} {shown.length === 1 ? "lexeme" : "lexemes"}
          {needle ? ` matching “${filter.trim()}”` : shownOrder === "root" ? ` with a root in ${letter}` : ` starting with ${letter}`}
        </p>
      </div>
      <ul className="min-h-0 flex-1 overflow-y-auto" aria-label="Lexemes">
        {shown.map((e) => (
          <li key={e.node}>
            <button
              type="button"
              onClick={() => onSelect(e.node)}
              aria-current={e.node === selected ? "true" : undefined}
              className="grid w-full grid-cols-[minmax(5rem,auto)_1fr_auto] items-baseline gap-3 border-b border-rule px-3 py-1.5 text-left hover:bg-woad-soft aria-[current=true]:bg-gold-leaf"
            >
              {/* the script class scales the size: 1.45 × 14px */}
              <span dir="rtl" className={`text-[14px] leading-snug ${shownOrder === "root" && e.bdbRoot === false ? "pe-4" : ""}`}>
                <span className={script}>{e.form || e.cons || e.lex}</span>
              </span>
              <span className="min-w-0">
                <span className="block truncate text-[14px]">{e.gloss}</span>
                <span className="block truncate text-[12px] text-stone">
                  <span className="font-mono">{e.lex}</span> · {part(e.sp)}
                  {e.bdbShort && e.bdbShort !== e.gloss ? <> · BDB: {e.bdbShort}</> : null}
                </span>
              </span>
              <span className="text-[12px] tabular-nums text-stone" title="Occurrences">
                {formatCount(e.freq)}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </aside>
  );
}

interface DetailProps {
  corpus: CorpusCard;
  node: number;
  settings: ViewSettings;
  onBack?: () => void;
  onLex: (node: number) => void;
  onOpenPassage: (ref: string) => void;
  onSearch: (template: string) => void;
}

function LexemeDetail({ corpus, node, settings, onBack, onLex, onOpenPassage, onSearch }: DetailProps) {
  const [book, setBook] = useState<string | null>(null);
  const [tab, setTabState] = useState<Tab>(keptTab);
  const setTab = (t: Tab) => {
    keepTab(t);
    setTabState(t);
  };
  const [shownNode, setShownNode] = useState(node);
  if (shownNode !== node) {
    // another lexeme: start again with all books
    setShownNode(node);
    setBook(null);
  }
  const { data: lexeme, error } = useQuery({
    queryKey: ["lexeme", corpus.name, node],
    queryFn: () => api.lexeme(corpus.name, node),
    staleTime: Infinity,
  });
  if (error) return <p className="p-6 text-madder">Could not load this lexeme: {String((error as Error).message)}</p>;
  if (!lexeme) return <p className="p-6 text-stone">Loading…</p>;

  const script = scriptClass(corpus.display, "rtl");
  const template = `word lex=${lexeme.lex}${lexeme.language ? ` language=${lexeme.language}` : ""}`;
  const total = book === null ? lexeme.verses : null;
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex flex-col gap-2 border-b border-rule px-4 py-3">
        {onBack ? (
          <button type="button" onClick={onBack} className="self-start text-[13px] text-woad hover:underline">
            ← Lexicon
          </button>
        ) : null}
        <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
          <h2 dir="rtl" className="text-[22px] leading-tight">
            <span className={script}>{lexeme.form || lexeme.cons || lexeme.lex}</span>
          </h2>
          <span className="text-[18px]">{lexeme.gloss}</span>
          <span className="font-mono text-[13px] text-stone">{lexeme.lex}</span>
        </div>
        <p className="text-[14px] text-stone">
          {[part(lexeme.sp), lexeme.language, lexeme.nametype ? `name of ${lexeme.nametype}` : null]
            .filter(Boolean)
            .join(" · ")}
          {" · "}
          {formatCount(lexeme.freq)} {lexeme.freq === 1 ? "occurrence" : "occurrences"} in {formatCount(lexeme.verses)}{" "}
          {lexeme.verses === 1 ? "verse" : "verses"}
          {lexeme.rank ? ` · rank ${formatCount(lexeme.rank)} by frequency` : ""}
        </p>
        <Books lexeme={lexeme} book={book} onBook={setBook} />
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[13px]">
          <button
            type="button"
            onClick={() => onSearch(template)}
            className="text-woad hover:underline"
            title={`Search with the template ${template}, to refine it or see statistics`}
          >
            Search for it
          </button>
          <a href={api.occurrencesUrl(corpus.name, node, book, settings.fmt)} className="text-woad hover:underline" download>
            Download (TSV)
          </a>
          <a
            href={api.occurrencesUrl(corpus.name, node, book, settings.fmt, true)}
            className="text-woad hover:underline"
            download
          >
            Download for Excel
          </a>
        </div>
        {lexeme.bdb !== undefined ? (
          <div role="tablist" className="-mb-3 flex gap-4 text-[14px]">
            {(["occurrences", "bdb"] as Tab[]).map((t) => (
              <button
                key={t}
                type="button"
                role="tab"
                aria-selected={tab === t}
                onClick={() => setTab(t)}
                className="border-b-2 border-transparent pb-1.5 text-stone hover:text-ink aria-selected:border-woad aria-selected:text-ink"
              >
                {t === "occurrences" ? "Occurrences" : "BDB"}
              </button>
            ))}
          </div>
        ) : null}
      </header>
      {tab === "bdb" && lexeme.bdb !== undefined ? (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <BdbSection
            key={node}
            corpus={corpus}
            link={lexeme.bdb}
            script={script}
            onOpenPassage={onOpenPassage}
            onLex={onLex}
          />
        </div>
      ) : (
        <Occurrences key={`${node}-${book}`} corpus={corpus} node={node} book={book} total={total} settings={settings} onOpenPassage={onOpenPassage} />
      )}
    </div>
  );
}

/** How often a lexeme occurs per book; choosing a book shows only its verses. */
function Books({ lexeme, book, onBook }: { lexeme: Lexeme; book: string | null; onBook: (book: string | null) => void }) {
  const [open, setOpen] = useState(false);
  if (lexeme.books.length <= 1) return null;
  const most = Math.max(...lexeme.books.map((b) => b.count));
  return (
    <div className="text-[13px]">
      <button type="button" onClick={() => setOpen(!open)} aria-expanded={open} className="text-stone hover:text-ink">
        <span aria-hidden className={`inline-block transition-transform ${open ? "rotate-90" : ""}`}>
          ▸
        </span>{" "}
        In {lexeme.books.length} books{book ? `; showing ${displayRef(book)}` : ""}
      </button>
      {book ? (
        <button type="button" onClick={() => onBook(null)} className="ms-3 text-woad hover:underline">
          All books
        </button>
      ) : null}
      {open ? (
        <ul className="mt-1.5 grid max-h-56 max-w-[40rem] grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2 gap-y-0.5 overflow-y-auto pe-1">
          {lexeme.books.map((b) => (
            <li key={b.book} className="contents">
              <button
                type="button"
                onClick={() => onBook(b.book === book ? null : b.book)}
                aria-pressed={b.book === book}
                className="truncate text-left hover:text-woad aria-pressed:font-semibold aria-pressed:text-woad"
              >
                {displayRef(b.book)}
              </button>
              <span className="h-2 rounded-sm bg-chart opacity-70" style={{ width: `${(100 * b.count) / most}%` }} />
              <span className="text-right tabular-nums text-stone">{formatCount(b.count)}</span>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

interface OccurrencesProps {
  corpus: CorpusCard;
  node: number;
  book: string | null;
  /** The number of verses, when known in advance (all books). */
  total: number | null;
  settings: ViewSettings;
  onOpenPassage: (ref: string) => void;
}

function useOccurrencePage(corpus: string, node: number, book: string | null, page: number, fmt: string | null) {
  return useQuery({
    queryKey: ["occurrences", corpus, node, book, page, fmt],
    queryFn: () => api.occurrences(corpus, node, page * PAGE, PAGE, book, fmt),
    staleTime: Infinity,
  });
}

/** Every verse a lexeme occurs in, loaded page by page while scrolling. */
function Occurrences({ corpus, node, book, total, settings, onOpenPassage }: OccurrencesProps) {
  const first = useOccurrencePage(corpus.name, node, book, 0, settings.fmt);
  const count = total ?? first.data?.total ?? 0;
  const scroller = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count,
    getScrollElement: () => scroller.current,
    estimateSize: () => 72,
    overscan: 6,
  });
  if (first.error) return <p className="p-6 text-madder">Could not load the verses: {String(first.error)}</p>;
  return (
    <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto" role="list" aria-label="Verses">
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((item) => (
          <Verse
            key={item.key}
            index={item.index}
            start={item.start}
            measure={virtualizer.measureElement}
            corpus={corpus.name}
            node={node}
            book={book}
            fmt={settings.fmt}
            onOpenPassage={onOpenPassage}
          />
        ))}
      </div>
    </div>
  );
}

interface VerseProps {
  index: number;
  start: number;
  measure: (el: Element | null) => void;
  corpus: string;
  node: number;
  book: string | null;
  fmt: string | null;
  onOpenPassage: (ref: string) => void;
}

function Verse({ index, start, measure, corpus, node, book, fmt, onOpenPassage }: VerseProps) {
  const page = Math.floor(index / PAGE);
  const { data } = useOccurrencePage(corpus, node, book, page, fmt);
  const row = data?.rows[index - page * PAGE];
  return (
    <div
      ref={measure}
      data-index={index}
      role="listitem"
      className="absolute left-0 right-0 grid grid-cols-[3rem_9rem_minmax(0,1fr)] items-baseline gap-3 border-b border-rule px-4 py-2"
      style={{ transform: `translateY(${start}px)` }}
    >
      <span className="font-mono text-[12px] text-stone">{index + 1}</span>
      {row ? (
        <button
          type="button"
          onClick={() => onOpenPassage(row.ref)}
          className="truncate text-left text-[13px] text-woad hover:underline"
          title={`Read ${displayRef(row.ref)}`}
        >
          {displayRef(row.ref)}
          {row.words.length > 1 ? <span className="text-stone"> ×{row.words.length}</span> : null}
        </button>
      ) : (
        <span className="h-3 w-20 rounded bg-rule" />
      )}
      <div className="min-w-0">{row ? <Plain tree={row.tree} /> : <span className="block h-4 w-2/3 rounded bg-rule" />}</div>
    </div>
  );
}
