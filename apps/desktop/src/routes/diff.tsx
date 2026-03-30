import { createFileRoute } from "@tanstack/react-router";
import { FileDiff, Loader2, Minus, Pencil, Plus, RefreshCw, Search } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useDiff } from "@/hooks/use-diff";
import { type FileChange } from "@/lib/ipc";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/diff")({
  component: DiffView,
});

function DiffView() {
  const { data: changes, isLoading, error, refetch } = useDiff();
  const [search, setSearch] = useState("");

  const grouped = changes
    ? changes.reduce<Record<string, FileChange[]>>((acc, c) => {
        (acc[c.dot_name] ??= []).push(c);
        return acc;
      }, {})
    : null;

  const filteredGrouped = grouped
    ? Object.fromEntries(
        Object.entries(grouped).filter(([name]) =>
          name.toLowerCase().includes(search.toLowerCase()),
        ),
      )
    : null;

  const totalChanges = changes?.length ?? 0;
  const visibleChanges = filteredGrouped
    ? Object.values(filteredGrouped).reduce((sum, files) => sum + files.length, 0)
    : 0;

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-6 py-4">
        <div>
          <h1 className="text-base font-semibold text-foreground">Diff</h1>
          {changes !== undefined && (
            <p className="mt-0.5 text-sm text-muted-foreground">
              {totalChanges === 0
                ? "Everything is in sync"
                : `${totalChanges} change${totalChanges !== 1 ? "s" : ""} since last backup`}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {totalChanges > 0 && (
            <div className="relative">
              <Search className="pointer-events-none absolute left-2.5 top-1/2 size-3 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Filter dotfiles…"
                className="h-7 w-32 pl-7 text-xs focus:w-44 transition-all duration-200"
              />
            </div>
          )}
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => refetch()}
            disabled={isLoading}
            title="Refresh"
          >
            <RefreshCw className={cn("size-3.5", isLoading && "animate-spin")} />
          </Button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto px-6 py-4">
        {isLoading && (
          <div className="flex items-center justify-center py-20 text-muted-foreground">
            <Loader2 className="mr-2 size-4 animate-spin" />
            Comparing…
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {String(error)}
          </div>
        )}

        {grouped && totalChanges === 0 && (
          <div className="flex flex-col items-center justify-center gap-3 py-20 text-muted-foreground">
            <FileDiff className="size-10 opacity-30" />
            <p className="text-sm">No changes detected.</p>
            <p className="text-xs opacity-70">Your source files match the vault snapshot.</p>
          </div>
        )}

        {filteredGrouped && totalChanges > 0 && visibleChanges === 0 && search && (
          <div className="flex flex-col items-center justify-center gap-2 py-16 text-muted-foreground">
            <Search className="size-8 opacity-20" />
            <p className="text-sm">No dotfiles match "{search}"</p>
          </div>
        )}

        {filteredGrouped && visibleChanges > 0 && (
          <div className="space-y-4">
            {Object.entries(filteredGrouped).map(([dotName, files]) => (
              <DotDiff key={dotName} name={dotName} files={files} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function DotDiff({ name, files }: { name: string; files: FileChange[] }) {
  const added = files.filter((f) => f.kind === "Added").length;
  const modified = files.filter((f) => f.kind === "Modified").length;
  const removed = files.filter((f) => f.kind === "Removed").length;

  return (
    <div className="rounded-lg border border-border bg-card overflow-hidden">
      <div className="flex items-center gap-3 border-b border-border bg-muted/30 px-4 py-2.5">
        <span className="font-medium text-sm text-foreground">{name}</span>
        <div className="flex items-center gap-2 text-xs">
          {added > 0 && <span className="text-green-500">+{added}</span>}
          {modified > 0 && <span className="text-yellow-500">~{modified}</span>}
          {removed > 0 && <span className="text-red-400">-{removed}</span>}
        </div>
      </div>
      <div className="divide-y divide-border/50">
        {files.map((f, i) => (
          <FileRow key={i} change={f} />
        ))}
      </div>
    </div>
  );
}

function FileRow({ change }: { change: FileChange }) {
  const config = {
    Added: {
      icon: Plus,
      color: "text-green-500",
      bg: "bg-green-500/5",
      label: "added",
    },
    Modified: {
      icon: Pencil,
      color: "text-yellow-500",
      bg: "bg-yellow-500/5",
      label: "modified",
    },
    Removed: {
      icon: Minus,
      color: "text-red-400",
      bg: "bg-red-500/5",
      label: "removed",
    },
  }[change.kind];

  const Icon = config.icon;

  return (
    <div className={cn("flex items-center gap-3 px-4 py-2 text-sm", config.bg)}>
      <Icon className={cn("size-3.5 shrink-0", config.color)} />
      <span className={cn("font-mono flex-1 truncate", config.color)}>{change.path}</span>
      <span className="text-xs text-muted-foreground">{config.label}</span>
    </div>
  );
}
