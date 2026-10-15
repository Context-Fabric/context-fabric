import { useEffect, useRef } from "react";
import { DisplayProvider, displayRef, Plain } from "../display/Display";
import type { PaneDiff } from "../differences";
import { HoverCard, useHover } from "../HoverCard";
import { PlaceSummary } from "../read/BookView";
import type { CorpusCard, ParallelChapter, ViewSettings } from "../types";

// The chapter as a table, as in Parabible: a column per corpus and a row per
// verse, so that the texts line up verse by verse.

/** A corpus' settings: a text format it does not have falls back to its default. */
export function settingsFor(card: CorpusCard, settings: ViewSettings): ViewSettings {
  return settings.fmt && !card.display.textFormats.includes(settings.fmt) ? { ...settings, fmt: null } : settings;
}

/** The verse number at the start of a cell. */
function VerseNumber({ verse }: { verse: number }) {
  return (
    <sup dir="ltr" className="me-1 text-[11px] font-semibold tabular-nums text-madder">
      {verse}
    </sup>
  );
}

type Cell = ParallelChapter["rows"][number]["cells"][string];

/** One corpus' text of one verse: the verse itself, or for the DSS the text of each scroll. */
function CellView({ cell, verse, diff }: { cell: Cell | undefined; verse: number; diff: PaneDiff | null }) {
  if (!cell) return null;
  if (cell.witnesses) {
    const places = diff?.places;
    return (
      <ol className="flex flex-col gap-1">
        {places?.verses.get(verse)?.map((place, k) => (
          <li key={`place-${k}`}>
            <PlaceSummary place={place} a={places.a} b={places.b} />
          </li>
        ))}
        {cell.witnesses.map((w, k) => (
          <li key={w.scroll} dir={w.tree.direction}>
            <span dir="ltr" className="me-1.5 inline-flex items-baseline gap-1 text-[11px] leading-none">
              {k === 0 ? <VerseNumber verse={verse} /> : null}
              <span className="font-semibold text-woad" title={`${w.scroll} ${w.place}`}>
                {w.scroll}
              </span>
              {w.alsoNonBiblical ? (
                <span className="text-gold-deep" title="Also listed among the non-biblical material of the DSS">
                  *
                </span>
              ) : null}
            </span>
            <Plain tree={w.tree} />
          </li>
        ))}
      </ol>
    );
  }
  return (
    <>
      {(cell.trees ?? []).map((tree) => (
        <p key={tree.node} dir={tree.direction}>
          <VerseNumber verse={verse} />
          <Plain tree={tree} />
        </p>
      ))}
    </>
  );
}

export function Table({
  data,
  cards,
  settings,
  verse,
  diffs,
  onRemove,
  onInspect,
}: {
  data: ParallelChapter;
  cards: Record<string, CorpusCard>;
  settings: ViewSettings;
  /** The verse to scroll to and mark. */
  verse: number | null;
  /** Per corpus, its differences with the others, when they are shown. */
  diffs: Record<string, PaneDiff | null>;
  onRemove: (name: string) => void;
  onInspect: (corpus: string, node: number) => void;
}) {
  const columns = data.columns.filter((c) => cards[c.name] !== undefined);
  // one hover card for the whole table; the corpus is the one last hovered over
  const [hovered, onHover] = useHover();
  const hoveredCorpus = useRef<string | null>(null);
  const table = useRef<HTMLDivElement>(null);

  useEffect(() => {
    onHover(null);
  }, [data.ref, onHover]);

  useEffect(() => {
    if (verse === null) return;
    table.current?.querySelector(`[data-verse="${verse}"]`)?.scrollIntoView({ block: "center" });
  }, [verse, data.ref]);

  const hoveredCard = hoveredCorpus.current ? cards[hoveredCorpus.current] : undefined;
  return (
    <>
      {hovered && hoveredCard ? (
        <HoverCard corpus={hoveredCard} hovered={hovered} note={diffs[hoveredCard.name]?.titles?.get(hovered.node)} />
      ) : null}
      <div
        ref={table}
        role="table"
        aria-label={displayRef(data.ref)}
        className="mx-auto grid px-3 pb-6"
        style={{
          // a corpus without this chapter keeps a narrow column, for its header
          gridTemplateColumns: columns
            .map((c) => (c.ref === null ? "minmax(9rem, 0.3fr)" : "minmax(0, 1fr)"))
            .join(" "),
          maxWidth: `${Math.min(
            columns.reduce((w, c) => w + (c.ref === null ? 10 : 26), 0),
            110,
          )}rem`,
        }}
      >
        <div role="row" className="contents">
          {columns.map((column) => {
            const card = cards[column.name];
            return (
              <div
                key={column.name}
                role="columnheader"
                className="sticky top-0 z-10 flex items-center border-b border-rule-strong bg-vellum px-2 py-1 text-[13px]"
              >
                <span className="min-w-0 flex-1 text-center" title={card.title}>
                  <span className="font-semibold">{card.name}</span>
                  {column.ref === null ? (
                    <span className="block text-[12px] text-stone">not in this corpus</span>
                  ) : column.kind === "witnesses" ? (
                    <span
                      className="text-stone"
                      title="Per verse, the text of each scroll. Dimmed: reconstructed by the editors; ◦ an illegible letter; […] lost text."
                    >
                      {" "}
                      · scrolls
                    </span>
                  ) : null}
                  {diffs[column.name]?.message ? (
                    <span className="block text-[11px] text-stone">{diffs[column.name]!.message}</span>
                  ) : null}
                </span>
                <button
                  type="button"
                  onClick={() => onRemove(column.name)}
                  className="rounded px-1.5 text-stone hover:bg-woad-soft hover:text-ink"
                  aria-label={`Remove ${card.name}`}
                  title="Remove this text"
                >
                  –
                </button>
              </div>
            );
          })}
        </div>
        {data.rows.map((row) => (
          <div key={row.verse} role="row" className="contents" aria-label={displayRef(row.ref)}>
            {columns.map((column) => {
              const card = cards[column.name];
              return (
                <div
                  key={column.name}
                  role="cell"
                  data-verse={row.verse}
                  className={`min-w-0 scroll-mt-10 border-b border-rule px-2 py-1.5 ${
                    row.verse === verse ? "bg-gold-leaf/50" : ""
                  }`}
                >
                  <DisplayProvider
                    config={card.display}
                    settings={settingsFor(card, settings)}
                    onInspect={(node) => onInspect(card.name, node)}
                    marks={diffs[column.name]?.marks}
                    markTitles={diffs[column.name]?.titles}
                    onHover={(node, target) => {
                      hoveredCorpus.current = card.name;
                      onHover(node, target);
                    }}
                  >
                    <CellView cell={row.cells[column.name]} verse={row.verse} diff={diffs[column.name] ?? null} />
                  </DisplayProvider>
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </>
  );
}
