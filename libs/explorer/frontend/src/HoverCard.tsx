import { useQuery } from "@tanstack/react-query";
import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore, type ReactNode } from "react";
import { api } from "./api";
import { scriptClass } from "./display/Display";
import { borrowedFrom, type CorpusCard } from "./types";

// The card shown when hovering over a word: the essentials of the word (gloss,
// lexeme, part of speech, morphology), next to it. Clicking the word still opens
// the inspector with everything.

const DELAY = 250; // ms of hovering before the card appears
const GAP = 6; // px between the word and the card

export interface Hovered {
  node: number;
  anchor: DOMRect;
}

/** Which word is hovered over, after a short delay, so that moving the mouse
 * across the text does not fetch every word on the way. Kept outside React state:
 * only `Hovering` (the card) follows it, so a card that appears or goes doesn't
 * draw the whole text again. */
export class HoverState {
  private hovered: Hovered | null = null;
  private listeners = new Set<() => void>();
  private timer: number | undefined;

  /** A word is hovered over (null: no longer); stable, to pass down. */
  readonly onHover = (node: number | null, target?: Element) => {
    window.clearTimeout(this.timer);
    if (node === null || !target) {
      this.set(null);
      return;
    }
    this.timer = window.setTimeout(() => this.set({ node, anchor: target.getBoundingClientRect() }), DELAY);
  };

  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  readonly current = () => this.hovered;

  stop() {
    window.clearTimeout(this.timer);
  }

  private set(hovered: Hovered | null) {
    if (hovered === this.hovered) return;
    this.hovered = hovered;
    this.listeners.forEach((listener) => listener());
  }
}

/** The hover state of one view, and the handler to pass to its words. */
export function useHover(): [HoverState, HoverState["onHover"]] {
  const [state] = useState(() => new HoverState());
  useEffect(() => () => state.stop(), [state]);
  return [state, state.onHover];
}

/** Shows `children(hovered)` (the card) while a word is hovered over. */
export function Hovering({ hover, children }: { hover: HoverState; children: (hovered: Hovered) => ReactNode }) {
  const hovered = useSyncExternalStore(hover.subscribe, hover.current);
  // a scrolled text moves away from the card
  useEffect(() => {
    if (!hovered) return;
    const hide = () => hover.onHover(null);
    window.addEventListener("scroll", hide, true);
    return () => window.removeEventListener("scroll", hide, true);
  }, [hovered, hover]);
  return hovered ? children(hovered) : null;
}

interface Props {
  corpus: CorpusCard;
  hovered: Hovered;
  /** What the word differs from in another pane, if anything. */
  note?: string;
}

export function HoverCard({ corpus, hovered, note }: Props) {
  const { data } = useQuery({
    queryKey: ["summary", corpus.name, hovered.node],
    queryFn: () => api.nodeSummary(corpus.name, hovered.node),
    staleTime: Infinity,
    // a translation's words have no gloss or morphology: no card
    enabled: !corpus.translation,
  });
  const card = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<{ left: number; top: number } | null>(null);

  // below the word, or above it near the bottom of the window; within the window
  useLayoutEffect(() => {
    const el = card.current;
    if (!el) return;
    const { anchor } = hovered;
    const width = el.offsetWidth;
    const height = el.offsetHeight;
    const below = anchor.bottom + GAP + height <= window.innerHeight;
    const top = below ? anchor.bottom + GAP : Math.max(GAP, anchor.top - GAP - height);
    const centre = anchor.left + anchor.width / 2 - width / 2;
    const left = Math.min(Math.max(GAP, centre), window.innerWidth - width - GAP);
    setPosition({ left, top });
  }, [hovered, data]);

  const script = scriptClass(corpus.display, corpus.display.direction);
  if (corpus.translation) return null;
  return (
    <div
      ref={card}
      role="tooltip"
      className="pointer-events-none fixed z-40 w-max max-w-[20rem] rounded border border-rule-strong bg-raised px-3 py-2 text-[13px] shadow-lg"
      style={position ? { left: position.left, top: position.top } : { left: -9999, top: -9999 }}
    >
      {data ? (
        <>
          {data.text ? (
            <p dir={corpus.display.direction} className="mb-1 text-[15px] leading-snug">
              <span className={script}>{data.text}</span>
            </p>
          ) : null}
          {data.summary.length ? (
            <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
              {data.summary.map((item) => (
                <div key={item.label} className="contents">
                  <dt className="text-stone">{item.label}</dt>
                  <dd className={`m-0 ${item.label === "lexeme" ? script : ""}`}>
                    {item.value}
                    {borrowedFrom(item) ? <span className="text-stone"> · {borrowedFrom(item)}</span> : null}
                  </dd>
                </div>
              ))}
            </dl>
          ) : (
            <p className="text-stone">{data.type}</p>
          )}
          {data.bdb?.entries.length ? (
            <p className="mt-1 border-t border-rule pt-1">
              <span className="text-stone">BDB</span>{" "}
              <span dir="rtl" className={script}>
                {data.bdb.entries[0].headword}
              </span>{" "}
              {data.bdb.entries[0].short}
            </p>
          ) : null}
          {note ? <p className="mt-1.5 border-t border-rule pt-1.5 text-madder">{note}</p> : null}
          <p className="mt-1.5 text-[11px] text-stone">Click for all features</p>
        </>
      ) : (
        <p className="text-stone">…</p>
      )}
    </div>
  );
}
