# Epic: Kiro Integration — AI-Powered Planning

**ID**: 12
**Priority**: P0
**Status**: backlog

## Overview

Integrate duir with Kiro CLI to create an AI-powered planning tool.
A "kiro node" in the tree opens an embedded terminal running kiro-cli.
Nodes can be sent as prompts, responses captured as sibling nodes.
An MCP server exposes the subtree for Kiro to read/write.

## Architecture

```
┌─ Tree ──────────────┬─ Note / Kiro Terminal ──────────┐
│ ▼ Project Plan      │ $ kiro-cli --classic             │
│   ☐ Design API      │ > Design a REST API for...       │
│   ☐ [kiro response] │ Here's my recommendation:        │
│   ☐ Implementation  │ 1. Use OpenAPI spec...           │
│ ▶ Other tasks       │                                  │
├─────────────────────┴──────────────────────────────────┤
│ :kiro  [Ctrl+K] toggle  [Ctrl+Enter] send prompt       │
└────────────────────────────────────────────────────────┘
```

## Components

### 12.001 — Kiro Terminal (PTY Embedding)

Embed a terminal running `kiro-cli --classic` in the note pane area.

**Approach** (from kairn):
- `portable-pty` crate for PTY management
- `vte` crate for terminal escape sequence parsing
- `TermBuf` for virtual terminal grid (ratatui-compatible rendering)
- Reader thread for async PTY output
- Resize handling

**UX:**
- `:kiro` command or `Ctrl+K` toggles kiro terminal on/off
- Terminal replaces note pane when active
- Layout positions: right (default), bottom, top, left — cycle with `Ctrl+Shift+K`
- Terminal is per-kiro-node (each kiro node gets its own session)
- Hidden/shown with hotkey, session persists

**FocusState addition:**
```rust
KiroTerminal {
    pty: Box<PtyTab>,
    kiro_node_fi: usize,
    kiro_node_path: Vec<usize>,
}
```

### 12.002 — Prompt/Response Flow

Send node content to kiro as a prompt, capture response as a new sibling.

**Send prompt (Ctrl+Enter on a node below kiro node):**
1. Serialize the node: title + note as markdown
2. Write to PTY stdin (paste into kiro-cli)
3. Mark node as "sent" (visual indicator?)

**Capture response:**
- Watch PTY output for kiro's response
- Detect response boundaries (prompt marker: `$ ` or configurable)
- When response complete: create new sibling node after the prompt node
  - Title: first line of response (truncated)
  - Note: full response text
  - Mark as "kiro response" (visual indicator: 🤖?)

**Alternative capture approach:**
- Named pipe (like kairn's KAIRN_CAPTURE)
- Set `DUIR_CAPTURE` env var in the PTY
- Kiro output redirected to pipe → duir reads and creates node

### 12.003 — MCP Server

Expose the kiro subtree as MCP tools for Kiro to use as "memory".

**Protocol:** MCP over stdio (kiro-cli supports this)

**Tools exposed:**

| Tool | Description |
|------|-------------|
| `read_node(path)` | Read node title, note, completion, importance |
| `list_children(path)` | List child nodes |
| `list_descendants(path, depth)` | List subtree to depth |
| `add_child(parent_path, title, note)` | Add child node |
| `add_sibling(path, title, note)` | Add sibling after node |
| `mark_done(path)` | Mark node as completed |
| `mark_important(path)` | Toggle importance |
| `reorder(path, direction)` | Move node up/down |
| `search(query)` | Search nodes by title/note |

**Constraints (by design):**
- NO delete (Kiro can't delete nodes)
- NO edit existing content (Kiro can't modify what human wrote)
- Read access to entire subtree
- Write access: add only (append-only log of AI contributions)

**Implementation:**
- MCP server runs as a thread inside duir
- Communicates with the kiro-cli process via stdio
- Uses `mcp-server` crate or raw JSON-RPC over stdio

## Dependencies

- `portable-pty` — PTY management (from kairn)
- `vte` — terminal escape sequence parser (from kairn)
- `serde_json` — MCP JSON-RPC (already have serde)

## Reference Implementation

kairn project at `/home/rotmistr/Workplace/kairn/main/`:
- `src/tab/shell.rs` — PtyTab: spawn, poll, write, resize
- `src/termbuf.rs` — TermBuf: VTE-based terminal grid
- `src/capture.rs` — Named pipe capture mechanism

## Execution Plan

1. **Story 12.001** — PTY embedding (can be built independently)
   - Port PtyTab and TermBuf from kairn
   - Add KiroTerminal to FocusState
   - Render terminal grid in note pane area
   - Toggle with :kiro / Ctrl+K

2. **Story 12.002** — Prompt/Response (depends on 12.001)
   - Ctrl+Enter sends node content to PTY
   - Response detection and node creation
   - Visual indicators for sent/response nodes

3. **Story 12.003** — MCP Server (can be built independently)
   - Define MCP tool schemas
   - Implement tool handlers against duir's tree model
   - Wire into kiro-cli's MCP client

## Notes

- Stories 12.001 and 12.003 can be built in parallel
- Story 12.002 depends on 12.001
- The MCP server is the most architecturally novel part
- Consider: should MCP server be a separate binary or embedded?
  Embedded is simpler but couples the lifecycle
