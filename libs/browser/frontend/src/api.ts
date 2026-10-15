import type {
  Agreement,
  BookSummary,
  CorpusCard,
  DiffKind,
  FeatureEntry,
  Lexeme,
  LexiconEntry,
  NodeInfo,
  NodeSummary,
  Occurrence,
  Problem,
  ReadBy,
  RenderTree,
  ResultRow,
  SearchStatus,
  SectionItem,
  WitnessPassage,
  ParallelChapter,
} from "./types";

export class ApiError extends Error {
  status: number;
  problems: Problem[];

  constructor(status: number, message: string, problems: Problem[] = []) {
    super(message);
    this.status = status;
    this.problems = problems;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { "content-type": "application/json", ...init?.headers },
  });
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    let problems: Problem[] = [];
    try {
      const body = await response.json();
      const detail = body.detail;
      if (typeof detail === "string") message = detail;
      else if (detail?.message) {
        message = detail.message;
        problems = detail.problems ?? [];
      }
    } catch {
      // keep the status text
    }
    throw new ApiError(response.status, message, problems);
  }
  return response.json() as Promise<T>;
}

function query(params: Record<string, string | number | boolean | null | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== null && value !== undefined && value !== "") search.set(key, String(value));
  }
  const text = search.toString();
  return text ? `?${text}` : "";
}

export interface RenderParams {
  mode?: "pretty" | "plain";
  fmt?: string | null;
  full?: boolean;
  hide_types?: boolean;
  extra_features?: string;
}

const corpusPath = (corpus: string) => `/api/corpora/${encodeURIComponent(corpus)}`;

export const api = {
  corpora: () => request<{ corpora: { name: string; title: string }[] }>("/api/corpora"),

  corpus: (corpus: string) => request<CorpusCard>(corpusPath(corpus)),

  library: () =>
    request<{
      enabled: boolean;
      corpora: {
        id: string;
        name: string;
        title: string | null;
        version: string;
        versions: string[];
        copies?: string[];
        open: boolean;
      }[];
    }>("/api/library"),

  openCorpus: (id: string) =>
    request<{ name: string; opened: boolean }>("/api/corpora", {
      method: "POST",
      body: JSON.stringify({ id }),
    }),

  align: (corpus: string, from: string, ref: string, view: ReadBy = "sections") =>
    request<{ node: number | null; ref: string; exact: boolean }>(
      `${corpusPath(corpus)}/align${query({ from, ref, view })}`,
    ),

  /** Which scrolls of `c` (DSS, book view) side with `a` (MT) or `b` (SP). */
  agreement: (a: string, aref: string, b: string, bref: string, c: string, cref: string) =>
    request<Agreement>(`/api/agreement${query({ a, aref, b, bref, c, cref })}`),

  books: (corpus: string) => request<{ books: BookSummary[] }>(`${corpusPath(corpus)}/books`),

  witnesses: (corpus: string, ref: string, params: RenderParams) =>
    request<WitnessPassage>(`${corpusPath(corpus)}/witnesses${query({ ref, ...params })}`),

  /** A chapter of `lead` in the corpora `texts`, as rows of verses. */
  parallel: (lead: string, ref: string, texts: string[], params: RenderParams) =>
    request<ParallelChapter>(`/api/parallel${query({ lead, ref, texts: texts.join(","), ...params })}`),

  /** A page of search results as parallel rows: each result's verse in its corpus and in `texts`. */
  searchParallel: (id: string, offset: number, limit: number, texts: string[], params: RenderParams) =>
    request<{
      search: SearchStatus;
      offset: number;
      columns: string[];
      rows: { index: number; nodes: number[]; ref: string; cells: ParallelChapter["rows"][number]["cells"] }[];
    }>(`/api/searches/${id}/parallel${query({ offset, limit, texts: texts.join(","), ...params })}`),

  /** Words that differ between a passage of `a` and the corresponding passage of `b`. */
  compare: (a: string, aref: string, b: string, bref: string, bview: ReadBy = "sections") =>
    request<{
      comparable: boolean;
      reason: string | null;
      verses: number;
      a: Record<string, DiffKind>;
      b: Record<string, DiffKind>;
      /** With `bview=books`: per marked word of `a`, the scrolls that differ there. */
      aWitnesses?: Record<string, string[]>;
    }>(`/api/compare${query({ a, aref, b, bref, bview })}`),

  features: (corpus: string) =>
    request<{ features: FeatureEntry[] }>(`${corpusPath(corpus)}/features`),

  feature: (corpus: string, name: string, sampleLimit = 200) =>
    request<{
      name: string;
      kind: string;
      value_type: string;
      description: string;
      node_types?: string[];
      unique_values?: number;
      sample_values?: { value: string | number; count: number }[];
    }>(`${corpusPath(corpus)}/features/${encodeURIComponent(name)}${query({ sample_limit: sampleLimit })}`),

  sections: (corpus: string, ref?: string) =>
    request<{ node: number | null; heading: string; children: SectionItem[] }>(
      `${corpusPath(corpus)}/sections${query({ ref })}`,
    ),

  passage: (corpus: string, ref: string, params: RenderParams) =>
    request<{
      node: number;
      ref: string;
      total: number;
      truncated: boolean;
      path: SectionItem[];
      prev: SectionItem | null;
      next: SectionItem | null;
      trees: RenderTree[];
    }>(
      `${corpusPath(corpus)}/passage${query({ ref, ...params })}`,
    ),

  nodeSummary: (corpus: string, node: number) =>
    request<NodeSummary>(`${corpusPath(corpus)}/nodes/${node}/summary`),

  nodeInfo: (corpus: string, node: number) =>
    request<NodeInfo>(`${corpusPath(corpus)}/nodes/${node}/info`),

  check: (corpus: string, template: string, signal?: AbortSignal) =>
    request<{ ok: boolean; problems: Problem[] }>(`${corpusPath(corpus)}/check`, {
      method: "POST",
      body: JSON.stringify({ template }),
      signal,
    }),

  search: (corpus: string, template: string) =>
    request<SearchStatus>(`${corpusPath(corpus)}/search`, {
      method: "POST",
      body: JSON.stringify({ template }),
    }),

  status: (id: string) => request<SearchStatus>(`/api/searches/${id}`),

  cancel: (id: string) => request<SearchStatus>(`/api/searches/${id}`, { method: "DELETE" }),

  results: (id: string, offset: number, limit: number, fmt?: string | null) =>
    request<{ search: SearchStatus; offset: number; rows: ResultRow[] }>(
      `/api/searches/${id}/results${query({ offset, limit, render: "plain", fmt })}`,
    ),

  result: (id: string, index: number, params: RenderParams) =>
    request<{ index: number; nodes: number[]; trees: RenderTree[] }>(
      `/api/searches/${id}/results/${index}${query({ ...params })}`,
    ),

  statsFeatures: (id: string, position: number) =>
    request<{ position: number; type: string | null; features: string[]; inTemplate: string[]; open: string[] }>(
      `/api/searches/${id}/stats/features${query({ position })}`,
    ),

  statsValues: (id: string, position: number, feature: string, top = 25) =>
    request<{
      nodes: number;
      type: string | null;
      feature: string;
      withoutValue: number;
      distinct: number;
      values: { value: string; count: number }[];
      other: { values: number; count: number };
    }>(`/api/searches/${id}/stats/values${query({ position, feature, top })}`),

  statsSections: (id: string, level: number) =>
    request<{ level: number; type: string; sections: (SectionItem & { count: number })[] }>(
      `/api/searches/${id}/stats/sections${query({ level })}`,
    ),

  exportUrl: (id: string, fmt?: string | null, excel = false, extra: string[] = []) => {
    const base = query({ fmt, excel: excel || null });
    const extras = extra.map((e) => `extra=${encodeURIComponent(e)}`).join("&");
    const sep = base ? "&" : "?";
    return `/api/searches/${id}/export.tsv${base}${extras ? sep + extras : ""}`;
  },

  events: (id: string) => new EventSource(`/api/searches/${id}/events`),

  lexicon: (corpus: string) =>
    request<{ features: Record<string, string>; entries: LexiconEntry[] }>(`${corpusPath(corpus)}/lexicon`),

  lexeme: (corpus: string, node: number) => request<Lexeme>(`${corpusPath(corpus)}/lexicon/${node}`),

  occurrences: (corpus: string, node: number, offset: number, limit: number, book: string | null, fmt?: string | null) =>
    request<{ total: number; offset: number; rows: Occurrence[] }>(
      `${corpusPath(corpus)}/lexicon/${node}/occurrences${query({ offset, limit, book, fmt })}`,
    ),

  occurrencesUrl: (corpus: string, node: number, book: string | null, fmt?: string | null, excel = false) =>
    `${corpusPath(corpus)}/lexicon/${node}/occurrences.tsv${query({ book, fmt, excel: excel || null })}`,
};
