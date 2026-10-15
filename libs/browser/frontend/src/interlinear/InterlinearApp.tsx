import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import { splitRef } from "../common";
import { Cite, citePassage } from "../Cite";
import { displayRef, scriptClass } from "../display/Display";
import { DisplayOptions } from "../DisplayOptions";
import { HoverCard, useHover } from "../HoverCard";
import { Inspector } from "../Inspector";
import { InterfaceMenu } from "../InterfaceMenu";
import { firstChapter, useCards, useReference } from "../shared/corpora";
import { Mark } from "../shared/Mark";
import { Navigate } from "../shared/Navigate";
import { useSettings, useTheme } from "../state";
import { describeInterlinear, switchInterface } from "../switch";
import type { CorpusCard, DiffKind, InterlinearVerse } from "../types";
import { alignRef, navigate } from "../ui";
import { Workspaces } from "../Workspaces";
import { useInterlinear } from "./state";
import { BUILT_IN, runs, type Colour, type Interlinear, type Line } from "./url";

// The Interlinear, after STEPBible, BibleHub and the Logos interlinears: every
// word as a small block with lines under it, chosen and ordered by the user.

type Word = InterlinearVerse["words"][number];

const TONES = 6;
const COLOUR_LABEL: Record<Colour, string> = { pos: "part of speech", stem: "stem" };

/** The value a word is coloured by: its part of speech, or its stem (verbs only). */
function colourValue(word: Word, colour: Colour): string | null {
  if (colour === "pos") return word.lines.pos ?? null;
  if (word.lines.pos !== "verb") return null;
  return word.lines.morph?.split(" ")[0] ?? null;
}

function lineName(line: Line): string {
  return BUILT_IN.find((b) => b.line === line)?.name ?? line.slice(2);
}

/** The menu of lines: on or off, in an order of choice, and any feature as a line. */
function Lines({
  lines,
  features,
  onChange,
}: {
  lines: Line[];
  features: string[];
  onChange: (lines: Line[]) => void;
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

  const listed: Line[] = [...lines, ...BUILT_IN.map((b) => b.line).filter((l) => !lines.includes(l))];
  const move = (line: Line, by: number) => {
    const i = lines.indexOf(line);
    const j = i + by;
    if (i < 0 || j < 0 || j >= lines.length) return;
    const next = [...lines];
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
        Lines
      </button>
      {open ? (
        <div className="absolute start-0 top-full z-30 mt-1 w-[19rem] rounded border border-rule-strong bg-raised py-1.5 shadow-lg">
          <ul>
            {listed.map((line) => {
              const on = lines.includes(line);
              const i = lines.indexOf(line);
              return (
                <li key={line} className="flex items-center gap-2 px-3 py-1 text-[14px]">
                  <label className="flex min-w-0 flex-1 items-center gap-2">
                    <input
                      type="checkbox"
                      checked={on}
                      onChange={(e) => onChange(e.target.checked ? [...lines, line] : lines.filter((l) => l !== line))}
                      className="accent-[var(--woad)]"
                    />
                    <span className="truncate">{lineName(line)}</span>
                  </label>
                  {on ? (
                    <span className="flex text-[12px] text-stone">
                      <button
                        type="button"
                        disabled={i === 0}
                        onClick={() => move(line, -1)}
                        className="rounded px-1 hover:bg-woad-soft disabled:opacity-30"
                        aria-label={`Move ${lineName(line)} up`}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        disabled={i === lines.length - 1}
                        onClick={() => move(line, 1)}
                        className="rounded px-1 hover:bg-woad-soft disabled:opacity-30"
                        aria-label={`Move ${lineName(line)} down`}
                      >
                        ↓
                      </button>
                    </span>
                  ) : null}
                </li>
              );
            })}
          </ul>
          <label className="mt-1 flex items-center gap-2 border-t border-rule px-3 pt-1.5 text-[13px]">
            <span className="text-stone">Add a feature</span>
            <select
              value=""
              onChange={(e) => e.target.value && onChange([...lines, `f:${e.target.value}`])}
              className="min-w-0 flex-1 rounded border border-rule-strong bg-raised px-1 py-0.5"
            >
              <option value="">…</option>
              {features
                .filter((f) => !lines.includes(`f:${f}`))
                .map((f) => (
                  <option key={f} value={f}>
                    {f}
                  </option>
                ))}
            </select>
          </label>
        </div>
      ) : null}
    </div>
  );
}

interface BlockProps {
  card: CorpusCard;
  word: Word;
  lines: Line[];
  hidden: Line | null;
  revealed: boolean;
  tone: number | null;
  mark: DiffKind | undefined;
  under: { text: string; mark: DiffKind | undefined }[] | null;
  underCard: CorpusCard | undefined;
  inspected: boolean;
  onInspect: () => void;
  onReveal: () => void;
  onHover: (node: number | null, target?: Element) => void;
}

/** One word: the word and its lines, stacked. */
function Block({
  card,
  word,
  lines,
  hidden,
  revealed,
  tone,
  mark,
  under,
  underCard,
  inspected,
  onInspect,
  onReveal,
  onHover,
}: BlockProps) {
  const script = scriptClass(card.display, card.display.direction);
  const value = (line: Line): string | null => {
    if (line === "word") return word.text;
    if (line.startsWith("f:")) return word.features[line.slice(2)] ?? null;
    return word.lines[line] ?? null;
  };
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onInspect}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onInspect()}
      onMouseEnter={(e) => onHover(word.node, e.currentTarget)}
      onMouseLeave={() => onHover(null)}
      className={`inline-flex max-w-[11rem] cursor-pointer flex-col items-center rounded px-1.5 py-1 text-center hover:bg-woad-soft/60 ${
        inspected ? "bg-gold-leaf/60" : ""
      }`}
    >
      {lines.map((line) => {
        const text = value(line);
        if (line === hidden && !revealed) {
          return (
            <button
              key={line}
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onReveal();
              }}
              className="my-0.5 min-w-[3rem] rounded border border-dashed border-rule-strong px-1 text-[12px] text-stone hover:border-woad"
              title={`Show the ${lineName(line).toLowerCase()}`}
            >
              ?
            </button>
          );
        }
        if (line === "word") {
          return (
            <span
              key={line}
              dir={card.display.direction}
              className={`${script} text-[1.25em] leading-snug ${tone !== null ? `tone-${tone + 1}` : ""} ${
                mark ? `diff diff-${mark}` : ""
              }`}
            >
              {text}
            </span>
          );
        }
        const style =
          line === "translit" || line.startsWith("f:")
            ? "font-mono text-[11px] text-stone"
            : line === "lexeme"
              ? `${script} text-[0.95em] text-stone`
              : line === "gloss"
                ? "text-[13px]"
                : "text-[11px] leading-tight text-stone";
        return (
          <span key={line} dir={line === "lexeme" ? card.display.direction : "ltr"} className={style}>
            {text ?? " "}
          </span>
        );
      })}
      {under !== null && underCard ? (
        <span
          dir={underCard.display.direction}
          className={`mt-0.5 border-t border-rule pt-0.5 ${scriptClass(underCard.display, underCard.display.direction)} text-[1em]`}
        >
          {under.length ? (
            under.map((u, i) => (
              <span key={i} className={u.mark ? `diff diff-${u.mark}` : ""}>
                {i ? " " : ""}
                {u.text}
              </span>
            ))
          ) : (
            <span className="text-stone" title={`Nothing in ${underCard.name} at this word`}>
              –
            </span>
          )}
        </span>
      ) : null}
    </div>
  );
}

interface VerseProps {
  card: CorpusCard;
  verse: InterlinearVerse;
  state: Interlinear;
  tones: Map<string, number>;
  revealed: Set<number>;
  underCard: CorpusCard | undefined;
  inspected: number | null;
  selected: boolean;
  onInspect: (node: number) => void;
  onReveal: (node: number) => void;
  onHover: (node: number | null, target?: Element) => void;
}

/** A verse: its words in blocks, grouped by clause and phrase when the structure is shown. */
function Verse({
  card,
  verse,
  state,
  tones,
  revealed,
  underCard,
  inspected,
  selected,
  onInspect,
  onReveal,
  onHover,
}: VerseProps) {
  const under = verse.under;
  const byNode = new Map((under?.words ?? []).map((w) => [w.node, w.text]));
  const block = (word: Word, i: number) => {
    const value = state.colour ? colourValue(word, state.colour) : null;
    return (
      <Block
        key={word.node}
        card={card}
        word={word}
        lines={state.lines}
        hidden={state.hide}
        revealed={revealed.has(word.node)}
        tone={value !== null ? (tones.get(value) ?? null) : null}
        mark={under?.marksHere[String(word.node)]}
        under={
          state.under
            ? under
              ? (under.under[i] ?? []).map((n) => ({ text: byNode.get(n) ?? "", mark: under.marks[String(n)] }))
              : []
            : null
        }
        underCard={underCard}
        inspected={inspected === word.node}
        onInspect={() => onInspect(word.node)}
        onReveal={() => onReveal(word.node)}
        onHover={onHover}
      />
    );
  };

  const words = verse.words;
  let content;
  if (state.structure && verse.clauses.length) {
    // clauses with a wide gap between them; phrases inside, underlined, with their function
    content = runs(words.map((w) => w.clause)).map(([c, from, to]) => (
      <span key={`c${from}`} className="me-4 inline-flex flex-col">
        <span dir="ltr" className="px-1 text-start text-[10px] font-semibold text-woad">
          {c >= 0 ? (verse.clauses[c].label ?? "") : ""}
        </span>
        <span className="inline-flex flex-wrap items-start gap-x-1.5">
          {runs(words.slice(from, to + 1).map((w) => w.phrase)).map(([p, pFrom, pTo]) => (
            <span key={`p${from + pFrom}`} className="inline-flex flex-col border-b-2 border-rule-strong">
              <span className="inline-flex flex-wrap items-start">
                {words.slice(from + pFrom, from + pTo + 1).map((w, k) => block(w, from + pFrom + k))}
              </span>
              <span dir="ltr" className="text-center text-[10px] text-stone">
                {p >= 0 ? (verse.phrases[p].label ?? "") : ""}
              </span>
            </span>
          ))}
        </span>
      </span>
    ));
  } else {
    content = words.map((w, i) => block(w, i));
  }

  return (
    <section
      data-verse={verse.verse}
      aria-label={displayRef(verse.ref)}
      className={`flex gap-3 border-b border-rule py-3 ${selected ? "bg-gold-leaf/30" : ""}`}
    >
      <span dir="ltr" className="w-7 shrink-0 pt-2 text-end text-[12px] font-semibold tabular-nums text-madder">
        {verse.verse}
      </span>
      <div dir={card.display.direction} className="flex min-w-0 flex-1 flex-wrap items-start gap-y-2">
        {content}
        {state.under && under === null ? (
          <p dir="ltr" className="w-full text-[12px] text-stone">
            Not in {state.under}.
          </p>
        ) : null}
      </div>
    </section>
  );
}

export function InterlinearApp() {
  const [state, actions] = useInterlinear();
  const [settings, setSettings] = useSettings();
  const [theme, toggleTheme] = useTheme();
  const { corpora, names, cards, ready } = useCards();
  const reference = useReference(cards, ready);
  const [navOpen, setNavOpen] = useState(false);
  const closeNav = useCallback(() => setNavOpen(false), []);
  const [inspected, setInspected] = useState<{ corpus: string; node: number } | null>(null);
  const [revealed, setRevealed] = useState<Set<number>>(new Set());
  const [hovered, onHover] = useHover();
  const scroller = useRef<HTMLElement>(null);

  const corpus = state.corpus && cards[state.corpus] ? state.corpus : null;
  const card = corpus ? cards[corpus] : undefined;
  const underCard = state.under ? cards[state.under] : undefined;

  // the BHSA by default (or the first corpus), at its first chapter
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

  const features = state.lines.filter((l) => l.startsWith("f:")).map((l) => l.slice(2));
  const passage = useQuery({
    queryKey: ["interlinear", corpus, state.ref, settings.fmt, features.join(","), state.under],
    queryFn: () => api.interlinear(corpus!, state.ref!, { fmt: settings.fmt, features, under: state.under }),
    enabled: corpus !== null && state.ref !== null,
    placeholderData: (previous) => previous,
  });
  const featureList = useQuery({
    queryKey: ["features", corpus],
    queryFn: () => api.features(corpus!),
    enabled: corpus !== null,
  });
  const data = passage.data;
  const prev = data?.prev ?? null;
  const next = data?.next ?? null;

  useEffect(() => setRevealed(new Set()), [state.ref, state.hide]);

  // a verse in the URL is scrolled to once shown
  useEffect(() => {
    if (state.verse === null || !data) return;
    scroller.current?.querySelector(`[data-verse="${state.verse}"]`)?.scrollIntoView({ block: "center" });
  }, [state.verse, data]);

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
    document.title = state.ref ? `${displayRef(state.ref)} · interlinear: Context-Fabric` : "Context-Fabric Browser";
  }, [state.ref]);

  // colours: the values in order of first use, the first six with a tone
  const tones = useMemo(() => {
    const map = new Map<string, number>();
    if (!state.colour || !data) return map;
    const counts = new Map<string, number>();
    for (const v of data.verses)
      for (const w of v.words) {
        const value = colourValue(w, state.colour);
        if (value) counts.set(value, (counts.get(value) ?? 0) + 1);
      }
    [...counts.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, TONES)
      .forEach(([value], i) => map.set(value, i));
    return map;
  }, [state.colour, data]);

  const describe = useMemo(() => () => describeInterlinear(state), [state]);
  const toParallel = async (change: Parameters<typeof switchInterface>[1]) =>
    switchInterface("parallel", { ...(await describeInterlinear(state)), ...change });

  const arrow = "rounded px-2 py-1 text-[16px] leading-none hover:bg-woad-soft disabled:opacity-30";
  const select = "rounded border border-rule-strong bg-raised px-1.5 py-0.5 text-[13px]";
  const comparable = names.filter((n) => n !== corpus && cards[n] && !cards[n].bookView);
  const inspectedCard = inspected ? cards[inspected.corpus] : undefined;

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
        {card && state.ref ? (
          <Cite
            align="start"
            text={(software, date) =>
              citePassage(card, state.verse ? `${state.ref}:${state.verse}` : state.ref!, software, date)
            }
          />
        ) : null}
        <select
          value={corpus ?? ""}
          onChange={async (e) => {
            const target = e.target.value;
            const aligned = corpus && state.ref ? await alignRef(corpus, state.ref, target) : null;
            actions.setCorpus(target, aligned ? splitRef(aligned).chapter : null);
          }}
          className={select}
          title="The text shown word by word"
        >
          {names.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
        <Lines
          lines={state.lines}
          features={(featureList.data?.features ?? []).map((f) => f.name)}
          onChange={actions.setLines}
        />
        <label className="flex items-center gap-1 text-[13px]">
          <span className="text-stone">Under</span>
          <select
            value={state.under ?? ""}
            onChange={(e) => actions.set({ under: e.target.value || null })}
            className={select}
            title="Another text, word by word under the words"
          >
            <option value="">nothing</option>
            {comparable.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <label
          className="flex items-center gap-1.5 text-[13px]"
          title="Phrases with their function, clauses with their type"
        >
          <input
            type="checkbox"
            checked={state.structure}
            onChange={(e) => actions.set({ structure: e.target.checked })}
            className="accent-[var(--woad)]"
          />
          Structure
        </label>
        <label className="flex items-center gap-1 text-[13px]">
          <span className="text-stone">Practise</span>
          <select
            value={state.hide ?? ""}
            onChange={(e) => actions.set({ hide: (e.target.value || null) as Line | null })}
            className={`${select} max-w-[10rem]`}
            title="Hide a line; click a word's ? to see it"
          >
            <option value="">off</option>
            {state.lines.map((l) => (
              <option key={l} value={l}>
                hide {lineName(l).toLowerCase()}
              </option>
            ))}
          </select>
        </label>
        <label className="flex items-center gap-1 text-[13px]">
          <span className="text-stone">Colour</span>
          <select
            value={state.colour ?? ""}
            onChange={(e) => actions.set({ colour: (e.target.value || null) as Colour | null })}
            className={select}
          >
            <option value="">none</option>
            <option value="pos">by part of speech</option>
            <option value="stem">verbs by stem</option>
          </select>
        </label>
        <div className="ms-auto flex flex-wrap items-center justify-end gap-1 sm:gap-2">
          <DisplayOptions
            settings={settings}
            formats={card?.display.textFormats ?? []}
            defaultFormat={card?.display.defaultFormat ?? null}
            onChange={setSettings}
          />
          <Workspaces corpora={names} current="interlinear" onLoad={navigate} />
          <button
            type="button"
            onClick={toggleTheme}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft"
            aria-label={theme === "dark" ? "Use light colours" : "Use dark colours"}
          >
            {theme === "dark" ? "Light" : "Dark"}
          </button>
          <InterfaceMenu current="interlinear" describe={describe} />
        </div>
      </header>

      {(state.colour && tones.size) || state.hide ? (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-rule px-4 py-1.5 text-[13px]">
          {state.colour && tones.size ? (
            <span className="flex flex-wrap items-center gap-x-3">
              <span className="text-stone">Coloured by {COLOUR_LABEL[state.colour]}:</span>
              {[...tones.entries()].map(([value, i]) => (
                <span key={value} className={`tone-${i + 1} font-semibold`}>
                  {value}
                </span>
              ))}
            </span>
          ) : null}
          {state.hide ? (
            <span className="flex items-center gap-2">
              <span className="text-stone">Practising: the {lineName(state.hide).toLowerCase()} is hidden.</span>
              <button
                type="button"
                onClick={() => setRevealed(new Set(data?.verses.flatMap((v) => v.words.map((w) => w.node)) ?? []))}
                className="text-woad hover:underline"
              >
                Show all
              </button>
              <button type="button" onClick={() => setRevealed(new Set())} className="text-woad hover:underline">
                Hide again
              </button>
            </span>
          ) : null}
        </div>
      ) : null}

      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        <main ref={scroller} className="min-h-0 min-w-0 flex-1 overflow-y-auto">
          {corpora.data !== undefined && names.length === 0 ? (
            <p className="p-10 text-stone">No corpus is open.</p>
          ) : !card || !state.ref ? (
            <p className="p-10 text-stone">Opening…</p>
          ) : passage.error ? (
            <p className="p-10 text-madder">{String((passage.error as Error).message)}</p>
          ) : !data ? (
            <p className="p-10 text-stone">Loading…</p>
          ) : (
            <div className={`mx-auto max-w-[72rem] px-4 pb-10 ${passage.isFetching ? "opacity-70" : ""}`}>
              {data.verses.map((v) => (
                <Verse
                  key={v.ref}
                  card={card}
                  verse={v}
                  state={state}
                  tones={tones}
                  revealed={revealed}
                  underCard={underCard}
                  inspected={inspected?.corpus === card.name ? inspected.node : null}
                  selected={state.verse === v.verse}
                  onInspect={(node) => setInspected({ corpus: card.name, node })}
                  onReveal={(node) => setRevealed((r) => new Set([...r, node]))}
                  onHover={onHover}
                />
              ))}
              {data.truncated ? <p className="py-3 text-stone">Only the first verses are shown.</p> : null}
            </div>
          )}
          {hovered && card ? <HoverCard corpus={card} hovered={hovered} /> : null}
        </main>
        {inspected && inspectedCard ? (
          <div className="max-h-[45vh] min-h-0 border-t border-rule md:max-h-none md:border-t-0">
            <Inspector
              corpus={inspectedCard}
              node={inspected.node}
              onInspect={(node) => setInspected({ corpus: inspected.corpus, node })}
              onClose={() => setInspected(null)}
              onOpenPassage={async (ref) => {
                const aligned = await alignRef(inspected.corpus, ref, corpus!);
                if (aligned) {
                  const { chapter, verse } = splitRef(aligned);
                  actions.open(chapter, true, verse);
                }
              }}
              onLexicon={(lex) => void toParallel({ lexicon: { corpus: inspected.corpus, lex }, searching: false })}
            />
          </div>
        ) : null}
      </div>

      {navOpen && corpus ? (
        <Navigate
          corpus={corpus}
          current={state.ref}
          onOpen={(ref) => {
            setNavOpen(false);
            actions.open(ref);
          }}
          onClose={closeNav}
        />
      ) : null}
    </div>
  );
}
