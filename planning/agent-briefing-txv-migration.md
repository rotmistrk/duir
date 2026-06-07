# duir TXV Migration — Agent Briefing

## Context

duir is a todo/planning TUI tool written in Rust. It currently uses ratatui for
rendering, which has proven too primitive for complex widget interactions (same
problem kairn had before its rewrite).

The decision is made: **rewrite duir-tui using txv (rusticle-tk)** — the same
framework that powers kairn (a sibling project in ../kairn/).

## Key Projects

- **duir** (`/home/rotmistr/Workplace/duir/`) — the project being rewritten
  - `crates/duir-core/` — domain logic (model, storage, tree ops, MCP, crypto, export/import). Keep as-is.
  - `crates/duir-tui/` — current ratatui TUI. Being **replaced**.
- **kairn** (`/home/rotmistr/Workplace/kairn/`) — reference implementation. A terminal editor/IDE built on txv.
  - `rusticle-tk/` — the txv Rust framework (widget toolkit). This is the dependency to use.
  - `src/views/todo_tree/` — kairn's todo tree implementation (port from here)
  - `src/slots.rs`, desktop layout — TiledWorkspace (port from here)
  - `src/views/terminal.rs` — PTY terminal (port from here)
  - `src/editor/` — vi-modal editor (port from here)
  - `src/mcp/` — MCP server (adapt for duir)

## Design Documents (READ THESE FIRST)

1. `planning/ondeck/federated-coordinator.md` — Architecture for multi-instance
   coordination (coordinator/project model, socket protocol, pockets). Implementation
   is Phase 2; understand the design so Phase 1 doesn't conflict.

2. `planning/ondeck/txv-migration-requirements.md` — Detailed requirements for the
   TXV migration. Covers: layout, columns, badges, connectors, timestamps, time
   tracking, what comes from kairn vs. what's new.

3. `../kairn/doc/f4-design/STATUS.md` — kairn's feature table and dev SOP.
   Follow the same conventions: 240 code-line limit per file, test-first, clippy clean.

4. `../kairn/doc/f4-design/v-016-todo-improvements.md` — Badge column design,
   priority/effort/status model, FocusGatedGroup widget.

## Architecture Decisions

- **Tree is primary.** duir starts with tree zoomed. Everything else is secondary.
- **TiledWorkspace layout:** Left=tree, Center=note editor, Right=shell/kiro, Bottom=messages.
- **Model lives in duir-core.** The TUI reads/writes through duir-core's API. Don't duplicate model logic.
- **MCP server** is how kiro talks to duir. Full access to tree operations.
- **No coordinator protocol in Phase 1.** Just standalone local instances.

## Implementation Plan (Phase 1: TXV Migration)

### Step 1: Scaffold
- Add rusticle-tk as dependency (path dep from ../kairn/rusticle-tk or publish)
- Create new binary using TiledWorkspace (4-slot layout)
- Tree zoomed on start
- Basic key dispatch: F2-F5 (slot focus), Ctrl-Q (quit), F5 (zoom toggle)
- Wire up duir-core: load `.duir/todo.json`, display tree

### Step 2: Todo Tree View
- Port kairn's `src/views/todo_tree/` as starting point
- Integrate with duir-core's model (richer: NodeType, KironMeta, cipher, etc.)
- Implement 5-char badge column: [status][priority][effort][notes][type]
- Implement tree connectors (configurable: unicode/powerline/none)
- Implement 3 timestamp columns (created/started/last-action, each 5 chars, age-adaptive)
- Time-spent tracking (status-driven, explicit pause)
- All tree operations: add, delete, move, promote, demote, fold, edit title, toggle status

### Step 3: Note Editor
- Port kairn's editor view into center slot
- Connect to item's note field (load on select, save on change)
- Markdown syntax highlighting

### Step 4: Shell & Kiro
- Port kairn's terminal view into right slot
- PTY management, kiro launch command
- MCP server with duir-specific tools (tree read/write, item operations)

### Step 5: Polish
- Status bar with context-sensitive sections (FocusGatedGroup)
- File watcher (auto-reload on external .duir/todo.json change)
- Session persistence (remember layout state, selected item)
- Encryption UI (password prompt, lock/unlock indicators)
- Config loading (init.tcl via rusticle-tk scripting)

## Model (duir-core, existing + new fields needed)

Existing fields (see `crates/duir-core/src/model.rs`):
- id, title, completed, important, folded, note, items
- priority, effort, work_status (Idle/InProgress/Paused)
- node_type (Kiron/Prompt/Response), kiron (KironMeta)
- cipher, unlocked

New fields to add in duir-core:
```rust
pub created_at: Option<u64>,      // UTC epoch seconds, set on creation
pub updated_at: Option<u64>,      // UTC epoch seconds, set on any mutation
pub time_spent_secs: u64,         // accumulated in-progress time
#[serde(skip)]
pub progress_started_at: Option<u64>,  // runtime-only: current session start
```

## Key Conventions

- **240 code lines per file** (blank/comment lines don't count). Split by responsibility.
- **Test-first.** Write failing test, implement, verify.
- **clippy -D warnings.** No warnings allowed.
- **Backward-compatible serialization.** New fields use `skip_serializing_if` + `default`.
  Existing .duir/todo.json files must load without error.

## What NOT to Do

- Don't touch duir-core's domain logic unless adding the timestamp fields.
- Don't implement coordinator protocol yet (Phase 2).
- Don't implement rusticle-tk plugin API yet (Phase 4).
- Don't add git integration (duir is not a code editor).
- Don't fight the txv framework style — it's OOP-ish, that's fine, it works.
