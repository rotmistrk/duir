# Shared Components: txv-components Proposal

## What Can Be Extracted from kairn (f4)

### 1. LayoutGroup → `txv-components::LayoutGroup` (819 lines)

**Current location:** `f4/src/layout_group/` (5 files)

**Dependencies on f4:**
- `crate::glyphs::glyphs()` — chrome rendering (powerline chars). Trivial to make configurable.
- `crate::commands::*` — command IDs (CM_FOCUS_LEFT, CM_ZOOM_TOGGLE, etc.). These are just `u16` constants — move to txv-core or txv-components.

**What it provides:**
- 4-slot desktop (Left, Center, Right, Bottom) — each slot is a TabGroup
- Auto/Wide/Tall layout modes with hysteresis
- Zoom (maximize one slot)
- Panel resize (grow/shrink width/height)
- Focus cycling between non-empty slots
- Tab management per slot (insert, close, rename, focus by title)
- Chrome rendering (dividers, tab bars with powerline glyphs)

**Extraction effort:** ~4h. Move command constants to txv-core, make glyphs configurable (trait or struct param).

**Relevance to duir:** Direct replacement for our 3-mode layout (wide/tall/compact). duir uses 3 slots (tree, note, kiro) vs kairn's 4 (left, center, right, bottom). We'd use Left=tree, Center=note, Right=kiro, Bottom=unused.

---

### 2. Editor Core → `txv-components::Editor` (2,861 lines)

**Current location:** `f4/src/editor/` (18 files)

**Dependencies on f4:**
- `crate::buffer::PieceTable` — the text buffer. Also extractable.

**What it provides:**
- Full vim keymap: normal, insert, visual, command, search modes
- Motions: h/j/k/l, w/b/e, 0/$, gg/G, f/F/t/T, word objects, paragraph
- Operators: d, c, y, >, <, with count prefix and dot-repeat
- Ex commands: :w, :q, :set, :%s/foo/bar/g, range operations, :!shell
- Search: /, n/N, * (word under cursor)
- Visual mode: character and line selection
- Registers, clipboard integration
- Undo/redo (via PieceTable history)

**Extraction effort:** ~2h. Only depends on PieceTable (extract together). Zero txv dependency — pure logic.

---

### 3. PieceTable Buffer → `txv-components::PieceTable` (484 lines)

**Current location:** `f4/src/buffer/` (6 files)

**Dependencies on f4:** None. Completely self-contained.

**What it provides:**
- Piece table text buffer (efficient insert/delete)
- Line index (O(1) line-to-offset lookup)
- Undo/redo history with grouping
- File I/O (load/save)
- Dirty tracking

**Extraction effort:** ~1h. Copy and publish.

---

### 4. EditorView → `txv-components::EditorView` (1,650 lines)

**Current location:** `f4/src/views/editor/` (10 files)

**Dependencies on f4:**
- `crate::editor::Editor` — the core (extractable)
- `crate::highlight::Highlighter` — syntax highlighting (syntect wrapper)
- `crate::settings::EditorSettings` — config struct
- `crate::lsp::*` — LSP integration (diagnostics, completion)
- `crate::commands::*` — command IDs
- `crate::diff::*` — git diff

**What it provides:**
- View trait implementation wrapping Editor
- Drawing: gutter (line numbers), text with syntax highlighting, cursor, selection
- Diff mode rendering (inline hunks)
- Completion popup
- Autosave on idle
- Close prompt (unsaved changes)

**Extraction complexity:** MEDIUM. The core draw/handle is reusable, but LSP, diff, and completion are kairn-specific features. Need to make these optional/pluggable.

**Extraction effort:** ~8h. Factor out LSP/diff as optional traits or feature flags. Keep core draw + handle + settings.

---

## Proposed Crate Structure

```
txv/
├── txv-core/          — (existing) View, GroupState, Program, Surface, events
├── txv-render/        — (existing) Backend, crossterm, TermBuf
├── txv-widgets/       — (existing) TreeView, PtyTerminal, TabGroup, SplitPane, etc.
└── txv-components/    — NEW: higher-level reusable components
    ├── buffer/        — PieceTable + LineIndex + Undo (484 lines)
    ├── editor/        — Editor core: vim keymap, motions, operators (2,861 lines)
    ├── editor_view/   — EditorView: View impl with draw/handle (1,650 lines, optional features)
    └── layout_group/  — LayoutGroup: slotted desktop with layout modes (819 lines)
```

**Total extractable:** ~5,814 lines from kairn → shared library.

---

## Impact on duir Migration LOE

### Before (duplicating effort)

| Phase | Hours |
|-------|-------|
| Phase 2: Note Panel (vim editor from scratch) | 20h |
| Phase 4: Layout & Composition (build from SplitPane) | 8h |
| **Total affected** | **28h** |

### After (reusing txv-components)

| Phase | Hours | Savings |
|-------|-------|---------|
| Phase 2: Note Panel (integrate EditorView) | 6h | -14h |
| Phase 4: Layout (integrate LayoutGroup) | 3h | -5h |
| Extraction work (one-time, benefits both projects) | 15h | — |
| **Total affected** | **24h** | **-4h net** |

But the real value isn't the 4h savings on duir — it's:
1. **No divergence** — bug fixes in editor/layout benefit both projects
2. **Better testing** — shared components get tested from two angles
3. **Faster future projects** — any txv app gets a vim editor and slotted layout for free

---

## Revised Migration LOE (with txv-components)

| Phase | Description | Effort | Notes |
|-------|-------------|--------|-------|
| Pre | Extract txv-components from kairn | 15h | One-time investment |
| 0 | Foundation (feature flag, skeleton) | 4h | Same |
| 1 | Tree Panel | 10h | Slightly less — LayoutGroup handles chrome |
| 2 | Note Panel | 6h | Integrate EditorView, adapt for markdown |
| 3 | Kiro Panel (PTY) | 6h | Same (PtyTerminal already in txv-widgets) |
| 4 | Layout & Composition | 3h | LayoutGroup does the heavy lifting |
| 5 | Commands & Status Bar | 6h | Slightly less — InputLine + StatusBar exist |
| 6 | Overlays & Dialogs | 4h | Same |
| 7 | Integration & Cleanup | 10h | Less old code to remove |
| **Total** | | **64h** | **vs 76h without sharing** |

**Net savings: 12h on duir + shared infrastructure for future.**

---

## Risks Specific to Extraction

### 1. Abstraction Leakage (MEDIUM)
- EditorView has kairn-specific features (LSP, diff, git). Must cleanly separate.
- Mitigation: Feature flags (`lsp`, `diff`) or trait-based extension points.

### 2. API Stability (LOW)
- Both projects evolve. Shared crate API must be stable enough.
- Mitigation: You control both repos. Use git dependency initially, publish to crates.io when stable.

### 3. LayoutGroup Slot Count (LOW)
- kairn uses 4 slots, duir needs 3. LayoutGroup is already flexible (empty slots get 0 width).
- Mitigation: duir simply doesn't insert into Bottom slot. Works today.

### 4. Editor Feature Mismatch (MEDIUM)
- duir's note editor is simpler (no LSP, no diff, no completion popup).
- kairn's EditorView has these baked in.
- Mitigation: Make them optional. `EditorView::new()` without LSP/diff = simple editor.

---

## Recommendation

**Extract txv-components first (15h), then migrate duir using it (49h).**

Total: 64h, with the bonus of a reusable component library.

Extraction order:
1. PieceTable (1h) — zero dependencies, immediate value
2. Editor core (2h) — depends only on PieceTable
3. LayoutGroup (4h) — needs command ID refactoring
4. EditorView (8h) — needs feature-flag work for LSP/diff

Start extraction as a separate branch in the txv repo. Once stable, both kairn and duir depend on it.
