import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc";
import { queryKeys } from "@/lib/query-keys";

export function useBackupAll() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.backupAll(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.diff() });
      toast.success("All dotfiles backed up");
    },
    onError: (e) => toast.error(String(e)),
  });
}

export function useRestoreAll() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => ipc.restoreAll(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      toast.success("All dotfiles restored");
    },
    onError: (e) => toast.error(String(e)),
  });
}

export function useBackupOne() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => ipc.backupOne(name),
    onSuccess: (_, name) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.diff() });
      toast.success(`Backed up "${name}"`);
    },
    onError: (e) => toast.error(String(e)),
  });
}

export function useRestoreOne() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => ipc.restoreOne(name),
    onSuccess: (_, name) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      toast.success(`Restored "${name}"`);
    },
    onError: (e) => toast.error(String(e)),
  });
}
