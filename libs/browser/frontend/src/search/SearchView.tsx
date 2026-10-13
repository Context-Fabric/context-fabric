import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";
import { api, ApiError } from "../api";
import type { CorpusCard, Problem, SearchStatus, ViewSettings } from "../types";
import { QueryEditor } from "./QueryEditor";
import { ResultList } from "./ResultList";

const HISTORY_KEY = "cfabric-browser/history";
const HISTORY_SIZE = 20;

function loadHistory(corpus: string): string[] {
  try {
    const all = JSON.parse(window.localStorage.getItem(HISTORY_KEY) ?? "{}");
    return Array.isArray(all[corpus]) ? all[corpus] : [];
  } catch {
    return [];
  }
}

function remember(corpus: string, template: string): string[] {
  const list = [template, ...loadHistory(corpus).filter((t) => t !== template)].slice(0, HISTORY_SIZE);
  try {
    const all = JSON.parse(window.localStorage.getItem(HISTORY_KEY) ?? "{}");
    all[corpus] = list;
    window.localStorage.setItem(HISTORY_KEY, JSON.stringify(all));
  } catch {
    // history is a convenience; searching works without it
  }
  return list;
}

/** Follow a search's status with server-sent events. */
function useSearchStatus(id: string | null): SearchStatus | null {
  const client = useQueryClient();
  const { data } = useQuery({
    queryKey: ["search", id],
    queryFn: () => api.status(id!),
    enabled: id !== null,
    retry: false,
  });

  useEffect(() => {
    if (!id || !data || (data.state !== "running" && data.state !== "queued")) return;
    const source = api.events(id);
    source.addEventListener("status", (event) => {
      const status: SearchStatus = JSON.parse((event as MessageEvent).data);
      client.setQueryData(["search", id], status);
      if (status.state !== "running" && status.state !== "queued") source.close();
    });
    source.onerror = () => source.close();
    return () => source.close();
    // reconnect only when the search changes, not on every status update
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, data?.state === "running" || data?.state === "queued"]);

  return data ?? null;
}

function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}

function StatusLine({ status, onCancel, exportUrl }: { status: SearchStatus; onCancel: () => void; exportUrl: string }) {
  const seconds = status.elapsed < 10 ? status.elapsed.toFixed(2) : status.elapsed.toFixed(1);
  const results = `${formatCount(status.count)} ${status.count === 1 ? "result" : "results"}`;
  let text: string;
  switch (status.state) {
    case "queued":
      text = "Waiting for a free search worker…";
      break;
    case "running":
      text = `Searching… ${results} so far`;
      break;
    case "done":
      text = status.truncated
        ? `First ${results} (stopped at the limit) in ${seconds}s`
        : `${results} in ${seconds}s`;
      break;
    case "timeout":
      text = `Stopped after ${status.timeout}s with ${results}. Narrow the template, or raise --timeout.`;
      break;
    case "cancelled":
      text = `Stopped with ${results}`;
      break;
    default:
      text = "The search failed";
  }
  const running = status.state === "running" || status.state === "queued";
  const finished = !running && status.state !== "failed";
  return (
    <div className="flex items-center gap-4 border-b border-rule px-4 py-2.5 text-[14px]" role="status">
      <span className={status.state === "timeout" ? "text-madder" : ""}>{text}</span>
      {running ? (
        <button
          type="button"
          onClick={onCancel}
          className="rounded border border-rule-strong px-2.5 py-0.5 text-[13px] hover:border-madder hover:text-madder"
        >
          Stop
        </button>
      ) : null}
      {finished && status.count > 0 ? (
        <a href={exportUrl} className="ms-auto text-[13px] text-woad hover:underline" download>
          Download as TSV
        </a>
      ) : null}
    </div>
  );
}

function Problems({ problems }: { problems: Problem[] }) {
  if (!problems.length) return null;
  return (
    <ul className="flex flex-col gap-1.5 rounded border border-madder/40 bg-madder-soft px-3 py-2 text-[13px]" role="alert">
      {problems.map((p, i) => (
        <li key={i}>
          {p.line ? <span className="font-mono text-[12px] text-madder">line {p.line} </span> : null}
          {p.message}
        </li>
      ))}
    </ul>
  );
}

interface Props {
  corpus: CorpusCard;
  template: string;
  searchId: string | null;
  expanded: number | null;
  settings: ViewSettings;
  onTemplate: (template: string) => void;
  onSearch: (id: string | null) => void;
  onExpand: (index: number | null) => void;
  onOpenPassage: (ref: string) => void;
}

export function SearchView({
  corpus,
  template,
  searchId,
  expanded,
  settings,
  onTemplate,
  onSearch,
  onExpand,
  onOpenPassage,
}: Props) {
  const status = useSearchStatus(searchId);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [starting, setStarting] = useState(false);
  const [history, setHistory] = useState<string[]>(() => loadHistory(corpus.name));
  const { data: features } = useQuery({
    queryKey: ["features", corpus.name],
    queryFn: () => api.features(corpus.name),
  });

  const run = useCallback(async () => {
    if (!template.trim()) return;
    setStarting(true);
    setProblems([]);
    try {
      const started = await api.search(corpus.name, template);
      setHistory(remember(corpus.name, template));
      onSearch(started.id);
    } catch (e) {
      if (e instanceof ApiError && e.problems.length) setProblems(e.problems);
      else setProblems([{ kind: "error", line: null, message: String(e instanceof Error ? e.message : e) }]);
    } finally {
      setStarting(false);
    }
  }, [corpus.name, template, onSearch]);

  const shownProblems = problems.length ? problems : (status?.problems ?? []);
  const running = status?.state === "running" || status?.state === "queued";

  return (
    <div className="grid h-full min-h-0 grid-cols-1 md:grid-cols-[minmax(20rem,26rem)_1fr]">
      <aside className="flex min-h-0 flex-col gap-3 overflow-y-auto border-rule p-4 md:border-e">
        <h2 className="text-[13px] text-stone">Search template</h2>
        <QueryEditor
          corpus={corpus.name}
          value={template}
          nodeTypes={corpus.nodeTypes.map((t) => t.type)}
          features={features?.features ?? []}
          onChange={onTemplate}
          onRun={run}
        />
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={run}
            disabled={starting || running || !template.trim()}
            className="rounded bg-woad px-4 py-1.5 text-[14px] font-semibold text-vellum disabled:opacity-50"
          >
            Search
          </button>
          <span className="text-[12px] text-stone">or Ctrl+Enter</span>
        </div>
        <Problems problems={shownProblems} />
        {history.length ? (
          <section className="mt-2">
            <h2 className="mb-1 text-[13px] text-stone">Recent searches</h2>
            <ul className="flex flex-col">
              {history.map((t) => (
                <li key={t}>
                  <button
                    type="button"
                    onClick={() => onTemplate(t)}
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
        {status ? (
          <>
            <StatusLine
              status={status}
              onCancel={() => api.cancel(status.id)}
              exportUrl={api.exportUrl(status.id, settings.fmt)}
            />
            <div className="min-h-0 flex-1">
              {status.count > 0 ? (
                <ResultList
                  search={status}
                  settings={settings}
                  expanded={expanded}
                  onExpand={onExpand}
                  onOpenPassage={onOpenPassage}
                />
              ) : status.state === "done" ? (
                <p className="p-6 text-stone">
                  Nothing matches this template. Check the feature values, or loosen a constraint.
                </p>
              ) : null}
            </div>
          </>
        ) : (
          <Intro corpus={corpus} onTry={onTemplate} />
        )}
      </section>
    </div>
  );
}

function Intro({ corpus, onTry }: { corpus: CorpusCard; onTry: (t: string) => void }) {
  const types = corpus.nodeTypes.filter((t) => !t.isSlot).map((t) => t.type);
  const slot = corpus.slotType;
  const container = corpus.display.condenseType ?? types[types.length - 1] ?? slot;
  const example = `${container}\n  ${slot}`;
  return (
    <div className="max-w-[38rem] p-8 text-[15px] leading-relaxed">
      <h1 className="mb-3 text-[22px] font-semibold">Search {corpus.title.split(" = ").pop()}</h1>
      <p className="mb-3">
        Write a search template: one object per line, with indentation for what lies inside what, and
        features to narrow it down. Templates are the same as in the Text-Fabric browser.
      </p>
      <p className="mb-4 text-stone">
        Object types in this corpus: {types.concat(slot).join(", ")}.
      </p>
      <button
        type="button"
        onClick={() => onTry(example)}
        className="rounded border border-rule-strong px-3 py-1 font-mono text-[13px] hover:border-woad"
      >
        Try “{example.replace("\n  ", " / ")}”
      </button>
    </div>
  );
}
