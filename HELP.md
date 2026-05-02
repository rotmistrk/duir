# Duir — Quick Reference

## Tree Navigation

| Key | Action |
|-----|--------|
| `↑`/`↓` | Move cursor up/down |
| `←` | Collapse node / go to parent |
| `→` | Expand node (prompts password if encrypted) |
| `Space` | Toggle completed |
| `Enter` | Edit item title |
| `Tab` | Switch to note editor |
| `]`/`[` | Grow/shrink note panel |
| `F1` | Open help |

## Tree Operations

| Key | Action |
|-----|--------|
| `n` | New sibling task |
| `b` | New child task (branch) |
| `d` | Delete task (y to confirm if incomplete/has children) |
| `c` | Clone subtree |
| `!` | Toggle importance |
| `S` | Sort children |
| `Shift+↑`/`K` | Swap up (reorder) |
| `Shift+↓`/`J` | Swap down (reorder) |
| `Shift+←`/`H` | Promote (to parent level) |
| `Shift+→`/`L` | Demote (child of prev sibling) |

## Filter (`/` in tree mode)

| Key | Action |
|-----|--------|
| `/` | Open filter (pre-filled with current filter) |
| Type | Live filter-as-you-type |
| `Enter` | Confirm filter |
| `Esc` | Revert to previous filter |
| `!text` | Exclude mode (hide matches) |

Filter searches titles AND notes. Tree title shows `[/text]` when filtered.
Filter persists through expand/collapse.

## App Commands (`:` in tree mode)

Tab completes commands. Tab also completes file paths for file commands.

### File Operations

| Command | Action |
|---------|--------|
| `:w` | Save current file |
| `:wa` | Save all files |
| `:q` | Close current file |
| `:qa` / `:q!` | Quit |
| `:e <name>` | New empty file |
| `:o <path>` | Open file (add as top-level tree) |
| `:open <file>` | Open file (auto-detect .md/.json) |
| `:import <file.md>` | Import markdown under current item |
| `:export [file.md]` | Export subtree as markdown |
| `:export [file.docx]` | Export subtree as Word document |
| `:write <name>` | Save copy as todo JSON (doesn't switch) |
| `:saveas <name>` | Save as todo JSON and switch to it |

### Tree Operations

| Command | Action |
|---------|--------|
| `:collapse` | Collapse children to markdown note |
| `:expand` | Expand markdown note to children |
| `:yank` | Copy subtree as markdown to clipboard |

### Encryption

| Command | Action |
|---------|--------|
| `:encrypt` | Encrypt current subtree (prompts password) |
| `:decrypt` | Remove encryption (must unlock first) |
| `→` on 🔒 | Unlock (prompts password) |
| `←` on 🔓 | Lock (re-encrypts, forgets password) |

### Settings

| Command | Action |
|---------|--------|
| `:autosave` | Toggle autosave (current file) |
| `:autosave all` | Toggle global autosave |
| `:init` | Create `.duir/` in current directory |
| `:config` | Show effective config |
| `:config write` | Write config to file |
| `:help` | Show this help |
| `:about` | About duir |

## Note Editor (Tab to enter, Tab to return)

### Normal Mode

| Key | Action |
|-----|--------|
| `i`/`a`/`I`/`A`/`o`/`O` | Enter insert mode |
| `v` | Visual (character) selection |
| `V` | Visual (line) selection |
| `h`/`j`/`k`/`l` | Navigate |
| `w`/`b` | Word forward/back |
| `0`/`$` | Line start/end |
| `g`/`G` | File top/bottom |
| `Ctrl+U`/`Ctrl+D` | Half-page up/down |
| `PgUp`/`PgDn` | Half-page up/down |
| `x` | Delete char |
| `dd` | Delete line(s) — `3dd` deletes 3 |
| `yy` | Yank line(s) — `2yy` yanks 2 |
| `p`/`P` | Paste after/before |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `>`/`<` | Indent/unindent — `3>` indents 3 lines |
| `/` | Search (live preview) |
| `n`/`N` | Next/prev search match |
| `Shift+Enter` | Open URL under cursor |
| `:` | Command mode |

### Insert Mode

| Key | Action |
|-----|--------|
| `Esc` | Return to normal mode |
| `Tab` | Insert to next tab stop |
| `Backspace` | Delete (back to prev tab stop on leading whitespace) |
| `Enter` | New line with auto-indent |

### Visual Mode

| Key | Action |
|-----|--------|
| `h`/`j`/`k`/`l` | Extend selection |
| `y` | Yank selection (copies to system clipboard) |
| `d`/`x` | Cut selection |
| `>`/`<` | Indent/unindent selection |
| `Esc` | Cancel |

### Editor Commands (`:` in note editor)

| Command | Action |
|---------|--------|
| `:set nu` / `:set num` | Line numbers on |
| `:set nonu` | Line numbers off |
| `:1,$y` | Yank all lines |
| `:%s/foo/bar/g` | Global find/replace |
| `:3,7d` | Delete lines 3-7 |
| `:.-5,.+5!sort` | Pipe range through shell |
| `:!date` | Insert shell output at cursor |

### Count Prefix

Most normal-mode commands accept a count: `3dd`, `5j`, `2yy`, `4w`, `3>`.

All yank/copy/cut operations sync to system clipboard (OSC 52).

## Config

```
~/.config/duir/config.toml    — global
~/.duirrc                      — user shorthand
.duir/config.toml              — project-local (highest priority)
```

```toml
[storage]
central = "~/.local/share/duir"
local = ".duir"

[editor]
autosave = true
autosave_interval_secs = 30
tab_width = 4
line_numbers = false

[ui]
note_panel_pct = 50
```

## Status Colors

| Color | Meaning |
|-------|---------|
| Green | Success (saved, encrypted, unlocked) |
| Yellow | Warning (confirm delete, unlock first) |
| Red | Error (wrong password, save failed) |
| Gray | Info |
