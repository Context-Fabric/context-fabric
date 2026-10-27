import { useQueries, useQuery } from "@tanstack/react-query";
import { api } from "./api";
import type { Pane } from "./state";
import type { Agreement, DiffKind, Place } from "./types";

// Differences between the texts being read: shared by both interfaces. The
// parallel interface describes its columns as reading panes.

/** What a reading pane shows about differences with other panes. */
export interface PaneDiff {
  marks?: Map<number, DiffKind>;
  /** Per marked word: what it differs from, for a tooltip. */
  titles?: Map<number, string>;
  message: string;
  /** In a DSS pane next to MT and SP: per verse, the scrolls at each place where MT and SP differ. */
  places?: { a: string; b: string; verses: Map<number, Place[]> };
}

/**
 * With MT, SP and a DSS pane in book view open: which scrolls side with MT and
 * which with SP, at each place where MT and SP differ. The first two reading
 * panes not in book view count as MT and SP, the first in book view as the DSS.
 */
function useAgreement(panes: Pane[], on: boolean) {
  const open = (p: Pane) => p.view === "read" && p.corpus && p.ref;
  const plain = panes.map((p, i) => (open(p) && p.by !== "books" ? i : -1)).filter((i) => i >= 0);
  const scrolls = panes.findIndex((p) => open(p) && p.by === "books");
  const trio = on && plain.length >= 2 && scrolls >= 0 ? { a: plain[0], b: plain[1], c: scrolls } : null;
  const query = useQuery({
    queryKey: ["agreement", ...(trio ? [trio.a, trio.b, trio.c].flatMap((i) => [panes[i].corpus, panes[i].ref]) : [])],
    queryFn: () => {
      const [a, b, c] = [panes[trio!.a], panes[trio!.b], panes[trio!.c]];
      return api.agreement(a.corpus!, a.ref!, b.corpus!, b.ref!, c.corpus!, c.ref!);
    },
    enabled: trio !== null,
    staleTime: Infinity,
  });
  return trio && query.data?.comparable ? { ...trio, data: query.data } : null;
}

/** A scroll's agreement for people: `4Q7 = sp`, `4Q10 other (מארות)`. */
function agreementLabel(w: Place["witnesses"][number], a: string, b: string): string {
  if (w.agrees === "a") return `${w.scroll} = ${a}`;
  if (w.agrees === "b") return `${w.scroll} = ${b}`;
  return `${w.scroll} other (${w.text || "—"})`;
}

/** Add tooltips and per-verse places from the scrolls' agreement (see useAgreement). */
function addAgreement(
  diffs: (PaneDiff | null)[],
  panes: Pane[],
  { a, b, c, data }: { a: number; b: number; c: number; data: Agreement },
) {
  const [aName, bName] = [panes[a].corpus!, panes[b].corpus!];
  const titles = [new Map<number, string>(), new Map<number, string>(), new Map<number, string>()];
  const verses = new Map<number, Place[]>();
  for (const verse of data.verses) {
    verses.set(verse.verse, verse.units);
    for (const place of verse.units) {
      const where = `${aName} ${place.a || "—"} | ${bName} ${place.b || "—"}`;
      const title = `${where}: ${place.witnesses.map((w) => agreementLabel(w, aName, bName)).join(", ")}`;
      place.aWords.forEach((n) => titles[0].set(n, title));
      place.bWords.forEach((n) => titles[1].set(n, title));
      for (const w of place.witnesses) {
        const side = w.agrees === "a" ? aName : w.agrees === "b" ? bName : "neither";
        w.words.forEach((n) => titles[2].set(n, `Agrees with ${side} (${where})`));
      }
    }
  }
  [a, b, c].forEach((i, k) => {
    const merged = new Map(diffs[i]?.titles);
    for (const [n, t] of titles[k]) merged.set(n, merged.has(n) ? `${merged.get(n)}\n${t}` : t);
    diffs[i] = { message: "", ...diffs[i], titles: merged };
  });
  diffs[c] = { ...diffs[c]!, places: { a: aName, b: bName, verses } };
}

/**
 * Differences between the texts of reading panes: the first reading pane is
 * compared with each of the others. It marks the words that differ from any
 * of them; each other pane marks how it differs from the first. A pane that
 * reads the DSS by biblical book is compared scroll by scroll, and cannot be
 * the first. Translations are not compared.
 */
export function useDifferences(allPanes: Pane[], on: boolean): (PaneDiff | null)[] {
  // translations are left out quietly: their words cannot be compared with the Hebrew
  const corpora = useQuery({ queryKey: ["corpora"], queryFn: api.corpora });
  const translations = new Set(corpora.data?.corpora.filter((c) => c.translation).map((c) => c.name));
  const panes = allPanes.map((p) => (p.corpus && translations.has(p.corpus) ? { ...p, corpus: null } : p));
  const agreement = useAgreement(panes, on);
  const reading = panes.map((p, i) => (p.view === "read" && p.corpus ? i : -1)).filter((i) => i >= 0);
  const base = reading.find((i) => panes[i].by !== "books") ?? -1;
  const others = on && base >= 0 ? reading.filter((i) => i !== base) : [];
  const ready = (i: number) => panes[base].ref !== null && panes[i].ref !== null;
  const results = useQueries({
    queries: others.map((i) => ({
      queryKey: ["compare", panes[base].corpus, panes[base].ref, panes[i].corpus, panes[i].ref, panes[i].by],
      queryFn: () => api.compare(panes[base].corpus!, panes[base].ref!, panes[i].corpus!, panes[i].ref!, panes[i].by),
      enabled: ready(i),
      staleTime: Infinity,
    })),
  });

  const diffs: (PaneDiff | null)[] = panes.map(() => null);
  if (!others.length) return diffs;
  const baseMarks = new Map<number, DiffKind>();
  const differsIn = new Map<number, string[]>();
  const compared: string[] = [];
  others.forEach((i, k) => {
    const { data, error } = results[k];
    const name = panes[base].corpus;
    if (!ready(i)) {
      diffs[i] = { message: "Open a passage to compare it." };
    } else if (error) {
      diffs[i] = { message: `Could not compare with ${name}: ${(error as Error).message}` };
    } else if (!data) {
      diffs[i] = { message: `Comparing with ${name}…` };
    } else if (!data.comparable) {
      diffs[i] = { message: `Not compared with ${name}: ${data.reason}` };
    } else {
      const marks = new Map(Object.entries(data.b).map(([n, kind]) => [Number(n), kind]));
      for (const [n, kind] of Object.entries(data.a)) {
        if (baseMarks.get(Number(n)) !== "variant") baseMarks.set(Number(n), kind);
        const scrolls = data.aWitnesses?.[n];
        const where = scrolls ? `${panes[i].corpus} ${scrolls.join(", ")}` : panes[i].corpus!;
        differsIn.set(Number(n), [...(differsIn.get(Number(n)) ?? []), where]);
      }
      compared.push(panes[i].corpus!);
      diffs[i] = { marks, message: `${marks.size} words differ from ${name}` };
    }
  });
  diffs[base] = compared.length
    ? {
        marks: baseMarks,
        titles: new Map([...differsIn].map(([n, where]) => [n, `Differs in ${where.join("; ")}`])),
        message: `${baseMarks.size} words differ from ${compared.join(", ")}`,
      }
    : null;
  if (agreement) addAgreement(diffs, panes, agreement);
  return diffs;
}
