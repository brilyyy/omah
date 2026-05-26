import { useQuery } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";

export function useConfig() {
  return useQuery({
    queryKey: queryKeys.config(),
    queryFn: () => ipc.getConfig(),
  });
}
