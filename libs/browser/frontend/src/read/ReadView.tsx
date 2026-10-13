import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../api";
import { Cite, citePassage } from "../Cite";
import { displayRef, Plain, Pretty } from "../display/Display";
import type { CorpusCard, ReadBy, SectionItem, ViewSettings } from "../types";
import { ReadBySwitch } from "./BookView";

interface Props {
  corpus: CorpusCard;
  passageRef: string | null;
  settings: ViewSettings;
  onOpen: (ref: string) => void;
  /** In a narrow pane the passage list folds away behind a button. */
  compact?: boolean;
  /** Shown above the text, e.g. when synchronised reading found only the chapter. */
  note?: string | null;
  /** About differences with the text in other panes, when they are shown. */
  diff?: string | null;
  /** For a corpus that can also be read by biblical book (DSS). */
  onBy?: (by: ReadBy) => void;
}

/** Books, then the chapters of the current book. */
function Navigator({
  corpus,
  path,
  onOpen,
}: {
  corpus: CorpusCard;
  path: SectionItem[];
  onOpen: (ref: string) => void;
}) {
  const [filter, setFilter] = useState("");
  const top = useQuery({ queryKey: ["sections", corpus.name, null], queryFn: () => api.sections(corpus.name) });
  const book = path[0] ?? null;
  const inner = useQuery({
    queryKey: ["sections", corpus.name, book?.ref],
    queryFn: () => api.sections(corpus.name, book!.ref),
    enabled: book !== null && corpus.sectionTypes.length > 1,
  });
  const current = new Set(path.map((p) => p.node));
  const books = (top.data?.children ?? []).filter((b) =>
    b.label.toLowerCase().includes(filter.trim().toLowerCase()),
  );

  return (
    <nav className="flex min-h-0 flex-col gap-3 overflow-y-auto p-4" aria-label="Passages">
      <input
        type="search"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder={`Find a ${corpus.sectionTypes[0] ?? "section"}`}
        className="rounded border border-rule-strong bg-raised px-2.5 py-1.5 text-[14px] placeholder:text-stone"
      />
      {book && inner.data?.children.length ? (
        <section>
          <h2 className="mb-1.5 text-[13px] text-stone">
            {displayRef(book.label)}, {corpus.sectionTypes[1]}s
          </h2>
          <ol className="grid grid-cols-[repeat(auto-fill,minmax(2.6rem,1fr))] gap-1">
            {inner.data.children.map((c) => (
              <li key={c.node}>
                <button
                  type="button"
                  onClick={() => onOpen(c.ref)}
                  aria-current={current.has(c.node) ? "page" : undefined}
                  className="w-full rounded px-1 py-1 text-center text-[13px] tabular-nums hover:bg-woad-soft aria-[current=page]:bg-woad aria-[current=page]:text-vellum"
                >
                  {c.label}
                </button>
              </li>
            ))}
          </ol>
        </section>
      ) : null}
      <ol className="flex flex-col">
        {books.map((b) => (
          <li key={b.node}>
            <button
              type="button"
              onClick={() => openFirst(corpus, b.ref, onOpen)}
              aria-current={current.has(b.node) ? "true" : undefined}
              className="w-full rounded px-2 py-1 text-left text-[14px] hover:bg-woad-soft aria-[current=true]:font-semibold aria-[current=true]:text-woad"
            >
              {displayRef(b.label)}
            </button>
          </li>
        ))}
      </ol>
    </nav>
  );
}

/** Open the first part of a section one level down (a book opens at its first chapter). */
async function openFirst(corpus: CorpusCard, ref: string, onOpen: (ref: string) => void) {
  if (corpus.sectionTypes.length < 3) return onOpen(ref);
  const { children } = await api.sections(corpus.name, ref);
  onOpen(children[0]?.ref ?? ref);
}

/** Where to start reading when no passage is chosen: the first chapter of the first book. */
function useStartRef(corpus: CorpusCard, enabled: boolean): string | null {
  const { data } = useQuery({
    queryKey: ["start", corpus.name],
    enabled,
    queryFn: async () => {
      const top = await api.sections(corpus.name);
      const first = top.children[0];
      if (!first || corpus.sectionTypes.length < 3) return first?.ref ?? null;
      const inner = await api.sections(corpus.name, first.ref);
      return inner.children[0]?.ref ?? first.ref;
    },
  });
  return data ?? null;
}

export function ReadView({ corpus, passageRef, settings, onOpen, compact = false, note = null, diff = null, onBy }: Props) {
  const [mode, setMode] = useState<"pretty" | "plain">("plain");
  const startRef = useStartRef(corpus, passageRef === null);
  const ref = passageRef ?? startRef;
  const passage = useQuery({
    queryKey: ["passage", corpus.name, ref, mode, settings.fmt, settings.hideTypes],
    queryFn: () => api.passage(corpus.name, ref!, { mode, fmt: settings.fmt, hide_types: settings.hideTypes }),
    enabled: ref !== null,
    placeholderData: (previous) => previous,
  });
  const data = passage.data;
  const [navOpen, setNavOpen] = useState(false);
  const open = (r: string) => {
    setNavOpen(false);
    onOpen(r);
  };

  return (
    <div className={`grid h-full min-h-0 grid-cols-1 ${compact ? "" : "md:grid-cols-[16rem_1fr]"}`}>
      <aside
        className={`${navOpen ? "flex" : "hidden"} max-h-[60vh] min-h-0 border-b border-rule ${
          compact ? "" : "md:flex md:max-h-none md:border-b-0 md:border-e"
        }`}
      >
        <Navigator corpus={corpus} path={data?.path ?? []} onOpen={open} />
      </aside>
      <section className="flex min-h-0 flex-col">
        <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-rule px-5 py-2.5">
          <button
            type="button"
            aria-expanded={navOpen}
            onClick={() => setNavOpen((o) => !o)}
            className={`rounded border border-rule-strong px-2.5 py-0.5 text-[13px] ${compact ? "" : "md:hidden"}`}
          >
            Passages
          </button>
          <h1 className="text-[20px] font-semibold">{data ? displayRef(data.ref) : ref ? displayRef(ref) : corpus.title}</h1>
          <div className="flex items-center gap-1 text-[13px]">
            <button
              type="button"
              disabled={!data?.prev}
              onClick={() => data?.prev && onOpen(data.prev.ref)}
              className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
              title={data?.prev ? `Previous: ${displayRef(data.prev.ref)}` : undefined}
            >
              Previous
            </button>
            <button
              type="button"
              disabled={!data?.next}
              onClick={() => data?.next && onOpen(data.next.ref)}
              className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
              title={data?.next ? `Next: ${displayRef(data.next.ref)}` : undefined}
            >
              Next
            </button>
          </div>
          <div className="ms-auto flex items-center gap-2">
          {data ? <Cite text={(software, date) => citePassage(corpus, data.ref, software, date)} /> : null}
          {corpus.bookView && onBy ? <ReadBySwitch by="sections" onBy={onBy} /> : null}
          <div className="flex rounded border border-rule-strong text-[13px]" role="group" aria-label="Display">
            {(["plain", "pretty"] as const).map((m) => (
              <button
                key={m}
                type="button"
                aria-pressed={mode === m}
                onClick={() => setMode(m)}
                className="px-3 py-1 aria-pressed:bg-woad aria-pressed:text-vellum"
              >
                {m === "plain" ? "Text" : "Structure"}
              </button>
            ))}
          </div>
          </div>
        </header>
        <div className={`min-h-0 flex-1 overflow-y-auto px-5 py-4 ${passage.isFetching ? "opacity-70" : ""}`}>
          {note ? <p className="mb-3 text-[13px] text-stone">{note}</p> : null}
          {diff ? (
            <p className="mb-3 flex flex-wrap items-baseline gap-x-3 text-[13px] text-stone">
              <span>{diff}.</span>
              <span>
                <span className="diff diff-variant diff-key text-ink">variant</span>{" "}
                <span className="diff diff-matres diff-key text-ink">spelling (matres lectionis)</span>
              </span>
            </p>
          ) : null}
          {passage.error ? (
            <p className="text-madder">{String((passage.error as Error).message)}</p>
          ) : !ref ? (
            <p className="text-stone">Choose a passage on the left.</p>
          ) : data ? (
            <div className="flex max-w-[60rem] flex-col">
              {data.trees.map((tree) =>
                mode === "plain" ? (
                  <p key={tree.node} dir={tree.direction} className="flex items-baseline gap-4 py-1.5">
                    <span dir="ltr" className="w-8 shrink-0 text-end text-[12px] tabular-nums text-stone">
                      {tree.heading.split(/[ :.]/).pop()}
                    </span>
                    <span className="flex-1">
                      <Plain tree={tree} />
                    </span>
                  </p>
                ) : (
                  <div key={tree.node} className="py-2.5">
                    <Pretty tree={tree} />
                  </div>
                ),
              )}
              {data.truncated ? (
                <p className="py-3 text-stone">
                  Showing the first {data.trees.length} of {data.total}. Open a smaller part to see the rest.
                </p>
              ) : null}
            </div>
          ) : (
            <p className="text-stone">Loading…</p>
          )}
        </div>
      </section>
    </div>
  );
}
