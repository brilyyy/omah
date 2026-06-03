import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ipc, type AppSettings } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";

export function useAppSettings() {
  return useQuery({
    queryKey: queryKeys.appSettings(),
    queryFn: () => ipc.getAppSettings(),
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useSaveAppSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (settings: AppSettings) => ipc.saveAppSettings(settings),
    onSuccess: (_, settings) => {
      queryClient.setQueryData(queryKeys.appSettings(), settings);
    },
    onError: (e) => toast.error(String(e)),
  });
}
