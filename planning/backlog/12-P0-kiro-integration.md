# Epic: Kiro Integration — AI-Powered Planning (Revised)

**ID**: 12
**Priority**: P0
**Status**: backlog

## Core Concepts

### Kiron (Kiro Node)

Any node can be **marked as a kiron** (`:kiron` command or hotkey). A kiron is:
- A subtree root where AI collaboration happens
- Has a **session ID** (UUID, stored in node metadata) for session persistence
- Can be **inactive** (marked but no process) or **active** (kiro-cli running)
- Multiple kirons can exist in the tree, multiple can be active simultaneously

**Kiron metadata** (stored in the node's JSON):
```json
{
  "kiron": {
    "session_id": "uuid-here",
    "active": false
  }
}
```

### Kiron Hierarchy

- A kiron's descendants can also be marked as kirons (nested AI sessions)
- Each kiron has its own independent kiro-cli process and session
- Use case: project kiron delegates to sub-task kirons

### Visual Indicators

| Icon | State |
|------|-------|
| 🤖 | Kiron inactive (marked, no process) |
| 🤖💬 | Kiron active, waiting for input (no PTY activity for N seconds) |
| 🤖⚡ | Kiron active, working (PTY output detected) |

## Component 1: PTY Terminal

### Activation

- `:kiron` on a node → marks it as kiron (allocates session ID)
- `:kiro` on a kiron node → activates it (spawns kiro-cli)
- `:kiro` again → deactivates (kills process, keeps session ID)
- Hotkey: `Ctrl+K` toggles activation on current kiron

### Terminal Display

When user navigates within an active kiron's subtree:
- The kiro PTY is shown in the note pane area (replaces note view)
- When user navigates OUTSIDE the kiron subtree → PTY hidden, note view restored
- Multiple active kirons: the one whose subtree contains the cursor is shown

**Layout options** (cycle with `Ctrl+Shift+K`):
- Right panel (default, replaces note)
- Bottom panel
- Top panel
- Left panel

### PTY Management

- One PTY per active kiron
- `portable-pty` + `vte` + `TermBuf` (from kairn)
- Command: `kiro-cli chat --classic` (or configurable)
- Session persistence: `kiro-cli chat --session-id <uuid>` if supported
- Resize on layout change

## Component 2: Prompt/Response Flow

### Sending a Prompt

User selects any node under an active kiron, presses `Ctrl+Enter`:

1. Serialize the node as markdown:
   - Title as heading
   - Note as body
   - All descendants as nested headings/checkboxes
2. Write to PTY stdin as a **paste block**:
   - Send `\x1b[200~` (bracketed paste start)
   - Send the markdown content
   - Send `\x1b[201~` (bracketed paste end)
   - Send `\n` (submit)
3. Mark the node with a "prompt sent" indicator (📤?)
4. Record PTY cursor position as "response start"

### Capturing Response

Detection strategy:
- **Start**: PTY position when prompt was sent
- **End**: No PTY output for N seconds (configurable, default 5s)
- **Problem**: Kiro may pause for tool confirmations (trust prompts)

**Trust/confirmation handling:**
- When output stalls, check last line for known patterns:
  - `[Y/n]`, `Allow?`, `Continue?` → show indicator "⚠️ Kiro needs confirmation"
  - User can press a hotkey to send `y\n` to the PTY
  - Or press another hotkey to send `n\n` (deny)
  - Or type directly in the PTY
- If no confirmation pattern detected after N seconds → treat as response complete

**Response node creation** (happens even if user navigated away):
- Create new sibling AFTER the prompt node
- Title: first non-empty line of response (truncated to 80 chars)
- Note: full response text
- Mark with response indicator (📥? 🤖?)
- Auto-scroll tree to show the new node (optional, configurable)

### Markers in Notes

Prompt nodes get a marker at the top of their note:
```
<!-- duir:prompt kiron=uuid timestamp=2025-05-02T12:00:00 -->
```

Response nodes get:
```
<!-- duir:response kiron=uuid timestamp=2025-05-02T12:00:05 -->
```

## Component 3: MCP Server

### Configuration

Kiro needs to know about the MCP server. Options:

**Option A: Launch kiro-cli with MCP config**
```
kiro-cli chat --classic --mcp-server "duir://localhost:PORT"
```

**Option B: Kiro's MCP config file**
Add to kiro's MCP settings:
```json
{
  "mcpServers": {
    "duir": {
      "command": "duir",
      "args": ["--mcp-server"],
      "transportType": "stdio"
    }
  }
}
```

**Option C: Embedded MCP over PTY**
Duir runs the MCP server as a separate process, kiro-cli connects to it.
The MCP server process is a child of duir, started alongside the PTY.

**Recommendation: Option B** — standard MCP stdio transport. Duir spawns
a separate thread that acts as the MCP server, communicating with the
kiro-cli process. The MCP server has read/write access to the kiron's subtree.

### Threading Model

- **One MCP server thread per active kiron**
- Each thread has a reference to the kiron's subtree in the model
- Thread communicates with main thread via channels:
  - MCP thread → main: "add node at path X with content Y"
  - Main → MCP thread: "here's the current subtree state"
- This allows kiron to use kiro with sub-agents (each sub-agent
  sees the same MCP server, same subtree)

### MCP Tools

| Tool | Args | Description |
|------|------|-------------|
| `read_node` | `path` | Read title, note, completion, importance |
| `list_children` | `path` | List immediate children |
| `list_subtree` | `path, max_depth` | List descendants to depth |
| `search` | `query` | Search nodes by title/note |
| `add_child` | `parent_path, title, note` | Add child node |
| `add_sibling` | `path, title, note` | Add sibling after node |
| `mark_done` | `path` | Mark as completed |
| `mark_important` | `path` | Toggle importance |
| `reorder` | `path, direction` | Move up/down |
| `get_context` | | Get kiron root info + stats |

**Constraints:**
- NO delete
- NO edit existing content
- Read: entire subtree
- Write: append-only (add nodes, mark status)

## FocusState Addition

```rust
KiroTerminal {
    pty: Box<PtyTab>,
    kiron_fi: usize,
    kiron_path: Vec<usize>,
}
```

## Model Addition

```rust
pub struct TodoItem {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kiron: Option<KironMeta>,
}

pub struct KironMeta {
    pub session_id: String,
}
```

`active` is runtime-only (not serialized) — determined by whether a PTY exists.

## Dependencies

- `portable-pty` — PTY management
- `vte` — terminal escape parser
- `uuid` — session IDs (already have)

## Execution Order

1. **12.001** — Kiron marking + PTY embedding (foundation)
2. **12.003** — MCP server (can start in parallel with 12.002)
3. **12.002** — Prompt/Response flow (depends on 12.001)

## Open Questions

1. Does `kiro-cli` support `--session-id` for session persistence?
2. Does `kiro-cli` support MCP server configuration via command line?
3. Should response detection timeout be per-kiron configurable?
4. Should we support sending partial subtrees (selected nodes only) as prompt?
