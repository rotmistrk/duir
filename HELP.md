# Duir — Quick Reference

## Tree Navigation

| Key | Action |
|-----|--------|
| `↑`/`↓` | Move cursor up/down |
| `←` | Collapse node / go to parent |
| `→` | Expand node (prompts password if encrypted) |
| `Space` | Toggle completed |
| `Enter` | Send to kiro (in kiron subtree) / no-op otherwise |
| `e` | Edit item title |
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
| `:open <file>` | Open file (auto-detect .md/.json/.todo/.docx) |
| `:import <file.md>` | Import markdown under current item |
| `:import <file.docx>` | Import Word document under current item |
| `:export [file.md]` | Export subtree as markdown |
| `:export [file.docx]` | Export subtree as Word document |
| `:export [file.pdf]` | Export subtree as PDF |
| `:write <name>` | Save copy as todo JSON (doesn't switch) |
| `:saveas <name>` | Save as todo JSON and switch to it |

All file commands accept `s3://` paths (e.g. `:open s3://bucket/file.todo.json`).

Legacy Qt ToDo `.todo` XML files are auto-detected and imported by `:open`.

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

### Kiro Integration (AI Planning)

| Command | Action |
|---------|--------|
| `:kiron` | Mark current node as AI session (kiron) |
| `:kiron disable` | Remove kiron marking (must stop first) |
| `:kiro start` | Start kiro-cli on current kiron node (with MCP) |
| `:kiro stop` | Stop kiro session |
| `:kiro new` | New session (stop, new session ID, start fresh) |
| `:kiro capture` | Capture kiro response as sibling node |
| `Ctrl+\` / `Opt+\` | Send current node as prompt to kiro |
| `Ctrl+T` | Cycle focus: Tree → Note → Kiro → Tree |
| `Ctrl+R` | Capture kiro response (tree focus, in kiron subtree) |
| `F2` / `Alt+2` | Focus tree (keep right panel as-is) |
| `F3` / `Alt+3` | Focus note panel |
| `F4` / `Alt+4` | Focus kiro panel |
| `PgUp` / `PgDn` | Scroll kiro terminal buffer (in kiro panel) |
| `Tab` | In kiro panel: tab completion (passed to kiro) |
|  | In tree: open note editor (normal behavior) |

Kiron (kiro node; the -on suffix like in electron, proton, neutron) nodes show 🤖 in the tree.
Running kiro shows 🤖▶ on the kiron node.
When inside an active kiron's subtree, the right panel shows
tabs: 📝 Note │ 🤖 Kiro. Active panel has cyan border.

Ctrl+T switches between tree and kiro panel. All typing goes
to kiro when its panel is focused (including Ctrl keys like
Ctrl+C, Ctrl+J). Ctrl+S still saves globally.

Ctrl+\ (or Opt+\ on macOS) sends the current node and its
descendants as markdown to kiro. Use Ctrl+R or `:kiro capture`
to grab the response as a new sibling node marked with 💡.
Sending a new prompt auto-captures any pending previous response.

MCP server starts automatically with `:kiro start`. Kiro can
read and modify the task tree via MCP tools (read_node,
list_children, add_child, mark_done, etc.).

Kiro configuration in `config.toml`:

```toml
[kiro]
command = "kiro-cli"
args = ["chat", "--resume"]
sop = """
After each user request, use add_child to record what you did.
Use the user's request as title and your summary as note.
Mark completed items with mark_done.
"""
```

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

## Syntax Highlighting

The note editor highlights markdown and fenced code blocks:

- Headings, bold, italic, inline code, links, checkboxes, blockquotes
- Fenced code blocks with 100+ languages via syntect (base16-ocean.dark theme)
- Cursor preserves syntax colors in normal mode

## Diagrams

Diagram blocks in notes are rendered as images in `.docx` export:

| Block | Tool required |
|-------|---------------|
| ` ```mermaid ` | `mmdc` (mermaid-cli, requires Node.js) |
| ` ```plantuml ` | `plantuml` (requires Java) |
| ` ```dot ` / ` ```graphviz ` | `dot` (graphviz) |

If the tool is not installed, the source text is included as a code block.

Tool paths are configurable:

```toml
[diagrams]
mmdc = "mmdc"
plantuml = "plantuml"
dot = "dot"
```

## S3 Storage

Path format: `s3://bucket/prefix/file`

Tab completion lists buckets and objects.

Supported commands: `:open`, `:import`, `:export`, `:write`, `:saveas`.

Credentials use the standard AWS chain: environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`), `~/.aws/credentials`, or instance role.

Examples:

```
:open s3://my-bucket/todos/work.todo.json
:export s3://my-bucket/reports/sprint.md
:saveas s3://my-bucket/todos/backup.todo.json
```

## Status Colors

| Color | Meaning |
|-------|---------|
| Green | Success (saved, encrypted, unlocked) |
| Yellow | Warning (confirm delete, unlock first) |
| Red | Error (wrong password, save failed) |
| Gray | Info |
