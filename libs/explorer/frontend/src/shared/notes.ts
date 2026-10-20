import { useCallback, useEffect, useState } from "react";

// The user's own notes and bookmarks on verses, kept in this browser (or the
// standalone program's window) and in exported workspaces; never sent anywhere.

const KEY = "cfabric-browser/notes";
const CHANGED = "cfabric-browser/notes-changed";

export interface Note {
  text: string;
  /** A bookmark: the verse is starred. */
  starred: boolean;
  /** ISO date of the last change. */
  updated: string;
}

/** Per corpus, per verse reference (`Genesis 1:3`), its note. */
export type Notes = Record<string, Record<string, Note>>;

export function loadNotes(): Notes {
  try {
    const raw = window.localStorage.getItem(KEY);
    const notes = raw ? JSON.parse(raw) : {};
    return notes && typeof notes === "object" ? notes : {};
  } catch {
    return {};
  }
}

function storeNotes(notes: Notes): boolean {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(notes));
    window.dispatchEvent(new Event(CHANGED));
    return true;
  } catch {
    return false;
  }
}

/** Add imported notes; on the same verse, the newer note wins. */
export function mergeNotes(current: Notes, imported: Notes): Notes {
  const merged: Notes = { ...current };
  for (const [corpus, verses] of Object.entries(imported ?? {})) {
    if (!verses || typeof verses !== "object") continue;
    merged[corpus] = { ...merged[corpus] };
    for (const [ref, note] of Object.entries(verses)) {
      if (!note || typeof note.text !== "string") continue;
      const mine = merged[corpus][ref];
      if (!mine || (note.updated ?? "") > (mine.updated ?? "")) {
        merged[corpus][ref] = { text: note.text, starred: Boolean(note.starred), updated: note.updated ?? "" };
      }
    }
  }
  return merged;
}

export function importNotes(imported: Notes): void {
  storeNotes(mergeNotes(loadNotes(), imported));
}

/** The notes of a corpus, and a way to change one; a note without text or star is removed. */
export function useNotes(
  corpus: string,
): [Record<string, Note>, (ref: string, note: Omit<Note, "updated">) => boolean] {
  const [notes, setNotes] = useState<Notes>(loadNotes);
  useEffect(() => {
    const reload = () => setNotes(loadNotes());
    window.addEventListener(CHANGED, reload);
    window.addEventListener("storage", reload);
    return () => {
      window.removeEventListener(CHANGED, reload);
      window.removeEventListener("storage", reload);
    };
  }, []);
  const save = useCallback(
    (ref: string, note: Omit<Note, "updated">) => {
      const all = loadNotes();
      const verses = { ...all[corpus] };
      if (note.text.trim() || note.starred) verses[ref] = { ...note, updated: new Date().toISOString() };
      else delete verses[ref];
      return storeNotes({ ...all, [corpus]: verses });
    },
    [corpus],
  );
  return [notes[corpus] ?? {}, save];
}
