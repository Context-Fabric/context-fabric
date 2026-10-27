import { useCallback, useRef, useState } from "react";
import type { CorpusCard } from "../types";
import { useCloseOutside } from "../shared/popover";

/** The columns shown: on or off per open corpus, in an order of choice, and more from the library. */
export function Texts({
  names,
  cards,
  texts,
  onChange,
  onAdd,
}: {
  names: string[];
  cards: Record<string, CorpusCard>;
  texts: string[];
  onChange: (texts: string[]) => void;
  onAdd: () => void;
}) {
  const [open, setOpen] = useState(false);
  const panel = useRef<HTMLDivElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useCloseOutside(open, close, panel);

  // the shown texts in their order, then the others
  const listed = [...texts.filter((t) => names.includes(t)), ...names.filter((n) => !texts.includes(n))];
  const move = (name: string, by: number) => {
    const i = texts.indexOf(name);
    const j = i + by;
    if (i < 0 || j < 0 || j >= texts.length) return;
    const next = [...texts];
    [next[i], next[j]] = [next[j], next[i]];
    onChange(next);
  };

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="rounded px-2.5 py-1 text-[14px] hover:bg-woad-soft aria-expanded:bg-woad-soft"
      >
        Texts
      </button>
      {open ? (
        <div className="absolute end-0 top-full z-30 mt-1 w-[19rem] rounded border border-rule-strong bg-raised py-1.5 shadow-lg">
          <ul>
            {listed.map((name) => {
              const shown = texts.includes(name);
              const i = texts.indexOf(name);
              return (
                <li key={name} className="flex items-center gap-2 px-3 py-1 text-[14px]">
                  <label className="flex min-w-0 flex-1 items-center gap-2">
                    <input
                      type="checkbox"
                      checked={shown}
                      onChange={(e) => onChange(e.target.checked ? [...texts, name] : texts.filter((t) => t !== name))}
                      className="accent-[var(--woad)]"
                    />
                    <span className="truncate" title={cards[name]?.title}>
                      {name} <span className="text-[12px] text-stone">{cards[name]?.version}</span>
                    </span>
                  </label>
                  {shown ? (
                    <span className="flex text-[12px] text-stone">
                      <button
                        type="button"
                        disabled={i === 0}
                        onClick={() => move(name, -1)}
                        className="rounded px-1 hover:bg-woad-soft disabled:opacity-30"
                        aria-label={`Move ${name} to the left`}
                      >
                        ←
                      </button>
                      <button
                        type="button"
                        disabled={i === texts.length - 1}
                        onClick={() => move(name, 1)}
                        className="rounded px-1 hover:bg-woad-soft disabled:opacity-30"
                        aria-label={`Move ${name} to the right`}
                      >
                        →
                      </button>
                    </span>
                  ) : null}
                </li>
              );
            })}
          </ul>
          <div className="mt-1 border-t border-rule px-3 pt-1.5">
            <button
              type="button"
              onClick={() => {
                setOpen(false);
                onAdd();
              }}
              className="text-[14px] text-woad hover:underline"
            >
              Add a text…
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
