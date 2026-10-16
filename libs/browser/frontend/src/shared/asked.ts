// Searches proposed by a language model: which to show where, and the
// provenance a search keeps in the URL (`ask`): the user's request, the model
// and when, so that workspaces, links and citations say where it came from.

import type { Proposal } from "../types";

/** Where a search came from, when a model proposed it. */
export interface Asked {
  request: string;
  model: string | null;
  /** ISO time of the proposal */
  created: string;
  /** the template as proposed, to tell when it was edited */
  template: string;
}

/** The proposals still waiting, the newest first, for these corpora. */
export function waiting(proposals: Proposal[], corpora: string[]): Proposal[] {
  return proposals.filter((p) => p.state === "new" && corpora.includes(p.corpus));
}

export function askedOf(p: Proposal): Asked {
  return { request: p.request, model: p.model, created: p.created, template: p.template };
}

/** The provenance as the value of the URL parameter `ask`. */
export function encodeAsked(asked: Asked | null): string | null {
  return asked ? JSON.stringify([asked.request, asked.model, asked.created, asked.template]) : null;
}

export function decodeAsked(value: string | null): Asked | null {
  if (!value) return null;
  try {
    const [request, model, created, template] = JSON.parse(value);
    if (typeof request !== "string" || typeof created !== "string") return null;
    return {
      request,
      model: typeof model === "string" ? model : null,
      created,
      template: typeof template === "string" ? template : "",
    };
  } catch {
    return null;
  }
}

/** "Proposed by Claude, 16 October 2026, for: …", for the results and
 * citations; "(edited)" when the search's template is no longer the proposed one. */
export function describeAsked(asked: Asked, template?: string): string {
  const when = new Date(asked.created);
  const date = Number.isNaN(when.getTime())
    ? asked.created
    : when.toLocaleDateString("en-GB", { day: "numeric", month: "long", year: "numeric", timeZone: "UTC" });
  const edited = template !== undefined && template.trim() !== asked.template.trim() ? " (edited)" : "";
  return `Proposed by ${asked.model ?? "a language model"}, ${date}, for: “${asked.request}”${edited}`;
}

/** The trial's count in words: "2,601 results", "at least 10,000 results". */
export function trialCount(p: Proposal): string | null {
  if (p.count === null) return null;
  const n = p.count.toLocaleString("en-US");
  return `${p.atLeast ? "at least " : ""}${n} ${p.count === 1 && !p.atLeast ? "result" : "results"}`;
}
