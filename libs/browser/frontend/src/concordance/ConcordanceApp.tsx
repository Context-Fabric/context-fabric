import { useQuery } from "@tanstack/react-query";
import { decodeAsked, encodeAsked } from "../shared/asked";
import { AskedLine, ProposalCard } from "../shared/Proposals";
import { useEffect, useMemo, useState } from "react";
import { api, ApiError } from "../api";
import { splitRef } from "../common";
import { displayRef, scriptClass } from "../display/Display";
import { DisplayOptions } from "../DisplayOptions";
import { HoverCard, useHover } from "../HoverCard";
import { Inspector } from "../Inspector";
import { InterfaceMenu } from "../InterfaceMenu";
import { TermCard } from "../parallel/Search";
import { QueryEditor } from "../search/QueryEditor";
import { loadHistory, Problems, remember, StatusLine, useSearchGone, useSearchStatus } from "../search/SearchView";
import { Stats } from "../search/Stats";
import { useCards, useReference } from "../shared/corpora";
import { Mark } from "../shared/Mark";
import { buildTemplate, rangesOf, termFrom, type Term } from "../shared/terms";
import { useSettings, useTheme } from "../state";
import { describeConcordance, switchInterface } from "../switch";
import type { CorpusCard, KwicLine, KwicWord, Problem, SearchStatus } from "../types";
import { alignRef, navigate } from "../ui";
import { Workspaces } from "../Workspaces";
import { useConcordance } from "./state";
import { CONTEXTS, filterParam, linesToTsv, toggleFilter, TABS, type Concordance, type Tab } from "./url";

// The Concordance, after Sketch Engine and AntConc: a search's results as lines
// with the hit in the middle, sorted, filtered and counted.

const PAGE = 50;
const POSITIONS = ["L3", "L2", "L1", "hit", "R1", "R2", "R3"];
const POSITION_NAMES: Record<string, string> = {
  L3: "3rd word before",
  L2: "2nd word before",
  L1: "word before",
  hit: "the hit",
  R1: "word after",
  R2: "2nd word after",
  R3: "3rd word after",
};
// `text` is a word's consonants, so for Hebrew; a translation has `lower`
const SORT_FEATURES: [string, string][] = [
  ["text", "word"],
  ["lower", "word"],
  ["lex", "lexeme"],
  ["sp", "part of speech"],
];
/** The features offered as facets, where the corpus has them. */
const FACETS: [string, string][] = [
  ["book", "Book"],
  ["sp", "Part of speech"],
  ["vs", "Stem"],
  ["vt", "Tense"],
  ["ps", "Person"],
  ["gn", "Gender"],
  ["nu", "Number"],
  ["st", "State"],
  ["language", "Language"],
];
const TAB_NAMES: Record<Tab, string> = { lines: "Lines", collocations: "Collocations", statistics: "Statistics" };

function finished(status: SearchStatus | null): boolean {
  return status !== null && status.state !== "queued" && status.state !== "running";
}

/** Words of a line, each hoverable and clickable. */
function Words({
  words,
  card,
  onWord,
  onHover,
}: {
  words: KwicWord[];
  card: CorpusCard;
  onWord: (node: number) => void;
  onHover: (node: number | null, target?: Element) => void;
}) {
  return (
    <>
      {words.map((w) => (
        <span
          key={w.node}
          onClick={() => onWord(w.node)}
          onMouseEnter={(e) => onHover(w.node, e.currentTarget)}
          onMouseLeave={() => onHover(null)}
          className={`cursor-pointer rounded px-0.5 hover:bg-woad-soft ${scriptClass(card.display, card.display.direction)}`}
        >
          {w.text}
        </span>
      ))}
    </>
  );
}

/** One facet: the values of a feature of the hits, with their counts; a click filters. */
function Facet({
  search,
  feature,
  name,
  state,
  onToggle,
}: {
  search: SearchStatus;
  feature: string;
  name: string;
  state: Concordance;
  onToggle: (value: string) => void;
}) {
  const [all, setAll] = useState(false);
  const filter = filterParam(state.filters);
  const { data } = useQuery({
    queryKey: ["kwic-facet", search.id, search.version, feature, filter, state.per],
    queryFn: () => api.kwicFacets(search.id, feature, filter, state.per),
    enabled: finished(search),
    staleTime: Infinity,
  });
  if (!data || !data.values.length) return null;
  const max = Math.max(...data.values.map((v) => v.count));
  const shown = all ? data.values : data.values.slice(0, 8);
  const active = new Set(state.filters.filter(([f]) => f === feature).map(([, v]) => v));
  return (
    <section className="mb-4">
      <h3 className="mb-1 text-[13px] font-semibold">{name}</h3>
      <ul className="flex flex-col">
        {shown.map((v) => (
          <li key={v.value}>
            <button
              type="button"
              aria-pressed={active.has(v.value)}
              onClick={() => onToggle(v.value)}
              className="group flex w-full items-center gap-2 rounded px-1 py-0.5 text-left text-[13px] hover:bg-woad-soft aria-pressed:bg-woad aria-pressed:text-vellum"
              title={active.has(v.value) ? "Take this filter away" : "Only results with this"}
            >
              <span className="w-24 shrink-0 truncate">{feature === "book" ? displayRef(v.value) : v.value}</span>
              <span className="relative h-2 flex-1 rounded bg-woad-soft group-aria-pressed:bg-vellum/30">
                <span
                  className="absolute inset-y-0 start-0 rounded bg-chart"
                  style={{ width: `${(v.count / max) * 100}%` }}
                />
              </span>
              <span className="w-12 shrink-0 text-end tabular-nums">{v.count.toLocaleString("en-US")}</span>
            </button>
          </li>
        ))}
      </ul>
      {data.values.length > 8 ? (
        <button type="button" onClick={() => setAll(!all)} className="mt-0.5 text-[12px] text-woad hover:underline">
          {all ? "Fewer" : `All ${data.values.length}`}
        </button>
      ) : null}
    </section>
  );
}

/** The words that are typical near the hits. */
function Collocations({ search, card, state }: { search: SearchStatus; card: CorpusCard; state: Concordance }) {
  const [feature, setFeature] = useState(card.lexicon ? "lex" : card.translation ? "lower" : "text");
  const [window, setWindow] = useState(3);
  const filter = filterParam(state.filters);
  const { data, isFetching } = useQuery({
    queryKey: ["collocations", search.id, search.version, feature, window, filter, state.per],
    queryFn: () => api.collocations(search.id, feature, window, filter, state.per),
    enabled: finished(search),
    staleTime: Infinity,
  });
  const lexicon = useQuery({
    queryKey: ["lexicon", card.name],
    queryFn: () => api.lexicon(card.name),
    staleTime: Infinity,
    enabled: card.lexicon && feature === "lex",
  });
  const byLex = useMemo(() => new Map((lexicon.data?.entries ?? []).map((e) => [e.lex, e])), [lexicon.data]);
  const script = scriptClass(card.display, card.display.direction);
  const max = Math.max(1, ...(data?.values ?? []).map((v) => v.logDice));

  return (
    <div className="max-w-[44rem] p-4">
      <div className="mb-3 flex flex-wrap items-center gap-3 text-[13px]">
        <label className="flex items-center gap-1.5">
          <span className="text-stone">Words</span>
          <select
            value={feature}
            onChange={(e) => setFeature(e.target.value)}
            className="rounded border border-rule-strong bg-raised px-1.5 py-0.5"
          >
            {card.lexicon ? <option value="lex">by lexeme</option> : null}
            {card.translation ? (
              <option value="lower">by word</option>
            ) : (
              <>
                <option value="text">by form (consonants)</option>
                <option value="sp">by part of speech</option>
              </>
            )}
          </select>
        </label>
        <label className="flex items-center gap-1.5">
          <span className="text-stone">within</span>
          <select
            value={window}
            onChange={(e) => setWindow(Number(e.target.value))}
            className="rounded border border-rule-strong bg-raised px-1.5 py-0.5"
          >
            {[1, 2, 3, 4, 5].map((n) => (
              <option key={n} value={n}>
                {n} {n === 1 ? "word" : "words"}
              </option>
            ))}
          </select>
        </label>
        <span className="text-stone">
          of each hit, sorted by logDice: how typical a word is near the hit compared with the whole corpus (at most
          14).
        </span>
      </div>
      {!data ? (
        <p className="text-stone">{finished(search) ? "Counting…" : "When the search is done."}</p>
      ) : (
        <table className={`w-full text-[14px] ${isFetching ? "opacity-70" : ""}`}>
          <thead>
            <tr className="border-b border-rule text-left text-[12px] text-stone">
              <th className="py-1 font-normal">Word</th>
              <th className="py-1 text-end font-normal">Near the hits</th>
              <th className="py-1 text-end font-normal">In the corpus</th>
              <th className="py-1 ps-3 font-normal">logDice</th>
            </tr>
          </thead>
          <tbody>
            {data.values.map((v) => {
              const entry = feature === "lex" ? byLex.get(v.value) : undefined;
              return (
                <tr key={v.value} className="border-b border-rule">
                  <td className="py-1">
                    {entry ? (
                      <>
                        <span dir={card.display.direction} className={script}>
                          {entry.form ?? entry.cons}
                        </span>{" "}
                        <span className="text-[13px]">{entry.gloss}</span>{" "}
                        <span className="font-mono text-[11px] text-stone">{v.value}</span>
                      </>
                    ) : (
                      <span
                        dir={feature === "text" ? card.display.direction : "ltr"}
                        className={feature === "text" ? script : ""}
                      >
                        {v.value}
                      </span>
                    )}
                  </td>
                  <td className="py-1 text-end tabular-nums">{v.count.toLocaleString("en-US")}</td>
                  <td className="py-1 text-end tabular-nums text-stone">{v.frequency.toLocaleString("en-US")}</td>
                  <td className="py-1 ps-3">
                    <span className="flex items-center gap-2">
                      <span className="relative h-2 w-24 rounded bg-woad-soft">
                        <span
                          className="absolute inset-y-0 start-0 rounded bg-chart"
                          style={{ width: `${(v.logDice / max) * 100}%` }}
                        />
                      </span>
                      <span className="tabular-nums">{v.logDice.toFixed(1)}</span>
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

export function ConcordanceApp() {
  const [state, actions] = useConcordance();
  const [settings, setSettings] = useSettings();
  const [theme, toggleTheme] = useTheme();
  const { corpora, names, cards, ready } = useCards();
  const reference = useReference(cards, ready);
  const [inspected, setInspected] = useState<{ corpus: string; node: number } | null>(null);
  const [hovered, onHover] = useHover();
  const [page, setPage] = useState(0);
  const [terms, setTerms] = useState<Term[]>([]);
  const [range, setRange] = useState<string | null>(null);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [starting, setStarting] = useState(false);
  const [downloading, setDownloading] = useState(false);

  const corpus = state.corpus && cards[state.corpus] ? state.corpus : null;
  const card = corpus ? cards[corpus] : undefined;
  // a search to try: in English for a translation
  const example = card?.translation ? "word lower=lord" : "word lex=HLK[";
  const [history, setHistory] = useState<string[]>([]);
  useEffect(() => setHistory(corpus ? loadHistory(corpus) : []), [corpus]);

  // the BHSA by default (or the first corpus)
  useEffect(() => {
    if (state.corpus || !ready || !reference.settled) return;
    const start = reference.name ?? names[0];
    if (start) actions.set({ corpus: start });
  }, [state.corpus, ready, reference.settled, reference.name, names, actions]);

  const status = useSearchStatus(state.s);
  const [facetsOpen, setFacetsOpen] = useState(false);
  const gone = useSearchGone(state.s);
  const { data: features } = useQuery({
    queryKey: ["features", corpus],
    queryFn: () => api.features(corpus!),
    enabled: corpus !== null,
  });
  const featureNames = new Set((features?.features ?? []).map((f) => f.name));

  // `ask`: where the template came from, when a model proposed it
  const run = async (template = state.q, ask: string | null = state.ask) => {
    if (!corpus || !template.trim()) return;
    setStarting(true);
    setProblems([]);
    try {
      const started = await api.search(corpus, template);
      setHistory(remember(corpus, template));
      setPage(0);
      actions.set({ q: template, s: started.id, ask, filters: [], sample: null }, true);
    } catch (e) {
      if (e instanceof ApiError && e.problems.length) setProblems(e.problems);
      else setProblems([{ kind: "error", line: null, message: String(e instanceof Error ? e.message : e) }]);
    } finally {
      setStarting(false);
    }
  };

  // a link to a search the server no longer has (it restarted): search again
  const [rerun, setRerun] = useState<string | null>(null);
  useEffect(() => {
    if (gone && state.s && rerun !== state.s && state.q.trim() && corpus) {
      setRerun(state.s);
      api
        .search(corpus, state.q)
        .then((started) => actions.set({ s: started.id }))
        .catch(() => undefined);
    }
  }, [gone, state.s, state.q, rerun, corpus, actions]);

  // terms from words make the template
  const ranges = card ? rangesOf(card) : [];
  const currentRange = range && ranges.includes(range) ? range : (ranges[0] ?? "verse");
  useEffect(() => {
    const own = terms.filter((t) => t.corpus === corpus);
    if (own.length) actions.set({ q: buildTemplate(own, currentRange, null) });
  }, [terms, currentRange, corpus, actions]);

  useEffect(() => setPage(0), [state.s, state.sort, state.filters, state.per, state.sample, state.seed]);

  const filter = filterParam(state.filters);
  const lines = useQuery({
    queryKey: [
      "kwic",
      state.s,
      status?.version,
      page,
      state.context,
      state.sort,
      filter,
      state.per,
      state.sample,
      state.seed,
      settings.fmt,
    ],
    queryFn: () =>
      api.kwic(state.s!, {
        offset: page * PAGE,
        limit: PAGE,
        left: state.context,
        right: state.context,
        sort: state.sort,
        filter,
        per: state.per,
        sample: state.sample,
        seed: state.seed,
        fmt: settings.fmt,
      }),
    enabled: state.s !== null && status !== null && status.count > 0,
    placeholderData: (previous) => previous,
  });

  useEffect(() => {
    document.title = corpus ? `Concordance of ${corpus}: Context-Fabric` : "Context-Fabric Browser";
  }, [corpus]);

  const describe = useMemo(() => () => describeConcordance(state), [state]);
  /** Read a line's passage in the Reader. */
  const openInReader = async (ref: string) => {
    if (!corpus) return;
    const lead = reference.name ?? corpus;
    const aligned = lead === corpus ? ref : await alignRef(corpus, ref, lead);
    const { chapter, verse } = splitRef(aligned ?? ref);
    await switchInterface("reader", { ...(await describeConcordance(state)), ref: chapter, verse, searching: false });
  };

  const download = async () => {
    if (!state.s || !lines.data) return;
    setDownloading(true);
    try {
      const all: KwicLine[] = [];
      for (let offset = 0; offset < Math.min(lines.data.total, 20000); offset += 500) {
        const chunk = await api.kwic(state.s, {
          offset,
          limit: 500,
          left: state.context,
          right: state.context,
          sort: state.sort,
          filter,
          per: state.per,
          sample: state.sample,
          seed: state.seed,
          fmt: settings.fmt,
        });
        all.push(...chunk.lines);
      }
      // with a byte order mark, so that spreadsheets read the Hebrew right
      const blob = new Blob(["﻿" + linesToTsv(all)], { type: "text/tab-separated-values" });
      const link = document.createElement("a");
      link.href = URL.createObjectURL(blob);
      link.download = `concordance-${corpus}.tsv`;
      link.click();
      URL.revokeObjectURL(link.href);
    } finally {
      setDownloading(false);
    }
  };

  const [sortPos, sortFeature] = (state.sort ?? ".").split(".");
  const dir = card?.display.direction ?? "rtl";
  const total = lines.data?.total ?? 0;
  const per10k = lines.data ? (lines.data.total / lines.data.words) * 10000 : null;
  const pages = Math.ceil(total / PAGE);
  const inspectedCard = inspected ? cards[inspected.corpus] : undefined;
  const select = "rounded border border-rule-strong bg-raised px-1.5 py-0.5 text-[13px]";

  return (
    // on a phone the whole page scrolls; on a wider screen the lines and facets scroll apart
    <div className="flex h-full flex-col overflow-y-auto md:overflow-hidden">
      <header className="flex flex-wrap items-center gap-x-2 gap-y-1 border-b border-rule bg-raised px-2 py-1.5 sm:px-3">
        <span className="me-1" title="Context-Fabric">
          <Mark />
        </span>
        <span className="text-[15px] font-semibold">Concordance</span>
        <select
          value={corpus ?? ""}
          onChange={(e) => actions.set({ corpus: e.target.value, s: null, filters: [], sort: null }, true)}
          className={select}
          title="The text searched"
        >
          {names.map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
        <div className="ms-auto flex flex-wrap items-center justify-end gap-1 sm:gap-2">
          <DisplayOptions
            settings={settings}
            formats={card?.display.textFormats ?? []}
            defaultFormat={card?.display.defaultFormat ?? null}
            onChange={setSettings}
          />
          <Workspaces corpora={names} current="concordance" onLoad={navigate} />
          <button
            type="button"
            onClick={toggleTheme}
            className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft"
            aria-label={theme === "dark" ? "Use light colours" : "Use dark colours"}
          >
            {theme === "dark" ? "Light" : "Dark"}
          </button>
          <InterfaceMenu current="concordance" describe={describe} />
        </div>
      </header>

      {corpora.data !== undefined && names.length === 0 ? (
        <p className="p-10 text-stone">No corpus is open.</p>
      ) : !card ? (
        <p className="p-10 text-stone">Opening…</p>
      ) : (
        <>
          {corpus ? (
            <div className="max-w-[48rem] px-4 pt-3 empty:hidden">
              <ProposalCard
                corpora={[corpus]}
                onRun={(p, asked) => void run(p.template, encodeAsked(asked))}
                onEdit={(p, asked) => actions.set({ q: p.template, ask: encodeAsked(asked) })}
              />
            </div>
          ) : null}
          <section className="border-b border-rule bg-raised/50 px-4 py-3">
            <AskedLine asked={decodeAsked(state.ask)} template={state.q} />
            <div className="flex flex-wrap items-start gap-3">
              <div className="min-w-[18rem] max-w-[40rem] flex-1">
                <QueryEditor
                  corpus={card.name}
                  value={state.q}
                  nodeTypes={card.nodeTypes.map((t) => t.type)}
                  typeCounts={Object.fromEntries(card.nodeTypes.map((t) => [t.type, t.count]))}
                  features={features?.features ?? []}
                  onChange={(q) => actions.set({ q })}
                  onRun={() => run()}
                />
              </div>
              <div className="flex flex-col gap-2">
                <button
                  type="button"
                  onClick={() => run()}
                  disabled={starting || !state.q.trim()}
                  className="rounded bg-woad px-4 py-1.5 text-[14px] font-semibold text-vellum disabled:opacity-50"
                >
                  Search
                </button>
                {history.length ? (
                  <select
                    value=""
                    onChange={(e) => e.target.value && actions.set({ q: e.target.value })}
                    className={`${select} max-w-[14rem]`}
                  >
                    <option value="">Recent searches…</option>
                    {history.map((t) => (
                      <option key={t} value={t}>
                        {t.trim().replace(/\n\s*/g, " / ")}
                      </option>
                    ))}
                  </select>
                ) : null}
              </div>
              {terms.some((t) => t.corpus === corpus) ? (
                <div className="flex flex-col gap-1">
                  <div className="flex items-center gap-2 text-[13px]">
                    <span className="text-stone">Search terms, all in one</span>
                    <select value={currentRange} onChange={(e) => setRange(e.target.value)} className={select}>
                      {ranges.map((r) => (
                        <option key={r} value={r}>
                          {r}
                        </option>
                      ))}
                    </select>
                  </div>
                  <ul className="flex flex-wrap gap-2">
                    {terms
                      .filter((t) => t.corpus === corpus)
                      .map((term) => (
                        <TermCard
                          key={term.id}
                          term={term}
                          onChange={(t) => setTerms(terms.map((x) => (x.id === t.id ? t : x)))}
                          onRemove={() => setTerms(terms.filter((x) => x.id !== term.id))}
                        />
                      ))}
                  </ul>
                </div>
              ) : (
                <p className="max-w-[20rem] text-[12px] text-stone">
                  Write a search template, or click a word in the results and choose <em>Add as search term</em>.
                </p>
              )}
            </div>
            <div className="mt-2">
              <Problems problems={problems.length ? problems : (status?.problems ?? [])} />
            </div>
          </section>

          {status && card ? (
            <StatusLine
              status={status}
              onCancel={() => api.cancel(status.id)}
              onDownload={download}
              corpus={card}
              tab="results"
              ask={state.ask}
            />
          ) : null}

          <div className="flex flex-1 flex-col md:min-h-0 md:flex-row">
            {status && status.count > 0 ? (
              // on a phone the facets are folded away, so that the lines have the room
              <button
                type="button"
                aria-expanded={facetsOpen}
                onClick={() => setFacetsOpen(!facetsOpen)}
                className="border-b border-rule px-4 py-1.5 text-start text-[13px] text-woad md:hidden"
              >
                {facetsOpen ? "▾" : "▸"} Filters{state.filters.length ? ` (${state.filters.length})` : ""}
              </button>
            ) : null}
            {status && status.count > 0 ? (
              <aside
                className={`${facetsOpen ? "" : "hidden"} min-h-0 border-b border-rule p-3 md:block md:overflow-y-auto md:w-[17rem] md:border-b-0 md:border-e`}
              >
                {state.filters.length ? (
                  <div className="mb-3 flex flex-wrap gap-1">
                    {state.filters.map(([f, v]) => (
                      <button
                        key={`${f}:${v}`}
                        type="button"
                        onClick={() => actions.set({ filters: toggleFilter(state.filters, f, v) })}
                        className="rounded bg-woad px-2 py-0.5 text-[12px] text-vellum"
                        title="Take this filter away"
                      >
                        {f === "book" ? displayRef(v) : v} ×
                      </button>
                    ))}
                  </div>
                ) : null}
                {FACETS.filter(([f]) => f === "book" || featureNames.has(f)).map(([f, name]) => (
                  <Facet
                    key={f}
                    search={status}
                    feature={f}
                    name={name}
                    state={state}
                    onToggle={(v) => actions.set({ filters: toggleFilter(state.filters, f, v) })}
                  />
                ))}
              </aside>
            ) : null}

            <main className="min-w-0 flex-1 md:min-h-0 md:overflow-y-auto">
              {status && status.count > 0 ? (
                <>
                  <nav
                    className="flex flex-wrap items-center gap-x-4 gap-y-1 border-b border-rule px-4 py-1.5 text-[13px]"
                    aria-label="Views"
                  >
                    <span className="flex gap-0.5">
                      {TABS.map((t) => (
                        <button
                          key={t}
                          type="button"
                          aria-pressed={state.tab === t}
                          onClick={() => actions.set({ tab: t })}
                          className="rounded px-2.5 py-0.5 hover:bg-woad-soft aria-pressed:bg-ink aria-pressed:text-vellum"
                        >
                          {TAB_NAMES[t]}
                        </button>
                      ))}
                    </span>
                    {state.tab === "lines" ? (
                      <>
                        <span className="tabular-nums">
                          {total.toLocaleString("en-US")} {total === 1 ? "line" : "lines"}
                          {per10k !== null ? (
                            <span className="text-stone"> · {per10k.toFixed(1)} per 10,000 words</span>
                          ) : null}
                        </span>
                        <label className="flex items-center gap-1">
                          <span className="text-stone">Sort by</span>
                          <select
                            value={state.sort ? sortPos : ""}
                            onChange={(e) =>
                              actions.set({
                                sort: e.target.value
                                  ? `${e.target.value}.${sortFeature || (card?.translation ? "lower" : "text")}`
                                  : null,
                              })
                            }
                            className={select}
                          >
                            <option value="">result order</option>
                            {POSITIONS.map((p) => (
                              <option key={p} value={p}>
                                {POSITION_NAMES[p]}
                              </option>
                            ))}
                          </select>
                          {state.sort ? (
                            <select
                              value={sortFeature}
                              onChange={(e) => actions.set({ sort: `${sortPos}.${e.target.value}` })}
                              className={select}
                            >
                              {SORT_FEATURES.filter(([f]) =>
                                f === "text" ? !card?.translation : featureNames.has(f),
                              ).map(([f, name]) => (
                                <option key={f} value={f}>
                                  {name}
                                </option>
                              ))}
                            </select>
                          ) : null}
                        </label>
                        <label className="flex items-center gap-1">
                          <span className="text-stone">Context</span>
                          <select
                            value={state.context}
                            onChange={(e) => actions.set({ context: Number(e.target.value) })}
                            className={select}
                          >
                            {CONTEXTS.map((c) => (
                              <option key={c} value={c}>
                                {c} words
                              </option>
                            ))}
                          </select>
                        </label>
                        <label className="flex items-center gap-1.5">
                          <input
                            type="checkbox"
                            checked={state.per === "verse"}
                            onChange={(e) => actions.set({ per: e.target.checked ? "verse" : "result" })}
                            className="accent-[var(--woad)]"
                          />
                          One per verse
                        </label>
                        <label className="flex items-center gap-1">
                          <span className="text-stone">Sample</span>
                          <select
                            value={state.sample ?? ""}
                            onChange={(e) => actions.set({ sample: e.target.value ? Number(e.target.value) : null })}
                            className={select}
                          >
                            <option value="">all</option>
                            {[50, 100, 250, 500].map((n) => (
                              <option key={n} value={n}>
                                {n} at random
                              </option>
                            ))}
                          </select>
                          {state.sample ? (
                            <button
                              type="button"
                              onClick={() => actions.set({ seed: state.seed + 1 })}
                              className="text-woad hover:underline"
                            >
                              another
                            </button>
                          ) : null}
                        </label>
                        <button
                          type="button"
                          onClick={download}
                          disabled={downloading}
                          className="text-woad hover:underline disabled:opacity-50"
                          title="The lines as a table (TSV, readable by spreadsheets), in this order, with these filters; at most 20,000"
                        >
                          {downloading ? "Preparing…" : "Download lines"}
                        </button>
                      </>
                    ) : null}
                  </nav>

                  {state.tab === "statistics" ? (
                    <Stats search={status} corpus={card} onOpenPassage={(ref) => void openInReader(ref)} />
                  ) : state.tab === "collocations" ? (
                    <Collocations search={status} card={card} state={state} />
                  ) : lines.error ? (
                    <p className="p-6 text-madder">{String((lines.error as Error).message)}</p>
                  ) : !lines.data ? (
                    <p className="p-6 text-stone">Loading…</p>
                  ) : (
                    <div className={`px-3 py-2 ${lines.isFetching ? "opacity-70" : ""}`}>
                      {lines.data.lines.map((line) => (
                        <div
                          key={line.index}
                          dir={dir}
                          className="grid grid-cols-[6.5rem_minmax(0,1fr)_auto_minmax(0,1fr)] items-baseline gap-x-3 border-b border-rule py-1"
                        >
                          <button
                            type="button"
                            dir="ltr"
                            onClick={() => line.ref && void openInReader(line.ref)}
                            className="truncate text-start text-[12px] text-woad hover:underline"
                            title={line.ref ? `Read ${displayRef(line.ref)} in the Reader` : undefined}
                          >
                            {line.ref ? displayRef(line.ref) : ""}
                          </button>
                          <span className="flex justify-end gap-x-1 overflow-hidden whitespace-nowrap text-stone">
                            <Words
                              words={line.left}
                              card={card}
                              onWord={(n) => setInspected({ corpus: card.name, node: n })}
                              onHover={onHover}
                            />
                          </span>
                          <span className="rounded bg-gold-leaf/70 px-1 font-semibold">
                            <Words
                              words={line.hit}
                              card={card}
                              onWord={(n) => setInspected({ corpus: card.name, node: n })}
                              onHover={onHover}
                            />
                          </span>
                          <span className="flex justify-start gap-x-1 overflow-hidden whitespace-nowrap text-stone">
                            <Words
                              words={line.right}
                              card={card}
                              onWord={(n) => setInspected({ corpus: card.name, node: n })}
                              onHover={onHover}
                            />
                          </span>
                        </div>
                      ))}
                      {pages > 1 ? (
                        <nav
                          className="flex items-center justify-center gap-3 py-4 text-[13px]"
                          aria-label="Pages of lines"
                        >
                          <button
                            type="button"
                            disabled={page === 0}
                            onClick={() => setPage(page - 1)}
                            className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
                          >
                            Previous
                          </button>
                          <span className="tabular-nums text-stone">
                            {(page * PAGE + 1).toLocaleString("en-US")}–
                            {Math.min(total, (page + 1) * PAGE).toLocaleString("en-US")} of{" "}
                            {total.toLocaleString("en-US")}
                          </span>
                          <button
                            type="button"
                            disabled={page + 1 >= pages}
                            onClick={() => setPage(page + 1)}
                            className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
                          >
                            Next
                          </button>
                        </nav>
                      ) : null}
                    </div>
                  )}
                </>
              ) : status && status.state === "done" ? (
                <p className="p-6 text-stone">Nothing matches.</p>
              ) : !status ? (
                <div className="max-w-[38rem] p-8 text-[15px] leading-relaxed">
                  <h1 className="mb-3 text-[20px] font-semibold">Concordance of {card.name}</h1>
                  <p className="mb-3">
                    Search, and the results come as lines with the hit in the middle and its context on both sides. Sort
                    them by the words before or after the hit to see patterns, filter them by book, stem or tense on the
                    left, and see which words are typical near the hit under <em>Collocations</em>.
                  </p>
                  <button
                    type="button"
                    onClick={() => run(example, null)}
                    className="rounded border border-rule-strong px-3 py-1 font-mono text-[13px] hover:border-woad"
                  >
                    Try “{example}”
                  </button>
                </div>
              ) : null}
              {hovered && card ? <HoverCard corpus={card} hovered={hovered} /> : null}
            </main>

            {inspected && inspectedCard ? (
              <div className="max-h-[45vh] min-h-0 border-t border-rule md:max-h-none md:border-t-0">
                <Inspector
                  corpus={inspectedCard}
                  node={inspected.node}
                  onInspect={(node) => setInspected({ corpus: inspected.corpus, node })}
                  onClose={() => setInspected(null)}
                  onOpenPassage={(ref) => void openInReader(ref)}
                  onSearchTerm={(info) => setTerms([...terms, termFrom(inspected.corpus, info)])}
                />
              </div>
            ) : null}
          </div>
        </>
      )}
    </div>
  );
}
