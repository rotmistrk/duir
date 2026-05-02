# Contributing to duir

## Setup

```sh
git clone git@github.com:rotmistrk/duir.git
cd duir
git config core.hooksPath .githooks
cargo build
```

## Pre-commit Hook

The `.githooks/pre-commit` hook runs:
1. `cargo fmt --check`
2. `cargo clippy -D warnings` (pedantic)
3. `cargo test`

Commits are rejected if any step fails.

## Code Standards

- **Clippy pedantic** with `-D warnings`
- **No `unsafe`** (`forbid(unsafe_code)`)
- **No `unwrap`/`expect`/`panic`** in non-test code
- **Typestate pattern**: use the type system to prevent illegal states
- **FileId + NodeId**: never use raw indices as identity keys
- **Private modified flag**: all mutations go through `mark_modified()`
- **Tests exercise real code paths**, not mocks
- **HELP.md must be updated** with every feature change (embedded in binary)

## Architecture

```
crates/
  duir-core/    Data model, storage, crypto, export, MCP server
  duir-tui/     Terminal UI, input handling, rendering
```

Key invariants:
- Model is source of truth for notes (editor is a working copy)
- `rebuild_rows()` = `rebuild_rows_raw()` + `reapply_filter()`
- `mark_modified()` invalidates cipher on encrypted ancestors
- `save_editor()` before any operation that reads from model

## Testing

```sh
make check            # fmt + clippy + tests
cargo test --workspace # just tests
cargo tarpaulin       # coverage
```

## Pull Requests

Not currently accepting external PRs. This is a personal project.
