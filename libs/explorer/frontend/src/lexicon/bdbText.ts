// Helpers for showing BDB's entries (`Bdb.tsx`): its abbreviations, the link
// to Sefaria, and how sure the link from a lexeme is.

import type { BdbLink } from "../types";

/** The entry on Sefaria, from its reference (`BDB, בָּרָא`). */
export function sefariaUrl(ref: string): string {
  return `https://www.sefaria.org/${encodeURIComponent(ref.replace(/ /g, "_"))}`;
}

/** A pattern that finds BDB's abbreviations in text: whole words only, the
 * longest first, and none of one letter (too many false hits). */
export function abbreviationPattern(abbreviations: Record<string, string>): RegExp | null {
  const keys = Object.keys(abbreviations)
    .filter((k) => k.length > 1)
    .sort((a, b) => b.length - a.length)
    .map((k) => k.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  if (keys.length === 0) return null;
  return new RegExp(`(?<![\\p{L}\\p{N}])(${keys.join("|")})(?![\\p{L}\\p{N}])`, "gu");
}

export type Piece = string | { abbr: string; meaning: string };

/** Text cut into plain pieces and abbreviations with their meaning. */
export function splitAbbreviations(text: string, pattern: RegExp | null, abbreviations: Record<string, string>): Piece[] {
  if (!pattern) return [text];
  const pieces: Piece[] = [];
  let last = 0;
  for (const m of text.matchAll(pattern)) {
    const at = m.index ?? 0;
    if (at > last) pieces.push(text.slice(last, at));
    pieces.push({ abbr: m[0], meaning: abbreviations[m[0]] });
    last = at + m[0].length;
  }
  if (last < text.length) pieces.push(text.slice(last));
  return pieces;
}

/** A note on how a lexeme was linked, when it is worth saying. */
export function linkNote(link: BdbLink): string | null {
  if (link.how === "manual") return "Linked by hand.";
  if (link.how === "cited") return "Linked by the verses BDB cites for it.";
  if (link.share != null && link.share < 0.9) {
    return `Linked through the OSHB's lemmas; ${Math.round(link.share * 100)}% of its occurrences lead to this entry.`;
  }
  return null;
}

/** An entry this long (in characters of HTML) is shown with its stems folded. */
export const LONG = 3000;

/** The parts of an entry, divided where Sefaria divides its stems (`<br>`). */
export function stems(html: string): string[] {
  return html.split(/<br\s*\/?>/i).filter((part) => part.trim() !== "");
}

const ENTITIES: Record<string, string> = { amp: "&", lt: "<", gt: ">", quot: '"', "#39": "'" };

/** The first words of a part, as the heading of its fold. */
export function preview(html: string, length = 70): string {
  const text = html
    .replace(/<[^>]+>/g, "")
    .replace(/&(amp|lt|gt|quot|#39);/g, (_, e: string) => ENTITIES[e])
    .replace(/\s+/g, " ")
    .trim();
  return text.length > length ? `${text.slice(0, length).trimEnd()}…` : text;
}
