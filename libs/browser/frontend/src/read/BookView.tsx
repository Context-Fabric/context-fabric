import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "../api";
import { Cite, citePassage } from "../Cite";
import { displayRef, Plain, Pretty } from "../display/Display";
import type { BookSummary, CorpusCard, Place, ReadBy, ViewSettings } from "../types";

// Reading a corpus of manuscripts (the DSS) by biblical book: per verse, the
// text of every scroll that contains it, so that it can be read next to MT or SP.

/** Switch between reading by the corpus' own sections (scrolls) and by biblical book. */
export function ReadBySwitch({ by, onBy }: { by: ReadBy; onBy: (by: ReadBy) => void }) {
  return (
    <div className="flex rounded border border-rule-strong text-[13px]" role="group" aria-label="Read by">
      {(["sections", "books"] as const).map((b) => (
        <button
          key={b}
          type="button"
          aria-pressed={by === b}
          onClick={() => onBy(b)}
          className="px-3 py-1 aria-pressed:bg-woad aria-pressed:text-vellum"
        >
          {b === "sections" ? "Scrolls" : "Bible books"}
        </button>
      ))}
    </div>
  );
}

/** What the marks for differences with other panes mean. */
export function DiffNote({ message }: { message: string }) {
  return (
    <p className="mb-3 flex flex-wrap items-baseline gap-x-3 text-[13px] text-stone">
      <span>{message}.</span>
      <span>
        <span className="diff diff-variant diff-key text-ink">variant</span>{" "}
        <span className="diff diff-matres diff-key text-ink">spelling (matres lectionis)</span>
      </span>
    </p>
  );
}

function BookNavigator({
  books,
  current,
  onOpen,
}: {
  books: BookSummary[];
  current: { book: string; chapter: number } | null;
  onOpen: (ref: string) => void;
}) {
  const [filter, setFilter] = useState("");
  const book = books.find((b) => b.book === current?.book) ?? null;
  const shown = books.filter((b) => b.label.toLowerCase().includes(filter.trim().toLowerCase()));

  return (
    <nav className="flex min-h-0 flex-col gap-3 overflow-y-auto p-4" aria-label="Biblical books">
      <input
        type="search"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="Find a book"
        className="rounded border border-rule-strong bg-raised px-2.5 py-1.5 text-[14px] placeholder:text-stone"
      />
      {book ? (
        <section>
          <h2 className="mb-1.5 text-[13px] text-stone">{book.label}, chapters with text</h2>
          <ol className="grid grid-cols-[repeat(auto-fill,minmax(2.6rem,1fr))] gap-1">
            {book.chapters.map((c) => (
              <li key={c.chapter}>
                <button
                  type="button"
                  onClick={() => onOpen(c.ref)}
                  aria-current={current?.chapter === c.chapter ? "page" : undefined}
                  title={`${c.verses} verses, ${c.witnesses} witnesses`}
                  className="w-full rounded px-1 py-1 text-center text-[13px] tabular-nums hover:bg-woad-soft aria-[current=page]:bg-woad aria-[current=page]:text-vellum"
                >
                  {c.chapter}
                </button>
              </li>
            ))}
          </ol>
        </section>
      ) : null}
      <ol className="flex flex-col">
        {shown.map((b) => (
          <li key={b.book}>
            <button
              type="button"
              onClick={() => onOpen(b.ref)}
              aria-current={b.book === current?.book ? "true" : undefined}
              className="flex w-full items-baseline justify-between gap-2 rounded px-2 py-1 text-left text-[14px] hover:bg-woad-soft aria-[current=true]:font-semibold aria-[current=true]:text-woad"
            >
              {b.label}
              <span className="text-[12px] font-normal tabular-nums text-stone" title="scrolls">
                {b.scrolls}
              </span>
            </button>
          </li>
        ))}
      </ol>
    </nav>
  );
}

interface Props {
  corpus: CorpusCard;
  passageRef: string | null;
  settings: ViewSettings;
  onOpen: (ref: string) => void;
  onBy: (by: ReadBy) => void;
  /** Open a witness at its place in the scroll view. */
  onOpenScroll: (ref: string) => void;
  compact?: boolean;
  note?: string | null;
  /** About differences with the text in another pane, when they are shown. */
  diff?: string | null;
  /** Next to MT and SP: per verse, the scrolls at each place where MT and SP differ. */
  places?: { a: string; b: string; verses: Map<number, Place[]> };
}

const AGREES = {
  a: "border-woad text-woad",
  b: "border-gold-deep text-gold-deep",
  other: "border-madder text-madder",
} as const;

/** One place where MT and SP differ, and which scrolls side with which. */
export function PlaceSummary({ place, a, b }: { place: Place; a: string; b: string }) {
  return (
    <p className="mb-1 flex flex-wrap items-baseline gap-x-2 gap-y-1 text-[12px] text-stone">
      <span>
        {a}{" "}
        <span className="script text-[14px] text-ink" dir="rtl">
          {place.a || "—"}
        </span>
      </span>
      <span>
        {b}{" "}
        <span className="script text-[14px] text-ink" dir="rtl">
          {place.b || "—"}
        </span>
      </span>
      {place.witnesses.map((w) => (
        <span
          key={w.scroll}
          className={`rounded border px-1.5 ${AGREES[w.agrees]}`}
          title={w.agrees === "other" ? `${w.scroll} reads ${w.text || "—"}` : undefined}
        >
          {w.scroll} {w.agrees === "a" ? `= ${a}` : w.agrees === "b" ? `= ${b}` : "other"}
        </span>
      ))}
    </p>
  );
}

export function BookView({ corpus, passageRef, settings, onOpen, onBy, onOpenScroll, compact = false, note = null, diff = null, places }: Props) {
  const [mode, setMode] = useState<"pretty" | "plain">("plain");
  const [navOpen, setNavOpen] = useState(false);
  const books = useQuery({ queryKey: ["books", corpus.name], queryFn: () => api.books(corpus.name) });
  const ref = passageRef ?? books.data?.books[0]?.ref ?? null;
  const passage = useQuery({
    queryKey: ["witnesses", corpus.name, ref, mode, settings.fmt],
    queryFn: () => api.witnesses(corpus.name, ref!, { mode, fmt: settings.fmt }),
    enabled: ref !== null,
    placeholderData: (previous) => previous,
  });
  const data = passage.data;
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
        <BookNavigator
          books={books.data?.books ?? []}
          current={data ? { book: data.book, chapter: data.chapter } : null}
          onOpen={open}
        />
      </aside>
      <section className="flex min-h-0 flex-col">
        <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-rule px-5 py-2.5">
          <button
            type="button"
            aria-expanded={navOpen}
            onClick={() => setNavOpen((o) => !o)}
            className={`rounded border border-rule-strong px-2.5 py-0.5 text-[13px] ${compact ? "" : "md:hidden"}`}
          >
            Books
          </button>
          <h1 className="text-[20px] font-semibold">{data ? data.ref : ref ?? corpus.title}</h1>
          <div className="flex items-center gap-1 text-[13px]">
            <button
              type="button"
              disabled={!data?.prev}
              onClick={() => data?.prev && onOpen(data.prev)}
              className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
              title={data?.prev ? `Previous: ${data.prev}` : undefined}
            >
              Previous
            </button>
            <button
              type="button"
              disabled={!data?.next}
              onClick={() => data?.next && onOpen(data.next)}
              className="rounded px-2 py-0.5 hover:bg-woad-soft disabled:opacity-40"
              title={data?.next ? `Next: ${data.next}` : undefined}
            >
              Next
            </button>
          </div>
          <div className="ms-auto flex flex-wrap items-center gap-2">
            {data ? <Cite text={(software, date) => citePassage(corpus, data.ref, software, date)} /> : null}
            <ReadBySwitch by="books" onBy={onBy} />
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
          {diff ? <DiffNote message={diff} /> : null}
          {books.error || passage.error ? (
            <p className="text-madder">{String(((books.error ?? passage.error) as Error).message)}</p>
          ) : data ? (
            <>
              <p className="mb-3 text-[13px] text-stone">
                {data.verses.length} {data.verses.length === 1 ? "verse" : "verses"} with text in the scrolls.{" "}
                <span className="dim">Dimmed</span>: reconstructed by the editors; ◦ an illegible letter; […] lost
                text.
              </p>
              <div className="flex max-w-[60rem] flex-col">
                {data.verses.map((v) => (
                  <section key={v.verse} className="border-b border-rule py-2 last:border-b-0" aria-label={v.ref}>
                    <h2 dir="ltr" className="mb-1 text-[12px] tabular-nums text-stone">
                      {data.chapter}:{v.verse}
                    </h2>
                    {places?.verses.get(v.verse)?.map((place, k) => (
                      <PlaceSummary key={k} place={place} a={places.a} b={places.b} />
                    ))}
                    <ol className="flex flex-col gap-1.5">
                      {v.witnesses.map((w) => (
                        <li key={w.scroll} className="flex items-baseline gap-3">
                          <span dir="ltr" className="flex w-28 shrink-0 flex-col text-[13px] leading-tight">
                            <button
                              type="button"
                              onClick={() => w.ref && onOpenScroll(w.ref)}
                              className="text-start font-semibold text-woad hover:underline"
                              title={w.ref ? `Open ${displayRef(w.ref)} in the scroll` : undefined}
                            >
                              {w.scroll}
                            </button>
                            <span className="text-[12px] text-stone">{w.place}</span>
                            {w.alsoNonBiblical ? (
                              <span
                                className="text-[11px] text-gold-deep"
                                title="This text is also listed among the non-biblical material of the DSS: its identification is disputed, or the scroll is partly non-biblical."
                              >
                                also non-biblical
                              </span>
                            ) : null}
                          </span>
                          <span className="min-w-0 flex-1">
                            {mode === "plain" ? <Plain tree={w.tree} /> : <Pretty tree={w.tree} />}
                          </span>
                        </li>
                      ))}
                    </ol>
                  </section>
                ))}
              </div>
            </>
          ) : (
            <p className="text-stone">Loading…</p>
          )}
        </div>
      </section>
    </div>
  );
}
