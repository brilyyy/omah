# Omah — UX Improvement Plan

_Last updated: 2026-03-30_

---

## Desktop App — Friction Points

### High priority

**1. Symlink toggle silently triggers filesystem ops**

- Toggling `symlink` in the edit dialog auto-runs backup/restore without any confirmation
- User expects a form save, gets unexpected file operations on disk
- Fix: gate the symlink-triggered operation behind a clear confirmation dialog before saving

**2. No "last backed up" timestamp**

- Main list and detail page show sync state but not _when_ it was last synced
- Users can't tell if they're looking at a 5-minute-old or 2-month-old backup
- Fix: store and display a `last_backed_up` timestamp per dotfile (could live in a sidecar file in the vault)

**3. Delete dotfile UX is ambiguous**

- Confirmation says "removes from config" — but does it delete vault files? Source files?
- Fix: be explicit in the dialog about exactly what gets deleted vs. what stays

**4. "Skip" setup step silently modifies config**

- Clicking "Skip" writes `check: "skip"` to `omah-config.toml` permanently
- Feels like a UI-only toggle but is actually a persistent config mutation
- Fix: rename to "Mark as done" or show a tooltip that makes the persistence clear

**5. Diff page has no per-dotfile filter**

- If you have 20 dotfiles and want to review changes for just one before backing it up, there's no way to filter
- Fix: add search/filter on diff page, or allow "backup this dotfile" inline from the diff section

### Lower priority

| Gap | Why it matters | Complexity |
|-----|---------------|------------|
| No conflict resolution on restore (silently overwrites local) | Risk of losing local edits | Medium |
| No scheduled/automatic backups | Manual-only means config drift goes unnoticed | High |
| No vault browsing in UI | Vault is a black box, users can't inspect it | Low |
| `list` and status overlap in sidebar | Slightly redundant screens | Low |

### Skip for now

- Multi-machine sync, config sharing, vault integrity checks — the current model (vault → git → pull on new machine) covers the 80% case and expanding scope would significantly increase complexity.

---

## CLI — Friction Points

### High priority

**1. No `add` / `edit` / `remove` subcommands**

- Users must manually edit TOML to manage dotfiles
- Zero discoverability — `omah --help` gives no hint that config editing is needed
- Fix: add `omah add <name> <source>` and `omah remove <name>` as thin wrappers around config mutations

**2. `omah init` gives no next-step guidance**

- Prints `Initialized: /path/to/config` then exits
- New users are left wondering what to do next (edit the config? how? what's the format?)
- Fix: print a "what's next" hint:

  ```
  Initialized: ~/.config/omah/omah-config.toml

  Next: edit the config to add your dotfiles, then run:
    omah backup     — copy dotfiles to the vault
    omah status     — check sync state
  ```

**3. `omah status` output has no colors**

- `diff` uses `owo_colors` nicely (green/yellow/red), but `status` is plain monospace
- Hard to scan quickly — "NOT backed up" looks the same as "backed up" at a glance
- Column alignment also breaks for names longer than 20 chars
- Fix: colorize status labels (green = backed up, red = NOT backed up, yellow = issues) and add a summary line

**4. `omah restore` proceeds even when user says "no" to pre-restore steps**

- When user declines to run pre-restore steps it prints:
  `"Skipping pre-restore steps. Continuing anyway..."`
- This is confusing — the user said no and the command continues anyway, potentially into a broken state
- Fix: ask "Continue restore without running setup steps? [y/N]" as a second prompt, default to abort

**5. No per-dotfile granularity for backup/restore**

- `omah backup` and `omah restore` always operate on all dotfiles
- Can't do `omah backup nvim` or `omah restore zsh`
- Fix: add an optional `[name]` argument: `omah backup [name]` / `omah restore [name]`

### Medium priority

**6. No `--dry-run` flag on backup/restore**

- No way to preview what would change before committing
- Fix: `omah backup --dry-run` prints what would be copied/symlinked without touching files

**7. `list` and `status` overlap**

- `list` shows: name, source, symlink flag, deps
- `status` shows: name, source, backed_up, symlinked, missing deps, pending setup
- First-time users run `list` when they actually need `status`
- Fix: consider merging `list` into `status`, or clarify help text to differentiate them clearly

**8. No error summary**

- If one dotfile fails mid-backup, errors are printed inline mixed with success lines
- No final "1 error, 4 succeeded" summary
- Fix: collect errors and print a clean summary at the end

### Low priority

| Gap | Notes |
|-----|-------|
| No `--quiet` / `--verbose` flags | Useful for scripting |
| `diff` can't target a single dotfile | `omah diff nvim` |
| `backup` success message doesn't list what was backed up | Just says "Backup complete → vault" |

---

## Overall Priority Order

1. CLI `init` next-step guidance (tiny change, high impact for new users)
2. CLI `status` colorization + alignment fix
3. CLI `restore` abort-on-no behavior
4. Desktop symlink toggle confirmation
5. Desktop delete clarity
6. CLI per-dotfile backup/restore argument
7. Desktop timestamps
8. Desktop diff filter
9. CLI `add`/`remove` commands
10. CLI `--dry-run`
