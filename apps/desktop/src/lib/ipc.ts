import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ── Domain types (mirror Rust DTOs in src-tauri/src/types.rs) ───────────────

export type DotfileStatus = {
  name: string;
  source: string;
  source_exists: boolean;
  backed_up: boolean;
  symlinked: boolean;
  missing_deps: string[];
  pending_setup: string[];
};

export type SetupStep = {
  check: string | null;
  install: string;
};

export type Dotfile = {
  name: string;
  source: string;
  symlink: boolean | null;
  deps: string[] | null;
  exclude: string[] | null;
  setup: SetupStep[] | null;
};

export type Config = {
  vault_path: string;
  git: boolean | null;
  os: string | null;
  pkg_manager: string | null;
  dots: Dotfile[];
};

export type ChangeKind = "Added" | "Modified" | "Removed";

export type FileChange = {
  dot_name: string;
  path: string;
  kind: ChangeKind;
};

export type RunResult = {
  success: boolean;
  output: string;
};

export type SetupStepOutputEvent = {
  run_id: string;
  line: string;
  is_stderr: boolean;
  done: boolean;
  success: boolean | null;
};

// ── Typed wrappers around Tauri invoke ──────────────────────────────────────
// Result-returning Rust commands throw (reject) on Err — handle via
// try/catch or React Query's onError / throwOnError options.

export const ipc = {
  getVersion: () => invoke<string>("get_version"),
  getConfig: () => invoke<Config>("get_config"),
  saveConfig: (config: Config) => invoke<void>("save_config", { config }),
  getStatus: () => invoke<DotfileStatus[]>("get_status"),
  backupAll: () => invoke<void>("backup_all"),
  restoreAll: () => invoke<void>("restore_all"),
  getDiff: () => invoke<FileChange[]>("get_diff"),
  backupOne: (name: string) => invoke<void>("backup_one", { name }),
  restoreOne: (name: string) => invoke<void>("restore_one", { name }),
  runSetupStep: (command: string) => invoke<RunResult>("run_setup_step", { command }),

  /** Stream setup step output line-by-line via Tauri events.
   *  `onEvent` is called for each line and once more with `done: true` when finished.
   *  Resolves when the process exits. */
  runSetupStepStream: async (
    runId: string,
    command: string,
    onEvent: (event: SetupStepOutputEvent) => void,
  ): Promise<void> => {
    // Subscribe BEFORE invoking to avoid missing early output lines
    const unlisten = await listen<SetupStepOutputEvent>("setup_step_output", (e) => {
      if (e.payload.run_id !== runId) return;
      onEvent(e.payload);
      if (e.payload.done) unlisten();
    });
    try {
      await invoke<void>("run_setup_step_stream", { runId, command });
    } catch (err) {
      unlisten();
      throw err;
    }
  },
};
