import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "../api";
import type { Proposal } from "../types";
import { askedOf, describeAsked, trialCount, waiting, type Asked } from "./asked";

// Searches proposed by a language model (the user's AI app, through MCP),
// shown above the search form until the user runs, edits or dismisses them.

/** All proposals, kept up to date by the server's events. */
export function useProposals(): Proposal[] {
  const client = useQueryClient();
  const { data } = useQuery({ queryKey: ["proposals"], queryFn: api.proposals, staleTime: Infinity });
  useEffect(() => {
    const source = api.proposalEvents();
    source.addEventListener("proposals", (event) => {
      client.setQueryData(["proposals"], JSON.parse((event as MessageEvent).data));
    });
    // the browser reconnects by itself after an error
    return () => source.close();
  }, [client]);
  return data?.proposals ?? [];
}

interface Props {
  /** the corpora whose proposals are shown here */
  corpora: string[];
  /** run the template (in its corpus), with where it came from */
  onRun: (proposal: Proposal, asked: Asked) => void;
  /** put the template in the editor, to change it first */
  onEdit: (proposal: Proposal, asked: Asked) => void;
}

/** The newest waiting proposal, with the others folded under it. */
export function ProposalCard({ corpora, onRun, onEdit }: Props) {
  const client = useQueryClient();
  const proposals = useProposals();
  const [showOthers, setShowOthers] = useState(false);
  const shown = waiting(proposals, corpora);
  if (shown.length === 0) return null;
  const [p, ...others] = shown;
  const settle = (proposal: Proposal, state: Proposal["state"]) => {
    void api.setProposal(proposal.id, state).then(() => client.invalidateQueries({ queryKey: ["proposals"] }));
  };
  return (
    <section className="flex flex-col gap-2 rounded border border-woad bg-woad-soft/40 p-3 text-[13px]" aria-label="Proposed search">
      <p>
        <span className="font-semibold">Proposed by {p.model ?? "a language model"}</span>
        {corpora.length > 1 ? <span className="text-stone"> in {p.corpus}</span> : null}, for:{" "}
        <q className="italic">{p.request}</q>
      </p>
      <pre className="overflow-x-auto whitespace-pre-wrap rounded bg-raised px-2 py-1.5 font-mono text-[12px]">
        {p.template}
      </pre>
      {p.explanation ? <p className="whitespace-pre-wrap">{p.explanation}</p> : null}
      {trialCount(p) ? <p className="text-stone">Its trial found {trialCount(p)}.</p> : null}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => {
            settle(p, "run");
            onRun(p, askedOf(p));
          }}
          className="rounded bg-woad px-3 py-1 text-vellum hover:opacity-90"
        >
          Run
        </button>
        <button
          type="button"
          onClick={() => {
            settle(p, "run");
            onEdit(p, askedOf(p));
          }}
          className="rounded border border-rule-strong px-3 py-1 hover:border-woad"
        >
          Edit
        </button>
        <button
          type="button"
          onClick={() => settle(p, "dismissed")}
          className="rounded border border-rule-strong px-3 py-1 hover:border-woad"
        >
          Dismiss
        </button>
      </div>
      {others.length ? (
        <div>
          <button type="button" onClick={() => setShowOthers(!showOthers)} className="text-stone hover:text-ink" aria-expanded={showOthers}>
            {showOthers ? "▾" : "▸"} {others.length} earlier {others.length === 1 ? "proposal" : "proposals"}
          </button>
          {showOthers ? (
            <ul className="mt-1 flex flex-col gap-1">
              {others.map((o) => (
                <li key={o.id} className="flex items-baseline gap-2">
                  <span className="min-w-0 flex-1 truncate" title={`${describeAsked(askedOf(o))}\n${o.template}`}>
                    <q className="italic">{o.request}</q> <span className="font-mono text-[12px] text-stone">{o.template.split("\n")[0]}</span>
                  </span>
                  <button type="button" onClick={() => { settle(o, "run"); onEdit(o, askedOf(o)); }} className="text-woad hover:underline">
                    Use
                  </button>
                  <button type="button" onClick={() => settle(o, "dismissed")} className="text-stone hover:underline">
                    Dismiss
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

/** Above results: where the search came from, when a model proposed it. */
export function AskedLine({ asked, template }: { asked: Asked | null; template?: string }) {
  if (!asked) return null;
  return <p className="text-[12px] text-stone">{describeAsked(asked, template)}</p>;
}
