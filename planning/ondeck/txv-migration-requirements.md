# duir TXV Migration — Requirements

## Goal

Replace duir-tui's ratatui-based rendering and ad-hoc layout with txv (rusticle-tk),
porting the improved patterns from kairn while extending them for duir's specific needs.

## Layout

Use kairn's `TiledWorkspace` (LayoutGroup / 4-slot desktop):

- **Left slot:** Todo tree (primary view — this is duir's main artifact)
- **Center slot:** Note editor (markdown, vi-modal)
- **Right slot:** Shell / Kiro tabs
- **Bottom slot:** Messages, MCP log, status

**Default on start:** Tree zoomed (F5 zoom state). User unzooms to reveal
other slots as needed. This reinforces "tree is primary."

## Todo Tree View — Columns

The tree row is a fixed-width columnar layout. Left to right:

```
[indent+connector] [badges] [title] [padding] [timestamps]
```

### 1. Indent + Connectors (variable width)

Each depth level = 2 chars indent.

**Tree connectors (optional, toggle with `T`):**
```
├─ Sibling item
│  ├─ Child
│  └─ Last child
└─ Last sibling
```

When connectors are off, indent is plain spaces (current kairn behavior).

### 2. Badge Column (fixed width, 5 chars)

Wider than kairn's 3-char badge to accommodate duir's richer model.

Layout: `[status][priority][effort][notes][type]`

| Position | Content | Values |
|----------|---------|--------|
| 0: status | Work state icon | ○ ▶ ⏸ ✓ ◐ 🔒 |
| 1: priority | Braille fill (0-9) | ` ` ⠁⠃⠇⡇⣇⣧⣷⣿ |
| 2: effort | Fibonacci indicator | ` ` 1-9 or letter code |
| 3: notes | Has notes | ♪ or ` ` |
| 4: type | Node type | ` ` 🤖(kiron) 💬(prompt) 📋(response) |

Effective values bubble up for collapsed nodes (same as kairn v-016 design).

### 3. Title (variable width, fills remaining space)

Plain text. Truncated with `…` if too long.
Important items: bold or highlighted.
Done items: dim/strikethrough.

### 4. Timestamp Columns (3 independent columns, each 5 chars, right-aligned)

Each toggled independently. Key TBD (maybe `D` cycles: none → created → all → none).

| Column | Shows | Trigger |
|--------|-------|---------|
| Created | When item was created | Always set on creation |
| Started | When in-progress first activated | Set on first ▶ transition |
| Last action | Most recent modification | Updated on any mutation |

**Age-adaptive format (5 chars fixed):**
- Today: `hh:mm` (e.g. `14:32`)
- This year: `MM/DD` (e.g. `06/01`)
- Older: `YY/MM` (e.g. `25/11`)

Full datetime shown in status bar when item is selected.

**Time spent tracking:**
- Purely status-driven. Clock runs while status = in-progress (▶).
- User explicitly pauses/stops when switching contexts.
- Optional `auto_pause_minutes` config for single-project users (disabled by default).

### Column visibility toggles

| Key | Column | Default |
|-----|--------|---------|
| `T` | Tree connectors | Off |
| `D` | Timestamps | Off |
| `L` | LOE (effort in badges) | On |
| `B` | Badges | On |

## Model Extensions

New fields on `TodoItem` (backward-compatible, serde skip_serializing_if):

```rust
/// When the item was created (UTC epoch seconds).
#[serde(default, skip_serializing_if = "Option::is_none")]
pub created_at: Option<u64>,

/// When the item was last modified (UTC epoch seconds).
#[serde(default, skip_serializing_if = "Option::is_none")]
pub updated_at: Option<u64>,

/// Accumulated time spent in in-progress state (seconds).
#[serde(default, skip_serializing_if = "is_zero")]
pub time_spent_secs: u64,

/// Runtime-only: when current in-progress session started.
#[serde(skip)]
pub progress_started_at: Option<u64>,
```

`NodeId::new()` should also set `created_at = Some(now)`.
Any mutation sets `updated_at = Some(now)`.

## Shell & Kiro — Right Slot

Same TiledWorkspace, shell and kiro tabs in the right slot.
Uses kairn's PTY infrastructure (VTE terminal emulation, non-blocking writes,
scrollback buffer, OSC 52 clipboard).

Kiro integration changes vs. current duir:
- MCP-based communication (not terminal scraping)
- Named sessions (from federated-coordinator design, Phase 3)
- Todo items as prompts (send-to-kiro command)
- Response capture back into tree

## What Comes from kairn Directly

| Component | kairn source | Adaptation needed |
|-----------|-------------|-------------------|
| LayoutGroup (TiledWorkspace) | `src/slots.rs` + desktop | Minimal — slot count/names |
| Todo tree view | `src/views/todo_tree/` | Extend with connectors, timestamps, wider badges |
| Todo model | `src/views/todo_tree/model.rs` | Merge with duir-core's richer model |
| Note editor (vi) | `src/editor/` | Direct reuse |
| PTY terminal | `src/views/terminal.rs` | Direct reuse |
| MCP server | `src/mcp/` | Adapt tool set for duir's operations |
| InputLine (inline edit) | rusticle-tk | Direct reuse |
| Status bar | `src/status/` + `src/status_items/` | Customize items |
| Session persistence | `src/session/` | Adapt for duir's state shape |
| Palette/theming | `src/app_palette*.rs` + `src/config_colors.rs` | Direct reuse |

## What Is New for duir (not in kairn)

| Feature | Notes |
|---------|-------|
| Tree connectors (├─└─│) | Draw logic in tree view, toggle state |
| Timestamp columns | Model fields + age-adaptive formatting |
| Time-spent tracking | Timer start/stop on status transitions |
| 5-char badge column | Extended from kairn's 3-char |
| Node type badges (🤖💬📋) | For kiron/prompt/response types |
| Coordinator protocol | Phase 2 (separate from TXV migration) |
| Encryption (🔒 nodes) | Already in duir-core, needs UI in new tree view |
| Rusticle-tk scripting | Tcl extension points for custom views |

## Migration Order

### Step 1: Scaffold
- Add rusticle-tk dependency
- Create new `duir-tui` binary using TiledWorkspace
- Tree zoomed on start, empty center/right/bottom slots
- Wire up basic key dispatch (F2-F5, Ctrl-Q)

### Step 2: Todo Tree View
- Port kairn's todo_tree view
- Integrate duir-core model (merge kairn's simpler model with duir-core's richer one)
- Implement 5-char badge column
- Implement tree connectors (optional)
- Implement timestamp columns (optional)
- Wire up all existing tree operations (add, delete, move, promote, demote, fold, etc.)

### Step 3: Note Editor
- Port kairn's editor view into center slot
- Connect to duir-core's note field (load/save on item selection)
- Markdown highlighting

### Step 4: Shell & Kiro
- Port kairn's terminal view into right slot
- PTY management, kiro launch
- MCP server with duir-specific tool set

### Step 5: Polish
- Status bar with context-sensitive sections
- File watcher (auto-reload on external change)
- Session persistence
- Encryption UI (password prompt, lock/unlock)
- Config loading (init.tcl)

## Non-Goals for Initial Migration

- Coordinator protocol (Phase 2 of federated design)
- Rusticle-tk plugin API (Phase 4)
- Multi-file support (duir handles one tree per instance)
- Git integration (duir is not a code editor)

## Design Decisions

### Connectors

Configurable style (setting in config/init.tcl):
- `unicode`: `├─ └─ │` (box-drawing)
- `powerline`: custom glyphs (if available)
- `none`: plain indent (default)

### Badge Column Width

Start at 5. Extend to 6+ if needed. No hard cap — badges have fixed positions,
column width = number of active badge slots.

### Timestamp Formats

Three **separate columns**, each independently togglable:

| Column | Content | Format by age |
|--------|---------|---------------|
| Created | When item was created | see below |
| Started | When in-progress was first activated | see below |
| Last action | Most recent modification | see below |

**Format tiers (age-adaptive, 5-char fixed width):**
- Less than 24h: `hh:mm`
- This year: `MM/DD`
- Older: `YY.YY` (year as `26.26`... or rather `'26`) — TBD exact, keep it 5 chars

Actually, let's reconsider: we have room if columns are independent.
Each timestamp column = 5 chars. Format:
- Today: `hh:mm` (e.g. `14:32`)
- This year: `MM/DD` (e.g. `06/01`)
- Older: `YY/MM` (e.g. `25/11`)

This gives you instant "how old is this" at a glance without needing full dates.
Full datetime available in status bar or tooltip when cursor is on the item.

### Time Tracking & Idle Detection

**The problem:** User juggles multiple projects simultaneously. Multiple duir
instances may have items in-progress. "Idle" in one instance doesn't mean idle
overall — user is just working in another window.

**Design:**
- Time tracking is **purely status-driven**. Clock runs while status = in-progress (▶).
- No automatic idle detection. User explicitly pauses (⏸) or stops (✓/○) when switching.
- The coordinator (Phase 2) could optionally implement **focus-based tracking**:
  when a project instance loses the coordinator's "active project" signal, it could
  suggest pausing. But this is opt-in and future.

**Rationale:** Idle detection is unreliable (what timeout? what if user is thinking?
reading docs? on a call about this task?). Explicit state transitions are honest.
The cost is one extra keystroke (`\` to pause) when switching contexts — acceptable
for accurate data.

**Alternative for single-project users:** A config option `auto_pause_minutes = 15`
that pauses in-progress items after N minutes of no keypress *in this instance*.
Disabled by default. Detects idle via: no key/mouse event received by the TUI
event loop for N minutes → emit synthetic "idle" event → pause all in-progress items
with a note "(auto-paused: idle)".
