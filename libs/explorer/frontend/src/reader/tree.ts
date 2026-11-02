import type { CorpusCard, Fragment, RenderTree } from "../types";

/** The type of a corpus' words: `word` where it has them (the SP and the DSS,
 * whose slots are signs), else its slot type. */
export function wordType(card: CorpusCard): string {
  return card.nodeTypes.some((t) => t.type === "word") ? "word" : card.slotType;
}

/** The words of a render tree, in order, each once (a word cut into parts by
 * other nodes' boundaries appears in several fragments). */
export function wordsOf(tree: RenderTree, type: string): number[] {
  const words: number[] = [];
  const seen = new Set<number>();
  const walk = (fragments: Fragment[]) => {
    for (const f of fragments) {
      if (f.type === type) {
        if (!seen.has(f.node)) {
          seen.add(f.node);
          words.push(f.node);
        }
      } else if (f.children) walk(f.children);
    }
  };
  walk(tree.fragments);
  return words;
}

/** The text of a render tree, as it is shown. */
export function textOf(tree: RenderTree): string {
  const parts: string[] = [];
  const walk = (fragments: Fragment[]) => {
    for (const f of fragments) {
      if (f.children?.length) walk(f.children);
      else if (f.text) parts.push(f.text);
    }
  };
  walk(tree.fragments);
  return parts.join("");
}
