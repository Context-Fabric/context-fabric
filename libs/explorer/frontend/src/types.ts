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
  /** `org/repo` when the corpus is in Text-Fabric layout (`ETCBC/bhsa`). */
  id: string | null;
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
  /** For a translation (JPS 1917): its licence, source and how to cite it. */
  translation: TranslationInfo | null;
  display: DisplayConfig;
}

export interface TranslationInfo {
  id: string;
  licence: string;
  licenceNote: string;
  source: string;
  cite: string;
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
  /** with BDB: its main entry's short definition, place in BDB, the letter
   * of its root's part, and whether it is a root */
  bdbShort?: string;
  bdbOrder?: number;
  bdbLetter?: string | null;
  bdbRoot?: boolean;
}

export interface Lexeme extends LexiconEntry {
  verses: number;
  books: { book: string; count: number }[];
  /** Its entries in BDB, the main one first; null when BDB hasn't been added
   * (missing from servers of before BDB). */
  bdb?: BdbLink | null;
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
  /** The feature that names it and its own value (`book`, `Jesaia`), for search templates. */
  feature?: string;
  value?: string;
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
  /** The essentials: gloss, lexeme, part of speech, morphology, in words. */
  summary: SummaryItem[];
  /** a word's entries in BDB (see NodeSummary) */
  bdb?: WordBdb | null;
  features: Record<string, string | number>;
  edges: Record<string, { from: { node: number; value?: unknown }[]; to: { node: number; value?: unknown }[] }>;
  up: { node: number; type: string }[];
  down: { node: number; type: string }[];
  downCounts: Record<string, number>;
}

export interface SummaryItem {
  label: string;
  value: string;
  /** The feature it comes from; `gloss in bhsa` for a gloss from another corpus. */
  feature: string;
}

/** The corpus a borrowed gloss comes from (the DSS has its glosses from the BHSA). */
export function borrowedFrom(item: SummaryItem): string | null {
  return item.feature.startsWith("gloss in ") ? item.feature.slice("gloss in ".length) : null;
}

/** What the card shown when hovering over a word needs. */
export interface NodeSummary {
  node: number;
  type: string;
  ref: string;
  text: string | null;
  summary: SummaryItem[];
  /** a word's entries in BDB, with its lexeme in the corpus whose lexicon
   * has it (the BHSA, also for the DSS) */
  bdb?: WordBdb | null;
}

export interface WordBdb extends BdbLink {
  corpus: string;
  node: number;
  lex: string;
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

/** A chapter in several corpora, verse by verse (the parallel interface). */
export interface ParallelChapter {
  ref: string;
  path: SectionItem[];
  prev: SectionItem | null;
  next: SectionItem | null;
  /** Per corpus: its chapter, or null when it has no counterpart. */
  columns: { name: string; kind: "text" | "witnesses"; ref: string | null }[];
  /** A row per verse number in any of the corpora; a cell only where a corpus has the verse. */
  rows: {
    verse: number;
    ref: string;
    cells: Record<
      string,
      {
        trees?: RenderTree[];
        witnesses?: Witness[];
        /** A translation: a note printed after the verse (Malachi 3:24). */
        note?: string;
        /** A translation: the verse's number in the print, where it divides differently. */
        printed?: string;
      }
    >;
  }[];
}

/** A passage word by word, for the interlinear (`/interlinear`). */
export interface InterlinearPassage {
  ref: string;
  path: SectionItem[];
  prev: SectionItem | null;
  next: SectionItem | null;
  truncated: boolean;
  verses: InterlinearVerse[];
}

export interface InterlinearVerse {
  verse: number | string;
  ref: string;
  words: {
    node: number;
    text: string;
    /** translit, lexeme, gloss, pos, morph, language: in words. */
    lines: Record<string, string>;
    features: Record<string, string>;
    /** Indexes in `phrases` and `clauses` (-1: in none). */
    phrase: number;
    clause: number;
  }[];
  phrases: { node: number; label: string | null }[];
  clauses: { node: number; label: string | null }[];
  /** Another corpus' verse under this one, if asked for and it has the verse. */
  under?: {
    corpus: string;
    ref: string;
    words: { node: number; text: string }[];
    /** Per word of this verse, the nodes of the words under it. */
    under: number[][];
    marks: Record<string, DiffKind>;
    marksHere: Record<string, DiffKind>;
    /** A translation: the verse as a line (`text`), not word by word. */
    translation?: boolean;
    text?: string;
    note?: string | null;
  } | null;
}

/** A word in a concordance line. */
export interface KwicWord {
  node: number;
  text: string;
  features: Record<string, string>;
}

/** A search result as a concordance line (`/kwic`). */
export interface KwicLine {
  index: number;
  ref: string | null;
  left: KwicWord[];
  hit: KwicWord[];
  right: KwicWord[];
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

/** A translation that can be added from the library (`/api/translations`). */
export interface Translation {
  id: string;
  title: string;
  name: string;
  language: string;
  /** the library id of its corpus, once added */
  corpus: string;
  licence: string;
  licenceNote: string;
  source: string;
  added: boolean;
  open: boolean;
  /** removed when the browser starts again, because it is open */
  removing: boolean;
  job: {
    state: "running" | "done" | "failed";
    step: "download" | "build";
    done: number;
    total: number;
    error: string | null;
  } | null;
}

/** A corpus that can be downloaded from its own release (`/api/downloads`). */
export interface CorpusDownload {
  /** org/repo, as in the library */
  id: string;
  name: string;
  title: string;
  version: string;
  /** bytes to download, and on disk */
  size: number;
  unpacked: number;
  /** where its terms of use are */
  terms: string;
  /** the licence of the data, where it is known (CC BY-NC 4.0) */
  licence: string | null;
  present: boolean;
  job: {
    state: "running" | "done" | "failed" | "cancelled";
    step: "download" | "unpack" | "prepare";
    done: number;
    total: number;
    error: string | null;
  } | null;
}

/** An entry of BDB in a list: the lexeme page, the root family. */
export interface BdbSummary {
  id: string;
  lexicon: "heb" | "arc";
  headword: string;
  pos: string | null;
  /** Open Scriptures' short definitions */
  defs: string[];
  /** a short definition: `defs`, else the first gloss in the text */
  short: string;
  /** Sefaria's text stops short, or Sefaria lacks the entry */
  incomplete: boolean;
  /** Sefaria's reference (`BDB, בָּרָא`), null for an entry Sefaria lacks */
  ref: string | null;
}

/** How a lexeme is linked to BDB. */
export interface BdbLink {
  entries: BdbSummary[];
  /** oshb: through the OSHB's lemmas; cited: BDB cites its verses; manual: by hand */
  how: "oshb" | "cited" | "manual" | null;
  /** the share of its occurrences that lead to the main entry */
  share?: number | null;
}

/** A lexeme of the corpus linked to an entry. */
export interface BdbLexeme {
  node: number;
  lex: string;
  form: string | null;
  gloss: string | null;
  language: string | null;
}

/** An entry of BDB with its text (`/api/dictionaries/bdb/entries/{id}`). */
export interface BdbArticle extends BdbSummary {
  /** cleaned HTML: big, sub, sup, strong, em, br, span[dir], a[data-osis], a[data-entry] */
  html: string;
  root: boolean;
  strong: string[];
  twot: string[];
  previous: { id: string; headword: string } | null;
  next: { id: string; headword: string } | null;
  /** the root and the words BDB derives from it, with their lexemes */
  family: (BdbSummary & { lexemes: BdbLexeme[] })[];
  lexemes: BdbLexeme[];
  /** the cited verses (`Gen.1.1`) as references in the corpus (`Genesis 1:1`) */
  passages: Record<string, string>;
}

/** A dictionary that can be added (`/api/dictionaries`). */
export interface DictionaryInfo {
  id: string;
  title: string;
  name: string;
  licence: string;
  licenceNote: string;
  sources: string;
  corpus: string;
  size: number;
  /** can be downloaded (not before it has been published) */
  available: boolean;
  added: boolean;
  built: string | null;
  job: {
    state: "running" | "done" | "failed" | "cancelled";
    step: string;
    done: number;
    total: number;
    error: string | null;
  } | null;
  abbreviations?: Record<string, string>;
}

/** A search proposed by a language model (`/api/proposals`). */
export interface Proposal {
  id: string;
  corpus: string;
  template: string;
  /** the user's request in their own words */
  request: string;
  explanation: string;
  /** which model proposed it, as its app says */
  model: string | null;
  created: string;
  /** the model's trial: how many results, and whether more */
  count: number | null;
  atLeast: boolean;
  state: "new" | "run" | "dismissed";
}

/** What the server says about itself (`/api/health`). */
export interface Health {
  ok: boolean;
  searchReady: boolean;
  version: string;
  coreVersion: string | null;
  /** the interface to start in (`--ui`) */
  ui?: string;
  /** interfaces left out of the menu until the user chooses otherwise */
  hiddenUis?: string[];
  /** the standalone program offers the missing corpora at the start */
  askDownloads?: boolean;
}
