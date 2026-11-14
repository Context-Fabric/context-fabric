/** One of the translators' notes on a verse: the words it is about, and the note. */
export interface MarginNote {
  catchword: string | null;
  text: string;
}

/**
 * The translators' notes on a verse (feature `margin`, the Statenvertaling's
 * kanttekeningen): " | " between notes, each its catchword, " — " and the note.
 */
export function marginNotes(margin: string): MarginNote[] {
  return margin
    .split(" | ")
    .filter((note) => note.trim())
    .map((note) => {
      const at = note.indexOf(" — ");
      return at > 0 ? { catchword: note.slice(0, at), text: note.slice(at + 3) } : { catchword: null, text: note };
    });
}
