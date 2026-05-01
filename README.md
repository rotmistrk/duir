# omela 🌿

Hierarchical todo tree manager with markdown notes.

Terminal UI (TUI) application for managing nested task trees across multiple files,
with cloud sync support.

Named after омела (mistletoe) — the plant that grows as a bush on tree branches,
and the only thing that could kill the god Baldr.

## Features (planned)

- Tree of tasks with checkboxes, importance flags, completion percentages
- Markdown notes per item
- Multiple files as top-level tree nodes
- Move items between files
- Filter/search across tree and notes
- Export subtree as `.md`, import `.md` as subtree
- JSON storage (YAML import/export supported)
- S3 sync backend
- Legacy import from Qt ToDo `.todo` XML files

## Building

```sh
cargo build --release
```

## Running

```sh
cargo run -p omela-tui
```

## Project Structure

```
crates/
  omela-core/    — data model, storage, markdown import/export
  omela-tui/     — terminal UI (ratatui)
planning/        — epics, stories, tasks (agent-driven development)
```

## License

MIT
