# TXV Migration Plan — duir-tui

## Executive Summary

Migrate duir-tui from ratatui (immediate-mode) to txv (retained-mode) framework.
This replaces the rendering layer, event loop, and widget architecture while
preserving all business logic in duir-core and the app/ layer.

**Scope:** ~9,120 lines of production code across 50 files.
**Target:** Retained-mode views with encapsulated state, three-phase event dispatch,
dirty-region redraw, and full screen-based test coverage.

---

## Architecture Mapping

### Current (ratatui) → Target (txv)

| Current | Target | Notes |
|---------|--------|-------|
| `App` struct (god object, 30+ fields) | `DuirDesktop` (GroupState) + per-view state | Split state into views |
| `FocusState` enum | GroupState focus + view selection | Focus managed by framework |
| `render.rs` (immediate redraw) | `View::draw()` per widget (dirty only) | Retained mode |
| `event_loop.rs` (manual dispatch) | `Program::run()` + three-phase dispatch | Framework handles routing |
| `input/` (match key → action) | `View::handle()` per widget | Each view handles its own keys |
| `tree_view.rs` (StatefulWidget) | `DuirTreeView` impl View (wraps txv TreeView) | Adapt TreeData trait |
| `note_editor/` (wraps tui-textarea) | `DuirNoteEditor` impl View | Custom view, keep vim logic |
| `render_note.rs` + `render_kiro.rs` | `TabGroup` or `SplitPane` containing views | Framework composition |
| `pty_tab.rs` + `termbuf/` | `txv_widgets::PtyTerminal` | Direct replacement |
| `completer.rs` + command palette | `InputLine` + custom completion view | Adapt existing logic |
| `password.rs` | `InputDialog` (modal) | Use txv dialog |
| `help.rs` | `ScrollView` with static content | Simple view |
| `file_watcher.rs` | Backend event injection from thread | Custom integration |

---

## Phase Plan

### Phase 0: Foundation (no behavior change)

**Goal:** Add txv dependency, create the DuirDesktop skeleton, run both old and new
code paths side-by-side with a feature flag.

**Tasks:**
1. Add `txv-core`, `txv-render`, `txv-widgets` to Cargo.toml
2. Create `views/` module directory
3. Create `DuirDesktop` struct (GroupState with placeholder children)
4. Create `DuirProgram` wrapper that calls `Program::run()`
5. Feature flag `--txv` on CLI to switch between old and new code paths
6. Verify old path still works, new path shows empty screen

**Testing:**
- Existing 166 tests still pass (old path)
- New path: 1 test — program starts and exits on CM_QUIT

**Files created:** `views/mod.rs`, `views/desktop.rs`, `txv_main.rs`
**Files modified:** `Cargo.toml`, `main.rs`

---

### Phase 1: Tree Panel

**Goal:** Implement `DuirTreeView` as a txv View that renders the task tree.

**Tasks:**
1. Implement `TreeData` trait for duir-core's tree model
2. Create `DuirTreeView` wrapping `txv_widgets::TreeView`
3. Handle: cursor movement, expand/collapse, toggle complete, importance
4. Handle: inline title editing (switch to InputLine overlay)
5. Emit commands for operations that affect other views (CM_NODE_SELECTED, CM_FILTER, etc.)
6. Wire into DuirDesktop as left panel

**Testing (screen-based, following kairn pattern):**
- `test_tree_navigation` — j/k moves cursor, verify highlighted row
- `test_tree_expand_collapse` — →/← expands/collapses, verify children visible/hidden
- `test_tree_toggle_complete` — Space toggles ☑/☐
- `test_tree_importance` — ! toggles marker
- `test_tree_filter` — / opens filter, typing filters, Esc reverts
- `test_tree_reorder` — Shift+J/K swaps, verify order
- `test_tree_promote_demote` — H/L changes depth
- `test_tree_new_sibling_child` — n/b creates nodes
- `test_tree_delete` — d with confirmation
- `test_tree_clone` — c duplicates subtree
- `test_tree_sort` — S sorts children
- `test_tree_inline_edit` — e enters edit, Enter confirms, Esc cancels

**Files created:** `views/tree_view.rs`, `views/tree_data.rs`
**Lines estimate:** ~400

---

### Phase 2: Note Panel

**Goal:** Implement `DuirNoteView` (read-only markdown) and `DuirNoteEditor` (vim editor).

**Tasks:**
1. Create `DuirNoteView` — renders markdown with syntax highlighting
2. Create `DuirNoteEditor` — full vim emulation (port existing note_editor/ logic)
3. Handle mode switching: normal → insert → visual → command → search
4. Handle: all vim motions, editing operations, undo/redo
5. Handle: ex-commands (:set nu, :%s/foo/bar/g, range operations, shell pipe)
6. Markdown syntax highlighting via syntect (port existing logic)
7. Wire into DuirDesktop as center panel

**Testing:**
- `test_note_view_renders_markdown` — headings, bold, code blocks visible
- `test_note_editor_insert` — i enters insert, typing adds text, Esc returns
- `test_note_editor_motions` — hjkl, w/b, 0/$, gg/G move cursor
- `test_note_editor_delete` — dd, x, dw delete correctly
- `test_note_editor_visual` — v selects, y yanks, d cuts
- `test_note_editor_search` — / searches, n/N navigates matches
- `test_note_editor_undo` — u undoes, Ctrl+R redoes
- `test_note_editor_command` — :set nu shows line numbers
- `test_note_editor_replace` — :%s/old/new/g replaces
- `test_note_editor_indent` — >/< indents/unindents
- `test_note_editor_shell_pipe` — :!date inserts output

**Files created:** `views/note_view.rs`, `views/note_editor.rs`, `views/note_editor_vim.rs`
**Lines estimate:** ~800 (significant — vim emulation is complex)

**Decision point:** Keep tui-textarea or rewrite? tui-textarea is ratatui-specific.
**Recommendation:** Rewrite using txv Surface directly. The vim logic (motions, operators)
is already in our code — only the rendering and cursor management used tui-textarea.

---

### Phase 3: Kiro Panel (PTY Terminal)

**Goal:** Replace custom pty_tab.rs + termbuf/ with `txv_widgets::PtyTerminal`.

**Tasks:**
1. Use `PtyTerminal::spawn_command()` for kiro-cli
2. Wire environment variables (DUIR_MCP_SOCKET)
3. Handle: scrollback (PgUp/PgDn), resize, process exit detection
4. Handle: focus routing (all keys go to PTY when focused)
5. Integrate MCP server (Unix socket listener, mutation dispatch)
6. Response capture: detect idle → extract last response from terminal buffer

**Testing:**
- `test_kiro_spawn` — :kiro start spawns process, panel shows content
- `test_kiro_input` — typing in kiro panel sends to PTY
- `test_kiro_scroll` — PgUp/PgDn scrolls buffer
- `test_kiro_stop` — :kiro stop kills process
- `test_kiro_capture` — Ctrl+R captures response as node
- `test_kiro_mcp_read` — MCP read_node returns correct data
- `test_kiro_mcp_mutate` — MCP add_child creates node in tree

**Files created:** `views/kiro_panel.rs`, `views/kiro_mcp.rs`
**Lines estimate:** ~350

**Note:** txv's PtyTerminal already has TermBuf + scrollback + key encoding.
This eliminates our custom termbuf/ (564 lines) and pty_tab.rs (205 lines).

---

### Phase 4: Layout & Composition

**Goal:** Implement the desktop layout with SplitPane/TabGroup composition.

**Tasks:**
1. `DuirDesktop` uses SplitPane (tree | right-panel)
2. Right panel: SplitPane or TabGroup depending on layout mode
3. Implement layout modes: wide (3-col), tall (top-bottom), compact (tabbed)
4. Implement `:layout` command and Ctrl+L cycling
5. Implement zoom (F5) — maximize focused view
6. Implement panel resize (]/[)
7. Wire focus cycling (Ctrl+T, F2/F3/F4)

**Testing:**
- `test_layout_wide` — at 160+ cols, 3 panels visible
- `test_layout_tall` — at 50+ rows, kiro below
- `test_layout_compact` — small terminal, tabbed
- `test_layout_cycle` — Ctrl+L cycles modes
- `test_layout_zoom` — F5 maximizes, F5 again restores
- `test_layout_resize` — ]/[ changes panel width
- `test_focus_cycle` — Ctrl+T cycles tree→note→kiro

**Files created:** `views/desktop.rs` (rewrite), `views/layout.rs`
**Lines estimate:** ~300

---

### Phase 5: Commands & Status Bar

**Goal:** Implement command mode, status bar, and all : commands.

**Tasks:**
1. `DuirStatusBar` — extends txv StatusBar with duir-specific items
2. Command input: : opens InputLine in status area
3. Tab completion (existing completer logic)
4. Command history (arrow up/down)
5. All commands: file ops, tree ops, encryption, kiro, layout, settings
6. Status messages with color levels

**Testing:**
- `test_command_save` — :w saves file
- `test_command_open` — :open loads file
- `test_command_export` — :export creates file
- `test_command_encrypt_decrypt` — full encrypt/unlock/lock cycle
- `test_command_completion` — Tab completes commands
- `test_command_history` — arrow up recalls previous
- `test_status_message` — operations show colored status

**Files created:** `views/status_bar.rs`, `views/command_input.rs`
**Lines estimate:** ~250

---

### Phase 6: Overlays & Dialogs

**Goal:** Implement modal overlays (help, password, resolve, about).

**Tasks:**
1. Help overlay — ScrollView with search (existing HELP.md content)
2. Password prompt — InputDialog (modal, masked input)
3. Conflict resolution — custom modal view
4. About dialog — static content

**Testing:**
- `test_help_opens_closes` — F1 opens, Esc closes
- `test_help_search` — / filters help content
- `test_password_prompt` — encrypt triggers prompt, input masked
- `test_resolve_navigation` — j/k navigates conflicts, m/t/b resolves

**Files created:** `views/help_view.rs`, `views/resolve_view.rs`
**Lines estimate:** ~200

---

### Phase 7: Integration & Cleanup

**Goal:** Remove ratatui, remove old code paths, final integration testing.

**Tasks:**
1. Remove `--txv` feature flag (txv becomes the only path)
2. Remove: render.rs, render_note.rs, render_kiro.rs, render_resolve.rs
3. Remove: event_loop.rs, event_focus.rs, event_helpers.rs
4. Remove: input/ directory (logic moved into views)
5. Remove: tree_view.rs (old StatefulWidget)
6. Remove: termbuf/ (replaced by txv PtyTerminal)
7. Remove: pty_tab.rs (replaced by txv PtyTerminal)
8. Remove ratatui, tui-textarea, vte from Cargo.toml
9. Update all integration tests to use TestHarness pattern
10. Final pass: ensure all 166+ existing test scenarios are covered

**Testing:**
- All existing test scenarios ported to screen-based assertions
- Full regression suite passes
- Manual testing of all features

**Files removed:** ~15 files (~2,500 lines)
**Dependencies removed:** ratatui, tui-textarea, vte

---

## Testing Strategy

### Approach: Screen-Based Integration Tests (kairn pattern)

Following kairn's proven approach:

1. **TestHarness** wraps the full `Program` (real code, not mocks)
2. **MockBackend** captures rendered screen for assertions
3. **Inject keystrokes** → run cycles → assert screen content
4. **No internal state assertions** — test what the user sees
5. **Temp directories** for file fixtures (isolated, parallel-safe)

### Test Infrastructure

```rust
// tests/helpers.rs
pub struct TestHarness {
    program: Program,
    backend: MockBackend,
}

impl TestHarness {
    pub fn new(files: &[(&str, &str)]) -> Self { ... }
    pub fn with_size(files: &[(&str, &str)], w: u16, h: u16) -> Self { ... }
    pub fn inject_key(&mut self, code: KeyCode, mods: KeyMod) { ... }
    pub fn inject_str(&mut self, s: &str) { ... }
    pub fn run_cycles(&mut self, n: usize) { ... }
    pub fn contains(&self, text: &str) -> bool { ... }
    pub fn content_contains(&self, text: &str) -> bool { ... }
    pub fn row(&self, y: usize) -> String { ... }
    pub fn cursor_row(&self) -> Option<usize> { ... }
}

// tests/fixtures.rs
pub fn temp_project(files: &[(&str, &str)]) -> TempDir { ... }
pub fn sample_todo() -> &'static str { ... }  // minimal .todo.json
```

### Test Coverage Targets

| Phase | New Tests | Cumulative |
|-------|-----------|------------|
| Phase 0 | 2 | 2 |
| Phase 1 | 12 | 14 |
| Phase 2 | 11 | 25 |
| Phase 3 | 7 | 32 |
| Phase 4 | 7 | 39 |
| Phase 5 | 7 | 46 |
| Phase 6 | 4 | 50 |
| Phase 7 | Port remaining ~120 scenarios | 170+ |

**Final target:** 170+ integration tests (more than current 166) covering all
user-visible behavior through screen assertions.

### What Gets Tested Differently

| Current approach | New approach |
|-----------------|--------------|
| Unit tests on App methods | Screen-based: inject keys, verify output |
| Assert internal state (cursor, rows) | Assert rendered text (what user sees) |
| Mock nothing (test real code) | Mock only the terminal (MockBackend) |
| No rendering tests | Every test verifies rendering |

---

## Dependency Changes

### Added
```toml
txv-core = { git = "https://github.com/rotmistrk/txv.git" }
txv-render = { git = "https://github.com/rotmistrk/txv.git" }
txv-widgets = { git = "https://github.com/rotmistrk/txv.git" }
```

### Removed (Phase 7)
```toml
ratatui = "0.29"
tui-textarea = "0.7"
vte = "0.13"
```

### Kept
```toml
crossterm = "0.28"     # txv-render uses crossterm internally
syntect = "5.3"        # syntax highlighting (used in note view)
portable-pty = "0.8"   # MAY be removable if txv PtyTerminal suffices
tokio = "1"            # MCP server async
```

---

## File Structure (Post-Migration)

```
crates/duir-tui/src/
├── main.rs                    — CLI, terminal setup, Program::run()
├── views/
│   ├── mod.rs                 — View module exports
│   ├── desktop.rs             — DuirDesktop (GroupState, layout logic)
│   ├── layout.rs              — Layout mode resolution
│   ├── tree_view.rs           — DuirTreeView (wraps txv TreeView)
│   ├── tree_data.rs           — TreeData trait impl for duir-core
│   ├── note_view.rs           — Read-only markdown rendering
│   ├── note_editor.rs         — Vim editor view
│   ├── note_editor_vim.rs     — Vim motions/operators (pure logic)
│   ├── kiro_panel.rs          — Kiro PTY terminal view
│   ├── kiro_mcp.rs            — MCP server integration
│   ├── status_bar.rs          — Status bar + command input
│   ├── command_input.rs       — : command mode with completion
│   ├── help_view.rs           — Help overlay
│   └── resolve_view.rs        — Conflict resolution overlay
├── app/                       — KEPT: business logic, state, commands
│   ├── (existing files)       — Minimal changes (remove rendering concerns)
├── completer.rs               — KEPT: completion logic
├── syntax.rs                  — KEPT: syntect initialization
├── clipboard.rs               — KEPT: OSC 52
├── file_watcher.rs            — KEPT: notify integration
├── mcp_log.rs                 — KEPT: debug logging
└── tests/
    ├── helpers.rs             — TestHarness, temp_project, cursor_at
    ├── tree_tests.rs          — Tree navigation & operations
    ├── editor_tests.rs        — Note editor vim tests
    ├── kiro_tests.rs          — Kiro panel tests
    ├── layout_tests.rs        — Layout mode tests
    ├── command_tests.rs       — : command tests
    ├── crypto_tests.rs        — Encryption tests
    ├── multi_file_tests.rs    — Multi-file operations
    └── ...
```

---

## Migration Principles

1. **Feature flag first** — old and new paths coexist until Phase 7
2. **One phase at a time** — each phase is independently shippable
3. **Tests before removal** — new tests must pass before old code is deleted
4. **Business logic untouched** — app/ and duir-core stay the same
5. **No behavior changes** — migration is purely architectural
6. **Screen-based testing** — verify what the user sees, not internal state
