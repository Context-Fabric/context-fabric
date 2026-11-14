import { useState } from "react";
import { marginNotes } from "./marginNotes";

/**
 * The translators' notes on a verse (the Statenvertaling's kanttekeningen),
 * folded under a button. Made of spans, so that it fits in a paragraph.
 */
export function Margin({ text }: { text: string }) {
  const [open, setOpen] = useState(false);
  const notes = marginNotes(text);
  return (
    <span dir="ltr" className="mt-1 block text-[0.85em] text-stone">
      <button
        type="button"
        aria-expanded={open}
        title="The translators' notes on this verse"
        className="cursor-pointer hover:text-ink"
        onClick={(e) => {
          // not also selecting the verse
          e.stopPropagation();
          setOpen(!open);
        }}
      >
        {open ? "▾" : "▸"} Kanttekeningen ({notes.length})
      </button>
      {open ? (
        <span className="mt-1 block space-y-1 border-s-2 border-rule ps-2">
          {notes.map((note, i) => (
            <span key={i} className="block">
              {note.catchword ? <em className="text-ink">{note.catchword}</em> : null}
              {note.catchword ? " — " : null}
              {note.text}
            </span>
          ))}
        </span>
      ) : null}
    </span>
  );
}
