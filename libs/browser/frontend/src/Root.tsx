import { useQuery } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useState } from "react";
import { App } from "./App";
import { rememberedUi, rememberUi, uiOfUrl, type Ui } from "./ui";

const ParallelApp = lazy(() => import("./parallel/ParallelApp").then((m) => ({ default: m.ParallelApp })));

/** Shows the interface the URL is for; at the bare start page the one used last,
 * or the server's default (`--ui`). */
export function Root() {
  const [fromUrl, setFromUrl] = useState<Ui | null>(() => uiOfUrl());
  useEffect(() => {
    const onPop = () => setFromUrl(uiOfUrl());
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const remembered = rememberedUi();
  const health = useQuery({
    queryKey: ["health"],
    queryFn: async () =>
      (await fetch("/api/health").then((r) => r.json())) as { version: string; coreVersion: string | null; ui?: Ui },
    enabled: fromUrl === null && remembered === null,
  });
  const ui: Ui | null =
    fromUrl ?? remembered ?? (health.data ? (health.data.ui ?? "classic") : health.isError ? "classic" : null);

  useEffect(() => {
    if (ui) rememberUi(ui);
  }, [ui]);

  if (ui === null) return null;
  if (ui === "classic") return <App key="classic" />;
  return (
    <Suspense fallback={<main className="p-10 text-stone">Opening…</main>}>
      <ParallelApp key="parallel" />
    </Suspense>
  );
}
