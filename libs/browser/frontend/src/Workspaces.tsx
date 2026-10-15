import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { navigate, rememberUi, uiOfUrl, type Ui } from "./ui";

// Saved workspaces: named snapshots of the workspace URL (panes, corpora,
// passages, templates, sync; or the parallel interface's chapter, texts and
// search), kept in this browser and exportable as files.
// Text-Fabric browser job files can be imported too: their query opens in a
// search pane, their first passage in a reading pane.

const STORE = "cfabric-browser/workspaces";
const FORMAT = "cfabric-browser-workspace";

interface Saved {
  name: string;
  saved: string; // ISO date
  url: string; // "?c0=bhsa&v0=read..." (panes) or "/Genesis/1?texts=bhsa,sp" (parallel)
}

function loadSaved(): Saved[] {
  try {
    const list = JSON.parse(window.localStorage.getItem(STORE) ?? "[]");
    return Array.isArray(list) ? list.filter((w) => typeof w?.url === "string") : [];
  } catch {
    return [];
  }
}

function storeSaved(list: Saved[]): boolean {
  try {
    window.localStorage.setItem(STORE, JSON.stringify(list));
    return true;
  } catch {
    return false;
  }
}

function download(name: string, content: string) {
  const blob = new Blob([content], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = `${name.replace(/[^\w.-]+/g, "_") || "workspace"}.cfabric.json`;
  link.click();
  URL.revokeObjectURL(link.href);
}

/** Turn an imported file into a workspace URL, or explain why it can't be used. */
export function workspaceFromFile(
  data: unknown,
  corpora: string[],
): { url: string; name: string; run?: { pane: number; corpus: string; query: string } } | string {
  if (!data || typeof data !== "object") return "This file does not contain a workspace.";
  const obj = data as Record<string, unknown>;
  if (obj.format === FORMAT && typeof obj.url === "string") {
    return { url: obj.url, name: String(obj.name ?? "Imported workspace") };
  }
  // a Text-Fabric browser job: appName "ETCBC/bhsa", query, sections, jobName
  if (typeof obj.appName === "string" && ("query" in obj || "sections" in obj)) {
    const repo = obj.appName.split("/").pop()?.split(":")[0]?.toLowerCase() ?? "";
    const corpus = corpora.find((c) => c.toLowerCase() === repo);
    if (!corpus) {
      return `This Text-Fabric job is for ${obj.appName}, which is not open. Open it first with "Open a corpus".`;
    }
    const p = new URLSearchParams();
    let i = 0;
    const query = typeof obj.query === "string" ? obj.query.trim() : "";
    if (query) {
      p.set(`c${i}`, corpus);
      p.set(`v${i}`, "search");
      p.set(`q${i}`, query);
      i++;
    }
    const firstSection =
      typeof obj.sections === "string"
        ? obj.sections
            .split("\n")
            .map((l) => l.trim())
            .find(Boolean)
        : undefined;
    if (firstSection) {
      p.set(`c${i}`, corpus);
      p.set(`v${i}`, "read");
      p.set(`ref${i}`, firstSection);
      i++;
    }
    if (!i) return "This Text-Fabric job has no query and no passages.";
    return {
      url: `?${p.toString()}`,
      name: String(obj.jobName ?? "Text-Fabric job"),
      // as in the Text-Fabric browser, opening a job runs its query
      run: query ? { pane: 0, corpus, query } : undefined,
    };
  }
  return "This file is neither a saved workspace nor a Text-Fabric job.";
}

interface Props {
  corpora: string[];
  /** Load a workspace of this interface (for the panes: its query string). */
  onLoad: (url: string) => void;
  /** The interface this menu is in; a workspace of the other one switches to it. */
  current?: Ui;
}

/** The URL to save: the panes' query string, or the parallel interface's path and query. */
function currentUrl(current: Ui): string {
  return current === "classic" ? window.location.search : window.location.pathname + window.location.search;
}

export function Workspaces({ corpora, onLoad, current = "classic" }: Props) {
  const openUrl = (url: string) => {
    const target = new URL(url, window.location.origin);
    const ui = uiOfUrl(target) ?? "classic";
    if (ui !== current) {
      rememberUi(ui);
      navigate(target.pathname + target.search);
    } else onLoad(ui === "classic" ? target.search : target.pathname + target.search);
  };
  const [open, setOpen] = useState(false);
  const [saved, setSaved] = useState<Saved[]>(loadSaved);
  const [name, setName] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const panel = useRef<HTMLDivElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setSaved(loadSaved());
    const onDown = (e: MouseEvent) => {
      if (panel.current && !panel.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const save = () => {
    const title = name.trim();
    if (!title) return;
    const entry: Saved = { name: title, saved: new Date().toISOString(), url: currentUrl(current) };
    const list = [entry, ...saved.filter((w) => w.name !== title)];
    if (storeSaved(list)) {
      setSaved(list);
      setName("");
      setMessage(`Saved “${title}”.`);
    } else {
      setMessage("This browser does not allow saving here. Use “Export to a file” instead.");
    }
  };

  const remove = (title: string) => {
    const list = saved.filter((w) => w.name !== title);
    storeSaved(list);
    setSaved(list);
  };

  const exportCurrent = () => {
    const title = name.trim() || "workspace";
    download(
      title,
      JSON.stringify(
        { format: FORMAT, version: 1, name: title, saved: new Date().toISOString(), url: currentUrl(current) },
        null,
        2,
      ),
    );
  };

  const importFile = async (file: File) => {
    try {
      const result = workspaceFromFile(JSON.parse(await file.text()), corpora);
      if (typeof result === "string") {
        setMessage(result);
        return;
      }
      let url = result.url;
      if (result.run) {
        try {
          const started = await api.search(result.run.corpus, result.run.query);
          url += `&s${result.run.pane}=${started.id}`;
        } catch {
          // an invalid template stays in the editor, where its problems show on searching
        }
      }
      openUrl(url);
      setMessage(null);
      setOpen(false);
    } catch {
      setMessage("This file is not valid JSON.");
    }
  };

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => {
          setMessage(null);
          setOpen((o) => !o);
        }}
        className="rounded px-2 py-1 text-[14px] hover:bg-woad-soft aria-expanded:bg-woad-soft sm:px-2.5"
      >
        Workspaces
      </button>
      {open ? (
        <div className="absolute end-0 z-20 mt-1 flex w-80 flex-col gap-3 rounded border border-rule-strong bg-raised p-4 text-[14px] shadow-[0_8px_24px_-12px_rgba(31,43,77,0.35)]">
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              save();
            }}
          >
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Name this workspace"
              aria-label="Name of the workspace"
              className="min-w-0 flex-1 rounded border border-rule-strong bg-vellum px-2 py-1 placeholder:text-stone"
            />
            <button
              type="submit"
              disabled={!name.trim()}
              className="rounded bg-woad px-3 py-1 font-semibold text-vellum disabled:opacity-50"
            >
              Save
            </button>
          </form>
          {message ? (
            <p className="text-[13px] text-stone" role="status">
              {message}
            </p>
          ) : null}
          {saved.length ? (
            <ul className="flex max-h-64 flex-col overflow-y-auto">
              {saved.map((w) => (
                <li key={w.name} className="flex items-baseline gap-2 border-b border-rule py-1.5 last:border-b-0">
                  <button
                    type="button"
                    onClick={() => {
                      openUrl(w.url);
                      setOpen(false);
                    }}
                    className="min-w-0 flex-1 truncate text-left hover:text-woad"
                    title={`Saved ${new Date(w.saved).toLocaleString()}`}
                  >
                    {w.name}
                  </button>
                  <button
                    type="button"
                    onClick={() => remove(w.name)}
                    className="shrink-0 text-[12px] text-stone hover:text-madder"
                    aria-label={`Delete ${w.name}`}
                  >
                    Delete
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-[13px] text-stone">No saved workspaces in this browser yet.</p>
          )}
          <div className="flex flex-wrap gap-3 border-t border-rule pt-3 text-[13px]">
            <button type="button" onClick={exportCurrent} className="text-woad hover:underline">
              Export to a file
            </button>
            <button type="button" onClick={() => fileInput.current?.click()} className="text-woad hover:underline">
              Import a file
            </button>
            <input
              ref={fileInput}
              type="file"
              accept=".json,application/json"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) importFile(file);
                e.target.value = "";
              }}
            />
          </div>
          <p className="text-[12px] text-stone">Import accepts exported workspaces and Text-Fabric browser jobs.</p>
        </div>
      ) : null}
    </div>
  );
}
