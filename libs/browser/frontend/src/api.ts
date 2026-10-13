import type {
  CorpusCard,
  FeatureEntry,
  NodeInfo,
  Problem,
  RenderTree,
  ResultRow,
  SearchStatus,
  SectionItem,
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
      corpora: { id: string; name: string; title: string | null; versions: string[]; open: boolean }[];
    }>("/api/library"),

  openCorpus: (id: string) =>
    request<{ name: string; opened: boolean }>("/api/corpora", {
      method: "POST",
      body: JSON.stringify({ id }),
    }),

  align: (corpus: string, from: string, ref: string) =>
    request<{ node: number; ref: string; exact: boolean }>(
      `${corpusPath(corpus)}/align${query({ from, ref })}`,
    ),

  features: (corpus: string) =>
    request<{ features: FeatureEntry[] }>(`${corpusPath(corpus)}/features`),

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
};
