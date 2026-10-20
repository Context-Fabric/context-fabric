import type { CorpusCard, NodeInfo } from "../types";

// Searching as on Parabible: search terms are made from words (in the word
// panel), each with the features that should count, and are turned into a
// search template with a range (all terms in one verse, clause, ...) and,
// optionally, a choice of books.

export interface TermFeature {
  feature: string;
  label: string; // as in the word's summary: lexeme, part of speech, ...
  value: string; // the feature's value, as it goes into the template
  shown: string; // the value in words
  on: boolean;
}

export interface Term {
  id: string;
  corpus: string;
  type: string; // the node type, e.g. word
  text: string | null; // the word it was made from
  features: TermFeature[];
}

/** Whether a value can go into a template as it is (no spaces, no alternatives). */
function usable(value: string): boolean {
  return value !== "" && !/[\s|]/.test(value);
}

/** A search term from a word, with its lexeme (or else its first essential) switched on. */
export function termFrom(corpus: string, info: NodeInfo): Term {
  const features: TermFeature[] = [];
  for (const item of info.summary) {
    const raw = info.features[item.feature];
    if (raw === undefined || features.some((f) => f.feature === item.feature)) continue;
    const value = String(raw);
    if (!usable(value)) continue;
    features.push({ feature: item.feature, label: item.label, value, shown: item.value, on: false });
  }
  // the transliterated lexeme tells homonyms apart (BR>[ and BR>=[); else the lexeme
  const first =
    features.find((f) => f.label === "transliteration") ?? features.find((f) => f.label === "lexeme") ?? features[0];
  if (first) first.on = true;
  return { id: `${corpus}-${info.node}-${Date.now()}`, corpus, type: info.type, text: info.text, features };
}

/** The ranges a search can have in a corpus: its lowest section (verse) and the
 * linguistic units that it has (sentence, clause, phrase). */
export function rangesOf(card: CorpusCard): string[] {
  const types = new Set(card.nodeTypes.map((t) => t.type));
  const unit = card.sectionTypes[card.sectionTypes.length - 1];
  return [unit, ...["sentence", "clause", "phrase"].filter((t) => types.has(t))].filter((t): t is string => Boolean(t));
}

/** Whether books can be chosen: the corpus' top sections are books with a `book` feature. */
export function hasBooks(card: CorpusCard): boolean {
  return card.sectionTypes[0] === "book";
}

/** The search template for the terms of one corpus. */
export function buildTemplate(
  terms: Term[],
  range: string,
  books: { feature: string; values: string[] } | null,
): string {
  const lines: string[] = [];
  let indent = "";
  if (books?.values.length) {
    lines.push(`book ${books.feature}=${books.values.join("|")}`);
    indent = "  ";
  }
  lines.push(`${indent}${range}`);
  indent += "  ";
  for (const term of terms) {
    const constraints = term.features.filter((f) => f.on).map((f) => `${f.feature}=${f.value}`);
    lines.push(`${indent}${[term.type, ...constraints].join(" ")}`);
  }
  return lines.join("\n");
}
