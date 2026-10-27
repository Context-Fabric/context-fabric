import { QueryClient } from "@tanstack/react-query";

// The one cache of the page, also for code outside components (switching interfaces).
// A corpus doesn't change while it is open: what was fetched stays good (staleTime),
// so opening a view, pane or panel again doesn't fetch it again. What does change
// (searches, jobs, the library, proposals) is fetched again by invalidating it, or
// polls with its own settings.
export const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 1, staleTime: Infinity } },
});
