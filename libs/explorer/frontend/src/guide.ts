import type { Ui } from "./ui";

// The user guide is built from libs/explorer/guide/*.md by scripts/guide.mjs and
// served at /guide/; each interface's Help opens the guide where it is explained.
const PAGES: Record<Ui, string> = {
  classic: "interfaces.html#panes",
  parallel: "interfaces.html#parallel",
  reader: "interfaces.html#reader",
  interlinear: "interfaces.html#interlinear",
  concordance: "concordance.html",
};

/** Where the guide explains an interface. */
export function guideUrl(ui: Ui): string {
  return `/guide/${PAGES[ui]}`;
}
