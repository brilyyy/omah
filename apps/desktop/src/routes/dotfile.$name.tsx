import { createFileRoute, Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  ArrowDownToLine,
  ArrowLeft,
  ArrowUpFromLine,
  CheckCircle2,
  HardDrive,
  Link2,
  Loader2,
  Pencil,
  Play,
  SkipForward,
  XCircle,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { DotfileDialog } from "@/components/dotfile-dialog";
import { ipc, type SetupStep } from "@/lib/ipc";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/dotfile/$name")({
  component: DotfileDetail,
});

function DotfileDetail() {
  const { name } = Route.useParams();
  const queryClient = useQueryClient();

  const { data: config } = useQuery({
    queryKey: ["config"],
    queryFn: () => ipc.getConfig(),
  });
  const { data: statuses, isLoading } = useQuery({
    queryKey: ["status"],
    queryFn: () => ipc.getStatus(),
  });

  const dot = config?.dots.find((d) => d.name === name);
  const dotIndex = config?.dots.findIndex((d) => d.name === name) ?? -1;
  const status = statuses?.find((s) => s.name === name);

  const backupMutation = useMutation({
    mutationFn: () => ipc.backupOne(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["status"] });
      queryClient.invalidateQueries({ queryKey: ["diff"] });
      toast.success(`Backed up "${name}"`);
    },
    onError: (e) => toast.error(String(e)),
  });

  const restoreMutation = useMutation({
    mutationFn: () => ipc.restoreOne(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["status"] });
      toast.success(`Restored "${name}"`);
    },
    onError: (e) => toast.error(String(e)),
  });

  const skipStepMutation = useMutation({
    mutationFn: (stepIndex: number) => {
      if (!config || !dot) throw new Error("Config not loaded");
      const updatedSetup =
        dot.setup?.map((s, i) => (i === stepIndex ? { ...s, check: "skip" } : s)) ?? [];
      const updatedDot = { ...dot, setup: updatedSetup };
      const dots = config.dots.map((d, i) => (i === dotIndex ? updatedDot : d));
      return ipc.saveConfig({ ...config, dots });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["config"] });
      queryClient.invalidateQueries({ queryKey: ["status"] });
      toast.success("Step skipped");
    },
    onError: (e) => toast.error(String(e)),
  });

  const isBusy = backupMutation.isPending || restoreMutation.isPending;

  if (isLoading || !config) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 size-4 animate-spin" />
        Loading…
      </div>
    );
  }

  if (!dot || !status) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-muted-foreground">
        <HardDrive className="size-10 opacity-30" />
        <p className="text-sm">Dotfile "{name}" not found.</p>
        <Link to="/" className="text-xs text-primary underline-offset-4 hover:underline">
          Back to dotfiles
        </Link>
      </div>
    );
  }

  // Compute vault entry path for display
  const sourceBasename = dot.source.replace(/\/$/, "").split("/").pop() ?? dot.source;
  const vaultEntry = `${config.vault_path}/${dot.name}/${sourceBasename}`;

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-6 py-4 gap-4">
        <div className="flex items-center gap-3 min-w-0">
          <Link
            to="/"
            className="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
          </Link>
          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <h1 className="text-base font-semibold">{dot.name}</h1>
              <StatusBadge status={status} />
              {status.symlinked && (
                <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-[11px] text-blue-400">
                  <Link2 className="size-2.5" />
                  symlink
                </span>
              )}
            </div>
            <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
              {dot.source}
            </p>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => restoreMutation.mutate()}
            disabled={isBusy}
          >
            {restoreMutation.isPending ? (
              <Loader2 className="animate-spin" />
            ) : (
              <ArrowDownToLine />
            )}
            Restore
          </Button>
          <Button size="sm" onClick={() => backupMutation.mutate()} disabled={isBusy}>
            {backupMutation.isPending ? <Loader2 className="animate-spin" /> : <ArrowUpFromLine />}
            Backup
          </Button>
          <DotfileDialog mode="edit" dotfile={dot} dotIndex={dotIndex}>
            <Button variant="ghost" size="icon-sm" title="Edit">
              <Pencil className="size-3.5" />
            </Button>
          </DotfileDialog>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto px-6 py-5 space-y-5">
        {/* Locations */}
        <Section title="Locations">
          <div className="divide-y divide-border/50">
            <LocationRow label="Source" path={dot.source} ok={status.source_exists} />
            <LocationRow label="Vault" path={vaultEntry} ok={status.backed_up} />
          </div>
        </Section>

        {/* Dependencies */}
        {dot.deps && dot.deps.length > 0 && (
          <Section title="Dependencies">
            <div className="divide-y divide-border/50">
              {dot.deps.map((dep) => {
                const missing = status.missing_deps.includes(dep);
                return (
                  <div key={dep} className="flex items-center gap-3 py-2.5">
                    {missing ? (
                      <XCircle className="size-3.5 shrink-0 text-yellow-500" />
                    ) : (
                      <CheckCircle2 className="size-3.5 shrink-0 text-green-500" />
                    )}
                    <span
                      className={cn(
                        "flex-1 font-mono text-xs",
                        missing ? "text-yellow-500" : "text-foreground",
                      )}
                    >
                      {dep}
                    </span>
                    <span
                      className={cn(
                        "text-[11px]",
                        missing ? "text-yellow-500/70" : "text-muted-foreground",
                      )}
                    >
                      {missing ? "not found" : "installed"}
                    </span>
                  </div>
                );
              })}
            </div>
          </Section>
        )}

        {/* Setup steps */}
        {dot.setup && dot.setup.length > 0 && (
          <Section title={`Setup steps · ${dot.setup.length}`}>
            <div className="divide-y divide-border/50">
              {dot.setup.map((step, i) => {
                const isSkipped =
                  step.check === "skip" || (step.check?.startsWith("skip:") ?? false);
                const isPending = !isSkipped && status.pending_setup.includes(step.install);
                const isDone = !isSkipped && !isPending;
                return (
                  <SetupStepRow
                    key={i}
                    step={step}
                    isSkipped={isSkipped}
                    isPending={isPending}
                    isDone={isDone}
                    onSkip={() => skipStepMutation.mutate(i)}
                    skipDisabled={skipStepMutation.isPending}
                  />
                );
              })}
            </div>
          </Section>
        )}

        {/* No issues notice */}
        {(!dot.deps || dot.deps.length === 0) &&
          (!dot.setup || dot.setup.length === 0) && (
            <div className="flex flex-col items-center justify-center gap-2 py-10 text-muted-foreground">
              <CheckCircle2 className="size-8 opacity-30" />
              <p className="text-sm">No dependencies or setup steps defined.</p>
            </div>
          )}
      </div>
    </div>
  );
}

// ── Setup step row ────────────────────────────────────────────────────────────

type TerminalLine = { text: string; isStderr: boolean; key: number };

function SetupStepRow({
  step,
  isSkipped,
  isPending,
  isDone,
  onSkip,
  skipDisabled,
}: {
  step: SetupStep;
  isSkipped: boolean;
  isPending: boolean;
  isDone: boolean;
  onSkip: () => void;
  skipDisabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [running, setRunning] = useState(false);
  const [termSuccess, setTermSuccess] = useState<boolean | null>(null);
  const termBodyRef = useRef<HTMLDivElement>(null);
  const lineKeyRef = useRef(0);

  const showTerminal = running || termSuccess !== null;

  async function run() {
    const runId = `${Date.now()}-${Math.random().toString(36).slice(2)}`;
    setLines([]);
    setTermSuccess(null);
    setRunning(true);
    try {
      await ipc.runSetupStepStream(runId, step.install, (event) => {
        if (event.done) {
          setRunning(false);
          setTermSuccess(event.success ?? false);
          if (event.success) queryClient.invalidateQueries({ queryKey: ["status"] });
        } else if (event.line) {
          setLines((prev) => [
            ...prev,
            { text: event.line, isStderr: event.is_stderr, key: lineKeyRef.current++ },
          ]);
        }
      });
    } catch (e) {
      setRunning(false);
      setTermSuccess(false);
      setLines((prev) => [
        ...prev,
        { text: String(e), isStderr: true, key: lineKeyRef.current++ },
      ]);
    }
  }

  // Auto-scroll terminal to bottom on new lines
  useEffect(() => {
    if (termBodyRef.current) {
      termBodyRef.current.scrollTop = termBodyRef.current.scrollHeight;
    }
  }, [lines]);

  // Human-readable check description
  const checkLabel = (() => {
    const c = step.check;
    if (!c) return null;
    if (c === "skip" || c.startsWith("skip:")) return "permanently skipped";
    if (c.startsWith("bin:")) return `binary: ${c.slice(4)}`;
    if (c.startsWith("file:")) return `file: ${c.slice(5)}`;
    if (c.startsWith("dir:")) return `dir: ${c.slice(4)}`;
    if (c.startsWith("cmd:")) return `command: ${c.slice(4)}`;
    return c;
  })();

  return (
    <div className="py-2.5 space-y-2">
      <div className="flex items-start gap-3">
        {/* State icon */}
        <span className="mt-0.5 shrink-0">
          {isSkipped ? (
            <SkipForward className="size-3.5 text-muted-foreground/50" />
          ) : isDone ? (
            <CheckCircle2 className="size-3.5 text-green-500" />
          ) : (
            <AlertTriangle className="size-3.5 text-yellow-500" />
          )}
        </span>

        {/* Command + check */}
        <div className="min-w-0 flex-1 space-y-0.5">
          <p
            className={cn(
              "truncate font-mono text-xs",
              isSkipped ? "text-muted-foreground/50 line-through" : "text-foreground",
            )}
          >
            {step.install}
          </p>
          {checkLabel && (
            <p className="font-mono text-[11px] text-muted-foreground">{checkLabel}</p>
          )}
          {!step.check && isPending && (
            <p className="text-[11px] text-muted-foreground/60">no check defined</p>
          )}
        </div>

        {/* State label + actions */}
        <div className="shrink-0 flex items-center gap-1">
          {isSkipped && <span className="text-[11px] text-muted-foreground/40">skipped</span>}
          {isDone && <span className="text-[11px] text-green-500/70">done</span>}
          {isPending && (
            <>
              <button
                type="button"
                onClick={run}
                disabled={running}
                className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] text-yellow-500 hover:bg-yellow-500/10 disabled:opacity-50 transition-colors"
              >
                {running ? <Loader2 className="size-3 animate-spin" /> : <Play className="size-3" />}
                Run
              </button>
              <button
                type="button"
                onClick={onSkip}
                disabled={skipDisabled}
                title="Mark as done — won't show as pending again"
                className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] text-muted-foreground hover:bg-muted disabled:opacity-50 transition-colors"
              >
                <SkipForward className="size-3" />
                Skip
              </button>
            </>
          )}
        </div>
      </div>

      {/* Animated terminal */}
      {showTerminal && (
        <div className="ml-6 rounded-lg overflow-hidden border border-white/10 bg-[#0e0a05] shadow-lg">
          {/* Title bar */}
          <div className="flex items-center gap-1.5 px-3 py-1.5 bg-[#1c1209] border-b border-white/8">
            <span className="size-2.5 rounded-full bg-[#ff5f57]" />
            <span className="size-2.5 rounded-full bg-[#febc2e]" />
            <span className="size-2.5 rounded-full bg-[#28c840]" />
            <span className="ml-3 flex-1 truncate font-mono text-[10px] text-white/35 select-none">
              {step.install}
            </span>
            <span
              className={cn(
                "shrink-0 text-[10px] font-medium transition-colors",
                running
                  ? "text-yellow-400/80"
                  : termSuccess
                    ? "text-green-400/80"
                    : "text-red-400/80",
              )}
            >
              {running ? "running…" : termSuccess ? "✓ done" : "✗ failed"}
            </span>
          </div>

          {/* Output lines */}
          <div
            ref={termBodyRef}
            className="max-h-52 overflow-y-auto px-3 py-2 space-y-px scrollbar-thin scrollbar-track-transparent scrollbar-thumb-white/10"
          >
            {lines.length === 0 && running && (
              <span className="text-white/25 font-mono text-[11px]">$ {step.install}</span>
            )}
            {lines.map((line) => (
              <div
                key={line.key}
                className={cn(
                  "font-mono text-[11px] leading-relaxed whitespace-pre-wrap break-all",
                  "animate-[term-line_0.12s_ease-out]",
                  line.isStderr ? "text-red-400/90" : "text-[#d4c4a0]",
                )}
              >
                {line.text}
              </div>
            ))}
            {running && (
              <span className="inline-block w-1.75 h-3.25 bg-[#d4c4a0]/70 animate-[cursor-blink_1s_step-end_infinite] align-middle" />
            )}
            {!running && termSuccess !== null && lines.length === 0 && (
              <span className="font-mono text-[11px] text-white/30">(no output)</span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Shared helpers ────────────────────────────────────────────────────────────

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <p className="mb-2 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </p>
      <div className="rounded-lg border border-border bg-card px-4">{children}</div>
    </div>
  );
}

function LocationRow({ label, path, ok }: { label: string; path: string; ok: boolean }) {
  return (
    <div className="flex items-center gap-3 py-2.5">
      <span className="w-10 shrink-0 text-[11px] uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      <span className="flex-1 truncate font-mono text-xs text-foreground" title={path}>
        {path}
      </span>
      {ok ? (
        <CheckCircle2 className="size-3.5 shrink-0 text-green-500" />
      ) : (
        <XCircle className="size-3.5 shrink-0 text-muted-foreground/40" />
      )}
    </div>
  );
}

function StatusBadge({ status }: { status: { source_exists: boolean; backed_up: boolean } }) {
  if (!status.source_exists) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">
        <XCircle className="size-2.5" />
        source missing
      </span>
    );
  }
  if (status.backed_up) {
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
