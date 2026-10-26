import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import type { CorpusDownload, DictionaryInfo, Translation } from "./types";

interface Props {
  onOpen: (corpus: string) => void;
  onClose: () => void;
}

/** Corpora found on this computer; opening one shows it in a new pane. */
export function Library({ onOpen, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const client = useQueryClient();
  const [filter, setFilter] = useState("");
  const { data, error, isLoading } = useQuery({ queryKey: ["library"], queryFn: api.library, staleTime: 0 });
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
                      <p
                        className="truncate text-[13px] text-stone"
                        title={c.copies?.length ? `Also on this computer: ${c.copies.join(", ")}` : undefined}
                      >
                        {c.id}, version {c.version}
                        {c.copies?.length
                          ? ` (also ${c.copies.length === 1 ? "a copy" : `${c.copies.length} copies`} elsewhere)`
                          : ""}
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
                  No corpora found in ~/text-fabric-data/github or ~/github. Download one below, or with Text-Fabric, or
                  start the browser with the path of a corpus.
                </p>
              ) : null}
              <Downloads onDone={(corpus) => open.mutate(corpus)} />
              <Translations onAdded={(corpus) => open.mutate(corpus)} />
              <Dictionaries />
            </>
          ) : null}
        </div>
      </div>
    </dialog>
  );
}

/** Translations that can be added: downloaded and built on this computer, then
 * opened like any other corpus. */
function Translations({ onAdded }: { onAdded: (corpus: string) => void }) {
  const client = useQueryClient();
  // the translations added here, to open once they are built
  const [started, setStarted] = useState<string[]>([]);
  const [removing, setRemoving] = useState<string | null>(null);
  const { data } = useQuery({
    queryKey: ["translations"],
    queryFn: api.translations,
    staleTime: 0, // jobs and files change
    refetchInterval: (q) => (q.state.data?.translations.some((t) => t.job?.state === "running") ? 500 : false),
  });
  const refresh = async () => {
    await client.invalidateQueries({ queryKey: ["translations"] });
    await client.invalidateQueries({ queryKey: ["library"] });
  };
  const add = useMutation({
    mutationFn: (id: string) => api.addTranslation(id),
    onSuccess: (entry) => {
      // show the running job straight away, so that a finished job of before isn't taken for this one
      client.setQueryData(["translations"], (old: { enabled: boolean; translations: Translation[] } | undefined) =>
        old ? { ...old, translations: old.translations.map((t) => (t.id === entry.id ? entry : t)) } : old,
      );
      if (!entry.added) setStarted((ids) => [...ids, entry.id]);
      void refresh();
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.removeTranslation(id),
    onSuccess: () => {
      setRemoving(null);
      void refresh();
    },
  });

  const translations = data?.enabled ? data.translations : [];
  useEffect(() => {
    for (const t of translations) {
      if (started.includes(t.id) && t.job && t.job.state !== "running") {
        setStarted((ids) => ids.filter((id) => id !== t.id));
        void refresh();
        if (t.added) onAdded(t.corpus);
      }
    }
  });

  if (translations.length === 0) return null;
  const error = add.error ?? remove.error;
  return (
    <section className="flex flex-col gap-2 border-t border-rule pt-3">
      <h3 className="text-[15px] font-semibold">Translations</h3>
      <p className="text-[13px] text-stone">
        Downloaded from their source and prepared on this computer, then shown next to the Hebrew texts.
      </p>
      {error ? (
        <p className="text-[13px] text-madder" role="alert">
          {String((error as Error).message)}
        </p>
      ) : null}
      <ul className="flex flex-col">
        {translations.map((t) => (
          <li key={t.id} className="flex items-start gap-3 border-b border-rule py-2 last:border-b-0">
            <div className="min-w-0 flex-1">
              <p className="font-semibold">
                {t.title} <span className="font-normal text-stone">({languageName(t.language)})</span>
              </p>
              <p className="text-[13px]">{t.name}</p>
              <p className="text-[12px] text-stone">
                {t.licenceNote} Source: {t.source}.
              </p>
              {t.job?.state === "failed" ? (
                <p className="text-[13px] text-madder" role="alert">
                  Could not add it: {t.job.error}
                </p>
              ) : null}
              {t.removing ? (
                <p className="text-[13px] text-stone">It is open, so it is removed when the browser starts again.</p>
              ) : null}
            </div>
            <TranslationButton
              t={t}
              confirming={removing === t.id}
              busy={add.isPending || remove.isPending}
              onAdd={() => add.mutate(t.id)}
              onRemove={() => (removing === t.id ? remove.mutate(t.id) : setRemoving(t.id))}
              onCancel={() => setRemoving(null)}
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

const BUTTON = "shrink-0 rounded border border-rule-strong px-3 py-1 text-[13px] hover:border-woad disabled:opacity-50";

/** Dictionaries that can be added (BDB): one file, shown on the pages of the
 * lexicon. */
function Dictionaries() {
  const client = useQueryClient();
  const [removing, setRemoving] = useState<string | null>(null);
  const { data } = useQuery({
    queryKey: ["dictionaries"],
    queryFn: api.dictionaries,
    staleTime: 0, // jobs and files change
    refetchInterval: (q) => (q.state.data?.dictionaries.some((d) => d.job?.state === "running") ? 500 : false),
  });
  const refresh = async () => {
    await client.invalidateQueries({ queryKey: ["dictionaries"] });
    // the lexeme pages show it, or no longer
    await client.invalidateQueries({ queryKey: ["lexeme"] });
    await client.invalidateQueries({ queryKey: ["dictionary"] });
    await client.invalidateQueries({ queryKey: ["bdb-entry"] });
    // the list's definitions, and words' entries on the card and in the inspector
    await client.invalidateQueries({ queryKey: ["lexicon"] });
    await client.invalidateQueries({ queryKey: ["summary"] });
    await client.invalidateQueries({ queryKey: ["node"] });
  };
  const add = useMutation({
    mutationFn: (id: string) => api.addDictionary(id),
    onSuccess: (entry) => {
      client.setQueryData(["dictionaries"], (old: { enabled: boolean; dictionaries: DictionaryInfo[] } | undefined) =>
        old ? { ...old, dictionaries: old.dictionaries.map((d) => (d.id === entry.id ? entry : d)) } : old,
      );
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.removeDictionary(id),
    onSuccess: () => {
      setRemoving(null);
      void refresh();
    },
  });
  const dictionaries = data?.enabled ? data.dictionaries : [];
  const finished = dictionaries.filter((d) => d.job?.state === "done").map((d) => d.id).join(",");
  useEffect(() => {
    if (finished) void refresh();
  }, [finished]);

  if (dictionaries.length === 0) return null;
  const error = add.error ?? remove.error;
  return (
    <section className="flex flex-col gap-2 border-t border-rule pt-3">
      <h3 className="text-[15px] font-semibold">Dictionaries</h3>
      <p className="text-[13px] text-stone">Shown on the pages of the lexicon, linked to its lexemes.</p>
      {error ? (
        <p className="text-[13px] text-madder" role="alert">
          {String((error as Error).message)}
        </p>
      ) : null}
      <ul className="flex flex-col">
        {dictionaries.map((d) => (
          <li key={d.id} className="flex items-start gap-3 border-b border-rule py-2 last:border-b-0">
            <div className="min-w-0 flex-1">
              <p className="font-semibold">{d.name}</p>
              <p className="text-[12px] text-stone">
                {d.licenceNote} About {Math.round(d.size / 1_000_000)} MB.
              </p>
              {!d.added && !d.available ? (
                <p className="text-[13px] text-stone">
                  Not published yet. A file built on this computer can be added with{" "}
                  <code className="font-mono">cfabric-browser add {d.id} FILE</code>.
                </p>
              ) : null}
              {d.job?.state === "failed" ? (
                <p className="text-[13px] text-madder" role="alert">
                  Could not add it: {d.job.error}
                </p>
              ) : null}
            </div>
            {d.job?.state === "running" ? (
              <span className="shrink-0 text-[13px] text-stone" role="status">
                Downloading {Math.round(d.job.done / 1000).toLocaleString("en-US")} kB…
              </span>
            ) : !d.added ? (
              d.available ? (
                <button type="button" disabled={add.isPending} onClick={() => add.mutate(d.id)} className={BUTTON}>
                  {d.job?.state === "failed" ? "Try again" : "Add"}
                </button>
              ) : null
            ) : removing === d.id ? (
              <span className="flex shrink-0 items-baseline gap-2 text-[13px]">
                Remove it?
                <button type="button" disabled={remove.isPending} onClick={() => remove.mutate(d.id)} className={`${BUTTON} text-madder`}>
                  Remove
                </button>
                <button type="button" onClick={() => setRemoving(null)} className={BUTTON}>
                  No
                </button>
              </span>
            ) : (
              <button type="button" onClick={() => setRemoving(d.id)} className={BUTTON}>
                Remove
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function TranslationButton(props: {
  t: Translation;
  confirming: boolean;
  busy: boolean;
  onAdd: () => void;
  onRemove: () => void;
  onCancel: () => void;
}) {
  const { t, confirming, busy } = props;
  if (t.job?.state === "running") {
    return (
      <span className="shrink-0 text-[13px] text-stone" role="status">
        {t.job.step === "download" && t.job.total ? `Downloading ${t.job.done} of ${t.job.total}…` : "Preparing…"}
      </span>
    );
  }
  if (!t.added) {
    return (
      <button type="button" disabled={busy} onClick={props.onAdd} className={BUTTON}>
        {t.job?.state === "failed" ? "Try again" : "Add"}
      </button>
    );
  }
  if (t.removing) {
    return (
      <button type="button" disabled={busy} onClick={props.onAdd} className={BUTTON}>
        Keep it
      </button>
    );
  }
  if (confirming) {
    return (
      <span className="flex shrink-0 items-baseline gap-2 text-[13px]">
        Remove it?
        <button type="button" disabled={busy} onClick={props.onRemove} className={`${BUTTON} text-madder`}>
          Remove
        </button>
        <button type="button" onClick={props.onCancel} className={BUTTON}>
          No
        </button>
      </span>
    );
  }
  return (
    <button type="button" disabled={busy} onClick={props.onRemove} className={BUTTON}>
      Remove
    </button>
  );
}

const LANGUAGES: Record<string, string> = { eng: "English", deu: "German", nld: "Dutch", fra: "French" };

function languageName(code: string): string {
  return LANGUAGES[code] ?? code;
}

/** Corpora that are not on this computer and can be downloaded from their own releases. */
function Downloads({ onDone }: { onDone: (corpus: string) => void }) {
  const client = useQueryClient();
  // the downloads started here, to open once they are done
  const [started, setStarted] = useState<string[]>([]);
  const { data } = useQuery({
    queryKey: ["downloads"],
    queryFn: api.downloads,
    staleTime: 0, // jobs and files change
    refetchInterval: (q) => (q.state.data?.downloads.some((d) => d.job?.state === "running") ? 500 : false),
  });
  const start = useMutation({
    mutationFn: (id: string) => api.startDownload(id),
    onSuccess: (entry) => {
      // show the running job straight away, so that a finished job of before isn't taken for this one
      client.setQueryData(["downloads"], (old: { enabled: boolean; downloads: CorpusDownload[] } | undefined) =>
        old ? { ...old, downloads: old.downloads.map((d) => (d.id === entry.id ? entry : d)) } : old,
      );
      if (!entry.present) setStarted((ids) => [...ids, entry.id]);
    },
  });

  const downloads = data?.enabled ? data.downloads : [];
  useEffect(() => {
    for (const d of downloads) {
      if (started.includes(d.id) && d.job && d.job.state !== "running") {
        setStarted((ids) => ids.filter((id) => id !== d.id));
        void client.invalidateQueries({ queryKey: ["library"] });
        void client.invalidateQueries({ queryKey: ["corpora"] });
        if (d.present) onDone(d.id);
      }
    }
  });

  // only those not on this computer (or still being downloaded)
  const missing = downloads.filter((d) => !d.present || started.includes(d.id));
  if (missing.length === 0) return null;
  return (
    <section className="flex flex-col gap-2 border-t border-rule pt-3">
      <h3 className="text-[15px] font-semibold">Can be downloaded</h3>
      <p className="text-[13px] text-stone">
        Not on this computer yet. They come from their own releases on GitHub, the same files Text-Fabric uses.
      </p>
      {start.error ? (
        <p className="text-[13px] text-madder" role="alert">
          {String((start.error as Error).message)}
        </p>
      ) : null}
      <ul className="flex flex-col">
        {missing.map((d) => (
          <li key={d.id} className="flex items-start gap-3 border-b border-rule py-2 last:border-b-0">
            <div className="min-w-0 flex-1">
              <p className="truncate font-semibold">{d.title.split(" = ").pop()}</p>
              <p className="text-[13px] text-stone">
                {d.id}, version {d.version}: {megabytes(d.size)} to download, {megabytes(d.unpacked)} on disk.
                {d.licence ? ` Licence: ${d.licence}.` : ""}{" "}
                <a href={d.terms} target="_blank" rel="noreferrer" className="text-woad hover:underline">
                  Terms of use
                </a>
              </p>
              {d.job?.state === "failed" ? (
                <p className="text-[13px] text-madder" role="alert">
                  Could not download it: {d.job.error}
                </p>
              ) : null}
            </div>
            {d.job?.state === "running" ? (
              <span className="shrink-0 text-[13px] text-stone" role="status">
                {d.job.step === "download"
                  ? `${megabytes(d.job.done)} of ${megabytes(d.job.total || d.size)}…`
                  : d.job.step === "unpack"
                    ? "Unpacking…"
                    : "Preparing…"}
              </span>
            ) : d.present ? (
              <span className="shrink-0 text-[13px] text-stone">Opening…</span>
            ) : (
              <button type="button" disabled={start.isPending} onClick={() => start.mutate(d.id)} className={BUTTON}>
                {d.job?.state === "failed" ? "Try again" : "Download"}
              </button>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function megabytes(bytes: number): string {
  return `${(bytes / 1e6).toFixed(bytes < 1e7 ? 1 : 0)} MB`;
}
