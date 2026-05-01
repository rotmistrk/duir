# duir 🌳

Hierarchical todo tree manager with markdown notes and vim-like editor.

Named after *duir* — Irish for "oak" in the Ogham tree alphabet, root of
"druid" (oak-knower), and it sounds like "do it".

## Features

- Tree of tasks with checkboxes, importance flags, completion percentages
- Vim-like markdown note editor per item (visual mode, search, ex-commands, shell pipe)
- Multiple files as top-level tree nodes
- Move items between files
- Filter/search across tree and notes
- Export subtree as `.md`, import `.md` as tree
- JSON storage (YAML import/export supported)
- Autosave (on by default)
- Command mode (`:`) for file ops, export, collapse/expand

## Building

```sh
cargo build --release
```

## Running

```sh
cargo run -p duir-tui --release
```

## Config

- `$XDG_CONFIG_HOME/duir/config.toml` — global config
- `~/.duirrc` — user shorthand
- `.duir/config.toml` — project-local config
- Data: `$XDG_DATA_HOME/duir/` (central), `.duir/` (local, opt-in via `:init`)

## Project Structure

```
crates/
  duir-core/    — data model, storage, markdown import/export
  duir-tui/     — terminal UI (ratatui)
planning/       — epics, stories, tasks (agent-driven development)
```

## License

MIT
