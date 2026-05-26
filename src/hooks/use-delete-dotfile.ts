import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { useConfig } from "./use-config";

export function useDeleteDotfile() {
  const queryClient = useQueryClient();
  const { data: config } = useConfig();

  return useMutation({
    mutationFn: (dotIndex: number) => {
      if (!config) throw new Error("Config not loaded");
      const dots = config.dots.filter((_, i) => i !== dotIndex);
      return ipc.saveConfig({ ...config, dots });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.config() });
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      toast.success("Dotfile removed");
    },
    onError: (e) => toast.error(String(e)),
  });
}
