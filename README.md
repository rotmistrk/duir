# duir 🌳

Hierarchical todo tree manager with vim-like editor and per-subtree encryption.

Named after *duir* — Irish for "oak" in the Ogham tree alphabet, root of
"druid" (oak-knower), and it sounds like "do it".

## Features

- **Tree**: checkboxes, importance, completion %, drag-reorder, clone, filter
- **Editor**: vim keybindings (normal/insert/visual), ex-commands, search, shell pipe
- **Markdown**: syntax highlighting, export/import, collapse subtree ↔ markdown
- **Encryption**: per-subtree with password, hierarchical, auto-lock on collapse
- **Files**: multi-file, autosave, JSON storage, YAML export, path completion
- **Config**: XDG-compliant, project-local `.duir/`, configurable autosave interval
- **Clipboard**: system clipboard via OSC 52 (works over SSH)
- **Self-contained**: single 5.7MB binary, all resources embedded

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

## Project Structure

```
crates/
  duir-core/    — data model, storage, crypto, markdown
  duir-tui/     — terminal UI (ratatui)
planning/       — epics, stories, tasks
```

## Tests

```sh
make check    # fmt + clippy + 201 tests
```

## License

MIT
