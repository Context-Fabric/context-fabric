import { useQuery } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useState } from "react";
import { App } from "./App";
import { NotYet } from "./NotYet";
import { INTERFACES, isUi, rememberedUi, rememberUi, uiOfUrl, type Ui } from "./ui";

const ParallelApp = lazy(() => import("./parallel/ParallelApp").then((m) => ({ default: m.ParallelApp })));
const ReaderApp = lazy(() => import("./reader/ReaderApp").then((m) => ({ default: m.ReaderApp })));
const ConcordanceApp = lazy(() => import("./concordance/ConcordanceApp").then((m) => ({ default: m.ConcordanceApp })));
const InterlinearApp = lazy(() => import("./interlinear/InterlinearApp").then((m) => ({ default: m.InterlinearApp })));

/** Shows the interface the URL is for; at the bare start page the one used last,
 * or the server's default (`--ui`). */
export function Root() {
  const [fromUrl, setFromUrl] = useState<Ui | null>(() => uiOfUrl());
  useEffect(() => {
    const onPop = () => setFromUrl(uiOfUrl());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  // only a built interface counts as remembered or as the server's default
  const ready = (ui: Ui | null | undefined): ui is Ui => isUi(ui) && INTERFACES.some((i) => i.id === ui && i.ready);
  const stored = rememberedUi();
  const remembered = ready(stored) ? stored : null;
  const health = useQuery({
    queryKey: ["health"],
    queryFn: async () =>
      (await fetch("/api/health").then((r) => r.json())) as { version: string; coreVersion: string | null; ui?: Ui },
    enabled: fromUrl === null && remembered === null,
  });
  const ui: Ui | null =
    fromUrl ??
    remembered ??
    (health.data ? (ready(health.data.ui) ? health.data.ui : "classic") : health.isError ? "classic" : null);

  useEffect(() => {
    if (ready(ui)) rememberUi(ui);
  }, [ui]);

  if (ui === null) return null;
  if (ui === "classic") return <App key="classic" />;
  if (!ready(ui)) return <NotYet ui={ui} />;
  return (
    <Suspense fallback={<main className="p-10 text-stone">Opening…</main>}>
      {ui === "parallel" ? (
        <ParallelApp key="parallel" />
      ) : ui === "reader" ? (
        <ReaderApp key="reader" />
      ) : ui === "interlinear" ? (
        <InterlinearApp key="interlinear" />
      ) : (
        <ConcordanceApp key="concordance" />
      )}
    </Suspense>
  );
}
