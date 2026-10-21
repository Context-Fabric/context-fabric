import { useQueries, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import type { PaneDiff } from "../differences";
import { DiffNote } from "../read/BookView";
import { displayRef, scriptClass } from "../display/Display";
import { Inspector } from "../Inspector";
import { Table } from "../parallel/Table";
import { useNotes } from "../shared/notes";
import { buildTemplate, rangesOf, termFrom } from "../shared/terms";
import { Texts } from "../shared/Texts";
import type { CorpusCard, NodeInfo, ParallelChapter, RenderTree, ViewSettings } from "../types";
import { wordType, wordsOf } from "./tree";
import { PANELS, type Panel } from "./url";

// The resource panel of the Reader, after Sefaria's: about the selected verse,
// its parallels, its words and their lexemes, searches from its words, and the
// user's own note on it.

const TAB_NAMES: Record<Panel, string> = {
  parallels: "Parallels",
  words: "Words",
  lexicon: "Lexicon",
  search: "Search",
  notes: "Notes",
};

export interface PanelProps {
  card: CorpusCard;
  cards: Record<string, CorpusCard>;
  /** All open corpora, and those shown as parallels. */
  names: string[];
  others: string[];
  /** The selected verse (`Genesis 1:3`), its number, and its render tree. */
  verseRef: string | null;
  verse: number | null;
  tree: RenderTree | undefined;
  /** The verse's chapter in the parallel corpora (from `/api/parallel`). */
  chapter: ParallelChapter | undefined;
  diffs: Record<string, PaneDiff | null>;
  diff: boolean;
  settings: ViewSettings;
  panel: Panel;
  /** The word shown in full, if one is, of the corpus read or of a parallel. */
  word: { corpus: string; node: number } | null;
  onTab: (panel: Panel) => void;
  onClose: () => void;
  onWord: (word: { corpus: string; node: number } | null) => void;
  onTexts: (texts: string[]) => void;
  onAddText: () => void;
  onDiff: (diff: boolean) => void;
  onInspect: (corpus: string, node: number) => void;
  /** Read a passage of `corpus` (`Genesis 1:3`) in the Reader. */
  onOpenPassage: (corpus: string, ref: string) => void;
  /** Show the lexicon at a lexeme, in another interface. */
  onLexicon: (lex: number) => void;
  /** Search with a template, in another interface. */
  onSearch: (template: string) => void;
}

/** The words of the verse with their essentials, one line each. */
function WordList({
  card,
  words,
  action,
  onPick,
}: {
  card: CorpusCard;
  words: number[];
  action: string;
  onPick: (node: number) => void;
}) {
  const summaries = useQueries({
    queries: words.map((n) => ({
      queryKey: ["summary", card.name, n],
      queryFn: () => api.nodeSummary(card.name, n),
      staleTime: Infinity,
    })),
  });
  const script = scriptClass(card.display, card.display.direction);
  return (
    <ul className="flex flex-col">
      {words.map((n, i) => {
        const data = summaries[i].data;
        const gloss = data?.summary.find((s) => s.label === "gloss")?.value;
        const rest = data?.summary
          .filter((s) => !["gloss", "lexeme", "transliteration"].includes(s.label))
          .map((s) => s.value)
          .join(" · ");
        return (
          <li key={n}>
            <button
              type="button"
              onClick={() => onPick(n)}
              className="flex w-full items-baseline gap-3 rounded px-2 py-1.5 text-left hover:bg-woad-soft"
              title={action}
            >
              <span dir={card.display.direction} className={`w-24 shrink-0 text-end ${script}`}>
                {data?.text ?? "…"}
              </span>
              <span className="min-w-0 flex-1 text-[13px]">
                {gloss ? <span className="font-semibold">{gloss}</span> : null}
                {rest ? <span className="block truncate text-stone">{rest}</span> : null}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** The lexemes of the verse's words, with their gloss and frequency. */
function Lexemes({ card, words, onLexicon }: { card: CorpusCard; words: number[]; onLexicon: (lex: number) => void }) {
  const infos = useQueries({
    queries: words.map((n) => ({
      queryKey: ["node", card.name, n],
      queryFn: () => api.nodeInfo(card.name, n),
    })),
  });
  const lexes = [
    ...new Set(infos.flatMap((q) => (q.data ? q.data.up.filter((u) => u.type === "lex").map((u) => u.node) : []))),
  ];
  // the whole lexicon, loaded once and shared with the lexicon view
  const lexicon = useQuery({
    queryKey: ["lexicon", card.name],
    queryFn: () => api.lexicon(card.name),
    staleTime: Infinity,
    enabled: card.lexicon,
  });
  const byNode = useMemo(() => new Map((lexicon.data?.entries ?? []).map((e) => [e.node, e])), [lexicon.data]);
  const script = scriptClass(card.display, card.display.direction);
  if (!card.lexicon) return <p className="text-[13px] text-stone">{card.name} has no lexicon.</p>;
  return (
    <ul className="flex flex-col">
      {lexes.map((lex) => {
        const e = byNode.get(lex);
        return (
          <li key={lex} className="flex items-baseline gap-3 border-b border-rule py-1.5 last:border-b-0">
            <span dir={card.display.direction} className={`w-24 shrink-0 text-end ${script}`}>
              {e?.form ?? e?.cons ?? "…"}
            </span>
            <span className="min-w-0 flex-1 text-[13px]">
              <span className="font-semibold">{e?.gloss ?? ""}</span>{" "}
              <span className="font-mono text-[12px] text-stone">{e?.lex}</span>
              {e ? (
                <span className="block text-stone">
                  {e.sp ? `${e.sp} · ` : ""}
                  {e.freq.toLocaleString("en-US")}× in the corpus
                </span>
              ) : null}
              {e?.bdbShort ? <span className="block">BDB: {e.bdbShort}</span> : null}
            </span>
            <button
              type="button"
              onClick={() => onLexicon(lex)}
              className="shrink-0 text-[12px] text-woad hover:underline"
              title="Every verse it occurs in, in the lexicon"
            >
              All
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** The user's note on the verse, and all their notes in this corpus. */
function NotesTab({
  corpus,
  verseRef,
  onOpen,
}: {
  corpus: string;
  verseRef: string | null;
  onOpen: (ref: string) => void;
}) {
  const [notes, save] = useNotes(corpus);
  const note = verseRef ? notes[verseRef] : undefined;
  const [text, setText] = useState(note?.text ?? "");
  const [failed, setFailed] = useState(false);
  useEffect(() => setText(note?.text ?? ""), [verseRef, note?.text]);
  const all = Object.entries(notes).sort(([, a], [, b]) => b.updated.localeCompare(a.updated));
  const store = (change: { text?: string; starred?: boolean }) => {
    if (!verseRef) return;
    setFailed(!save(verseRef, { text: change.text ?? text, starred: change.starred ?? note?.starred ?? false }));
  };

  return (
    <div className="flex flex-col gap-3">
      {verseRef ? (
        <section>
          <div className="mb-1 flex items-center gap-2">
            <h3 className="text-[13px] text-stone">Your note on {displayRef(verseRef)}</h3>
            <button
              type="button"
              aria-pressed={note?.starred ?? false}
              onClick={() => store({ starred: !(note?.starred ?? false) })}
              className="ms-auto rounded px-1.5 text-[15px] text-gold-deep hover:bg-woad-soft"
              title={note?.starred ? "Remove the bookmark" : "Bookmark this verse"}
            >
              {note?.starred ? "★" : "☆"}
            </button>
          </div>
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            onBlur={() => text !== (note?.text ?? "") && store({ text })}
            rows={4}
            placeholder="Write a note; it is kept on this computer"
            className="w-full rounded border border-rule-strong bg-vellum px-2 py-1.5 text-[14px] placeholder:text-stone"
          />
          {failed ? <p className="text-[12px] text-madder">This browser does not allow saving notes here.</p> : null}
        </section>
      ) : (
        <p className="text-[13px] text-stone">Select a verse to write a note on it.</p>
      )}
      {all.length ? (
        <section>
          <h3 className="mb-1 text-[13px] text-stone">All your notes in {corpus}</h3>
          <ul className="flex flex-col">
            {all.map(([ref, n]) => (
              <li key={ref}>
                <button
                  type="button"
                  onClick={() => onOpen(ref)}
                  className="flex w-full items-baseline gap-2 rounded px-1.5 py-1 text-left text-[13px] hover:bg-woad-soft"
                >
                  <span className="w-4 shrink-0 text-gold-deep">{n.starred ? "★" : ""}</span>
                  <span className="w-24 shrink-0 font-semibold">{displayRef(ref)}</span>
                  <span className="min-w-0 flex-1 truncate text-stone">{n.text}</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
    </div>
  );
}

export function ReaderPanel(props: PanelProps) {
  const { card, cards, verseRef, verse, tree, panel, word } = props;
  const words = tree ? wordsOf(tree, wordType(card)) : [];

  const parallels = () => {
    if (!props.chapter || verse === null) return <p className="text-[13px] text-stone">…</p>;
    const row = props.chapter.rows.find((r) => r.verse === verse);
    const columns = props.chapter.columns.filter((c) => c.name !== card.name && cards[c.name]);
    if (!columns.length) {
      return <p className="text-[13px] text-stone">No other text is open. Add one with *Texts* above.</p>;
    }
    const one: ParallelChapter = { ...props.chapter, columns, rows: row ? [row] : [] };
    return (
      <>
        {props.diff ? <DiffNote message={`Compared with ${card.name}, on consonants`} /> : null}
        <Table
          data={one}
          cards={cards}
          settings={props.settings}
          verse={null}
          diffs={props.diff ? props.diffs : {}}
          verseNumbers={false}
          onInspect={props.onInspect}
        />
        {!row ? <p className="text-[13px] text-stone">The other texts do not have this verse.</p> : null}
      </>
    );
  };

  const searchFrom = async (node: number) => {
    const info: NodeInfo = await api.nodeInfo(card.name, node);
    const term = termFrom(card.name, info);
    props.onSearch(buildTemplate([term], rangesOf(card)[0] ?? "verse", null));
  };

  return (
    <aside
      aria-label="About the verse"
      className="fixed inset-x-0 bottom-0 z-20 flex h-[55vh] flex-col border-t border-rule-strong bg-raised shadow-lg md:static md:z-auto md:h-full md:w-[26rem] md:border-s md:border-t-0 md:shadow-none"
    >
      <header className="flex items-center gap-1 border-b border-rule px-2 py-1.5">
        <nav className="flex min-w-0 flex-1 flex-wrap gap-0.5" aria-label="Panel">
          {PANELS.map((p) => (
            <button
              key={p}
              type="button"
              aria-current={panel === p ? "page" : undefined}
              onClick={() => props.onTab(p)}
              className="rounded px-2 py-0.5 text-[13px] hover:bg-woad-soft aria-[current=page]:bg-ink aria-[current=page]:text-vellum"
            >
              {TAB_NAMES[p]}
            </button>
          ))}
        </nav>
        <button
          type="button"
          onClick={props.onClose}
          className="rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          aria-label="Close the panel"
        >
          Close
        </button>
      </header>
      {verseRef && panel !== "notes" ? (
        <p className="border-b border-rule px-3 py-1 text-[13px] font-semibold">{displayRef(verseRef)}</p>
      ) : null}
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-2">
        {verseRef === null && panel !== "notes" ? (
          <p className="text-[13px] text-stone">Click a verse to see its parallels, words and lexemes.</p>
        ) : panel === "parallels" ? (
          <>
            <div className="mb-2 flex flex-wrap items-center gap-3 text-[13px]">
              <Texts
                names={props.names.filter((n) => n !== card.name)}
                cards={cards}
                texts={props.others}
                onChange={props.onTexts}
                onAdd={props.onAddText}
              />
              <label className="flex items-center gap-1.5">
                <input
                  type="checkbox"
                  checked={props.diff}
                  onChange={(e) => props.onDiff(e.target.checked)}
                  className="accent-[var(--woad)]"
                />
                Show differences
              </label>
            </div>
            {parallels()}
          </>
        ) : panel === "words" ? (
          word !== null ? (
            <div className="-mx-3 -my-2 h-full">
              <Inspector
                corpus={cards[word.corpus] ?? card}
                node={word.node}
                onInspect={(node) => props.onWord({ corpus: word.corpus, node })}
                onClose={() => props.onWord(null)}
                onOpenPassage={(ref) => props.onOpenPassage(word.corpus, ref)}
                onLexicon={word.corpus === card.name ? props.onLexicon : undefined}
                onSearchTerm={
                  word.corpus === card.name
                    ? (info) =>
                        props.onSearch(buildTemplate([termFrom(card.name, info)], rangesOf(card)[0] ?? "verse", null))
                    : undefined
                }
              />
            </div>
          ) : (
            <WordList
              card={card}
              words={words}
              action="All about this word"
              onPick={(node) => props.onWord({ corpus: card.name, node })}
            />
          )
        ) : panel === "lexicon" ? (
          <Lexemes card={card} words={words} onLexicon={props.onLexicon} />
        ) : panel === "search" ? (
          <>
            <p className="mb-2 text-[13px] text-stone">
              Search for words like one of these: the search opens in <em>Parallel</em>, where you can refine it.
            </p>
            <WordList card={card} words={words} action="Search for words like this one" onPick={searchFrom} />
            <button
              type="button"
              onClick={() => props.onSearch("")}
              className="mt-2 text-[13px] text-woad hover:underline"
            >
              Write a search template…
            </button>
          </>
        ) : (
          <NotesTab corpus={card.name} verseRef={verseRef} onOpen={(ref) => props.onOpenPassage(card.name, ref)} />
        )}
      </div>
    </aside>
  );
}
