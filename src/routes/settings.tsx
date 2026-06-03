import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowUpRight, CheckCircle2, Loader2, RotateCcw, Save } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAppSettings, useSaveAppSettings } from "@/hooks/use-app-settings";
import { ipc, type Config, type UpdateInfo } from "@/lib/ipc";

export const Route = createFileRoute("/settings")({
  component: SettingsView,
});

const PKG_MANAGERS = [
  "auto",
  "brew",
  "apt-get",
  "pacman",
  "dnf",
  "zypper",
] as const;
const OS_OPTIONS = ["auto", "macos", "linux"] as const;

function SettingsView() {
  const queryClient = useQueryClient();

  const { data: config, isLoading } = useQuery({
    queryKey: ["config"],
    queryFn: () => ipc.getConfig(),
  });

  const [form, setForm] = useState<Partial<Config>>({});
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (config) {
      setForm(config);
      setDirty(false);
    }
  }, [config]);

  const saveMutation = useMutation({
    mutationFn: (updated: Config) => ipc.saveConfig(updated),
    onSuccess: (_, updated) => {
      queryClient.setQueryData(["config"], updated);
      setDirty(false);
      toast.success("Settings saved");
    },
    onError: (e) => toast.error(String(e)),
  });

  function handleReset() {
    if (config) {
      setForm(config);
      setDirty(false);
    }
  }

  function update<K extends keyof Config>(key: K, value: Config[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
    setDirty(true);
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center text-muted-foreground">
        <Loader2 className="mr-2 size-4 animate-spin" />
        Loading config…
      </div>
    );
  }

  if (!config) return null;

  return (
    <div className="flex h-full flex-col">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-6 py-4">
        <div>
          <h1 className="text-base font-semibold text-foreground">Settings</h1>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Saved to{" "}
            <span className="font-mono text-xs">
              ~/.config/omah/omah-config.toml
            </span>
          </p>
        </div>
        <div className="flex items-center gap-2">
          {dirty && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleReset}
              disabled={saveMutation.isPending}
            >
              <RotateCcw />
              Reset
            </Button>
          )}
          <Button
            size="sm"
            onClick={() => saveMutation.mutate(form as Config)}
            disabled={!dirty || saveMutation.isPending || !form.vault_path?.trim()}
          >
            {saveMutation.isPending ? (
              <Loader2 className="animate-spin" />
            ) : (
              <Save />
            )}
            Save
          </Button>
        </div>
      </div>

      {/* Form */}
      <div className="flex-1 overflow-auto px-6 py-6">
        <div className="mx-auto max-w-xl space-y-6">
          {/* App settings */}
          <AppSection />

          <div className="border-t border-border" />

          {/* Vault path */}
          <Field label="Vault path" description="Where dotfiles are stored">
            <Input
              value={form.vault_path ?? ""}
              onChange={(e) => update("vault_path", e.target.value)}
              className="font-mono text-xs w-64"
              placeholder="~/Documents/OmahVault"
            />
          </Field>

          {/* OS */}
          <Field
            label="OS"
            description="Target operating system for this machine"
          >
            <Select
              value={form.os ?? "auto"}
              onValueChange={(v) => update("os", v)}
            >
              <SelectTrigger className="w-36">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {OS_OPTIONS.map((opt) => (
                  <SelectItem key={opt} value={opt}>
                    {opt}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {/* Package manager */}
          <Field
            label="Package manager"
            description="Used when installing missing deps"
          >
            <Select
              value={form.pkg_manager ?? "auto"}
              onValueChange={(v) => update("pkg_manager", v)}
            >
              <SelectTrigger className="w-36">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PKG_MANAGERS.map((opt) => (
                  <SelectItem key={opt} value={opt}>
                    {opt}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>

          {/* Dotfiles count (read-only info) */}
          <Field label="Dotfiles" description="Number of entries in config">
            <span className="text-sm font-medium tabular-nums text-muted-foreground">
              {config.dots.length} configured
            </span>
          </Field>
        </div>
      </div>
    </div>
  );
}

// ── App section (tray + updates) ─────────────────────────────────────────────

function AppSection() {
  const { data: appSettings, isLoading } = useAppSettings();
  const saveSettings = useSaveAppSettings();
  const { data: version } = useQuery({
    queryKey: ["version"],
    queryFn: () => ipc.getVersion(),
    staleTime: Number.POSITIVE_INFINITY,
  });
  const [updateState, setUpdateState] = useState<
    "idle" | "checking" | "up-to-date" | { info: UpdateInfo }
  >("idle");

  async function handleCheckUpdate() {
    setUpdateState("checking");
    try {
      const info = await ipc.checkUpdate();
      setUpdateState(info ? { info } : "up-to-date");
    } catch {
      setUpdateState("idle");
      toast.error("Update check failed — check your internet connection");
    }
  }

  if (isLoading || !appSettings) return null;

  function toggle(key: "run_in_tray" | "auto_update") {
    saveSettings.mutate({ ...appSettings!, [key]: !appSettings![key] });
  }

  return (
    <div className="space-y-3">
      <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
        App
      </p>
      <div className="space-y-2">
        <Field
          label="Tray mode"
          description="Close button hides to menu bar instead of quitting (macOS)"
        >
          <Switch
            checked={appSettings.run_in_tray}
            onCheckedChange={() => toggle("run_in_tray")}
          />
        </Field>

        <Field
          label="Check for updates on startup"
          description="Notifies you when a new release is available"
        >
          <Switch
            checked={appSettings.auto_update}
            onCheckedChange={() => toggle("auto_update")}
          />
        </Field>

        <Field label="Updates" description={version ? `Current: v${version}` : "Check for new releases"}>
          {updateState === "idle" && (
            <Button
              variant="outline"
              size="sm"
              onClick={handleCheckUpdate}
            >
              Check now
            </Button>
          )}
          {updateState === "checking" && (
            <Button variant="outline" size="sm" disabled>
              <Loader2 className="animate-spin" />
              Checking…
            </Button>
          )}
          {updateState === "up-to-date" && (
            <span className="flex items-center gap-1.5 text-sm text-muted-foreground">
              <CheckCircle2 className="size-3.5 text-green-500" />
              Up to date
            </span>
          )}
          {typeof updateState === "object" && (
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-primary">
                v{updateState.info.version} available
              </span>
              <Button
                size="sm"
                onClick={() => openUrl(updateState.info.url)}
              >
                <ArrowUpRight className="size-3.5" />
                Download
              </Button>
            </div>
          )}
        </Field>
      </div>
    </div>
  );
}

function Field({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 rounded-lg border border-border bg-card px-4 py-3.5 shadow-sm">
      <div className="min-w-0">
        <Label className="text-sm font-medium text-foreground">{label}</Label>
        {description && (
          <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
