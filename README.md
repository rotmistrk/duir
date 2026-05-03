# duir 🌳

Hierarchical todo tree manager with vim-like editor and per-subtree encryption.

Named after *duir* — Irish for "oak" in the Ogham tree alphabet, root of
"druid" (oak-knower), and it sounds like "do it".

## Features

- **Tree**: checkboxes, importance, completion %, drag-reorder, clone, filter
- **Editor**: vim keybindings (normal/insert/visual), ex-commands, search, shell pipe
- **Markdown**: syntax highlighting, fenced code blocks (100+ languages via syntect)
- **Export**: markdown, Word .docx (with diagram rendering), PDF, clipboard
- **Import**: markdown, Word .docx (headings, tables, code blocks), Qt ToDo XML
- **Diagrams**: mermaid, plantuml, graphviz rendered as images in docx export
- **Encryption**: per-subtree with password, hierarchical, auto-lock on collapse
- **Files**: multi-file, autosave, JSON storage, YAML export, S3 support, path completion
- **Config**: XDG-compliant, project-local `.duir/`, configurable autosave interval
- **Clipboard**: system clipboard via OSC 52 (works over SSH)
- **Kiro Integration**: embedded AI terminal (kiro-cli), prompt/response flow, MCP server
- **Stable Identity**: FileId + NodeId for corruption-proof tree operations
- **Self-contained**: single binary, all resources embedded

## Install

```sh
make install          # → /usr/local/bin/duir (sudo)
make install-local    # → ~/.local/bin/duir (no sudo)
```

## Usage

```sh
duir                          # load from ~/.local/share/duir/
duir -d ~/projects/.duir      # load from specific directory
duir file1.todo.json file2.todo.json  # open specific files
```

Press `F1` or `:help` for the full key reference.

## Config

```
~/.config/duir/config.toml    — global
~/.duirrc                      — user shorthand
.duir/config.toml              — project-local
```

```toml
[storage]
central = "~/.local/share/duir"

[editor]
autosave = true
autosave_interval_secs = 30

[ui]
note_panel_pct = 50
```

## S3

Open, save, import, and export files directly from S3:

```sh
duir s3://my-bucket/todos/work.todo.json
```

```
:open s3://my-bucket/todos/work.todo.json
:export s3://my-bucket/reports/sprint.md
```

Credentials use the standard AWS chain (env vars, `~/.aws/credentials`, instance role). Tab completion lists buckets and objects.

## Kiro Integration

Embed kiro-cli as an AI planning assistant.
A *kiron* (kiro node; the -on suffix like in electron, proton, neutron) is a tree node
that hosts an AI session:

```
:kiron              Mark node as AI session (shows 🤖)
:kiro start         Launch kiro-cli in embedded terminal (shows 🤖▶)
Enter               Send node as prompt to kiro (in kiron subtree)
Ctrl+\ / Opt+\      Send from any panel
Ctrl+T              Cycle focus: Tree → Note → Kiro
F2/F3/F4            Focus tree / note / kiro directly
Ctrl+R              Capture kiro response as 💡 node
```

Kiro can read and write to the subtree via MCP (Model Context Protocol).
Active panel has cyan border; inactive has gray.

Configure in config.toml:

```toml
[kiro]
command = "kiro-cli"
args = ["chat", "--resume"]
sop = "After each request, use add_child to record what you did."
```

## Legacy Import

```
:open file.todo     Import Qt ToDo XML files
```

## Project Structure

```
crates/
  duir-core/    — data model, storage, crypto, markdown, mcp_server, legacy_import
  duir-tui/     — terminal UI (ratatui)
planning/       — epics, stories, tasks
```

## Tests

```sh
make check    # fmt + clippy + lint checks + 298 tests
```

## License

MIT

## Roadmap

Future ideas (not currently planned):
- **Unicode diagram rendering**: Rust library to render mermaid/plantuml/graphviz as Unicode box-drawing art in the terminal (separate project)
- **HTML preview**: `:preview` command to render note as HTML and open in system browser
- **Collaborative editing**: CRDT-based real-time sync
