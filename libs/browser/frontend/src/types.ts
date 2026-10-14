// Shapes of the cfabric-browser API (see cfabric_browser/api.py and cfabric.display).

export type Edge = "node" | "gap" | "split";

export interface FeatureValue {
  name: string;
  value: string | number;
  kind: "bare" | "standard" | "extra";
  on?: string;
  extra?: boolean;
}

export interface Fragment {
  node: number;
  type: string;
  range: [number, number];
  start: Edge;
  end: Edge;
  label?: string;
  heading?: string;
  text?: string;
  features?: FeatureValue[];
  highlight?: string;
  base?: boolean;
  /** Reconstructed by a modern editor (DSS); shown dimmed. */
  dim?: boolean;
  children?: Fragment[];
}

export interface RenderTree {
  node: number;
  type: string;
  mode: "pretty" | "plain";
  fmt: string;
  direction: "rtl" | "ltr";
  heading: string;
  fragments: Fragment[];
}

export interface TypeDisplay {
  type: string;
  level: number;
  flow: "hor" | "ver";
  wrap: boolean;
  stretch: boolean;
  label: string | boolean;
  template: string | boolean;
  features: string[];
  featuresBare: string[];
  hidden: boolean;
  isBig: boolean;
  isLex: boolean;
}

export interface DisplayConfig {
  slotType: string;
  nodeTypes: string[];
  types: Record<string, TypeDisplay>;
  sectionTypes: string[];
  textFormats: string[];
  defaultFormat: string;
  formatStyles: Record<string, string | null>;
  language: string;
  languageCode: string;
  direction: "rtl" | "ltr";
  baseTypes: string[];
  condenseType: string | null;
  hiddenTypes: string[];
  excludedFeatures: string[];
  noneValues: string[];
  exampleSection: string | null;
}

export interface CorpusCard {
  name: string;
  title: string;
  version: string;
  doi: string | null;
  slotType: string;
  maxSlot: number;
  maxNode: number;
  nodeTypes: { type: string; count: number; isSlot: boolean }[];
  sectionTypes: string[];
  /** Whether the corpus can also be read by biblical book (DSS). */
  bookView: boolean;
  /** Whether the corpus has a lexicon with glosses (BHSA). */
  lexicon: boolean;
  display: DisplayConfig;
}

/** A lexeme in the lexicon; fields the corpus lacks are missing or null. */
export interface LexiconEntry {
  node: number;
  lex: string; // transliteration, e.g. BR>[
  form?: string | null; // vocalised
  cons?: string | null; // consonants
  letter: string | null; // the first letter
  gloss?: string | null;
  sp?: string | null; // part of speech
  language?: string | null;
  freq: number;
  rank?: number | null;
  nametype?: string | null;
}

export interface Lexeme extends LexiconEntry {
  verses: number;
  books: { book: string; count: number }[];
}

/** A verse a lexeme occurs in, as running text with its words marked. */
export interface Occurrence {
  node: number;
  ref: string;
  words: number[];
  tree: RenderTree;
}

export interface SectionItem {
  node: number;
  type: string;
  label: string;
  ref: string;
}

export interface Problem {
  kind: "syntax" | "semantics" | "error" | "timeout";
  line: number | null;
  message: string;
  feature?: string;
  nodeType?: string;
  quantifier?: string;
}

export type SearchState = "queued" | "running" | "done" | "failed" | "timeout" | "cancelled";

export interface SearchStatus {
  id: string;
  corpus: string;
  template: string;
  state: SearchState;
  count: number;
  truncated: boolean;
  sorted: boolean;
  limit: number;
  timeout: number;
  elapsed: number;
  problems: Problem[];
  version: number;
}

export interface ResultRow {
  index: number;
  nodes: number[];
  types: string[];
  ref: string;
  plain?: RenderTree[];
}

export interface NodeInfo {
  node: number;
  type: string;
  ref: string;
  slots: [number, number];
  slotCount: number;
  text: string | null;
  features: Record<string, string | number>;
  edges: Record<string, { from: { node: number; value?: unknown }[]; to: { node: number; value?: unknown }[] }>;
  up: { node: number; type: string }[];
  down: { node: number; type: string }[];
  downCounts: Record<string, number>;
}

export interface FeatureEntry {
  name: string;
  kind: "node" | "edge";
  value_type: string;
  description: string;
  excluded: boolean;
}

// Display settings chosen in the interface.
export interface ViewSettings {
  fmt: string | null;
  hideTypes: boolean;
  standardFeatures: boolean;
  showNodes: boolean;
  showTypes: boolean;
}

/** How a word differs from its counterpart in another corpus: only in matres lectionis, or otherwise. */
export type DiffKind = "matres" | "variant";

/** How a reading pane goes through a corpus: by its own sections, or by biblical book (DSS). */
export type ReadBy = "sections" | "books";

export interface BookSummary {
  book: string;
  label: string;
  ref: string;
  scrolls: number;
  verses: number;
  chapters: { chapter: number; ref: string; verses: number; witnesses: number }[];
}

/** The text of one verse in one scroll. */
export interface Witness {
  scroll: string;
  place: string;
  ref: string | null; // its fragment in scroll view
  words: number[];
  alsoNonBiblical: boolean;
  tree: RenderTree;
}

export interface WitnessPassage {
  ref: string;
  book: string;
  chapter: number;
  chapterRef: string;
  verses: { verse: number; ref: string; witnesses: Witness[] }[];
  prev: string | null;
  next: string | null;
}

/** A scroll at a place where two texts (MT, SP) differ: which one it agrees with. */
export interface PlaceWitness {
  scroll: string;
  agrees: "a" | "b" | "other";
  reading: string; // its letters there
  words: number[];
  text: string; // its words there
}

/** A place where two texts differ, with what the scrolls read there. */
export interface Place {
  kind: DiffKind;
  a: string; // the words of the first text there
  b: string;
  aWords: number[];
  bWords: number[];
  witnesses: PlaceWitness[];
}

export interface Agreement {
  comparable: boolean;
  reason: string | null;
  verses: { verse: number; ref: string; units: Place[] }[];
}
