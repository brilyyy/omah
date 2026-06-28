# omah — User Manual Testing

Build omah before testing:

```sh
cargo build --release
alias omah="$(pwd)/target/release/omah"
```

Use a temp directory for all test runs to avoid touching real config:

```sh
T="$(mktemp -d)"
```

---

## TC-001: `omah init` scaffolds config

**Setup:**
```sh
mkdir -p "$T/init-test"
```

**Test:**
1. Run `omah init --config "$T/init-test/omah-config.toml"`
2. Verify output contains `Initialized:`
3. Verify file created at `$T/init-test/omah-config.toml`
4. Inspect file — should contain `vault_path`, schema URL, commented example dotfile

**Expected:**
```
Initialized: /tmp/.../omah-config.toml

Next steps:
  omah add <name> <source>  — add a dotfile entry
  omah backup        — back up all dotfiles to the vault
  omah status        — check sync state
```

**Cleanup:** `rm -rf "$T/init-test"`

---

## TC-002: `omah init` is idempotent

**Setup:**
```sh
mkdir -p "$T/idem"
echo 'vault_path = "/custom/vault"' > "$T/idem/omah-config.toml"
echo 'dots = []' >> "$T/idem/omah-config.toml"
```

**Test:**
1. Run `omah init --config "$T/idem/omah-config.toml"` again
2. Read file — content should still say `/custom/vault`, NOT overwritten

**Expected:** File unchanged. No error.

**Cleanup:** `rm -rf "$T/idem"`

---

## TC-003: `omah add` creates dotfile entry

**Setup:**
```sh
omah init --config "$T/add-test/omah-config.toml"
echo "test content" > "$T/add-test/.zshrc"
```

**Test:**
1. Run `omah --config "$T/add-test/omah-config.toml" add zsh "$T/add-test/.zshrc"`
2. Verify output contains `Added 'zsh' to config.`
3. Run `omah --config "$T/add-test/omah-config.toml" list`
4. Verify `zsh` appears in the list

**Expected:**
```
Added 'zsh' to config.
Run `omah backup zsh` to back it up to the vault.
```

**Cleanup:** `rm -rf "$T/add-test"`

---

## TC-004: `omah add --symlink` creates symlink entry

**Setup:**
```sh
omah init --config "$T/sym-add/omah-config.toml"
echo "nvim config" > "$T/sym-add/init.lua"
```

**Test:**
1. `omah --config "$T/sym-add/omah-config.toml" add nvim "$T/sym-add/init.lua" --symlink`
2. Run `omah --config "$T/sym-add/omah-config.toml" list`
3. Verify `nvim  →  ... [symlink]` appears

**Expected:** `nvim [symlink]  →  .../init.lua`

**Cleanup:** `rm -rf "$T/sym-add"`

---

## TC-005: `omah add` rejects duplicate name

**Setup:**
```sh
omah init --config "$T/dup/omah-config.toml"
omah --config "$T/dup/omah-config.toml" add zsh /dev/null
```

**Test:**
1. `omah --config "$T/dup/omah-config.toml" add zsh /dev/null`

**Expected:** Error: `Dotfile 'zsh' already exists in config`

**Cleanup:** `rm -rf "$T/dup"`

---

## TC-006: `omah remove` deletes entry

**Setup:**
```sh
omah init --config "$T/rm-test/omah-config.toml"
omah --config "$T/rm-test/omah-config.toml" add zsh /dev/null
```

**Test:**
1. `omah --config "$T/rm-test/omah-config.toml" remove zsh`
2. Verify output `Removed 'zsh' from config.`
3. `omah --config "$T/rm-test/omah-config.toml" list`
4. Verify `zsh` no longer appears

**Expected:** `No dotfiles configured.`

**Cleanup:** `rm -rf "$T/rm-test"`

---

## TC-007: `omah remove` errors on nonexistent

**Setup:**
```sh
omah init --config "$T/rm-miss/omah-config.toml"
```

**Test:**
1. `omah --config "$T/rm-miss/omah-config.toml" remove nonexistent`

**Expected:** Error: `Dotfile 'nonexistent' not found in config`

**Cleanup:** `rm -rf "$T/rm-miss"`

---

## TC-008: `omah list` shows configured dotfiles

**Setup:**
```sh
omah init --config "$T/list-test/omah-config.toml"
omah --config "$T/list-test/omah-config.toml" add zsh /dev/null
omah --config "$T/list-test/omah-config.toml" add nvim /dev/null --symlink
```

**Test:**
1. `omah --config "$T/list-test/omah-config.toml" list`
2. Verify both dotfiles shown
3. Verify `nvim` shows `[symlink]` tag

**Expected:**
```
Vault: ~/.config/omah/vault

  zsh   →  /dev/null
  nvim  →  /dev/null [symlink]
```

**Cleanup:** `rm -rf "$T/list-test"`

---

## TC-009: `omah list --json` outputs JSON

**Setup:** Same as TC-008

**Test:**
1. `omah --config "$T/list-test/omah-config.toml" list --json`

**Expected:** Valid JSON with `vault_path` and `dots` array containing both entries.

**Cleanup:** `rm -rf "$T/list-test"`

---

## TC-010: `omah info` shows per-dotfile detail

**Setup:**
```sh
omah init --config "$T/info-test/omah-config.toml"
echo "zsh content" > "$T/info-test/.zshrc"
omah --config "$T/info-test/omah-config.toml" add zsh "$T/info-test/.zshrc"
omah --config "$T/info-test/omah-config.toml" backup zsh
```

**Test:**
1. `omah --config "$T/info-test/omah-config.toml" info`
2. Verify output includes `zsh` name, source path, vault path, state
3. `omah --config "$T/info-test/omah-config.toml" info zsh`
4. Verify same detail, single dotfile only

**Expected:** State shows `✓ deployed` or `✓ deployed (symlink)`. Vault path shown.

**Cleanup:** `rm -rf "$T/info-test"`

---

## TC-011: Backup a single file

**Setup:**
```sh
omah init --config "$T/bak1/omah-config.toml"
echo "zshrc content" > "$T/bak1/.zshrc"
omah --config "$T/bak1/omah-config.toml" add zsh "$T/bak1/.zshrc"
```

**Test:**
1. `omah --config "$T/bak1/omah-config.toml" backup`
2. Verify output `Backup complete → ...`
3. Verify vault file exists: `ls "$T/bak1"/*/vault/zsh/.zshrc` (expand tilde manually or check)
4. Check vault path from config, expand `~`, verify file inside

**Expected:** Vault directory created, `.zshrc` copied inside.

**Cleanup:** `rm -rf "$T/bak1"`

---

## TC-012: Backup a directory

**Setup:**
```sh
omah init --config "$T/bak-dir/omah-config.toml"
mkdir -p "$T/bak-dir/nvim"
echo "init.lua" > "$T/bak-dir/nvim/init.lua"
echo "lazy.lua" > "$T/bak-dir/nvim/lazy.lua"
mkdir "$T/bak-dir/nvim/plugin"
echo "foo.lua" > "$T/bak-dir/nvim/plugin/foo.lua"
omah --config "$T/bak-dir/omah-config.toml" add nvim "$T/bak-dir/nvim"
```

**Test:**
1. `omah --config "$T/bak-dir/omah-config.toml" backup`
2. Verify vault contains `nvim/init.lua`, `nvim/lazy.lua`, `nvim/plugin/foo.lua`

**Expected:** Directory structure preserved in vault.

**Cleanup:** `rm -rf "$T/bak-dir"`

---

## TC-013: Backup dry-run shows plan

**Setup:** Same as TC-011 (but don't run backup)

**Test:**
1. `omah --config "$T/bak1/omah-config.toml" backup --dry-run`

**Expected:**
```
Backup plan:

  zsh: .../.zshrc → .../vault/zsh/.zshrc (1 files)
```

No vault directory created.

**Cleanup:** `rm -rf "$T/bak1"`

---

## TC-014: Backup single dotfile by name

**Setup:**
```sh
omah init --config "$T/bak-name/omah-config.toml"
echo "a" > "$T/bak-name/.zshrc"
echo "b" > "$T/bak-name/.tmux.conf"
omah --config "$T/bak-name/omah-config.toml" add zsh "$T/bak-name/.zshrc"
omah --config "$T/bak-name/omah-config.toml" add tmux "$T/bak-name/.tmux.conf"
```

**Test:**
1. `omah --config "$T/bak-name/omah-config.toml" backup zsh`
2. Verify vault has `zsh/` but NOT `tmux/`

**Expected:** Only `zsh` backed up.

**Cleanup:** `rm -rf "$T/bak-name"`

---

## TC-015: Backup with symlink mode

**Setup:**
```sh
omah init --config "$T/bak-sym/omah-config.toml"
echo "content" > "$T/bak-sym/file"
omah --config "$T/bak-sym/omah-config.toml" add myfile "$T/bak-sym/file" --symlink
```

**Test:**
1. `omah --config "$T/bak-sym/omah-config.toml" backup`
2. Type `y` when prompted "Continue? [y/N]"
3. Verify `$T/bak-sym/file` is now a symlink: `readlink "$T/bak-sym/file"`
4. Verify symlink target points to vault

**Expected:** Source replaced with symlink → vault.

**Cleanup:** `rm -rf "$T/bak-sym"`

---

## TC-016: Backup — re-run is idempotent (vault content preserved)

**Setup:**
```sh
omah init --config "$T/bak-idem/omah-config.toml"
echo "original" > "$T/bak-idem/file"
omah --config "$T/bak-idem/omah-config.toml" add myfile "$T/bak-idem/file"
omah --config "$T/bak-idem/omah-config.toml" backup
# modify source
echo "modified" > "$T/bak-idem/file"
```

**Test:**
1. `omah --config "$T/bak-idem/omah-config.toml" backup`
2. Verify vault now has the modified content
3. Run backup again — no errors, vault unchanged

**Expected:** Second backup updates vault. Third backup is no-op.

**Cleanup:** `rm -rf "$T/bak-idem"`

---

## TC-017: Backup with exclude patterns

**Setup:**
```sh
omah init --config "$T/bak-excl/omah-config.toml"
mkdir -p "$T/bak-excl/nvim"
echo "init.lua" > "$T/bak-excl/nvim/init.lua"
echo "debug.log" > "$T/bak-excl/nvim/debug.log"
mkdir "$T/bak-excl/nvim/node_modules"
echo "dep" > "$T/bak-excl/nvim/node_modules/pkg.js"
```

Edit config `$T/bak-excl/omah-config.toml` to add:
```toml
[[dots]]
name = "nvim"
source = "..."  # fill actual path
exclude = ["*.log", "node_modules"]
```

**Test:**
1. `omah --config "$T/bak-excl/omah-config.toml" backup`
2. Check vault — `init.lua` exists
3. `debug.log` should NOT be in vault
4. `node_modules/` should NOT be in vault

**Expected:** Excluded files omitted.

**Cleanup:** `rm -rf "$T/bak-excl"`

---

## TC-018: Backup --no-exclude ignores exclude patterns

**Setup:** Same as TC-017, config with `exclude = ["*.log"]`

**Test:**
1. `omah --config "$T/bak-excl/omah-config.toml" backup --no-exclude`
2. Check vault — `debug.log` IS present (exclude bypassed)

**Expected:** Excluded files included.

**Cleanup:** `rm -rf "$T/bak-excl"`

---

## TC-019: Backup — missing source errors

**Setup:**
```sh
omah init --config "$T/bak-miss/omah-config.toml"
omah --config "$T/bak-miss/omah-config.toml" add ghost /nonexistent/path
```

**Test:**
1. `omah --config "$T/bak-miss/omah-config.toml" backup`

**Expected:** Error: `Source not found: /nonexistent/path`

**Cleanup:** `rm -rf "$T/bak-miss"`

---

## TC-020: Backup dry-run — missing source warning

**Setup:** Same as TC-019

**Test:**
1. `omah --config "$T/bak-miss/omah-config.toml" backup --dry-run`

**Expected:**
```
Backup plan:

  ghost: !!! source not found
```

No error exit — dry run continues.

**Cleanup:** `rm -rf "$T/bak-miss"`

---

## TC-021: Restore a backed-up dotfile

**Setup:**
```sh
omah init --config "$T/rest1/omah-config.toml"
echo "zshrc content" > "$T/rest1/.zshrc"
omah --config "$T/rest1/omah-config.toml" add zsh "$T/rest1/.zshrc"
omah --config "$T/rest1/omah-config.toml" backup
rm "$T/rest1/.zshrc"
```

**Test:**
1. `omah --config "$T/rest1/omah-config.toml" restore`
2. Verify `$T/rest1/.zshrc` exists again
3. Verify content matches original

**Expected:** `Restore complete ← ...` and file restored.

**Cleanup:** `rm -rf "$T/rest1"`

---

## TC-022: Restore dry-run shows plan

**Setup:** Same as TC-021 (after backup, before delete)

**Test:**
1. `omah --config "$T/rest1/omah-config.toml" restore --dry-run`

**Expected:**
```
Restore plan:

  zsh: .../vault/zsh/.zshrc → .../.zshrc (1 files)
```

No files copied.

**Cleanup:** `rm -rf "$T/rest1"`

---

## TC-023: Restore single dotfile by name

**Setup:**
```sh
omah init --config "$T/rest-name/omah-config.toml"
echo "a" > "$T/rest-name/.zshrc"
echo "b" > "$T/rest-name/.tmux.conf"
omah --config "$T/rest-name/omah-config.toml" add zsh "$T/rest-name/.zshrc"
omah --config "$T/rest-name/omah-config.toml" add tmux "$T/rest-name/.tmux.conf"
omah --config "$T/rest-name/omah-config.toml" backup
rm "$T/rest-name/.zshrc"
```

**Test:**
1. `omah --config "$T/rest-name/omah-config.toml" restore zsh`
2. Verify `$T/rest-name/.zshrc` restored
3. Verify `$T/rest-name/.tmux.conf` still missing (not restored)

**Expected:** Only `zsh` restored. `tmux` remains absent.

**Cleanup:** `rm -rf "$T/rest-name"`

---

## TC-024: Restore with symlink mode

**Setup:**
```sh
omah init --config "$T/rest-sym/omah-config.toml"
echo "content" > "$T/rest-sym/file"
omah --config "$T/rest-sym/omah-config.toml" add myfile "$T/rest-sym/file" --symlink
omah --config "$T/rest-sym/omah-config.toml" backup  # type y
rm "$T/rest-sym/file"
```

**Test:**
1. `omah --config "$T/rest-sym/omah-config.toml" restore`
2. Verify `$T/rest-sym/file` is a symlink
3. Verify target points to vault

**Expected:** Symlink restored, pointing to vault.

**Cleanup:** `rm -rf "$T/rest-sym"`

---

## TC-025: Restore — missing vault entry continues (no crash)

**Setup:**
```sh
omah init --config "$T/rest-miss/omah-config.toml"
echo "a" > "$T/rest-miss/.zshrc"
echo "b" > "$T/rest-miss/.tmux.conf"
omah --config "$T/rest-miss/omah-config.toml" add zsh "$T/rest-miss/.zshrc"
omah --config "$T/rest-miss/omah-config.toml" add tmux "$T/rest-miss/.tmux.conf"
omah --config "$T/rest-miss/omah-config.toml" backup
rm "$T/rest-miss/.zshrc" "$T/rest-miss/.tmux.conf"
rm -r "$(grep vault "$T/rest-miss/omah-config.toml" | head -1 | cut -d'"' -f2 | sed 's|~|'"$HOME"'|')/zsh"  # delete one vault entry
```

**Test:**
1. `omah --config "$T/rest-miss/omah-config.toml" restore`
2. Verify zsh restored (not applicable since vault entry removed, tmux restored)
3. Output should show error for zsh, continue with tmux

**Expected:** Error for `zsh`, but `tmux` restores successfully.

**Cleanup:** `rm -rf "$T/rest-miss"`

---

## TC-026: Status shows correct states

**Setup:**
```sh
omah init --config "$T/stat-test/omah-config.toml"
echo "live" > "$T/stat-test/.zshrc"
omah --config "$T/stat-test/omah-config.toml" add zsh "$T/stat-test/.zshrc"
```

**Test (unbacked):**
1. `omah --config "$T/stat-test/omah-config.toml" status`
2. `zsh` should show `⚠ unbacked`

**Setup (backup):**
3. `omah --config "$T/stat-test/omah-config.toml" backup`

**Test (deployed):**
4. `omah --config "$T/stat-test/omah-config.toml" status`
5. `zsh` should show `✓ deployed`

**Setup (symlink):**
6. Add `symlink = true` to the zsh entry in config
7. `omah --config "$T/stat-test/omah-config.toml" backup` (type y)

**Test (symlink):**
8. `omah --config "$T/stat-test/omah-config.toml" status`
9. `zsh` should show `🔗 deployed`

**Setup (available):**
10. `rm "$T/stat-test/.zshrc"`

**Test (available):**
11. `omah --config "$T/stat-test/omah-config.toml" status`
12. `zsh` should show `○ available`

**Expected:** Each state matches the table:
- Source exists + backed up + symlinked → `🔗 deployed`
- Source exists + backed up → `✓ deployed`
- Source missing + backed up → `○ available`
- Source exists + not backed up → `⚠ unbacked`
- Source missing + not backed up → `✗ missing`

**Cleanup:** `rm -rf "$T/stat-test"`

---

## TC-027: Status --json outputs JSON

**Setup:** Same as TC-026 after backup

**Test:**
1. `omah --config "$T/stat-test/omah-config.toml" status --json`

**Expected:** Valid JSON array with objects containing `name`, `source`, `source_exists`, `backed_up`, `symlinked`, `missing_deps`, `pending_setup`.

**Cleanup:** `rm -rf "$T/stat-test"`

---

## TC-028: Status summary counts correct

**Setup:**
```sh
omah init --config "$T/stat-sum/omah-config.toml"
echo "1" > "$T/stat-sum/a"
echo "2" > "$T/stat-sum/b"
echo "3" > "$T/stat-sum/c"
omah --config "$T/stat-sum/omah-config.toml" add a "$T/stat-sum/a"
omah --config "$T/stat-sum/omah-config.toml" add b "$T/stat-sum/b"
omah --config "$T/stat-sum/omah-config.toml" add c "$T/stat-sum/c"
omah --config "$T/stat-sum/omah-config.toml" backup
rm "$T/stat-sum/a"  # make one "available"
```

**Test:**
1. `omah --config "$T/stat-sum/omah-config.toml" status`

**Expected:** Summary line: `3 dotfiles · 2 deployed · 1 available`

**Cleanup:** `rm -rf "$T/stat-sum"`

---

## TC-029: Diff shows added files

**Setup:**
```sh
omah init --config "$T/diff-add/omah-config.toml"
echo "content" > "$T/diff-add/file"
omah --config "$T/diff-add/omah-config.toml" add myfile "$T/diff-add/file"
omah --config "$T/diff-add/omah-config.toml" backup
echo "new content" > "$T/diff-add/newfile"
```

**Test:**
1. `omah --config "$T/diff-add/omah-config.toml" diff`
2. Should show `myfile` with `+ newfile` (added in source)

Wait — diff only looks at configured dotfiles. The newfile isn't in config. Let me structure differently.

**Correct setup:**
```sh
omah init --config "$T/diff-add/omah-config.toml"
mkdir -p "$T/diff-add/nvim"
echo "init.lua" > "$T/diff-add/nvim/init.lua"
omah --config "$T/diff-add/omah-config.toml" add nvim "$T/diff-add/nvim"
omah --config "$T/diff-add/omah-config.toml" backup
echo "plugin.lua" > "$T/diff-add/nvim/plugin.lua"  # added in source
```

**Test:**
1. `omah --config "$T/diff-add/omah-config.toml" diff`

**Expected:**
```
nvim
  +  plugin.lua  new in source
```

**Cleanup:** `rm -rf "$T/diff-add"`

---

## TC-030: Diff shows modified files

**Setup:**
```sh
omah init --config "$T/diff-mod/omah-config.toml"
echo "original" > "$T/diff-mod/file"
omah --config "$T/diff-mod/omah-config.toml" add myfile "$T/diff-mod/file"
omah --config "$T/diff-mod/omah-config.toml" backup
echo "modified" > "$T/diff-mod/file"
```

**Test:**
1. `omah --config "$T/diff-mod/omah-config.toml" diff`

**Expected:**
```
myfile
  ~  file  modified
```

**Cleanup:** `rm -rf "$T/diff-mod"`

---

## TC-031: Diff shows removed files

**Setup:**
```sh
omah init --config "$T/diff-rm/omah-config.toml"
mkdir -p "$T/diff-rm/nvim"
echo "init.lua" > "$T/diff-rm/nvim/init.lua"
omah --config "$T/diff-rm/omah-config.toml" add nvim "$T/diff-rm/nvim"
omah --config "$T/diff-rm/omah-config.toml" backup
rm "$T/diff-rm/nvim/init.lua"
```

**Test:**
1. `omah --config "$T/diff-rm/omah-config.toml" diff`

**Expected:**
```
nvim
  -  init.lua  only in vault
```

**Cleanup:** `rm -rf "$T/diff-rm"`

---

## TC-032: Diff shows in-sync state

**Setup:**
```sh
omah init --config "$T/diff-sync/omah-config.toml"
echo "stable" > "$T/diff-sync/file"
omah --config "$T/diff-sync/omah-config.toml" add myfile "$T/diff-sync/file"
omah --config "$T/diff-sync/omah-config.toml" backup
```

**Test:**
1. `omah --config "$T/diff-sync/omah-config.toml" diff`

**Expected:** `✓ All dotfiles are in sync with the vault.`

**Cleanup:** `rm -rf "$T/diff-sync"`

---

## TC-033: Diff --json outputs JSON

**Setup:** Same as TC-031

**Test:**
1. `omah --config "$T/diff-rm/omah-config.toml" diff --json`

**Expected:** Valid JSON array with `dot_name`, `path`, `kind` fields.

**Cleanup:** `rm -rf "$T/diff-rm"`

---

## TC-034: Config override with --config flag

**Setup:**
```sh
omah init --config "$T/flag/omah-config.toml"
echo "content" > "$T/flag/.zshrc"
omah --config "$T/flag/omah-config.toml" add zsh "$T/flag/.zshrc"
```

**Test:**
1. Run all commands with `--config "$T/flag/omah-config.toml"`
2. Verify they operate on the custom config, not default `~/.config/omah/omah-config.toml`

**Expected:** All commands use the specified config. Default config untouched.

**Cleanup:** `rm -rf "$T/flag"`

---

## TC-035: Config override with OMAH_CONFIG env var

**Setup:**
```sh
omah init --config "$T/env/omah-config.toml"
echo "content" > "$T/env/.zshrc"
omah --config "$T/env/omah-config.toml" add zsh "$T/env/.zshrc"
```

**Test:**
1. `OMAH_CONFIG="$T/env/omah-config.toml" omah list`
2. `OMAH_CONFIG="$T/env/omah-config.toml" omah status`

**Expected:** Both commands use the env-supplied config path.

**Cleanup:** `rm -rf "$T/env"`

---

## TC-036: --config flag takes precedence over OMAH_CONFIG env var

**Setup:**
```sh
omah init --config "$T/prec1/omah-config.toml"
omah init --config "$T/prec2/omah-config.toml"
echo "content" > "$T/prec1/.zshrc"
omah --config "$T/prec1/omah-config.toml" add zsh "$T/prec1/.zshrc"
```

**Test:**
1. `OMAH_CONFIG="$T/prec2/omah-config.toml" omah --config "$T/prec1/omah-config.toml" list`
2. Should show zsh from prec1, not prec2

**Expected:** Flag wins. Shows `zsh` dotfile.

**Cleanup:** `rm -rf "$T/prec1" "$T/prec2"`

---

## TC-037: Empty config (no dots) — commands don't crash

**Setup:**
```sh
omah init --config "$T/empty/omah-config.toml"
# no dots added
```

**Test:**
1. `omah --config "$T/empty/omah-config.toml" backup`
2. `omah --config "$T/empty/omah-config.toml" restore`
3. `omah --config "$T/empty/omah-config.toml" status`
4. `omah --config "$T/empty/omah-config.toml" diff`

**Expected:** No errors. Each command handles gracefully (backup/restore no-op, status shows "No dotfiles configured.", diff shows "in sync").

**Cleanup:** `rm -rf "$T/empty"`

---

## TC-038: Invalid TOML config — parse error

**Setup:**
```sh
mkdir -p "$T/bad-toml"
echo "invalid }{ toml" > "$T/bad-toml/omah-config.toml"
```

**Test:**
1. `omah --config "$T/bad-toml/omah-config.toml" list`

**Expected:** Error: `Failed to parse config file: ...`

**Cleanup:** `rm -rf "$T/bad-toml"`

---

## TC-039: Missing config file — clear error

**Test:**
1. `omah --config /nonexistent/path/omah-config.toml list`

**Expected:** Error: `Failed to read config file: /nonexistent/path/omah-config.toml`

---

## TC-040: Missing required field (vault_path) — parse error

**Setup:**
```sh
mkdir -p "$T/no-vault"
echo 'dots = []' > "$T/no-vault/omah-config.toml"
```

**Test:**
1. `omah --config "$T/no-vault/omah-config.toml" status`

**Expected:** Error — missing field `vault_path`.

**Cleanup:** `rm -rf "$T/no-vault"`

---

## TC-041: Backup with progress bar (many files)

**Setup:**
```sh
omah init --config "$T/progress/omah-config.toml"
mkdir -p "$T/progress/bigdir"
for i in $(seq 1 20); do echo "file $i" > "$T/progress/bigdir/file$i"; done
omah --config "$T/progress/omah-config.toml" add big "$T/progress/bigdir"
```

**Test:**
1. Run `omah --config "$T/progress/omah-config.toml" backup` on a real TTY
2. Verify progress output: `big:  5/20 (25%)` updating on stderr

**Expected:** Progress bar shown when >5 files and stderr is a terminal.

**Cleanup:** `rm -rf "$T/progress"`

---

## TC-042: .DS_Store always excluded

**Setup:**
```sh
omah init --config "$T/ds/omah-config.toml"
mkdir -p "$T/ds/dir"
echo "real" > "$T/ds/dir/real.txt"
touch "$T/ds/dir/.DS_Store"
omah --config "$T/ds/omah-config.toml" add mydir "$T/ds/dir"
omah --config "$T/ds/omah-config.toml" backup
```

**Test:**
1. Check vault — `real.txt` present
2. `.DS_Store` should NOT be in vault (even without user exclude pattern)

**Expected:** `.DS_Store` excluded automatically.

**Cleanup:** `rm -rf "$T/ds"`

---

## TC-043: Symlink loop prevention

**Setup:**
```sh
omah init --config "$T/loop/omah-config.toml"
echo "content" > "$T/loop/file"
omah --config "$T/loop/omah-config.toml" add myfile "$T/loop/file" --symlink
omah --config "$T/loop/omah-config.toml" backup  # type y
```

**Test:**
1. `omah --config "$T/loop/omah-config.toml" backup`
2. Should report `myfile: up-to-date (symlink → vault)`
3. Vault content should not be zeroed

**Expected:** Second backup detects existing symlink → vault, skips, content preserved.

**Cleanup:** `rm -rf "$T/loop"`

---

## TC-044: Restore — overwrite prompt when source exists

**Setup:**
```sh
omah init --config "$T/overwrite/omah-config.toml"
echo "original" > "$T/overwrite/file"
omah --config "$T/overwrite/omah-config.toml" add myfile "$T/overwrite/file"
omah --config "$T/overwrite/omah-config.toml" backup
```

Then modify source and vault differently but in dev mode the restore uses `dev/restored/`. Let me design this properly for dev mode.

Actually, the overwrite prompt (`dev_overwrite_prompt`) is only used in dev mode (when `.env` file exists). For normal restore, there's no overwrite prompt — it just overwrites. Let me test the normal behavior:

**Simpler test:**
**Setup:**
```sh
omah init --config "$T/overwrite/omah-config.toml"
echo "vault content" > "$T/overwrite/file"
omah --config "$T/overwrite/omah-config.toml" add myfile "$T/overwrite/file"
omah --config "$T/overwrite/omah-config.toml" backup
echo "modified source" > "$T/overwrite/file"
```

**Test:**
1. `omah --config "$T/overwrite/omah-config.toml" restore`
2. File overwritten with vault content

**Expected:** Restore overwrites existing source with vault content.

**Cleanup:** `rm -rf "$T/overwrite"`

---

## TC-045: Config os field — explicit override

**Setup:**
```sh
omah init --config "$T/os-test/omah-config.toml"
```

Edit `$T/os-test/omah-config.toml` to add `os = "linux"` at top level.

**Test:**
1. `omah --config "$T/os-test/omah-config.toml" list`

**Expected:** No error. OS setting is stored and round-tripped. Check config file after list (it shouldn't modify the file, only read).

**Cleanup:** `rm -rf "$T/os-test"`

---

## TC-046: Config pkg_manager field — explicit override

**Setup:**
```sh
omah init --config "$T/pm-test/omah-config.toml"
```

Edit to add `pkg_manager = "nix-env"`.

**Test:**
1. `omah --config "$T/pm-test/omah-config.toml" list`

**Expected:** No error. Custom pkg_manager preserved.

**Cleanup:** `rm -rf "$T/pm-test"`

---

## TC-047: Setup step checks — each check type

**Setup:**
```sh
omah init --config "$T/checks/omah-config.toml"
echo "content" > "$T/checks/file"
```

Create a config `$T/checks/omah-config.toml` with a dotfile that has setup steps:

Note: use `omah add` first, then edit the config to add setup steps. Or write it directly.

Write this config:
```toml
vault_path = "~/temp-vault"

[[dots]]
name = "test"
source = ".../file"
setup = [
  { install = "echo 'bin:sh check'", check = "bin:sh" },
  { install = "echo 'skipped'", check = "skip" },
  { install = "echo 'cmd test'", check = "cmd:true" },
]
```

**Test:**
1. `omah --config "$T/checks/omah-config.toml" info`
2. Verify setup steps shown with status

**Expected:** Steps with satisfied checks show `(done)`. Pending steps show `(pending)`.

**Cleanup:** `rm -rf "$T/checks"`

---

## TC-048: Dependency checking in status

**Setup:**
```sh
omah init --config "$T/deps/omah-config.toml"
echo "content" > "$T/deps/file"
```

Write config:
```toml
vault_path = "~/temp-vault"

[[dots]]
name = "test"
source = ".../file"
deps = ["sh", "xyzzy_nope_does_not_exist"]
```

**Test:**
1. `omah --config "$T/deps/omah-config.toml" status`
2. Verify `sh` is not listed as missing (installed)
3. Verify `xyzzy_nope_does_not_exist` is shown as missing

**Expected:**
```
  test    ...   ⚠ unbacked
                     missing deps: xyzzy_nope_does_not_exist
```

**Cleanup:** `rm -rf "$T/deps"`

---

## TC-049: Install command generation for each package manager

**Test (verify via unit tests — these are tested in code):**
```sh
cargo test --package omah_lib test_install_command
```

But for manual confirmation, check that the formulas match:

| Package manager | Command template |
|----------------|-----------------|
| brew | `brew install pkg1 pkg2` |
| apt-get | `sudo apt-get install -y pkg1 pkg2` |
| pacman | `sudo pacman -S --noconfirm pkg1 pkg2` |
| dnf | `sudo dnf install -y pkg1 pkg2` |
| zypper | `sudo zypper install -y pkg1 pkg2` |
| (custom) | `custom-tool install pkg1 pkg2` |

---

## TC-050: Dev mode restore (dev/restored/ prefix)

**Setup:**
```sh
omah init --config "$T/dev/omah-config.toml"
echo "content" > "$T/dev/file"
omah --config "$T/dev/omah-config.toml" add myfile "$T/dev/file"
omah --config "$T/dev/omah-config.toml" backup
```

**Test:**
1. `cd "$T/dev" && touch .env`
2. `omah --config "$T/dev/omah-config.toml" restore`
3. Verify output mentions `dev mode: restoring to dev/restored/`
4. Verify `dev/restored/file` exists with content

**Expected:** In dev mode, sources are prefixed with `dev/restored/` instead of overwriting originals.

**Cleanup:** `rm -rf "$T/dev"`

---

## TC-051: Restore with dependency and setup steps flow

**Setup:**
```sh
omah init --config "$T/full-rest/omah-config.toml"
echo "content" > "$T/full-rest/file"
```

Write config:
```toml
vault_path = "~/temp-vault"

[[dots]]
name = "test"
source = ".../file"
deps = ["sh"]
setup = [
  { install = "echo 'post-restore step'", check = "skip" },
]
```

1. `omah --config "$T/full-rest/omah-config.toml" backup`
2. `omah --config "$T/full-rest/omah-config.toml" restore`

**Test:**
1. Restore should show the pending setup step
2. Prompt "Run all? [y/N]"
3. Type `y` — should run `echo 'post-restore step'`
4. Then restore should complete

**Expected:** Steps run before restore files are copied back.

**Note:** If `skip` check means it's always pending... actually `skip` means NEVER pending. So use `check` = none for always-pending, or just verify the flow works.

**Cleanup:** `rm -rf "$T/full-rest"`

**Revised — use no `check` field for always-pending:**
```toml
setup = [
  { install = "echo 'post-restore step'" },
]
```

---

## TC-052: `omah backup --dry-run` with symlink shows plan

**Setup:**
```sh
omah init --config "$T/bak-dr-sym/omah-config.toml"
echo "content" > "$T/bak-dr-sym/file"
omah --config "$T/bak-dr-sym/omah-config.toml" add myfile "$T/bak-dr-sym/file" --symlink
```

**Test:**
1. `omah --config "$T/bak-dr-sym/omah-config.toml" backup --dry-run`

**Expected:**
```
Backup plan:

  myfile: .../file → .../vault/myfile/file (1 files) [symlink]
```

**Cleanup:** `rm -rf "$T/bak-dr-sym"`

---

## TC-053: No subcommand shows banner + help

**Test:**
1. Run `omah` with no arguments

**Expected:**
- Animated "OMAH" ASCII banner in blue tones (static if non-TTY)
- Help output showing all subcommands
- No error (exit code 0)

---

## TC-054: `omah init` shows banner

**Test:**
1. `omah init --config /tmp/omah-init-banner-test.toml`

**Expected:** Banner displayed before "Initialized:" output.

**Cleanup:** `rm -f /tmp/omah-init-banner-test.toml`

---

## TC-055: Config with os and pkg_manager fields round-trips correctly

**Setup:**
```sh
omah init --config "$T/rtrip/omah-config.toml"
```

Edit config to add `os = "macos"` and `pkg_manager = "brew"`.

**Test:**
1. `omah --config "$T/rtrip/omah-config.toml" list`
2. Read config file — verify `os` and `pkg_manager` still present

**Expected:** Fields preserved. `list` does not modify config file.

**Cleanup:** `rm -rf "$T/rtrip"`

---

## TC-056: Backup creates vault directory if missing

**Setup:**
```sh
omah init --config "$T/novault/omah-config.toml"
echo "content" > "$T/novault/file"
omah --config "$T/novault/omah-config.toml" add myfile "$T/novault/file"
# vault path does not exist yet
```

**Test:**
1. `omah --config "$T/novault/omah-config.toml" backup`
2. Verify vault directory created at configured path

**Expected:** Vault auto-created. No error.

**Cleanup:** `rm -rf "$T/novault"`

---

## Test coverage summary

| Area | TC IDs | Count |
|------|--------|-------|
| Init | 001, 002, 054 | 3 |
| Config management (add/remove/list/info) | 003–010 | 8 |
| Backup | 011–020, 041–043, 052, 056 | 14 |
| Restore | 021–025, 044, 050, 051 | 8 |
| Status | 026–028, 048 | 4 |
| Diff | 029–033 | 5 |
| Setup & deps | 047–049, 051 | 4 |
| Config overrides | 034–036, 055 | 4 |
| Edge cases | 037–040, 045–046 | 6 |
| No-args / help | 053 | 1 |

**Total: 56 test cases**
