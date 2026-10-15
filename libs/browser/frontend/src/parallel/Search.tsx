import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api, ApiError } from "../api";
import { displayRef } from "../display/Display";
import { Download } from "../search/Download";
import { QueryEditor } from "../search/QueryEditor";
import {
  loadHistory,
  Problems,
  remember,
  StatusLine,
  useSearchGone,
  useSearchStatus,
  type Tab,
} from "../search/SearchView";
import { Stats } from "../search/Stats";
import type { CorpusCard, ParallelChapter, Problem, SearchStatus, ViewSettings } from "../types";
import type { SearchState } from "./state";
import { Table } from "./Table";
import { buildTemplate, hasBooks, rangesOf, type Term } from "../shared/terms";

// Searching in the parallel interface: search terms made from words, or a
// template, in one of the shown corpora; the results are rows with the same
// verse in every column.

const PAGE = 25;

interface Props {
  cards: Record<string, CorpusCard>;
  texts: string[];
  search: SearchState;
  terms: Term[];
  settings: ViewSettings;
  onTerms: (terms: Term[]) => void;
  onSearch: (change: Partial<SearchState>, push?: boolean) => void;
  onOpenPassage: (corpus: string, ref: string) => void;
  onInspect: (corpus: string, node: number) => void;
}

/** One search term: the word it was made from and the features that count. */
function TermCard({ term, onChange, onRemove }: { term: Term; onChange: (t: Term) => void; onRemove: () => void }) {
  return (
    <li className="rounded border border-rule-strong bg-raised px-3 py-2">
      <div className="mb-1 flex items-baseline gap-2">
        <span className="text-[12px] text-stone">{term.type}</span>
        <span className="script script-hbo flex-1 truncate text-[16px] leading-tight">{term.text}</span>
        <button
          type="button"
          onClick={onRemove}
          className="rounded px-1.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          aria-label="Remove this search term"
        >
          ×
        </button>
      </div>
      <ul className="flex flex-col gap-0.5 text-[13px]">
        {term.features.map((f, i) => (
          <li key={f.feature}>
            <label className="flex items-baseline gap-2" title={`${f.feature}=${f.value}`}>
              <input
                type="checkbox"
                checked={f.on}
                onChange={(e) =>
                  onChange({
                    ...term,
                    features: term.features.map((g, j) => (j === i ? { ...g, on: e.target.checked } : g)),
                  })
                }
                className="accent-[var(--woad)]"
              />
              <span className="w-28 shrink-0 text-stone">{f.label}</span>
              <span className="min-w-0 truncate">{f.shown}</span>
            </label>
          </li>
        ))}
      </ul>
    </li>
  );
}

/** A page of results as parallel rows. */
function Results({
  status,
  cards,
  texts,
  settings,
  onOpenPassage,
  onInspect,
}: {
  status: SearchStatus;
  cards: Record<string, CorpusCard>;
  texts: string[];
  settings: ViewSettings;
  onOpenPassage: (corpus: string, ref: string) => void;
  onInspect: (corpus: string, node: number) => void;
}) {
  const [page, setPage] = useState(0);
  useEffect(() => setPage(0), [status.id]);
  const shown = Math.min(status.count, (page + 1) * PAGE);
  const { data, error, isFetching } = useQuery({
    // while running, rows arrive in search order; once sorted they are fetched again
    queryKey: [
      "search-parallel",
      status.id,
      page,
      texts.join(","),
      settings.fmt,
      settings.hideTypes,
      status.sorted,
      shown,
    ],
    queryFn: () =>
      api.searchParallel(status.id, page * PAGE, PAGE, texts, {
        mode: "plain",
        fmt: settings.fmt,
        hide_types: settings.hideTypes,
      }),
    enabled: status.count > page * PAGE,
    placeholderData: (previous) => previous,
  });
  if (error) return <p className="p-6 text-madder">{String((error as Error).message)}</p>;
  if (!data) return <p className="p-6 text-stone">Loading…</p>;

  const corpus = status.corpus;
  const chapter: ParallelChapter = {
    ref: `results ${page * PAGE + 1}–${shown}`,
    path: [],
    prev: null,
    next: null,
    columns: data.columns
      .filter((n) => cards[n])
      .map((n) => ({ name: n, kind: cards[n].bookView ? "witnesses" : "text", ref: n })),
    rows: data.rows.map((r) => ({ verse: r.index, ref: r.ref, cells: r.cells })),
  };
  const pages = Math.ceil(status.count / PAGE);

  return (
    <div className={isFetching ? "opacity-70" : ""}>
      <Table
        data={chapter}
        cards={cards}
        settings={settings}
        verse={null}
        diffs={{}}
        verseNumbers={false}
        onInspect={onInspect}
        rowHeader={(row) => (
          <button
            type="button"
            onClick={() => onOpenPassage(corpus, row.ref)}
            className="text-start text-woad hover:underline"
            title={`Read ${displayRef(row.ref)}`}
          >
            <span className="block text-[11px] tabular-nums text-stone">{row.verse + 1}</span>
            {displayRef(row.ref)}
          </button>
        )}
      />
      {pages > 1 ? (
        <nav className="flex items-center justify-center gap-3 pb-6 text-[13px]" aria-label="Pages of results">
          <button
            type="button"
            disabled={page === 0}
            onClick={() => setPage(page - 1)}
            className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
          >
            Previous
          </button>
          <span className="tabular-nums text-stone">
            {page * PAGE + 1}–{shown} of {status.count.toLocaleString("en-US")}
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
  );
}

export function ParallelSearch({
  cards,
  texts,
  search,
  terms,
  settings,
  onTerms,
  onSearch,
  onOpenPassage,
  onInspect,
}: Props) {
  const shown = texts.filter((t) => cards[t]);
  const corpus = search.sc && cards[search.sc] ? search.sc : (terms[terms.length - 1]?.corpus ?? shown[0] ?? null);
  const card = corpus ? cards[corpus] : undefined;
  const ownTerms = terms.filter((t) => t.corpus === corpus);
  const otherTerms = terms.length - ownTerms.length;

  const ranges = card ? rangesOf(card) : [];
  const [range, setRange] = useState<string | null>(null);
  const currentRange = range && ranges.includes(range) ? range : (ranges[0] ?? "verse");
  const [books, setBooks] = useState<string[]>([]);
  const bookList = useQuery({
    queryKey: ["sections", corpus, null],
    queryFn: () => api.sections(corpus!),
    enabled: card !== undefined && hasBooks(card),
  });
  // a template written (or edited) by hand, instead of the terms
  const [byHand, setByHand] = useState(() => search.q !== "" && !terms.length);
  // books are chosen by their labels; the template needs the book feature's own values
  const bookItems = (bookList.data?.children ?? []).filter((b) => books.includes(b.ref));
  const bookValues = bookItems.length
    ? { feature: bookItems[0].feature ?? "book", values: bookItems.map((b) => b.value ?? b.ref) }
    : null;
  const generated = ownTerms.length ? buildTemplate(ownTerms, currentRange, bookValues) : "";
  const template = byHand ? search.q : generated;

  const status = useSearchStatus(search.s);
  const gone = useSearchGone(search.s);
  const [tab, setTab] = useState<Tab>("results");
  const [downloading, setDownloading] = useState(false);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [starting, setStarting] = useState(false);
  const [history, setHistory] = useState<string[]>(() => (corpus ? loadHistory(corpus) : []));
  const { data: features } = useQuery({
    queryKey: ["features", corpus],
    queryFn: () => api.features(corpus!),
    enabled: corpus !== null,
  });

  const run = async () => {
    if (!corpus || !template.trim()) return;
    setStarting(true);
    setProblems([]);
    try {
      const started = await api.search(corpus, template);
      setHistory(remember(corpus, template));
      setTab("results");
      onSearch({ sc: corpus, q: template, s: started.id });
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
    if (gone && search.s && rerun !== search.s && search.q.trim() && corpus) {
      setRerun(search.s);
      api
        .search(corpus, search.q)
        .then((started) => onSearch({ s: started.id }, false))
        .catch(() => undefined);
    }
  }, [gone, search.s, search.q, rerun, corpus, onSearch]);

  if (!card) return <p className="p-10 text-stone">Choose a text to search under Texts.</p>;
  const running = status?.state === "running" || status?.state === "queued";
  const shownProblems = problems.length ? problems : (status?.problems ?? []);
  const statusCard = status ? cards[status.corpus] : undefined;
  const label = "mb-1 text-[13px] text-stone";

  return (
    <div className="grid h-full min-h-0 grid-cols-1 md:grid-cols-[minmax(19rem,24rem)_1fr]">
      <aside className="flex max-h-[50vh] min-h-0 flex-col gap-4 overflow-y-auto border-b border-rule p-4 md:max-h-none md:border-b-0 md:border-e">
        <label className="flex items-center gap-2 text-[14px]">
          <span className="text-stone">Search in</span>
          <select
            value={corpus ?? ""}
            onChange={(e) => onSearch({ sc: e.target.value, s: null }, false)}
            className="rounded border border-rule-strong bg-raised px-2 py-1"
          >
            {shown.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
        </label>

        <div className="flex rounded border border-rule-strong text-[13px]" role="group" aria-label="Search with">
          {(
            [
              [false, "Search terms"],
              [true, "Template"],
            ] as const
          ).map(([hand, name]) => (
            <button
              key={name}
              type="button"
              aria-pressed={byHand === hand}
              onClick={() => {
                // editing starts from the template the terms make
                if (hand && !byHand && generated) onSearch({ q: generated }, false);
                setByHand(hand);
              }}
              className="flex-1 px-3 py-1 aria-pressed:bg-woad aria-pressed:text-vellum"
            >
              {name}
            </button>
          ))}
        </div>

        {byHand ? (
          <QueryEditor
            corpus={card.name}
            value={search.q}
            nodeTypes={card.nodeTypes.map((t) => t.type)}
            typeCounts={Object.fromEntries(card.nodeTypes.map((t) => [t.type, t.count]))}
            features={features?.features ?? []}
            onChange={(q) => onSearch({ q }, false)}
            onRun={run}
          />
        ) : (
          <>
            <section>
              <h2 className={label}>Search terms</h2>
              {ownTerms.length ? (
                <ul className="flex flex-col gap-2">
                  {ownTerms.map((term) => (
                    <TermCard
                      key={term.id}
                      term={term}
                      onChange={(t) => onTerms(terms.map((x) => (x.id === t.id ? t : x)))}
                      onRemove={() => onTerms(terms.filter((x) => x.id !== term.id))}
                    />
                  ))}
                </ul>
              ) : (
                <p className="text-[13px] text-stone">
                  Click a word in the text, then <em>Add as search term</em> in the word panel. Each term finds words
                  with the features you tick.
                </p>
              )}
              {otherTerms ? (
                <p className="mt-1 text-[12px] text-stone">
                  {otherTerms} {otherTerms === 1 ? "term is" : "terms are"} from another text; search there to use{" "}
                  {otherTerms === 1 ? "it" : "them"}.
                </p>
              ) : null}
            </section>
            <label className="flex items-center gap-2 text-[14px]">
              <span className="text-stone">All in one</span>
              <select
                value={currentRange}
                onChange={(e) => setRange(e.target.value)}
                className="rounded border border-rule-strong bg-raised px-2 py-1"
              >
                {ranges.map((r) => (
                  <option key={r} value={r}>
                    {r}
                  </option>
                ))}
              </select>
            </label>
            {hasBooks(card) ? (
              <label className="flex flex-col gap-1 text-[14px]">
                <span className="text-stone">
                  Books <span className="text-[12px]">({books.length ? `${books.length} chosen` : "all"})</span>
                </span>
                <select
                  multiple
                  value={books}
                  onChange={(e) => setBooks([...e.target.selectedOptions].map((o) => o.value))}
                  className="h-28 rounded border border-rule-strong bg-raised px-1 py-1 text-[13px]"
                >
                  {(bookList.data?.children ?? []).map((b) => (
                    <option key={b.node} value={b.ref}>
                      {displayRef(b.label)}
                    </option>
                  ))}
                </select>
                {books.length ? (
                  <button
                    type="button"
                    onClick={() => setBooks([])}
                    className="self-start text-[12px] text-woad hover:underline"
                  >
                    All books
                  </button>
                ) : null}
              </label>
            ) : null}
            {generated ? (
              <pre
                className="overflow-x-auto rounded bg-woad-soft px-2.5 py-1.5 font-mono text-[12px]"
                title="The search template"
              >
                {generated}
              </pre>
            ) : null}
          </>
        )}

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={run}
            disabled={starting || running || !template.trim()}
            className="rounded bg-woad px-4 py-1.5 text-[14px] font-semibold text-vellum disabled:opacity-50"
          >
            Search
          </button>
          {byHand ? <span className="text-[12px] text-stone">or Ctrl+Enter</span> : null}
        </div>
        <Problems problems={shownProblems} />
        {byHand && history.length ? (
          <section>
            <h2 className={label}>Recent searches</h2>
            <ul className="flex flex-col">
              {history.map((t) => (
                <li key={t}>
                  <button
                    type="button"
                    onClick={() => onSearch({ q: t }, false)}
                    className="w-full truncate rounded px-1.5 py-1 text-left font-mono text-[12px] hover:bg-woad-soft"
                    title={t}
                  >
                    {t.trim().replace(/\n\s*/g, " / ")}
                  </button>
                </li>
              ))}
            </ul>
          </section>
        ) : null}
      </aside>

      <section className="flex min-h-0 flex-col">
        {downloading && status ? (
          <Download search={status} fmt={settings.fmt} onClose={() => setDownloading(false)} />
        ) : null}
        {status && statusCard ? (
          <>
            <StatusLine
              status={status}
              onCancel={() => api.cancel(status.id)}
              onDownload={() => setDownloading(true)}
              corpus={statusCard}
              tab={tab}
              onTab={setTab}
            />
            <div className="min-h-0 flex-1 overflow-y-auto">
              {status.count > 0 && tab === "stats" ? (
                <Stats search={status} corpus={statusCard} onOpenPassage={(ref) => onOpenPassage(status.corpus, ref)} />
              ) : status.count > 0 ? (
                <Results
                  status={status}
                  cards={cards}
                  texts={shown}
                  settings={settings}
                  onOpenPassage={onOpenPassage}
                  onInspect={onInspect}
                />
              ) : status.state === "done" ? (
                <p className="p-6 text-stone">Nothing matches. Untick a feature, or choose a wider range.</p>
              ) : null}
            </div>
          </>
        ) : (
          <div className="max-w-[36rem] p-8 text-[15px] leading-relaxed">
            <h1 className="mb-3 text-[20px] font-semibold">Search {card.name}</h1>
            <p className="mb-3">
              Build a search from words: click a word in the text and choose <em>Add as search term</em>. Tick the
              features that should count, choose whether the terms must be in one verse, clause or phrase, and limit the
              search to some books if you like.
            </p>
            <p className="text-stone">
              Or write a search template by hand, as in the Text-Fabric browser. The results show each verse in all the
              texts.
            </p>
          </div>
        )}
      </section>
    </div>
  );
}
