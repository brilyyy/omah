import { useForm } from "@tanstack/react-form";
import { z } from "zod";
import { open as openFilePicker } from "@tauri-apps/plugin-dialog";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useConfig } from "@/hooks/use-config";
import { queryKeys } from "@/lib/query-keys";
import { FolderOpen, Loader2, Pencil, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { ipc, type Dotfile } from "@/lib/ipc";

// ── Schema ────────────────────────────────────────────────────────────────────

const CHECK_TYPES = ["none", "bin", "file", "dir", "cmd", "skip"] as const;
type CheckType = (typeof CHECK_TYPES)[number];

const CHECK_META: Record<CheckType, { label: string; placeholder: string; hint: string }> = {
  none:  { label: "No check",         placeholder: "",                                   hint: "Step always shows as pending" },
  bin:   { label: "Binary in PATH",   placeholder: "nvim",                               hint: "Skip when binary is found in PATH" },
  file:  { label: "File exists",      placeholder: "~/.config/nvim/init.lua",            hint: "Skip when the file exists" },
  dir:   { label: "Dir exists",       placeholder: "~/.config/nvim",                     hint: "Skip when the directory exists" },
  cmd:   { label: "Command exits 0",  placeholder: "ls ~/.local/share/nvim | grep lazy", hint: "Skip when the shell command exits 0" },
  skip:  { label: "Always skip",      placeholder: "",                                   hint: "Permanently mark this step as done" },
};

const setupStepSchema = z.object({
  checkType: z.enum(CHECK_TYPES),
  checkValue: z.string(),
  install: z.string().min(1, "Install command is required"),
});

const dotfileSchema = z.object({
  name: z.string().min(1, "Name is required"),
  source: z.string().min(1, "Source path is required"),
  symlink: z.boolean(),
  deps: z.string(),
  setup: z.array(setupStepSchema),
});

type DotfileFormValues = z.infer<typeof dotfileSchema>;

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Parse a stored `"bin:nvim"` / `"file:~/.zshrc"` / bare string into type + value. */
function parseCheck(raw: string | null | undefined): { checkType: CheckType; checkValue: string } {
  if (!raw) return { checkType: "none", checkValue: "" };
  if (raw === "skip" || raw.startsWith("skip:")) return { checkType: "skip", checkValue: "" };
  for (const t of CHECK_TYPES) {
    if (t !== "none" && t !== "skip" && raw.startsWith(`${t}:`)) {
      return { checkType: t, checkValue: raw.slice(t.length + 1) };
    }
  }
  // Backward-compat: bare path → file check, bare name → bin check
  if (raw.startsWith("/") || raw.startsWith("~")) {
    return { checkType: "file", checkValue: raw };
  }
  return { checkType: "bin", checkValue: raw };
}

/** Serialize type + value back to `"bin:nvim"` etc. */
function serializeCheck(type: CheckType, value: string): string | null {
  if (type === "none") return null;
  if (type === "skip") return "skip";
  if (!value.trim()) return null;
  return `${type}:${value.trim()}`;
}

function dotfileToFormValues(dotfile: Dotfile): DotfileFormValues {
  return {
    name: dotfile.name,
    source: dotfile.source,
    symlink: dotfile.symlink ?? false,
    deps: dotfile.deps?.join(", ") ?? "",
    setup:
      dotfile.setup?.map((s) => ({
        ...parseCheck(s.check),
        install: s.install,
      })) ?? [],
  };
}

const DEFAULT_VALUES: DotfileFormValues = {
  name: "",
  source: "",
  symlink: false,
  deps: "",
  setup: [],
};

function formValuesToDotfile(v: DotfileFormValues): Dotfile {
  const deps = v.deps.split(",").map((s) => s.trim()).filter(Boolean);
  const setup = v.setup
    .filter((s) => s.install.trim())
    .map((s) => ({
      check: serializeCheck(s.checkType, s.checkValue),
      install: s.install.trim(),
    }));
  return {
    name: v.name.trim(),
    source: v.source.trim(),
    symlink: v.symlink || null,
    deps: deps.length ? deps : null,
    exclude: null,
    setup: setup.length ? setup : null,
  };
}

// ── Public component ──────────────────────────────────────────────────────────

interface DotfileDialogProps {
  children: React.ReactNode;
  mode: "add" | "edit";
  dotfile?: Dotfile;
  dotIndex?: number;
}

export function DotfileDialog({ children, mode, dotfile, dotIndex }: DotfileDialogProps) {
  const [open, setOpen] = useState(false);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>{children}</DialogTrigger>
      {/*
       * Key forces a full remount of the inner form each time the dialog
       * opens — this guarantees array fields are initialised with the
       * dotfile's saved setup steps instead of the stale empty default.
       */}
      {open && (
        <DotfileFormDialog
          key={`${mode}-${dotIndex ?? "new"}`}
          mode={mode}
          dotfile={dotfile}
          dotIndex={dotIndex}
          onClose={() => setOpen(false)}
        />
      )}
    </Dialog>
  );
}

// ── Inner form dialog (remounts on every open) ────────────────────────────────

interface DotfileFormDialogProps {
  mode: "add" | "edit";
  dotfile?: Dotfile;
  dotIndex?: number;
  onClose: () => void;
}

function DotfileFormDialog({ mode, dotfile, dotIndex, onClose }: DotfileFormDialogProps) {
  const queryClient = useQueryClient();

  const { data: config } = useConfig();

  const saveMutation = useMutation({
    mutationFn: async ({
      updated,
      symlinkAction,
    }: {
      updated: Dotfile;
      /** null = no change; true = enable (backup); false = disable (restore) */
      symlinkAction: boolean | null;
    }) => {
      if (!config) throw new Error("Config not loaded");
      const dots =
        mode === "edit" && dotIndex !== undefined
          ? config.dots.map((d, i) => (i === dotIndex ? updated : d))
          : [...config.dots, updated];
      await ipc.saveConfig({ ...config, dots });

      // Apply the filesystem change if symlink mode was toggled
      if (symlinkAction === true) {
        await ipc.backupOne(updated.name);   // copies source → vault, creates symlink
      } else if (symlinkAction === false) {
        await ipc.restoreOne(updated.name);  // copies vault → source, removes symlink
      }
    },
    onSuccess: (_, { symlinkAction }) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.config() });
      queryClient.invalidateQueries({ queryKey: queryKeys.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.diff() });
      if (symlinkAction === true) {
        toast.success("Symlink enabled — source replaced with symlink");
      } else if (symlinkAction === false) {
        toast.success("Symlink removed — source restored as real file");
      } else {
        toast.success(mode === "edit" ? "Dotfile updated" : "Dotfile added");
      }
      onClose();
    },
    onError: (e) => toast.error(String(e)),
  });

  // Initialise with the correct values on first render — no useEffect needed.
  const form = useForm({
    defaultValues: mode === "edit" && dotfile ? dotfileToFormValues(dotfile) : DEFAULT_VALUES,
    validators: { onChange: dotfileSchema },
    onSubmit: async ({ value }) => {
      const updated = formValuesToDotfile(value);
      const oldSymlink = dotfile?.symlink ?? false;
      const symlinkAction =
        mode === "edit" && oldSymlink !== value.symlink ? value.symlink : null;
      await saveMutation.mutateAsync({ updated, symlinkAction });
    },
  });

  async function pickPath(directory: boolean) {
    const selected = await openFilePicker({ multiple: false, directory });
    if (typeof selected === "string") {
      form.setFieldValue("source", selected);
    }
  }

  const isEditMode = mode === "edit";

  return (
    <DialogContent className="flex max-h-[85vh] flex-col sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>{isEditMode ? "Edit Dotfile" : "Add Dotfile"}</DialogTitle>
        <DialogDescription>
          {isEditMode
            ? "Update the dotfile entry in your omah configuration."
            : "Add a new entry to your omah configuration."}
        </DialogDescription>
      </DialogHeader>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          e.stopPropagation();
          form.handleSubmit();
        }}
        className="flex min-h-0 flex-1 flex-col"
      >
        <div className="flex-1 space-y-4 overflow-y-auto py-1 pr-1">
          {/* Name */}
          <form.Field name="name">
            {(field) => (
              <div className="space-y-1.5">
                <Label htmlFor="field-name">Name</Label>
                <Input
                  id="field-name"
                  placeholder="Neovim"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  onBlur={field.handleBlur}
                />
                <FieldError field={field} />
              </div>
            )}
          </form.Field>

          {/* Source path */}
          <form.Field name="source">
            {(field) => (
              <div className="space-y-1.5">
                <Label htmlFor="field-source">Source path</Label>
                <div className="flex gap-2">
                  <Input
                    id="field-source"
                    placeholder="~/.config/nvim"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                    className="font-mono text-xs"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    title="Browse file"
                    onClick={() => pickPath(false)}
                  >
                    <FolderOpen className="size-4" />
                  </Button>
                  <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    title="Browse folder"
                    onClick={() => pickPath(true)}
                  >
                    <FolderOpen className="size-4 opacity-50" />
                  </Button>
                </div>
                <FieldError field={field} />
              </div>
            )}
          </form.Field>

          {/* Symlink */}
          <form.Field name="symlink">
            {(field) => (
              <div className="flex items-center justify-between rounded-lg border border-border px-3 py-2.5">
                <div>
                  <Label htmlFor="field-symlink" className="cursor-pointer font-normal">
                    Symlink mode
                  </Label>
                  <p className="text-xs text-muted-foreground">
                    Replace source with a symlink to the vault copy
                  </p>
                </div>
                <Switch
                  id="field-symlink"
                  checked={field.state.value}
                  onCheckedChange={(v) => field.handleChange(v)}
                />
              </div>
            )}
          </form.Field>

          {/* Deps */}
          <form.Field name="deps">
            {(field) => (
              <div className="space-y-1.5">
                <div className="flex items-baseline gap-1.5">
                  <Label htmlFor="field-deps">Dependencies</Label>
                  <span className="text-xs text-muted-foreground">(optional)</span>
                </div>
                <Input
                  id="field-deps"
                  placeholder="nvim, git, rg"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  onBlur={field.handleBlur}
                />
                <p className="text-xs text-muted-foreground">
                  Comma-separated binary names that must be in PATH
                </p>
              </div>
            )}
          </form.Field>

          {/* Setup steps */}
          <form.Field name="setup" mode="array">
            {(field) => (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-baseline gap-1.5">
                    <Label>Setup steps</Label>
                    <span className="text-xs text-muted-foreground">(optional)</span>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    className="h-7 gap-1 text-xs"
                    onClick={() =>
                      field.pushValue({ checkType: "none", checkValue: "", install: "" })
                    }
                  >
                    <Plus className="size-3" />
                    Add step
                  </Button>
                </div>

                {field.state.value.length === 0 ? (
                  <p className="text-xs text-muted-foreground">
                    Commands to run after restoring this dotfile (e.g. install plugins).
                  </p>
                ) : (
                  <div className="space-y-2">
                    {field.state.value.map((_step, i) => (
                      <div key={i} className="rounded-lg border border-border p-3 space-y-2.5">
                        <div className="flex items-center justify-between">
                          <span className="text-xs font-medium text-muted-foreground">
                            Step {i + 1}
                          </span>
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon-sm"
                            className="text-muted-foreground hover:text-destructive"
                            onClick={() => field.removeValue(i)}
                          >
                            <Trash2 className="size-3.5" />
                          </Button>
                        </div>

                        {/* Install command */}
                        <form.Field name={`setup[${i}].install`}>
                          {(subField) => (
                            <div className="space-y-1">
                              <Label htmlFor={`field-setup-${i}-install`} className="text-xs">
                                Install command
                              </Label>
                              <Input
                                id={`field-setup-${i}-install`}
                                placeholder="brew install --cask font-fira-code"
                                value={subField.state.value}
                                onChange={(e) => subField.handleChange(e.target.value)}
                                onBlur={subField.handleBlur}
                                className="font-mono text-xs"
                              />
                              <FieldError field={subField} />
                            </div>
                          )}
                        </form.Field>

                        {/* Check — type selector + optional value */}
                        <div className="space-y-1">
                          <div className="flex items-baseline gap-1.5">
                            <Label className="text-xs">Skip if</Label>
                            <span className="text-xs text-muted-foreground">(optional)</span>
                          </div>
                          {/* Check type + optional value — both rendered inside the
                              checkType Field so the input visibility reacts to the
                              live field value rather than the stale map snapshot */}
                          <form.Field name={`setup[${i}].checkType`}>
                            {(typeField) => (
                              <div className="flex gap-2">
                                <Select
                                  value={typeField.state.value}
                                  onValueChange={(v) => typeField.handleChange(v as CheckType)}
                                >
                                  <SelectTrigger className="h-8 w-36 shrink-0 text-xs">
                                    <SelectValue />
                                  </SelectTrigger>
                                  <SelectContent>
                                    {CHECK_TYPES.map((t) => (
                                      <SelectItem key={t} value={t} className="text-xs">
                                        {CHECK_META[t].label}
                                      </SelectItem>
                                    ))}
                                  </SelectContent>
                                </Select>

                                {/* Check value — hidden when "none" or "skip" */}
                                {typeField.state.value !== "none" &&
                                  typeField.state.value !== "skip" && (
                                    <form.Field name={`setup[${i}].checkValue`}>
                                      {(valField) => (
                                        <Input
                                          placeholder={
                                            CHECK_META[typeField.state.value].placeholder
                                          }
                                          value={valField.state.value}
                                          onChange={(e) => valField.handleChange(e.target.value)}
                                          onBlur={valField.handleBlur}
                                          className="h-8 font-mono text-xs"
                                        />
                                      )}
                                    </form.Field>
                                  )}
                              </div>
                            )}
                          </form.Field>
                          <form.Subscribe
                            selector={(s) =>
                              (s.values.setup[i] as (typeof s.values.setup)[number])?.checkType ?? "none"
                            }
                          >
                            {(checkType) => (
                              <p className="text-xs text-muted-foreground">
                                {CHECK_META[checkType].hint}
                              </p>
                            )}
                          </form.Subscribe>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </form.Field>
        </div>

        <DialogFooter className="pt-4">
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <form.Subscribe selector={(s) => [s.canSubmit, s.isSubmitting] as const}>
            {([canSubmit, isSubmitting]) => (
              <Button
                type="submit"
                disabled={!canSubmit || isSubmitting || saveMutation.isPending}
              >
                {isSubmitting || saveMutation.isPending ? (
                  <Loader2 className="animate-spin" />
                ) : isEditMode ? (
                  <Pencil />
                ) : (
                  <Plus />
                )}
                {isEditMode ? "Save Changes" : "Add Dotfile"}
              </Button>
            )}
          </form.Subscribe>
        </DialogFooter>
      </form>
    </DialogContent>
  );
}

// biome-ignore lint/suspicious/noExplicitAny: field type is generic
function FieldError({ field }: { field: any }) {
  const errors: unknown[] = field.state.meta.errors ?? [];
  if (!field.state.meta.isTouched || errors.length === 0) return null;
  // TanStack Form + Zod v4 StandardSchema may surface raw ZodIssue objects
  // ({ message, code, ... }) instead of plain strings — extract the message.
  const first = errors[0];
  const message =
    typeof first === "string" ? first : (first as any)?.message ?? String(first);
  return <p className="text-xs text-destructive">{message}</p>;
}
