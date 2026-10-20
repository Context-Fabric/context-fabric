import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../api";
import { displayRef } from "../display/Display";
import type { CorpusCard, SearchStatus } from "../types";

// Statistics over a search's results: how often each value of a feature occurs at
// one position of the result tuples, and how the results spread over the text.
// Charts are single-series horizontal bars (one hue, no legend; the heading says
// what is plotted), with the count at each bar's tip and a table view.

interface Bar {
  key: string;
  label: string;
  count: number;
  muted?: boolean;
  title?: string;
}

function formatCount(n: number): string {
  return n.toLocaleString("en-US");
}

function percent(part: number, whole: number): string {
  if (!whole) return "";
  const p = (100 * part) / whole;
  return p >= 10 ? `${p.toFixed(0)}%` : p >= 1 ? `${p.toFixed(1)}%` : "<1%";
}

function BarChart({ bars, total, caption, onBar }: { bars: Bar[]; total: number; caption: string; onBar?: (bar: Bar) => void }) {
  const [asTable, setAsTable] = useState(false);
  const max = Math.max(1, ...bars.map((b) => b.count));
  return (
    <figure className="m-0">
      <figcaption className="mb-2 flex items-baseline gap-3">
        <span className="text-[14px] font-semibold">{caption}</span>
        <button
          type="button"
          onClick={() => setAsTable((t) => !t)}
          className="ms-auto text-[12px] text-woad hover:underline"
        >
          {asTable ? "Show as chart" : "Show as table"}
        </button>
      </figcaption>
      {asTable ? (
        <table className="w-full border-collapse text-[13px]">
          <thead>
            <tr className="text-left text-stone">
              <th className="py-1 pe-3 font-normal">Value</th>
              <th className="py-1 pe-3 text-end font-normal">Count</th>
              <th className="py-1 text-end font-normal">Share</th>
            </tr>
          </thead>
          <tbody>
            {bars.map((b) => (
              <tr key={b.key} className="border-t border-rule">
                <td className="py-1 pe-3" dir="auto">
                  {b.label}
                </td>
                <td className="py-1 pe-3 text-end tabular-nums">{formatCount(b.count)}</td>
                <td className="py-1 text-end tabular-nums text-stone">{percent(b.count, total)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <ol className="m-0 flex list-none flex-col gap-[2px] p-0">
          {bars.map((b) => {
            const width = `${Math.max(0.5, (100 * b.count) / max)}%`;
            const description = `${b.label}: ${formatCount(b.count)} (${percent(b.count, total)})`;
            const Row = onBar && !b.muted ? "button" : "div";
            return (
              <li key={b.key}>
                <Row
                  type={Row === "button" ? "button" : undefined}
                  onClick={Row === "button" ? () => onBar?.(b) : undefined}
                  title={b.title ?? description}
                  aria-label={description}
                  className="group grid w-full grid-cols-[minmax(4rem,9rem)_1fr] items-center gap-3 rounded px-1 py-[3px] text-left hover:bg-woad-soft focus-visible:bg-woad-soft"
                >
                  <span className={`truncate text-end text-[13px] ${b.muted ? "text-stone" : ""}`} dir="auto">
                    {b.label}
                  </span>
                  <span className="flex min-w-0 items-center gap-2">
                    <span
                      className={`h-3.5 shrink-0 rounded-e-[4px] transition-opacity group-hover:opacity-80 ${
                        b.muted ? "bg-rule-strong" : "bg-chart"
                      }`}
                      style={{ width, maxWidth: "calc(100% - 6.5rem)" }}
                    />
                    <span className="shrink-0 text-[12px] tabular-nums text-stone">
                      {formatCount(b.count)} <span className="opacity-80">{percent(b.count, total)}</span>
                    </span>
                  </span>
                </Row>
              </li>
            );
          })}
        </ol>
      )}
    </figure>
  );
}

function ValueStats({
  search,
  position,
  types,
}: {
  search: SearchStatus;
  position: number;
  types: string[];
}) {
  const features = useQuery({
    queryKey: ["stats-features", search.id, search.version, position],
    queryFn: () => api.statsFeatures(search.id, position),
  });
  const [chosen, setChosen] = useState<Record<number, string>>({});
  const list = features.data?.features ?? [];
  // by default a feature the template leaves open (`lex*`), as that one varies
  const feature =
    chosen[position] ??
    features.data?.open[0] ??
    features.data?.inTemplate[0] ??
    (list.includes("sp") ? "sp" : list[0]);
  const values = useQuery({
    queryKey: ["stats-values", search.id, search.version, position, feature],
    queryFn: () => api.statsValues(search.id, position, feature!),
    enabled: Boolean(feature),
    placeholderData: (previous) => previous,
  });
  const data = values.data;

  const bars: Bar[] = data
    ? [
        ...data.values.map((v) => ({ key: `v:${v.value}`, label: v.value, count: v.count })),
        ...(data.other.values
          ? [
              {
                key: "other",
                label: `${formatCount(data.other.values)} other values`,
                count: data.other.count,
                muted: true,
              },
            ]
          : []),
        ...(data.withoutValue
          ? [{ key: "none", label: "no value", count: data.withoutValue, muted: true }]
          : []),
      ]
    : [];

  return (
    <section className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3 text-[14px]">
        <label className="flex items-center gap-2">
          <span className="text-stone">Feature</span>
          <select
            value={feature ?? ""}
            onChange={(e) => setChosen((c) => ({ ...c, [position]: e.target.value }))}
            className="rounded border border-rule-strong bg-raised px-2 py-0.5"
          >
            {features.data?.inTemplate.length ? (
              <optgroup label="In the template">
                {features.data.inTemplate.map((f) => (
                  <option key={`t:${f}`} value={f}>
                    {f}
                  </option>
                ))}
              </optgroup>
            ) : null}
            <optgroup label={`All features of ${types[position] ?? "this object"}`}>
              {list.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </optgroup>
          </select>
        </label>
        {data ? (
          <span className="text-[13px] text-stone">
            {formatCount(data.distinct)} different values over {formatCount(data.nodes)} {types[position] ?? "objects"}
            {data.nodes === 1 ? "" : "s"}
          </span>
        ) : null}
      </div>
      {values.error ? <p className="text-madder">{String((values.error as Error).message)}</p> : null}
      {data && feature ? (
        <div className={values.isFetching ? "opacity-70" : ""}>
          <BarChart
            bars={bars}
            total={data.nodes}
            caption={`Values of ${feature} on ${types[position] ?? "this object"}`}
          />
        </div>
      ) : features.isLoading || values.isLoading ? (
        <p className="text-stone">Counting…</p>
      ) : null}
    </section>
  );
}

function SectionStats({
  search,
  corpus,
  onOpenPassage,
}: {
  search: SearchStatus;
  corpus: CorpusCard;
  onOpenPassage: (ref: string) => void;
}) {
  const levels = corpus.sectionTypes.slice(0, 2);
  const [level, setLevel] = useState(0);
  const { data, error, isFetching } = useQuery({
    queryKey: ["stats-sections", search.id, search.version, level],
    queryFn: () => api.statsSections(search.id, level),
    enabled: levels.length > 0,
    placeholderData: (previous) => previous,
  });
  if (!levels.length) return null;
  const bars: Bar[] = (data?.sections ?? []).map((s) => ({
    key: String(s.node),
    label: displayRef(level === 0 ? s.label : s.ref),
    count: s.count,
    title: `${displayRef(s.ref)}: ${formatCount(s.count)}. Click to read it.`,
  }));
  return (
    <section className="flex flex-col gap-3">
      {levels.length > 1 ? (
        <div className="flex gap-1 text-[13px]" role="group" aria-label="Count per">
          {levels.map((t, i) => (
            <button
              key={t}
              type="button"
              aria-pressed={level === i}
              onClick={() => setLevel(i)}
              className="rounded px-2.5 py-0.5 hover:bg-woad-soft aria-pressed:bg-ink aria-pressed:text-vellum"
            >
              Per {t}
            </button>
          ))}
        </div>
      ) : null}
      {error ? <p className="text-madder">{String((error as Error).message)}</p> : null}
      {data ? (
        <div className={isFetching ? "opacity-70" : ""}>
          <BarChart
            bars={bars}
            total={search.count}
            caption={`Results per ${data.type}, in the order of the text (by the first object of each result)`}
            onBar={(bar) => {
              const section = data.sections.find((s) => String(s.node) === bar.key);
              if (section) onOpenPassage(section.ref);
            }}
          />
        </div>
      ) : (
        <p className="text-stone">Counting…</p>
      )}
    </section>
  );
}

export function Stats({
  search,
  corpus,
  onOpenPassage,
}: {
  search: SearchStatus;
  corpus: CorpusCard;
  onOpenPassage: (ref: string) => void;
}) {
  const first = useQuery({
    queryKey: ["stats-first-row", search.id],
    queryFn: () => api.results(search.id, 0, 1),
  });
  const types = first.data?.rows[0]?.types ?? [];
  // by default the last part of a result, usually the smallest (a word)
  const [chosenPosition, setPosition] = useState<number | null>(null);
  const position = chosenPosition ?? Math.max(0, types.length - 1);
  const running = search.state === "running" || search.state === "queued";

  return (
    <div className="flex h-full flex-col gap-6 overflow-y-auto px-4 py-4">
      {running ? (
        <p className="text-[13px] text-stone">The search is still running; these counts cover the results so far.</p>
      ) : null}
      {search.truncated ? (
        <p className="text-[13px] text-stone">
          The search stopped at {formatCount(search.count)} results; the counts cover those.
        </p>
      ) : null}
      {types.length ? (
        <>
          <div className="flex flex-wrap items-center gap-2 text-[14px]" role="group" aria-label="Part of the result">
            <span className="text-stone">Count over</span>
            {types.map((t, i) => (
              <button
                key={i}
                type="button"
                aria-pressed={position === i}
                onClick={() => setPosition(i)}
                className="rounded border border-rule-strong px-2.5 py-0.5 hover:border-woad aria-pressed:border-ink aria-pressed:bg-ink aria-pressed:text-vellum"
              >
                {types.length > 1 ? `${i + 1}. ` : ""}
                {t}
              </button>
            ))}
          </div>
          <ValueStats search={search} position={Math.min(position, types.length - 1)} types={types} />
          <SectionStats search={search} corpus={corpus} onOpenPassage={onOpenPassage} />
        </>
      ) : (
        <p className="text-stone">Counting…</p>
      )}
    </div>
  );
}
