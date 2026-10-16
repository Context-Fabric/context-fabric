// The tab of the lexeme page last chosen (Occurrences or BDB).

export type Tab = "occurrences" | "bdb";

// the tab last chosen, kept for the session (opening a verse and coming back)
const TAB_KEY = "cfabric-browser/lexeme-tab";

export function keptTab(): Tab {
  try {
    return window.sessionStorage.getItem(TAB_KEY) === "bdb" ? "bdb" : "occurrences";
  } catch {
    return "occurrences";
  }
}

/** Open the next lexeme page at its BDB tab (from the inspector). */
export function showBdbNext(): void {
  keepTab("bdb");
}

export function keepTab(tab: Tab): void {
  try {
    window.sessionStorage.setItem(TAB_KEY, tab);
  } catch {
    // no storage: the tab is not kept
  }
}
