import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";

export function useStatus() {
  return useQuery({
    queryKey: queryKeys.status(),
    queryFn: () => ipc.getStatus(),
  });
}
