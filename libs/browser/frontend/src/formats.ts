// The text formats the display menu offers. The server sends only the formats
// the browser shows (`cfabric_browser/formats.py`): the BHSA's vocalized and
// unvocalized text, and the full text of the other corpora.

export const FORMAT_NAMES: Record<string, string> = {
  "text-orig-full": "Vocalized",
  "text-orig-plain": "Unvocalized",
};

export const FORMAT_NOTE =
  "Vocalized: with vowels and accents, and the qere where it differs from the ketiv. Unvocalized: the consonants of the ketiv.";

/** The menu's options, or none when there is no choice. */
export function formatOptions(formats: string[]): { value: string; label: string }[] {
  if (formats.length < 2) return [];
  return formats.map((f) => ({ value: f, label: FORMAT_NAMES[f] ?? f }));
}

/** The format shown: the chosen one if the corpus offers it (a stored choice
 * may be one that is no longer offered), else the default. */
export function shownFormat(chosen: string | null, formats: string[], defaultFormat: string | null): string | null {
  return chosen !== null && formats.includes(chosen) ? chosen : defaultFormat;
}
