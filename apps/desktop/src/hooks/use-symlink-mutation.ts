import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";
import { useConfig } from "./use-config";

export function useSymlinkMutation(dotIndex: number, dotName: string) {
  const queryClient = useQueryClient();
  const { data: config } = useConfig();

  return useMutation({
    mutationFn: async (enable: boolean) => {
      if (!config) throw new Error("Config not loaded");
      const dot = config.dots[dotIndex];
      if (!dot) throw new Error("Dotfile not found");
      const updatedDot = { ...dot, symlink: enable };
      const dots = config.dots.map((d, i) => (i === dotIndex ? updatedDot : d));
      await ipc.saveConfig({ ...config, dots });
      if (enable) await ipc.backupOne(dotName);
      else await ipc.restoreOne(dotName);
    },
    onSuccess: (_, enable) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.config() });
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      toast.success(enable ? "Symlink enabled" : "Symlink removed");
    },
    onError: (e) => toast.error(String(e)),
  });
}
