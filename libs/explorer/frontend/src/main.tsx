import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Root } from "./Root";
import "./styles.css";

// A corpus doesn't change while it is open: what was fetched stays good (staleTime),
// so opening a view, pane or panel again doesn't fetch it again. What does change
// (searches, jobs, the library, proposals) is fetched again by invalidating it, or
// polls with its own settings.
const client = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 1, staleTime: Infinity } },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={client}>
      <Root />
    </QueryClientProvider>
  </StrictMode>,
);
