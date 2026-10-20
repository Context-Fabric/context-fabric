import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import type { CorpusCard } from "../types";
import { REFERENCE_IDS } from "../ui";

// The open corpora, the reference corpus (the BHSA) and where to start reading:
// shared by the interfaces that follow one passage across corpora.

/** The open corpora and their cards. */
export function useCards() {
  const corpora = useQuery({ queryKey: ["corpora"], queryFn: api.corpora });
  const names = corpora.data?.corpora.map((c) => c.name) ?? [];
  const cards = useQuery({
    queryKey: ["corpus-cards", ...names],
    queryFn: async () => {
      const entries = await Promise.allSettled(names.map((n) => api.corpus(n)));
      const result: Record<string, CorpusCard> = {};
      entries.forEach((e, i) => {
        if (e.status === "fulfilled") result[names[i]] = e.value;
      });
      return result;
    },
    enabled: names.length > 0,
    placeholderData: (previous) => previous,
  });
  return {
    corpora,
    names,
    cards: cards.data ?? {},
    ready: corpora.data !== undefined && (!names.length || !!cards.data),
  };
}

/** The corpus whose verse numbering the rows follow: the BHSA, opened in the
 * background when it is on this computer but not open. Otherwise null. */
export function useReference(
  cards: Record<string, CorpusCard>,
  ready: boolean,
): { name: string | null; settled: boolean } {
  const client = useQueryClient();
  const open = Object.values(cards).find((c) => REFERENCE_IDS.includes(c.id?.toLowerCase() ?? ""));
  const library = useQuery({ queryKey: ["library"], queryFn: api.library, enabled: ready && !open });
  const tried = useRef(false);
  const [failed, setFailed] = useState(false);
  const entry = library.data?.corpora.find((c) => REFERENCE_IDS.includes(c.id.toLowerCase()));

  useEffect(() => {
    if (!ready || open || !entry || tried.current) return;
    tried.current = true;
    api
      .openCorpus(entry.id)
      .then(() => client.invalidateQueries({ queryKey: ["corpora"] }))
      .catch(() => setFailed(true));
  }, [ready, open, entry, client]);

  if (open) return { name: open.name, settled: true };
  const settled =
    ready && (library.isError || (library.data !== undefined && (!entry || !library.data.enabled)) || failed);
  return { name: null, settled };
}

/** The first chapter of the first book, to start with. */
export async function firstChapter(corpus: string): Promise<string | null> {
  const books = await api.sections(corpus);
  const first = books.children[0];
  if (!first) return null;
  const chapters = await api.sections(corpus, first.ref);
  return chapters.children[0]?.ref ?? first.ref;
}
