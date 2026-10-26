import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** What the server says about itself: versions, the interfaces to start in and to
 * hide, whether to offer downloads. It doesn't change while the page is open. */
export function useHealth(enabled = true) {
  return useQuery({ queryKey: ["health"], queryFn: api.health, staleTime: Infinity, enabled });
}
