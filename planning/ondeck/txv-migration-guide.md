# TXV Migration Guide for duir-tui

## What is TXV?

TXV is a Turbo Vision-inspired TUI framework extracted from kairn. It replaces
ratatui's immediate-mode rendering with a retained-mode View tree.

**Repo:** https://github.com/rotmistrk/txv

**Crates:**
- `txv-core` — Pure logic. Zero I/O. View trait, GroupState, EventQueue, Surface, geometry.
- `txv-render` — Terminal backend (crossterm). Implements Backend trait, diff-flush.
- `txv-widgets` — Concrete views: TabGroup, PtyTerminal, TextArea, StatusBar, TreeView, etc.

## Why Migrate?

| ratatui (current) | txv (target) |
|---|---|
| Immediate mode: redraw everything every frame | Retained mode: views own state, only dirty regions redraw |
| App holds ALL state in one struct | Each view owns its state (high cohesion) |
| Manual event routing (match key → action) | Three-phase dispatch: preprocess → focused → postprocess |
| No framework for composition | GroupState + delegation macros = composable views |
| No tab/panel management | TabGroup, SplitPane built-in |
| Manual terminal emulation | PtyTerminal widget with scrollback |
| No modal support | exec_view() for nested modal event loops |

## Core Concepts

### View Trait

Every UI component implements `View`:

```rust
pub trait View: Send {
    fn draw(&self, surface: &mut Surface);
    fn handle(&mut self, event: &Event, queue: &mut EventQueue) -> HandleResult;
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, rect: Rect);
    fn select(&mut self) {}
    fn unselect(&mut self) {}
    fn needs_redraw(&self) -> bool;
    fn mark_redrawn(&mut self);
    fn title(&self) -> &str { "" }
    fn options(&self) -> ViewOptions;
}
```

### ViewState — Encapsulated State

Views embed `ViewState` and delegate via macros:

```rust
pub struct MyView {
    state: ViewState,
    // ... view-specific fields
}

impl View for MyView {
    delegate_view_state!(state);
    fn draw(&self, surface: &mut Surface) { /* ... */ }
    fn handle(&mut self, event: &Event, queue: &mut EventQueue) -> HandleResult { /* ... */ }
}
```

ViewState fields are **private** — access only through methods:
- `bounds()`, `is_dirty()`, `is_focused()`
- `mark_dirty()`, `mark_redrawn()`, `set_bounds()`, `set_focused()`

### GroupState — Parent of Children

Views that own children embed `GroupState`:

```rust
pub struct MyPanel {
    group: GroupState,
}

impl View for MyPanel {
    delegate_group_state!(group);
    fn draw(&self, surface: &mut Surface) { /* ... */ }
    fn handle(&mut self, event: &Event, queue: &mut EventQueue) -> HandleResult {
        self.group.dispatch(event, queue)
    }
}
```

GroupState provides:
- `child(i)`, `child_mut(i)`, `focused_child_mut()`
- `focused_index()`, `set_focused_index()`, `switch_focus()`
- `set_child_bounds(i, rect)`, `children_iter()`, `children_iter_mut()`
- Three-phase dispatch: preprocess → focused → postprocess

### Event-Driven Communication

Views NEVER call each other directly. They emit commands via EventQueue:

```rust
fn handle(&mut self, event: &Event, queue: &mut EventQueue) -> HandleResult {
    if let Event::Key(Key::Char('q')) = event {
        queue.put_command(CM_QUIT, None);
        return HandleResult::Consumed;
    }
    HandleResult::Ignored
}
```

The parent (or Program) receives unhandled commands in a handler closure.

### Program — The Event Loop

```rust
let mut program = Program::new(status_bar, desktop);
program.run(&mut backend, |ctx| {
    match ctx.command {
        CM_OPEN_FILE => { /* ... */ }
        _ => {}
    }
});
```

Program owns: status bar (preprocess) + desktop (focused). It handles:
- Event polling from backend
- Tick dispatch (50ms)
- Draw cycle (only dirty views)
- Command routing to handler

## Testing

TXV has a `MockBackend` for headless testing:

```rust
use txv_core::run::mock::MockBackend;

#[test]
fn test_my_view() {
    let mut backend = MockBackend::new(80, 24);
    backend.inject_keys(&[Key::Char('h'), Key::Char('i')]);
    // ... run program or dispatch events manually
    assert!(backend.content_contains("hi"));
}
```

**Testing principles:**
- Every test is deterministic — no timing, no shared state
- Use `content_contains()` not `contains()` (avoids status bar clock false positives)
- Tests run independently and in parallel
- One test file per feature/scenario

## Layout Reference: kairn's SlottedDesktop

duir currently has: tree panel (left) + note/editor (right) + optional kiro terminal.
kairn's layout is more general — study it for the migration:

**File:** `kairn/src/layout_group/` (5 files, ~820 lines total)

```
┌────────┬──────────────┬──────────┐
│  Left  │   Center     │  Right   │
│ (tree) │  (editors)   │ (terms)  │
└────────┴──────────────┴──────────┘
```

Each slot is a `TabGroup` (multiple tabs, one active). The LayoutGroup:
- Manages slot widths (resizable via commands)
- Handles zoom (maximize one slot)
- Routes focus between slots
- Draws chrome (borders, tab bars)

**Key pattern:** LayoutGroup embeds `GroupState` with 4 children (one TabGroup per slot).
Layout is computed in `layout.rs`, chrome in `chrome.rs`, dispatch in `dispatch.rs`.

## Migration Strategy

### Phase 1: Core Views
Replace ratatui widgets with txv equivalents:
- `tree_view.rs` → use `txv_widgets::TreeView` or custom View
- `note_editor/` → custom View wrapping the editor logic
- `pty_tab.rs` → use `txv_widgets::PtyTerminal`
- `render.rs` → becomes the desktop's `draw()` method

### Phase 2: Event Loop
Replace the tokio event loop with `Program::run()`:
- Key events → `Event::Key`
- Tick → `Event::Tick` (PTY poll, file watcher)
- Commands → `EventQueue::put_command()`

### Phase 3: Layout
Replace manual ratatui layout with a GroupState-based desktop:
- Left panel (tree) + Center (note/editor) + Right (kiro terminal)
- Each panel is a TabGroup
- Resize via commands, zoom via commands

### Phase 4: Remove ratatui
Once all rendering goes through `Surface`, remove ratatui dependency entirely.

## Key Differences from ratatui

| Concept | ratatui | txv |
|---------|---------|-----|
| Rendering | `f.render_widget(w, area)` | `view.draw(&mut surface)` |
| Layout | `Layout::default().split(area)` | `set_bounds(rect)` from parent |
| Events | `crossterm::event::read()` | `backend.poll_event()` → dispatch |
| State | App struct holds everything | Each View owns its state |
| Composition | Manual nesting | GroupState + delegate macros |
| Dirty tracking | None (full redraw) | `needs_redraw()` + diff flush |

## Dependencies

```toml
[dependencies]
txv-core = { git = "https://github.com/rotmistrk/txv.git" }
txv-render = { git = "https://github.com/rotmistrk/txv.git" }
txv-widgets = { git = "https://github.com/rotmistrk/txv.git" }
```
