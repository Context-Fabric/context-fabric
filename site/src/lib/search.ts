/**
 * Client-side search using Fuse.js
 */
"use client";

import Fuse from "fuse.js";
import type { SearchItem } from "@/types/docs";

let fuseInstance: Fuse<SearchItem> | null = null;
let searchIndex: SearchItem[] | null = null;

/**
 * Initialize the search index (call this once when loading docs)
 */
export function initializeSearch(items: SearchItem[]) {
  searchIndex = items;
  fuseInstance = new Fuse(items, {
    keys: [
      { name: "name", weight: 2 },
      { name: "summary", weight: 1 },
    ],
    threshold: 0.3,
    includeScore: true,
    minMatchCharLength: 2,
  });
}

/**
 * Search the documentation
 */
export function searchDocs(
  query: string,
  limit = 20
): Array<SearchItem & { score?: number }> {
  if (!fuseInstance || !query.trim()) {
    return [];
  }

  return fuseInstance
    .search(query, { limit })
    .map((result) => ({ ...result.item, score: result.score }));
}

/**
 * Get all items of a specific type
 */
export function getItemsByType(type: SearchItem["type"]): SearchItem[] {
  if (!searchIndex) return [];
  return searchIndex.filter((item) => item.type === type);
}

/**
 * Get all items in a specific package
 */
export function getItemsByPackage(packageName: string): SearchItem[] {
  if (!searchIndex) return [];
  return searchIndex.filter((item) => item.package === packageName);
}
