import { queryClient } from "./queryClient";
import { corporaQuery, corpusCard } from "./shared/corpora";
import {
  commonToConcordance,
  commonToInterlinear,
  commonToParallel,
  commonToReader,
  commonToWorkspace,
  concordanceToCommon,
  interlinearToCommon,
  mergeCommon,
  readerToCommon,
  workspaceToCommon,
  type Common,
} from "./common";
import { serializeConcordance, type Concordance } from "./concordance/url";
import { serializeInterlinear, type Interlinear } from "./interlinear/url";
import { serializeParallel } from "./parallel/state";
import { serializeReader, type Reader } from "./reader/url";
import { serialize, type Workspace } from "./state";
import type { CorpusCard } from "./types";
import { alignRef, navigate, REFERENCE_IDS, rememberUi, type Ui } from "./ui";

// Switching interfaces: the current interface describes what it shows (see
// common.ts), that is added to what was kept from earlier in the session, and
// the target interface is opened with the result.

const KEPT = "cfabric-browser/common";

function kept(): Common | null {
  try {
    const raw = window.sessionStorage.getItem(KEPT);
    return raw ? (JSON.parse(raw) as Common) : null;
  } catch {
    return null;
  }
}

function keep(common: Common): void {
  try {
    window.sessionStorage.setItem(KEPT, JSON.stringify(common));
  } catch {
    // not kept; switching still works, only without what the last interface could not show
  }
}

/** The cards of the open corpora, and the reference corpus (the BHSA) among them. */
async function openCards(): Promise<{ cards: CorpusCard[]; reference: string | null }> {
  const { corpora } = await queryClient.fetchQuery(corporaQuery);
  const settled = await Promise.allSettled(corpora.map((c) => corpusCard(c.name)));
  const cards = settled.flatMap((s) => (s.status === "fulfilled" ? [s.value] : []));
  const reference = cards.find((c) => REFERENCE_IDS.includes(c.id?.toLowerCase() ?? ""))?.name ?? null;
  return { cards, reference };
}

/** What the panes show, with their passage in the reference corpus' numbering. */
export async function describeWorkspace(ws: Workspace): Promise<Partial<Common>> {
  const source = ws.panes.find((p) => p.view === "read" && p.corpus && p.ref);
  let aligned: string | null = null;
  if (source) {
    const { reference } = await openCards();
    aligned = reference ? await alignRef(source.corpus!, source.ref!, reference) : source.ref;
  }
  return workspaceToCommon(ws, aligned);
}

/** What the Reader shows, with its chapter in the reference corpus' numbering. */
export async function describeReader(r: Reader): Promise<Partial<Common>> {
  const { cards, reference } = await openCards();
  let aligned: string | null = null;
  if (r.corpus && r.ref) aligned = reference ? await alignRef(r.corpus, r.ref, reference) : r.ref;
  return readerToCommon(
    r,
    aligned ? aligned.split(":")[0] : null,
    cards.map((c) => c.name),
  );
}

/** What the Interlinear shows, with its chapter in the reference corpus' numbering. */
export async function describeInterlinear(s: Interlinear): Promise<Partial<Common>> {
  const { cards, reference } = await openCards();
  let aligned: string | null = null;
  if (s.corpus && s.ref) aligned = reference ? await alignRef(s.corpus, s.ref, reference) : s.ref;
  return interlinearToCommon(
    s,
    aligned ? aligned.split(":")[0] : null,
    cards.map((c) => c.name),
  );
}

/** What the Concordance shows. */
export async function describeConcordance(s: Concordance): Promise<Partial<Common>> {
  const { cards } = await openCards();
  return concordanceToCommon(
    s,
    cards.map((c) => c.name),
  );
}

/** Open interface `target` with what the current one shows (`current`). */
export async function switchInterface(target: Ui, current: Partial<Common>): Promise<void> {
  const common = mergeCommon(kept(), current);
  keep(common);
  rememberUi(target);
  if (target === "classic") {
    const { cards, reference } = await openCards();
    // the passage is in the reference corpus' numbering, or else the first corpus'
    const lead = reference ?? common.corpora[0];
    const refs: Record<string, string | null> = {};
    await Promise.all(
      common.corpora.map(async (corpus) => {
        refs[corpus] = common.ref && lead ? await alignRef(lead, common.ref, corpus) : null;
      }),
    );
    const byBook = cards.filter((c) => c.bookView).map((c) => c.name);
    navigate(`/${serialize(commonToWorkspace(common, refs, byBook))}`);
  } else if (target === "reader") {
    const { reference } = await openCards();
    // the first corpus is read; the passage is in the reference corpus' numbering
    const corpus = common.corpora[0] ?? reference;
    const lead = reference ?? corpus;
    const ref = common.ref && corpus && lead ? await alignRef(lead, common.ref, corpus) : null;
    navigate(serializeReader(commonToReader(common, corpus, ref ? ref.split(":")[0] : null)));
  } else if (target === "interlinear") {
    const { reference } = await openCards();
    // the BHSA is shown when it is open; the passage is in its numbering
    const corpus = reference ?? common.corpora[0] ?? null;
    const lead = reference ?? corpus;
    const ref = common.ref && corpus && lead ? await alignRef(lead, common.ref, corpus) : null;
    navigate(serializeInterlinear(commonToInterlinear(common, corpus, ref ? ref.split(":")[0] : null)));
  } else if (target === "concordance") {
    const { reference } = await openCards();
    navigate(serializeConcordance(commonToConcordance(common, reference ?? common.corpora[0] ?? null)));
  } else if (target === "parallel") {
    navigate(serializeParallel(commonToParallel(common)));
  }
}
