# Duir — Quick Reference

## Tree Navigation

| Key | Action |
|-----|--------|
| `↑`/`↓` | Move cursor up/down |
| `←` | Collapse node / go to parent |
| `→` | Expand node |
| `Space` | Toggle completed |
| `Enter` | Edit item title |
| `Tab` | Switch to note editor |
| `]`/`[` | Grow/shrink note panel |

## Tree Operations

| Key | Action |
|-----|--------|
| `n` | New sibling task |
| `b` | New child task (branch) |
| `d` | Delete task |
| `c` | Clone subtree |
| `!` | Toggle importance |
| `S` | Sort children |
| `Shift+↑`/`K` | Swap up (reorder) |
| `Shift+↓`/`J` | Swap down (reorder) |
| `Shift+←`/`H` | Promote (to parent level) |
| `Shift+→`/`L` | Demote (child of prev sibling) |

## App Commands (`:` in tree mode)

| Command | Action |
|---------|--------|
| `:w` | Save current file |
| `:wa` | Save all files |
| `:q` | Close current file |
| `:qa` / `:q!` | Quit |
| `:e <name>` | New empty file |
| `:o <path>` | Open file |
| `:open md <file>` | Open markdown as tree |
| `:import md <file>` | Import markdown under current item |
| `:export md [file]` | Export subtree as markdown |
| `:collapse` | Collapse children to markdown note |
| `:expand` | Expand markdown note to children |
| `:autosave` | Toggle autosave (current file) |
| `:autosave all` | Toggle global autosave |
| `:init` | Create `.duir/` in current directory |
| `:config` | Show effective config |
| `:config write` | Write config to file |
| `:help` | Show this help |
| `:about` | About duir |
| `/` | Filter tree (searches titles + notes) |

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
| `x` | Delete char |
| `dd` | Delete line(s) — `3dd` deletes 3 |
| `yy` | Yank line(s) — `2yy` yanks 2 |
| `p`/`P` | Paste after/before |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `>`/`<` | Indent/unindent — `3>` indents 3 lines |
| `/` | Search (live preview) |
| `n`/`N` | Next/prev search match |
| `:` | Command mode |

### Visual Mode

| Key | Action |
|-----|--------|
| `h`/`j`/`k`/`l` | Extend selection |
| `y` | Yank selection |
| `d`/`x` | Cut selection |
| `>`/`<` | Indent/unindent selection |
| `Esc` | Cancel |

### Editor Commands (`:` in note editor)

| Command | Action |
|---------|--------|
| `:set nu` / `:set num` | Line numbers on |
| `:set nonu` | Line numbers off |
| `:1,$y` | Yank all lines |
| `:1,.s/^/# /` | Substitute in range |
| `:%s/foo/bar/g` | Global find/replace |
| `:3,7d` | Delete lines 3-7 |
| `:.-5,.+5!sort` | Pipe range through shell |
| `:!date` | Insert shell output at cursor |

### Count Prefix

Most normal-mode commands accept a count: `3dd`, `5j`, `2yy`, `4w`, `3>`.
