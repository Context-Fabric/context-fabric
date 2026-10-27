// Which lexemes the lexicon's list shows, and in what order.

import type { LexiconEntry } from "../types";

/** alphabetical; most frequent first; by root, as BDB orders its entries */
export type Order = "alpha" | "frequency" | "root";

/** Hebrew without vowels and accents, so that ברא also finds בָּרָא. */
export const bare = (text: string | null | undefined) => (text ?? "").normalize("NFD").replace(/[֑-ׇ]/g, "");

/** The letter a lexeme is listed under: by root, the letter of its root in
 * BDB (מִשְׁפָּט under ש); else its own first letter. */
export function letterOf(e: LexiconEntry, order: Order): string | null {
  return order === "root" ? (e.bdbLetter ?? e.letter) : e.letter;
}

export interface Shown {
  language: string;
  letter: string;
  sp: string;
  /** what is typed in the find box; finds in all letters at once */
  find: string;
  order: Order;
}

export function shownLexemes(entries: LexiconEntry[], { language, letter, sp, find, order }: Shown): LexiconEntry[] {
  const needle = find.trim().toLowerCase();
  const hebrew = bare(needle);
  let list = entries.filter((e) => {
    if ((e.language ?? "") !== language) return false;
    if (sp && e.sp !== sp) return false;
    if (!needle) return letterOf(e, order) === letter;
    return (
      (e.gloss ?? "").toLowerCase().includes(needle) ||
      (e.bdbShort ?? "").toLowerCase().includes(needle) ||
      e.lex.toLowerCase().includes(needle) ||
      (hebrew !== "" && (bare(e.form).includes(hebrew) || bare(e.cons).includes(hebrew)))
    );
  });
  if (order === "frequency") list = [...list].sort((a, b) => b.freq - a.freq);
  // lexemes without an entry at the end, in their alphabetical order
  if (order === "root") list = [...list].sort((a, b) => (a.bdbOrder ?? Infinity) - (b.bdbOrder ?? Infinity));
  return list;
}
