import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";

export function useDiff() {
  return useQuery({
    queryKey: queryKeys.diff(),
    queryFn: () => ipc.getDiff(),
    staleTime: 0,                // always re-fetch when window regains focus
    refetchOnWindowFocus: true,  // pick up external file edits on app focus
  });
}
