import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { displayRef, scriptClass } from "./display/Display";
import { borrowedFrom, type CorpusCard } from "./types";

interface Props {
  corpus: CorpusCard;
  node: number;
  onInspect: (node: number) => void;
  onClose: () => void;
  onOpenPassage: (ref: string) => void;
  /** Open the lexicon at this lexeme; offered for words of a corpus with a lexicon. */
  onLexicon?: (lex: number) => void;
  /** Which side of the screen the panel is on (the parallel interface has it on the left). */
  side?: "start" | "end";
}

/** Everything about one node: its features, edges, and what it lies in. */
export function Inspector({ corpus, node, onInspect, onClose, onOpenPassage, onLexicon, side = "end" }: Props) {
  const panel = useRef<HTMLElement>(null);
  const { data, error } = useQuery({
    queryKey: ["node", corpus.name, node],
    queryFn: () => api.nodeInfo(corpus.name, node),
  });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    panel.current?.focus();
  }, [node]);

  // the full list is folded away below the essentials; the g_ features (text in
  // other spellings and transcriptions) only on request
  const [allFeatures, setAllFeatures] = useState(false);
  const [technical, setTechnical] = useState(false);

  const excluded = new Set(corpus.display.excludedFeatures);
  const noValue = new Set(corpus.display.noneValues);
  const all = data ? Object.entries(data.features) : [];
  // an empty value (or only a space, as in a trailer) counts as no value
  const features = all.filter(
    ([name, value]) => !excluded.has(name) && !noValue.has(String(value)) && String(value).trim() !== "",
  );
  const hiddenExcluded = all.filter(([name]) => excluded.has(name)).length;
  const hiddenEmpty = all.length - features.length - hiddenExcluded;
  const isTechnical = (name: string) => name.startsWith("g_");
  const technicalCount = features.filter(([name]) => isTechnical(name)).length;
  const listed = technical ? features : features.filter(([name]) => !isTechnical(name));
  const summary = data?.summary ?? [];
  // the lexeme a word belongs to, in a corpus with a lexicon (BHSA)
  const lexeme = corpus.lexicon ? data?.up.find((u) => u.type === "lex") : undefined;
  // a node without essentials (a phrase, a clause) shows its features straight away
  const open = allFeatures || summary.length === 0;

  return (
    <aside
      ref={panel}
      tabIndex={-1}
      aria-label={`${data?.type ?? "Node"} ${node}`}
      className={`flex h-full min-h-0 w-full flex-col border-rule bg-raised outline-none md:w-[22rem] ${
        side === "end" ? "border-s" : "border-e"
      }`}
    >
      <header className="flex items-baseline gap-2 border-b border-rule px-4 py-3">
        <h2 className="text-[17px] font-semibold">{data?.type ?? "…"}</h2>
        <span className="font-mono text-[12px] text-stone">{node}</span>
        <button
          type="button"
          onClick={onClose}
          className="ms-auto rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          aria-label="Close"
        >
          Close
        </button>
      </header>
      {error ? <p className="p-4 text-madder">{String((error as Error).message)}</p> : null}
      {data ? (
        <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3 text-[14px]">
          {data.text ? (
            <p
              dir={corpus.display.direction}
              className={`mb-2 ${scriptClass(corpus.display, corpus.display.direction)}`}
            >
              {data.text}
            </p>
          ) : null}
          {data.ref ? (
            <button
              type="button"
              onClick={() => onOpenPassage(data.ref)}
              className="mb-3 text-[13px] text-woad hover:underline"
            >
              Read {displayRef(data.ref)}
            </button>
          ) : null}

          {summary.length ? (
            <dl className="mb-3 mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
              {summary.map((item) => (
                <div key={item.label} className="contents">
                  <dt className="text-stone" title={item.feature}>
                    {item.label}
                  </dt>
                  <dd
                    className={`m-0 break-words ${
                      item.label === "lexeme" ? scriptClass(corpus.display, corpus.display.direction) : ""
                    }`}
                  >
                    {item.value}
                    {borrowedFrom(item) ? (
                      <span className="text-stone" title={`From the lexicon of ${borrowedFrom(item)}`}>
                        {" "}
                        · {borrowedFrom(item)}
                      </span>
                    ) : null}
                  </dd>
                </div>
              ))}
            </dl>
          ) : null}

          {lexeme !== undefined && onLexicon ? (
            <button
              type="button"
              onClick={() => onLexicon(lexeme.node)}
              className="mb-3 block text-[13px] text-woad hover:underline"
            >
              Lexicon entry
            </button>
          ) : null}

          {summary.length ? (
            <button
              type="button"
              onClick={() => setAllFeatures(!allFeatures)}
              aria-expanded={open}
              className="mb-1 text-[13px] text-stone hover:text-ink"
            >
              <span aria-hidden className={`inline-block transition-transform ${open ? "rotate-90" : ""}`}>
                ▸
              </span>{" "}
              All features ({listed.length})
            </button>
          ) : (
            <h3 className="mb-1 mt-2 text-[13px] text-stone">Features</h3>
          )}
          {open ? (
            <>
              <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
                {listed.map(([name, value]) => (
                  <div key={name} className="contents">
                    <dt className="text-stone">{name}</dt>
                    <dd className="m-0 break-words">{String(value)}</dd>
                  </div>
                ))}
              </dl>
              {technicalCount ? (
                <label className="mt-1.5 flex items-center gap-1.5 text-[12px] text-stone">
                  <input
                    type="checkbox"
                    checked={technical}
                    onChange={(e) => setTechnical(e.target.checked)}
                    className="accent-[var(--woad)]"
                  />
                  Also the {technicalCount} g_ features (the text in other spellings)
                </label>
              ) : null}
            </>
          ) : null}
          {open && (hiddenEmpty || hiddenExcluded) ? (
            <p className="mt-1.5 text-[12px] text-stone">
              Not shown:{" "}
              {[
                hiddenEmpty ? `${hiddenEmpty} without a value (empty, ${[...noValue].join(", ")})` : "",
                hiddenExcluded ? `${hiddenExcluded} that the corpus marks as internal` : "",
              ]
                .filter(Boolean)
                .join("; ")}
              .
            </p>
          ) : null}

          {Object.keys(data.edges).length ? (
            <>
              <h3 className="mb-1 mt-4 text-[13px] text-stone">Relations</h3>
              <ul className="flex flex-col gap-1">
                {Object.entries(data.edges).map(([name, edge]) => (
                  <li key={name}>
                    <span className="italic">{name}</span>
                    {edge.from.length ? (
                      <span>
                        {" to "}
                        {edge.from.map((e) => (
                          <NodeLink key={e.node} node={e.node} onInspect={onInspect} />
                        ))}
                      </span>
                    ) : null}
                    {edge.to.length ? (
                      <span>
                        {" from "}
                        {edge.to.map((e) => (
                          <NodeLink key={e.node} node={e.node} onInspect={onInspect} />
                        ))}
                      </span>
                    ) : null}
                  </li>
                ))}
              </ul>
            </>
          ) : null}

          <h3 className="mb-1 mt-4 text-[13px] text-stone">Lies in</h3>
          <ul className="flex flex-col">
            {data.up.map((u) => (
              <li key={u.node}>
                <button
                  type="button"
                  onClick={() => onInspect(u.node)}
                  className="rounded px-1 text-left hover:bg-woad-soft"
                >
                  {u.type} <span className="font-mono text-[12px] text-stone">{u.node}</span>
                </button>
              </li>
            ))}
          </ul>

          {Object.keys(data.downCounts).length ? (
            <>
              <h3 className="mb-1 mt-4 text-[13px] text-stone">Contains</h3>
              <p>
                {Object.entries(data.downCounts)
                  .map(([type, count]) => `${count.toLocaleString("en-US")} ${type}`)
                  .join(", ")}
              </p>
            </>
          ) : null}
        </div>
      ) : null}
    </aside>
  );
}

function NodeLink({ node, onInspect }: { node: number; onInspect: (n: number) => void }) {
  return (
    <button
      type="button"
      onClick={() => onInspect(node)}
      className="me-1 font-mono text-[12px] text-woad hover:underline"
    >
      {node}
    </button>
  );
}
