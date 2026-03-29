import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, RotateCcw, Save } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ipc, type Config } from "@/lib/ipc";

export const Route = createFileRoute("/settings")({
  component: SettingsView,
});

const PKG_MANAGERS = ["auto", "brew", "apt-get", "pacman", "dnf", "zypper"] as const;
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
            <span className="font-mono text-xs">~/.config/omah/omah-config.toml</span>
          </p>
        </div>
        <div className="flex items-center gap-2">
          {dirty && (
            <Button variant="ghost" size="sm" onClick={handleReset} disabled={saveMutation.isPending}>
              <RotateCcw />
              Reset
            </Button>
          )}
          <Button
            size="sm"
            onClick={() => saveMutation.mutate(form as Config)}
            disabled={!dirty || saveMutation.isPending}
          >
            {saveMutation.isPending ? <Loader2 className="animate-spin" /> : <Save />}
            Save
          </Button>
        </div>
      </div>

      {/* Form */}
      <div className="flex-1 overflow-auto px-6 py-6">
        <div className="mx-auto max-w-xl space-y-4">

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
          <Field label="OS" description="Target operating system for this machine">
            <Select value={form.os ?? "auto"} onValueChange={(v) => update("os", v)}>
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
          <Field label="Package manager" description="Used when installing missing deps">
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
    <div className="flex items-center justify-between gap-6 rounded-lg border border-border bg-card px-4 py-3.5">
      <div className="min-w-0">
        <Label className="text-sm font-medium text-foreground">{label}</Label>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
