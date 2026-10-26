import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState, type ReactNode } from "react";
import { api } from "./api";
import type { CorpusDownload } from "./types";

// The standalone program's first screen when corpora are missing: it asks once
// whether to download them (nothing goes over the network without a click),
// then shows the progress per corpus, including preparing each one, which
// takes minutes the first time. Answering "Not now" is remembered; the library
// still offers them under "Can be downloaded".

const ASKED = "cfabric-browser/downloads-asked";

function wasAsked(): boolean {
  try {
    return localStorage.getItem(ASKED) === "1";
  } catch {
    return false;
  }
}

function rememberAsked() {
  try {
    localStorage.setItem(ASKED, "1");
  } catch {
    // only the question comes back next time
  }
}

function megabytes(bytes: number): string {
  return `${(bytes / 1e6).toFixed(bytes < 1e7 ? 1 : 0)} MB`;
}

/** Shows `children`, after the start screen if the program asks for missing corpora. */
export function StartDownloads({ children }: { children: ReactNode }) {
  const client = useQueryClient();
  const [asked, setAsked] = useState(wasAsked);
  // the ids being fetched from this screen; null before "Download"
  const [fetching, setFetching] = useState<string[] | null>(null);
  const [done, setDone] = useState(false);
  const health = useQuery({
    queryKey: ["health"],
    queryFn: async () => (await fetch("/api/health").then((r) => r.json())) as { askDownloads?: boolean },
    staleTime: Infinity,
  });
  const ask = health.data?.askDownloads === true && !asked && !done;
  const downloads = useQuery({
    queryKey: ["downloads"],
    queryFn: api.downloads,
    staleTime: 0, // jobs change
    enabled: ask,
    refetchInterval: (q) => (q.state.data?.downloads.some((d) => d.job?.state === "running") ? 500 : false),
  });

  const all = downloads.data?.enabled ? downloads.data.downloads : [];
  const missing = all.filter((d) => !d.present);
  const shown = fetching ? all.filter((d) => fetching.includes(d.id)) : missing;
  const ready = (d: CorpusDownload) => d.present && d.job?.state !== "running";
  const finished = fetching !== null && shown.length > 0 && shown.every(ready);

  useEffect(() => {
    if (!finished) return;
    // into the interface, with the corpora open
    void client.invalidateQueries({ queryKey: ["corpora"] });
    void client.invalidateQueries({ queryKey: ["library"] });
    setDone(true);
  }, [finished, client]);

  // nothing to ask, or not yet known whether to: the interface (or nothing, briefly)
  if (health.isPending) return null;
  if (!ask) return <>{children}</>;
  if (downloads.isPending) return null;
  if (fetching === null && missing.length === 0) return <>{children}</>;

  const start = async (ids: string[]) => {
    setFetching((current) => [...new Set([...(current ?? []), ...ids])]);
    await Promise.all(ids.map((id) => api.startDownload(id)));
    await client.invalidateQueries({ queryKey: ["downloads"] });
  };
  const cancel = async () => {
    await Promise.all(shown.filter((d) => d.job?.state === "running").map((d) => api.cancelDownload(d.id)));
    await client.invalidateQueries({ queryKey: ["downloads"] });
    // back to the question; what is downloaded already stays
    setFetching(null);
  };
  const notNow = () => {
    rememberAsked();
    setAsked(true);
  };
  const running = shown.some((d) => d.job?.state === "running");
  const failed = shown.filter((d) => !d.present && d.job?.state === "failed");
  const total = missing.reduce((n, d) => n + d.size, 0);

  return (
    <main className="mx-auto flex min-h-full max-w-[40rem] flex-col gap-4 px-4 py-10 text-ink">
      <h1 className="text-[22px] font-semibold">Getting the corpora</h1>
      {fetching === null ? (
        <p className="text-[15px] leading-relaxed">
          These corpora are not on this computer yet. The program can download them from their own releases on GitHub,
          the same files Text-Fabric uses, and put them where Text-Fabric keeps its downloads. After downloading, each
          is prepared once, which takes a few minutes.
        </p>
      ) : (
        <p className="text-[15px] leading-relaxed">
          Each corpus is downloaded, unpacked and prepared once; after that the program starts in a few seconds. You can
          leave this window open and come back.
        </p>
      )}
      <ul className="flex flex-col rounded border border-rule bg-raised">
        {shown.map((d) => (
          <li key={d.id} className="flex items-start gap-3 border-b border-rule px-4 py-3 last:border-b-0">
            <div className="min-w-0 flex-1">
              <p className="font-semibold">{d.title.split(" = ").pop()}</p>
              <p className="text-[13px] text-stone">
                {d.id}, version {d.version}: {megabytes(d.size)} to download, {megabytes(d.unpacked)} on disk.
                {d.licence ? ` Licence: ${d.licence}.` : ""}{" "}
                <a href={d.terms} target="_blank" rel="noreferrer" className="text-woad hover:underline">
                  Terms of use
                </a>
              </p>
              {d.job?.state === "failed" ? (
                <p className="mt-1 text-[13px] text-madder" role="alert">
                  {d.job.error}
                </p>
              ) : null}
            </div>
            <span className="shrink-0 pt-0.5 text-[13px] text-stone" role="status">
              {status(d, fetching !== null)}
            </span>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap items-center gap-3">
        {fetching === null ? (
          <>
            <button
              type="button"
              onClick={() => void start(missing.map((d) => d.id))}
              className="rounded bg-woad px-4 py-1.5 text-[14px] font-semibold text-vellum hover:opacity-90"
            >
              Download ({megabytes(total)})
            </button>
            <button
              type="button"
              onClick={notNow}
              className="rounded border border-rule-strong px-4 py-1.5 text-[14px] hover:border-woad"
            >
              Not now
            </button>
            <span className="text-[13px] text-stone">You can also download them later, in Open a corpus.</span>
          </>
        ) : running ? (
          <button
            type="button"
            onClick={() => void cancel()}
            className="rounded border border-rule-strong px-4 py-1.5 text-[14px] hover:border-woad"
          >
            Cancel
          </button>
        ) : failed.length ? (
          <>
            <p className="w-full text-[14px]">
              {failed.length === 1 ? "One corpus is" : `${failed.length} corpora are`} not there yet. Without an
              internet connection the program can't download them; the others can be used already.
            </p>
            <button
              type="button"
              onClick={() => void start(failed.map((d) => d.id))}
              className="rounded bg-woad px-4 py-1.5 text-[14px] font-semibold text-vellum hover:opacity-90"
            >
              Try again
            </button>
            <button
              type="button"
              onClick={() => setDone(true)}
              className="rounded border border-rule-strong px-4 py-1.5 text-[14px] hover:border-woad"
            >
              Go on without {failed.length === 1 ? "it" : "them"}
            </button>
          </>
        ) : null}
      </div>
    </main>
  );
}

/** Where a corpus is: waiting, downloading, unpacking, preparing, ready. */
function status(d: CorpusDownload, started: boolean): string {
  const job = d.job;
  if (job?.state === "running") {
    if (job.step === "download") return `${megabytes(job.done)} of ${megabytes(job.total || d.size)}`;
    if (job.step === "unpack") return "Unpacking…";
    return "Preparing… (a few minutes)";
  }
  if (d.present) return "Ready";
  if (job?.state === "cancelled") return "Cancelled";
  if (job?.state === "failed") return "Not downloaded";
  return started ? "Waiting…" : megabytes(d.size);
}
