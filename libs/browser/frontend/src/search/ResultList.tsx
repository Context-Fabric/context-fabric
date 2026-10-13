import { useQuery } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useRef } from "react";
import { api } from "../api";
import { displayRef, Plain, Pretty } from "../display/Display";
import type { RenderTree, ResultRow, SearchStatus, ViewSettings } from "../types";

const PAGE = 50;

interface Props {
  search: SearchStatus;
  settings: ViewSettings;
  expanded: number | null;
  onExpand: (index: number | null) => void;
  onOpenPassage: (ref: string) => void;
}

function usePage(search: SearchStatus, page: number, fmt: string | null) {
  const finished = search.state !== "queued" && search.state !== "running";
  return useQuery({
    // while running, rows arrive in search order; once sorted they are refetched
    queryKey: ["results", search.id, page, fmt, search.sorted],
    queryFn: () => api.results(search.id, page * PAGE, PAGE, fmt),
    enabled: search.count > page * PAGE,
    staleTime: finished ? Infinity : 0,
  });
}

/** Nodes of a tuple that are not already visible inside the first node's display. */
function treesToShow(row: ResultRow): RenderTree[] {
  const trees = row.plain ?? [];
  if (trees.length <= 1) return trees;
  const seen = new Set<number>();
  const collect = (frags: RenderTree["fragments"]) => {
    for (const f of frags) {
      seen.add(f.node);
      if (f.children) collect(f.children);
    }
  };
  const shown: RenderTree[] = [];
  for (const tree of trees) {
    if (seen.has(tree.node)) continue;
    shown.push(tree);
    collect(tree.fragments);
  }
  return shown;
}

function Expanded({ search, index, settings }: { search: SearchStatus; index: number; settings: ViewSettings }) {
  const { data, error, isLoading } = useQuery({
    queryKey: ["result", search.id, index, settings.fmt, settings.hideTypes, search.sorted],
    queryFn: () =>
      api.result(search.id, index, {
        mode: "pretty",
        fmt: settings.fmt,
        hide_types: settings.hideTypes,
        extra_features: queryFeatures(search.template),
      }),
  });
  if (isLoading) return <p className="px-4 py-3 text-stone">Loading the display…</p>;
  if (error) return <p className="px-4 py-3 text-madder">Could not load this result: {String(error)}</p>;
  return (
    <div className="flex flex-col gap-3 overflow-x-auto px-4 pb-4 pt-1">
      {data?.trees.map((tree) => (
        <Pretty key={tree.node} tree={tree} />
      ))}
    </div>
  );
}

/** Features mentioned in a template, shown in pretty displays of its results. */
export function queryFeatures(template: string): string {
  const names = new Set<string>();
  for (const match of template.matchAll(/([A-Za-z_][\w@]*)\s*(?:[=#~<>]|\*(?=\s|$))/g)) {
    names.add(match[1]);
  }
  return [...names].join(",");
}

export function ResultList({ search, settings, expanded, onExpand, onOpenPassage }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: search.count,
    getScrollElement: () => scroller.current,
    estimateSize: () => 64,
    overscan: 8,
  });
  const items = virtualizer.getVirtualItems();

  return (
    <div ref={scroller} className="h-full overflow-y-auto" role="list" aria-label="Search results">
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {items.map((item) => (
          <ResultItem
            key={item.key}
            index={item.index}
            start={item.start}
            measure={virtualizer.measureElement}
            search={search}
            settings={settings}
            page={Math.floor(item.index / PAGE)}
            expanded={expanded === item.index}
            onToggle={() => onExpand(expanded === item.index ? null : item.index)}
            onOpenPassage={onOpenPassage}
          />
        ))}
      </div>
    </div>
  );
}

interface ItemProps {
  index: number;
  start: number;
  measure: (el: Element | null) => void;
  search: SearchStatus;
  settings: ViewSettings;
  page: number;
  expanded: boolean;
  onToggle: () => void;
  onOpenPassage: (ref: string) => void;
}

function ResultItem({ index, start, measure, search, settings, page, expanded, onToggle, onOpenPassage }: ItemProps) {
  const { data } = usePage(search, page, settings.fmt);
  const row = data?.rows[index - page * PAGE];

  return (
    <div
      ref={measure}
      data-index={index}
      role="listitem"
      className={`absolute left-0 right-0 border-b border-rule ${expanded ? "bg-raised" : ""}`}
      style={{ transform: `translateY(${start}px)` }}
    >
      <div className="grid grid-cols-[4.5rem_9rem_1fr] items-baseline gap-3 px-4 py-2">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={expanded}
          className="flex items-baseline gap-1.5 text-left font-mono text-[12px] text-stone hover:text-woad"
          title={expanded ? "Hide the structure" : "Show the structure"}
        >
          <span aria-hidden className={`inline-block transition-transform ${expanded ? "rotate-90" : ""}`}>
            ▸
          </span>
          {index + 1}
        </button>
        {row ? (
          <button
            type="button"
            onClick={() => onOpenPassage(row.ref)}
            className="truncate text-left text-[13px] text-woad hover:underline"
            title={`Read ${displayRef(row.ref)}`}
          >
            {displayRef(row.ref)}
          </button>
        ) : (
          <span className="h-3 w-20 rounded bg-rule" />
        )}
        <div className="min-w-0">
          {row ? (
            <div className="flex flex-col gap-1">
              {treesToShow(row).map((tree) => (
                <Plain key={tree.node} tree={tree} />
              ))}
            </div>
          ) : (
            <span className="block h-4 w-2/3 rounded bg-rule" />
          )}
          {settings.showNodes && row ? (
            <p className="mt-1 font-mono text-[11px] text-stone">
              {row.nodes.map((n, i) => `${row.types[i]} ${n}`).join(", ")}
            </p>
          ) : null}
        </div>
      </div>
      {expanded ? <Expanded search={search} index={index} settings={settings} /> : null}
    </div>
  );
}
