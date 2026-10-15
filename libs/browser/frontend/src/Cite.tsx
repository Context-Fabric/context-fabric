import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { displayRef } from "./display/Display";
import type { CorpusCard } from "./types";

// A citation for what you are looking at: the passage, or the search with its
// template and number of results, plus the corpus (title, version, DOI) and the
// software that produced it. Copied as plain text.

function useVersions() {
  return useQuery({
    queryKey: ["health"],
    queryFn: async () =>
      (await fetch("/api/health").then((r) => r.json())) as { version: string; coreVersion: string | null },
    staleTime: Infinity,
  });
}

function corpusLine(corpus: CorpusCard): string {
  // a translation: its edition and where the text came from
  if (corpus.translation) return `${corpus.translation.cite}.`;
  const title = corpus.title || corpus.name;
  const doi = corpus.doi ? ` https://doi.org/${corpus.doi}.` : "";
  return `${title}, version ${corpus.version}.${doi}`;
}

export function citePassage(corpus: CorpusCard, ref: string, software: string, date: string): string {
  return `${displayRef(ref)}. ${corpusLine(corpus)} Read with ${software}, ${date}.`;
}

export function citeSearch(
  corpus: CorpusCard,
  template: string,
  count: number,
  truncated: boolean,
  software: string,
  date: string,
): string {
  const results = `${count.toLocaleString("en-US")}${truncated ? " (stopped at the limit)" : ""} results`;
  return `Search in ${corpusLine(corpus)}\nTemplate:\n${template.trim()}\n${results}, found with ${software}, ${date}.`;
}

interface Props {
  /** Builds the citation from the software description and today's date. */
  text: (software: string, date: string) => string;
  label?: string;
  /** Which edge of the button the panel lines up with: the end (default), or the start near the left of a bar. */
  align?: "start" | "end";
}

export function Cite({ text, label = "Cite", align = "end" }: Props) {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const panel = useRef<HTMLDivElement>(null);
  const { data } = useVersions();
  const software = data
    ? `Context-Fabric browser ${data.version}${data.coreVersion ? ` (context-fabric ${data.coreVersion})` : ""}`
    : "Context-Fabric browser";
  const date = new Date().toISOString().slice(0, 10);
  const citation = text(software, date);

  useEffect(() => {
    if (!open) return;
    setCopied(false);
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

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(citation);
      setCopied(true);
    } catch {
      // the text is selected in the box below for copying by hand
      setCopied(false);
    }
  };

  return (
    <div className="relative" ref={panel}>
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className="rounded px-2 py-0.5 text-[13px] text-woad hover:underline"
      >
        {label}
      </button>
      {open ? (
        <div className={`absolute ${align === "end" ? "end-0" : "start-0"} z-20 mt-1 flex w-[min(26rem,calc(100vw-2rem))] flex-col gap-2 rounded border border-rule-strong bg-raised p-3 text-[13px] shadow-[0_8px_24px_-12px_rgba(31,43,77,0.35)]`}>
          <textarea
            readOnly
            value={citation}
            // room for wrapped lines too (about 55 characters per line)
            rows={Math.min(
              12,
              citation.split("\n").reduce((n, line) => n + Math.max(1, Math.ceil(line.length / 55)), 0),
            )}
            onFocus={(e) => e.target.select()}
            aria-label="Citation"
            className="w-full resize-none rounded border border-rule bg-vellum p-2 font-serif leading-snug"
          />
          <div className="flex items-center gap-3">
            <button type="button" onClick={copy} className="rounded bg-woad px-3 py-1 font-semibold text-vellum">
              Copy
            </button>
            {copied ? <span className="text-stone" role="status">Copied</span> : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
