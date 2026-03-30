import { createFileRoute, Link } from "@tanstack/react-router";
import {
  AlertTriangle,
  ArrowDownToLine,
  ArrowUpFromLine,
  CheckCircle,
  CheckCircle2,
  HardDrive,
  Link2,
  Loader2,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  Search,
  Trash2,
  XCircle,
} from "lucide-react";
import { useState } from "react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { DotfileDialog } from "@/components/dotfile-dialog";
import { useConfig } from "@/hooks/use-config";
import { useStatus } from "@/hooks/use-status";
import { useBackupAll, useBackupOne, useRestoreAll, useRestoreOne } from "@/hooks/use-backup-restore";
import { useDeleteDotfile } from "@/hooks/use-delete-dotfile";
import { useSymlinkMutation } from "@/hooks/use-symlink-mutation";
import { ipc, type Dotfile, type DotfileStatus, type RunResult } from "@/lib/ipc";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/")({
  component: DotsView,
});

function DotsView() {
  const [search, setSearch] = useState("");

  const { data: statuses, isLoading, error, refetch } = useStatus();
  const { data: config } = useConfig();

  const hasSymlinkedDots = config?.dots.some((d) => d.symlink) ?? false;

  const backupMutation = useBackupAll();
  const restoreMutation = useRestoreAll();
  const backupOneMutation = useBackupOne();
  const restoreOneMutation = useRestoreOne();
  const deleteMutation = useDeleteDotfile();

  const isBusy =
    backupMutation.isPending ||
    restoreMutation.isPending ||
    backupOneMutation.isPending ||
    restoreOneMutation.isPending;

  const q = search.toLowerCase();
  const filtered = statuses?.filter(
    (s) =>
      s.name.toLowerCase().includes(q) || s.source.toLowerCase().includes(q),
  );

  const stats = statuses
    ? {
        total: statuses.length,
        backedUp: statuses.filter((s) => s.backed_up).length,
        symlinked: statuses.filter((s) => s.symlinked).length,
        issues: statuses.filter((s) => s.missing_deps.length > 0 || s.pending_setup.length > 0)
          .length,
      }
    : null;

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-6 py-4 gap-4">
        <div className="min-w-0">
          <h1 className="text-base font-semibold text-foreground">Dotfiles</h1>
          {stats && (
            <p className="mt-0.5 text-sm text-muted-foreground">
              {stats.backedUp}/{stats.total} backed up
              {stats.symlinked > 0 && ` · ${stats.symlinked} symlinked`}
              {stats.issues > 0 && (
                <span className="text-yellow-500"> · {stats.issues} with issues</span>
              )}
              {search && filtered && (
                <span className="text-muted-foreground/60">
                  {" "}· showing {filtered.length}
                </span>
              )}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {/* Search */}
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Filter…"
              className="h-7 w-28 pl-7 text-xs focus:w-40 transition-all duration-200"
            />
          </div>

          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => refetch()}
            disabled={isLoading}
            title="Refresh"
          >
            <RefreshCw className={cn("size-3.5", isLoading && "animate-spin")} />
          </Button>

          {/* Restore All */}
          <Button
            variant="outline"
            size="sm"
            onClick={() => restoreMutation.mutate()}
            disabled={isBusy || !statuses?.length}
          >
            {restoreMutation.isPending ? <Loader2 className="animate-spin" /> : <ArrowDownToLine />}
            Restore All
          </Button>

          {/* Backup All — confirm if any dotfiles use symlink mode */}
          {hasSymlinkedDots ? (
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button size="sm" disabled={isBusy || !statuses?.length}>
                  {backupMutation.isPending ? (
                    <Loader2 className="animate-spin" />
                  ) : (
                    <ArrowUpFromLine />
                  )}
                  Backup All
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>Backup All — symlink mode active</AlertDialogTitle>
                  <AlertDialogDescription>
                    One or more dotfiles are configured with{" "}
                    <span className="font-medium text-foreground">symlink = true</span>. During
                    backup, the source file or folder will be{" "}
                    <span className="font-medium text-foreground">deleted</span> and replaced with a
                    symlink pointing into your vault. This cannot be undone without a restore.
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>Cancel</AlertDialogCancel>
                  <AlertDialogAction onClick={() => backupMutation.mutate()}>
                    Backup anyway
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          ) : (
            <Button
              size="sm"
              onClick={() => backupMutation.mutate()}
              disabled={isBusy || !statuses?.length}
            >
              {backupMutation.isPending ? (
                <Loader2 className="animate-spin" />
              ) : (
                <ArrowUpFromLine />
              )}
              Backup All
            </Button>
          )}

          <DotfileDialog mode="add">
            <Button variant="outline" size="sm">
              <Plus />
              Add
            </Button>
          </DotfileDialog>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {isLoading && (
          <div className="flex items-center justify-center py-20 text-muted-foreground">
            <Loader2 className="mr-2 size-4 animate-spin" />
            Loading…
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {String(error)}
          </div>
        )}

        {statuses && statuses.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-3 py-20 text-muted-foreground">
            <HardDrive className="size-10 opacity-30" />
            <p className="text-sm">No dotfiles configured yet.</p>
            <p className="text-xs opacity-70">
              Use the <span className="font-medium">Add</span> button above or edit your
              omah-config.toml directly.
            </p>
          </div>
        )}

        {filtered && statuses && statuses.length > 0 && filtered.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-2 py-16 text-muted-foreground">
            <Search className="size-8 opacity-20" />
            <p className="text-sm">No dotfiles match "{search}"</p>
          </div>
        )}

        {filtered && filtered.length > 0 && (
          <div className="space-y-2">
            {filtered.map((dot) => {
              const dotIndex = config?.dots.findIndex((d) => d.name === dot.name) ?? -1;
              const dotfileConfig = dotIndex >= 0 ? config?.dots[dotIndex] : undefined;
              return (
                <DotCard
                  key={dot.name}
                  dot={dot}
                  dotIndex={dotIndex}
                  dotfileConfig={dotfileConfig}
                  onBackup={() => backupOneMutation.mutate(dot.name)}
                  onRestore={() => restoreOneMutation.mutate(dot.name)}
                  onDelete={() => deleteMutation.mutate(dotIndex)}
                  disabled={isBusy || deleteMutation.isPending}
                />
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Dot card ──────────────────────────────────────────────────────────────────

function DotCard({
  dot,
  dotIndex,
  dotfileConfig,
  onBackup,
  onRestore,
  onDelete,
  disabled,
}: {
  dot: DotfileStatus;
  dotIndex: number;
  dotfileConfig: Dotfile | undefined;
  onBackup: () => void;
  onRestore: () => void;
  onDelete: () => void;
  disabled: boolean;
}) {
  const symlinkMutation = useSymlinkMutation(dotIndex, dot.name);
  const [confirmSymlink, setConfirmSymlink] = useState(false);

  function handleSymlinkChange(checked: boolean) {
    if (checked) {
      setConfirmSymlink(true);
    } else {
      symlinkMutation.mutate(false);
    }
  }

  const hasIssues = dot.missing_deps.length > 0 || dot.pending_setup.length > 0;

  // Left accent color
  const accentClass = !dot.source_exists
    ? "before:bg-muted-foreground/30"
    : hasIssues
      ? "before:bg-yellow-500"
      : dot.backed_up
        ? "before:bg-green-500"
        : "before:bg-red-500";

  return (
    <div
      className={cn(
        "group relative rounded-lg border bg-card transition-colors",
        "before:absolute before:left-0 before:top-0 before:bottom-0 before:w-0.75 before:rounded-l-lg",
        accentClass,
        hasIssues
          ? "border-yellow-500/20 hover:border-yellow-500/35"
          : "border-border hover:border-border/80",
      )}
    >
      <div className="flex items-start justify-between gap-4 px-4 py-3 pl-5">
        {/* Info */}
        <Link
          to="/dotfile/$name"
          params={{ name: dot.name }}
          className="min-w-0 flex-1 cursor-pointer"
        >
          {/* Title row */}
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-medium text-foreground group-hover/card:underline decoration-muted-foreground/30 underline-offset-2">
              {dot.name}
            </span>
            <StatusBadge dot={dot} />
            {dot.symlinked && (
              <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-[11px] text-blue-400">
                <Link2 className="size-2.5" />
                symlink
              </span>
            )}
          </div>

          {/* Source path */}
          <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">{dot.source}</p>

          {/* Deps chips — all defined deps, missing ones highlighted */}
          {dotfileConfig?.deps && dotfileConfig.deps.length > 0 && (
            <div className="mt-1.5 flex flex-wrap gap-1">
              {dotfileConfig.deps.map((dep) => {
                const missing = dot.missing_deps.includes(dep);
                return (
                  <span
                    key={dep}
                    className={cn(
                      "inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-mono text-[11px]",
                      missing
                        ? "bg-yellow-500/10 text-yellow-500"
                        : "bg-muted text-muted-foreground",
                    )}
                  >
                    {missing && <AlertTriangle className="size-2.5 shrink-0" />}
                    {dep}
                  </span>
                );
              })}
            </div>
          )}

          {/* Missing deps not in the defined list (edge case) */}
          {dot.missing_deps
            .filter((d) => !dotfileConfig?.deps?.includes(d))
            .map((dep) => (
              <p
                key={dep}
                className="mt-1.5 flex items-center gap-1 text-xs text-yellow-500"
              >
                <AlertTriangle className="size-3 shrink-0" />
                missing: {dep}
              </p>
            ))}

          {/* Pending setup steps */}
          {dot.pending_setup.map((cmd, i) => (
            <SetupStepRow key={i} command={cmd} />
          ))}

          {/* All setup steps done indicator */}
          {dotfileConfig?.setup &&
            dotfileConfig.setup.length > 0 &&
            dot.pending_setup.length === 0 && (
              <p className="mt-1.5 flex items-center gap-1 text-xs text-muted-foreground">
                <CheckCircle2 className="size-3 shrink-0 text-green-500" />
                {dotfileConfig.setup.length} setup step
                {dotfileConfig.setup.length !== 1 ? "s" : ""} complete
              </p>
            )}
        </Link>

        {/* Symlink toggle — always visible */}
        <div
          className="flex shrink-0 items-center gap-1.5 self-start pt-0.5"
          onClick={(e) => e.stopPropagation()}
        >
          <span className="text-[11px] text-muted-foreground select-none">symlink</span>
          <Switch
            checked={dotfileConfig?.symlink ?? false}
            onCheckedChange={handleSymlinkChange}
            disabled={disabled || symlinkMutation.isPending}
            aria-label="Toggle symlink mode"
          />
          <AlertDialog open={confirmSymlink} onOpenChange={setConfirmSymlink}>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Enable symlink mode for "{dot.name}"?</AlertDialogTitle>
                <AlertDialogDescription>
                  This will back up the source and{" "}
                  <span className="font-medium text-foreground">replace it with a symlink</span>{" "}
                  pointing to the vault. Run a restore to undo this.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    symlinkMutation.mutate(true);
                    setConfirmSymlink(false);
                  }}
                >
                  Enable symlink
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>

        {/* Actions — visible on hover */}
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
          <Button
            variant="ghost"
            size="icon-sm"
            disabled={disabled}
            onClick={onRestore}
            title="Restore this dotfile"
          >
            <ArrowDownToLine className="size-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            disabled={disabled}
            onClick={onBackup}
            title="Backup this dotfile"
          >
            <ArrowUpFromLine className="size-3.5" />
          </Button>

          <div className="mx-0.5 h-4 w-px bg-border" />

          <DotfileDialog mode="edit" dotfile={dotfileConfig} dotIndex={dotIndex}>
            <Button variant="ghost" size="icon-sm" disabled={disabled} title="Edit">
              <Pencil className="size-3.5" />
            </Button>
          </DotfileDialog>

          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                disabled={disabled}
                title="Remove from config"
                className="text-muted-foreground hover:text-destructive"
              >
                <Trash2 className="size-3.5" />
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Remove "{dot.name}"?</AlertDialogTitle>
                <AlertDialogDescription>
                  This removes the entry from your omah configuration. Your source files and any
                  existing vault copy are <strong>not</strong> deleted.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={onDelete}
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                >
                  Remove
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>
    </div>
  );
}

// ── Setup step row ────────────────────────────────────────────────────────────

function SetupStepRow({ command }: { command: string }) {
  const [result, setResult] = useState<RunResult | null>(null);
  const [running, setRunning] = useState(false);

  async function run() {
    setRunning(true);
    setResult(null);
    try {
      const r = await ipc.runSetupStep(command);
      setResult(r);
    } catch (e) {
      setResult({ success: false, output: String(e) });
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="mt-1.5 space-y-1">
      <div className="flex items-center gap-1.5">
        <AlertTriangle className="size-3 shrink-0 text-yellow-500/80" />
        <span className="truncate font-mono text-xs text-yellow-500/80">{command}</span>
        <button
          type="button"
          onClick={run}
          disabled={running}
          title="Run this setup step"
          className="ml-auto shrink-0 flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] text-yellow-500 hover:bg-yellow-500/10 disabled:opacity-50 transition-colors"
        >
          {running ? <Loader2 className="size-3 animate-spin" /> : <Play className="size-3" />}
          Run
        </button>
      </div>
      {result && (
        <div
          className={cn(
            "rounded px-2 py-1.5 font-mono text-[11px] leading-relaxed",
            result.success ? "bg-green-500/10 text-green-400" : "bg-red-500/10 text-red-400",
          )}
        >
          <div className="mb-1 flex items-center gap-1 font-sans font-medium">
            {result.success ? <CheckCircle className="size-3" /> : <XCircle className="size-3" />}
            {result.success ? "Done" : "Failed"}
          </div>
          {result.output && <pre className="whitespace-pre-wrap break-all">{result.output}</pre>}
        </div>
      )}
    </div>
  );
}

// ── Status badge ──────────────────────────────────────────────────────────────

function StatusBadge({ dot }: { dot: DotfileStatus }) {
  if (!dot.source_exists) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">
        <XCircle className="size-2.5" />
        source missing
      </span>
    );
  }
  if (dot.backed_up) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-2 py-0.5 text-[11px] text-green-500">
        <CheckCircle2 className="size-2.5" />
        backed up
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-red-500/10 px-2 py-0.5 text-[11px] text-red-400">
      <XCircle className="size-2.5" />
      not backed up
    </span>
  );
}
