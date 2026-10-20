import { useQueries, useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { SearchStatus } from "../types";

interface Props {
  search: SearchStatus;
  fmt: string | null;
  onClose: () => void;
}

/** Choose the columns of the results file and download it. */
export function Download({ search, fmt, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [excel, setExcel] = useState(false);
  const [extra, setExtra] = useState<Record<number, string[]>>({});
  const [filter, setFilter] = useState("");

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const first = useQuery({
    queryKey: ["stats-first-row", search.id],
    queryFn: () => api.results(search.id, 0, 1),
  });
  const types = first.data?.rows[0]?.types ?? [];
  const featureLists = useQueries({
    queries: types.map((_, position) => ({
      queryKey: ["stats-features", search.id, search.version, position],
      queryFn: () => api.statsFeatures(search.id, position),
    })),
  });

  const toggle = (position: number, name: string) =>
    setExtra((current) => {
      const chosen = current[position] ?? [];
      return {
        ...current,
        [position]: chosen.includes(name) ? chosen.filter((n) => n !== name) : [...chosen, name],
      };
    });

  const extras = Object.entries(extra)
    .filter(([, names]) => names.length)
    .map(([position, names]) => `${Number(position) + 1}:${names.join(",")}`);
  const href = api.exportUrl(search.id, fmt, excel, extras);
  const needle = filter.trim().toLowerCase();

  return (
    <dialog
      ref={dialog}
      onClose={onClose}
      className="m-auto max-h-[85vh] w-[min(44rem,calc(100vw-2rem))] rounded border border-rule-strong bg-raised p-0 text-ink backdrop:bg-[rgba(21,23,29,0.45)]"
    >
      <div className="flex max-h-[85vh] flex-col">
        <header className="flex items-baseline gap-3 border-b border-rule px-5 py-3">
          <h2 className="text-[18px] font-semibold">Download the results</h2>
          <button
            type="button"
            onClick={() => dialog.current?.close()}
            className="ms-auto rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          >
            Close
          </button>
        </header>
        <div className="flex flex-col gap-4 overflow-y-auto px-5 py-4 text-[14px]">
          <p className="text-stone">
            One row per result: its passage, then for each part its node number, type and text, and the features
            below. Features in the template are always included.
          </p>
          <input
            type="search"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Find a feature"
            className="rounded border border-rule-strong bg-vellum px-2.5 py-1.5 placeholder:text-stone"
          />
          {types.map((type, i) => {
            const data = featureLists[i].data;
            const inTemplate = new Set(data?.inTemplate ?? []);
            const chosen = new Set(extra[i] ?? []);
            const shown = (data?.features ?? []).filter((f) => !needle || f.toLowerCase().includes(needle));
            return (
              <fieldset key={i} className="m-0 border-0 p-0">
                <legend className="mb-1.5 font-semibold">
                  {types.length > 1 ? `${i + 1}. ` : ""}
                  {type}
                  {chosen.size ? <span className="ms-2 font-normal text-stone">{chosen.size} added</span> : null}
                </legend>
                {!data ? (
                  <p className="text-stone">Finding its features…</p>
                ) : shown.length === 0 ? (
                  <p className="text-stone">No feature matches.</p>
                ) : (
                  <div className="grid grid-cols-[repeat(auto-fill,minmax(10rem,1fr))] gap-x-3 gap-y-0.5">
                    {shown.map((name) => (
                      <label key={name} className="flex items-center gap-1.5 truncate" title={name}>
                        <input
                          type="checkbox"
                          checked={inTemplate.has(name) || chosen.has(name)}
                          disabled={inTemplate.has(name)}
                          onChange={() => toggle(i, name)}
                          className="accent-[var(--woad)]"
                        />
                        <span className={inTemplate.has(name) ? "text-stone" : ""}>{name}</span>
                      </label>
                    ))}
                  </div>
                )}
              </fieldset>
            );
          })}
        </div>
        <footer className="flex flex-wrap items-center gap-4 border-t border-rule px-5 py-3 text-[14px]">
          <fieldset className="m-0 flex gap-4 border-0 p-0">
            <legend className="sr-only">File type</legend>
            <label className="flex items-center gap-1.5">
              <input type="radio" name="kind" checked={!excel} onChange={() => setExcel(false)} />
              TSV
            </label>
            <label className="flex items-center gap-1.5" title="UTF-16 text, which Excel opens with Hebrew and other scripts intact">
              <input type="radio" name="kind" checked={excel} onChange={() => setExcel(true)} />
              TSV for Excel
            </label>
          </fieldset>
          <span className="text-stone">{search.count.toLocaleString("en-US")} rows</span>
          <a
            href={href}
            download
            onClick={() => dialog.current?.close()}
            className="ms-auto rounded bg-woad px-4 py-1.5 font-semibold text-vellum"
          >
            Download
          </a>
        </footer>
      </div>
    </dialog>
  );
}
