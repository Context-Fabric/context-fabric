import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "./api";

interface Props {
  onOpen: (corpus: string) => void;
  onClose: () => void;
}

/** Corpora found on this computer; opening one shows it in a new pane. */
export function Library({ onOpen, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const client = useQueryClient();
  const [filter, setFilter] = useState("");
  const { data, error, isLoading } = useQuery({ queryKey: ["library"], queryFn: api.library });
  const open = useMutation({
    mutationFn: (id: string) => api.openCorpus(id),
    onSuccess: async (result) => {
      await client.invalidateQueries({ queryKey: ["corpora"] });
      await client.invalidateQueries({ queryKey: ["library"] });
      onOpen(result.name);
    },
  });

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const needle = filter.trim().toLowerCase();
  const corpora = (data?.corpora ?? []).filter(
    (c) => !needle || `${c.id} ${c.title ?? ""}`.toLowerCase().includes(needle),
  );

  return (
    <dialog
      ref={dialog}
      onClose={onClose}
      className="m-auto max-h-[80vh] w-[min(40rem,calc(100vw-2rem))] rounded border border-rule-strong bg-raised p-0 text-ink backdrop:bg-[rgba(21,23,29,0.45)]"
    >
      <div className="flex max-h-[80vh] flex-col">
        <header className="flex items-baseline gap-3 border-b border-rule px-5 py-3">
          <h2 className="text-[18px] font-semibold">Open a corpus</h2>
          <button
            type="button"
            onClick={() => dialog.current?.close()}
            className="ms-auto rounded px-2 py-0.5 text-[13px] text-stone hover:bg-woad-soft hover:text-ink"
          >
            Close
          </button>
        </header>
        <div className="flex flex-col gap-3 overflow-y-auto px-5 py-4">
          {isLoading ? <p className="text-stone">Looking for corpora on this computer…</p> : null}
          {error ? <p className="text-madder">{String((error as Error).message)}</p> : null}
          {data && !data.enabled ? (
            <p className="text-stone">This server only shows the corpora it was started with.</p>
          ) : null}
          {data?.enabled ? (
            <>
              <p className="text-[14px] text-stone">
                Corpora that Text-Fabric has downloaded or cloned on this computer.
              </p>
              <input
                type="search"
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder="Find a corpus"
                className="rounded border border-rule-strong bg-vellum px-2.5 py-1.5 text-[14px] placeholder:text-stone"
                autoFocus
              />
              {open.error ? (
                <p className="text-madder" role="alert">
                  Could not open it: {String((open.error as Error).message)}
                </p>
              ) : null}
              <ul className="flex flex-col">
                {corpora.map((c) => (
                  <li key={c.id} className="flex items-baseline gap-3 border-b border-rule py-2 last:border-b-0">
                    <div className="min-w-0 flex-1">
                      <p className="truncate font-semibold">{c.title?.split(" = ").pop() ?? c.name}</p>
                      <p className="truncate text-[13px] text-stone">
                        {c.id}, version {c.versions[c.versions.length - 1]}
                      </p>
                    </div>
                    <button
                      type="button"
                      disabled={open.isPending}
                      onClick={() => (c.open ? onOpen(c.name) : open.mutate(c.id))}
                      className="shrink-0 rounded border border-rule-strong px-3 py-1 text-[13px] hover:border-woad disabled:opacity-50"
                    >
                      {open.isPending && open.variables === c.id ? "Opening…" : c.open ? "Show" : "Open"}
                    </button>
                  </li>
                ))}
              </ul>
              {data.corpora.length === 0 ? (
                <p className="text-stone">
                  No corpora found in ~/text-fabric-data/github or ~/github. Download one with Text-Fabric
                  first, or start the browser with the path of a corpus.
                </p>
              ) : null}
            </>
          ) : null}
        </div>
      </div>
    </dialog>
  );
}
