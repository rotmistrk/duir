# Epic: Kiro Integration — AI-Powered Planning (v3)

**ID**: 12
**Priority**: P0
**Status**: backlog

## Core Concepts

### Kiron (Kiro Node)

Any node can be **marked as a kiron** (`:kiron` command). A kiron is:
- A subtree root where AI collaboration happens
- Has a **session ID** (UUID, stored in node metadata) for session persistence
- Can be **inactive** (marked but no process) or **active** (kiro-cli running)
- Multiple kirons can exist in the tree, multiple can be active simultaneously

**Kiron metadata** (stored in the node's JSON):
```json
{
  "kiron": {
    "session_id": "uuid-here"
  }
}
```

`active` is runtime-only — determined by whether a PTY process exists.

### Node Types (persisted in JSON)

```json
{
  "node_type": "kiron" | "prompt" | "response" | null
}
```

Prompt and response nodes also get unicode prefix in title for visibility
when viewing raw JSON or exported markdown:
- Prompt: `📤 <title>`
- Response: `📥 <title>`
- Kiron: shown via tree icon, not title prefix

### Kiron Hierarchy

- A kiron's descendants can also be marked as kirons (nested AI sessions)
- Each kiron has its own independent kiro-cli process and session
- Use case: project kiron delegates to sub-task kirons
- Delegation: user creates a child kiron, activates it, sends context
  from parent kiron's subtree as the initial prompt
- Parent and child kirons are independent processes — no automatic
  orchestration (user is the orchestrator)

### Visual Indicators (tree icons)

| Icon | State |
|------|-------|
| 🤖 | Kiron inactive (marked, no process) |
| 🤖💬 | Kiron active, waiting for input (idle > N seconds) |
| 🤖⚡ | Kiron active, working (PTY output detected recently) |
| ⚠️🤖 | Kiron needs user attention (confirmation/trust prompt) |

**Indicator placement:**
- Icon shown on the kiron node itself in the tree
- When kiron is not visible (collapsed ancestor): indicator bubbles up
  to the most specific VISIBLE ancestor node
- As user drills down, indicator migrates to the actual kiron
- When user selects the kiron PTY to respond, indicator clears

## Component 1: PTY Terminal

### Activation

- `:kiron` on a node → marks it as kiron (allocates session ID)
- `:kiro start` on a kiron node → activates (spawns kiro-cli)
- `:kiro stop` on a kiron node → deactivates (kills process, keeps session)
- No toggle hotkey (too dangerous — easy to accidentally kill session)

### Terminal Display

**Tab model:** The note panel gets tabs when inside an active kiron's subtree:
- **[Note]** tab — normal note editor (user can take notes on any node)
- **[Kiro]** tab — PTY terminal for the active kiron
- Tab switching: configurable hotkey (e.g., `Ctrl+T` or backtick)
- When user navigates OUTSIDE the kiron subtree → Kiro tab disappears,
  only Note tab remains

Multiple active kirons: the Kiro tab shows the kiron whose subtree
contains the current cursor position. If nested kirons, shows the
most specific (deepest) active kiron.

**Layout options** (cycle with `Ctrl+Shift+K`):
- Right panel (default)
- Bottom panel
- Top panel
- Left panel

### PTY Management

- One PTY per active kiron
- `portable-pty` + `vte` + `TermBuf` (from kairn)
- Command template from config (see Config section)
- Session persistence via kiro-cli session flags (TBD — must check kiro-cli capabilities)
- Resize on layout change

## Component 2: Prompt/Response Flow

### Sending a Prompt

User selects any node under an active kiron, presses `Ctrl+Enter`:

1. Serialize the node + all descendants as markdown
2. Write to PTY stdin as a **paste block**:
   - `\x1b[200~` (bracketed paste start)
   - markdown content
   - `\x1b[201~` (bracketed paste end)
   - `\n` (submit)
3. Mark the node as type "prompt" (persisted)
4. Add `📤` prefix to title if not already present
5. Record PTY output position as "response start"

### Capturing Response

**Start**: PTY position when prompt was sent
**End**: No PTY output for 5 seconds (hardcoded, keep simple)

**Trust/confirmation handling:**
- When output stalls, scan last few lines for confirmation patterns
- Patterns: `[Y/n]`, `[y/N]`, `Allow`, `Continue`, `approve`, `deny`
  (actual kiro patterns TBD — must check)
- If pattern detected:
  - Set kiron state to "needs attention" (⚠️🤖 icon)
  - Icon bubbles up to visible ancestor if kiron is not visible
  - User navigates to kiron, switches to Kiro tab, responds
  - Or: dedicated hotkeys when kiron tab is focused:
    `y` to approve, `n` to deny, or type freely
- If no pattern after 5 seconds → response complete

**Response node creation** (happens even if user navigated away):
- Create new sibling AFTER the prompt node
- Title: `📥 ` + first non-empty line (truncated to 80 chars)
- Note: full response text with marker:
  ```
  <!-- duir:response kiron=uuid timestamp=ISO8601 -->
  ```
- Set node type to "response"
- Tree auto-refreshes to show new node

### Markers in Notes

Prompt nodes:
```
<!-- duir:prompt kiron=uuid timestamp=ISO8601 -->
```

Response nodes:
```
<!-- duir:response kiron=uuid timestamp=ISO8601 -->
```

## Component 3: MCP Server

### Configuration

**Managed by duir.** When a kiron is activated, duir:
1. Writes an MCP config file to a temp location
2. Passes it to kiro-cli via environment or flag
3. The MCP server is a child process of duir (stdio transport)

**Kiro command template** in duir config (`config.toml`):
```toml
[kiro]
command = "kiro-cli"
args = ["chat", "--classic"]
# session_flag = "--session-id"  # if supported
# mcp_flag = "--mcp-config"     # if supported
```

**Kiro processes outside duir:** They don't see the MCP server.
The MCP server only exists while duir is running and a kiron is active.
This is by design — duir is the orchestrator.

### Threading Model

- **One MCP server thread per active kiron**
- Thread has channel-based access to the kiron's subtree
- MCP thread → main thread: mutation requests (add node, mark done, etc.)
- Main thread → MCP thread: subtree state queries
- Sub-agents spawned by kiro all connect to the same MCP server instance

### MCP Tools

| Tool | Args | Description |
|------|------|-------------|
| `read_node` | `path` | Read title, note, completion, importance, type |
| `list_children` | `path` | List immediate children |
| `list_subtree` | `path, max_depth` | List descendants to depth |
| `search` | `query` | Search nodes by title/note |
| `add_child` | `parent_path, title, note` | Add child node |
| `add_sibling` | `path, title, note` | Add sibling after node |
| `mark_done` | `path` | Mark as completed |
| `mark_important` | `path` | Toggle importance |
| `reorder` | `path, direction` | Move up/down within kiron subtree only |
| `get_context` | | Get kiron root info + stats |

**Constraints:**
- NO delete
- NO edit existing content
- Read: entire kiron subtree
- Write: append-only (add nodes, mark status)
- Reorder: only within the kiron subtree (can't move nodes out)

## Model Additions

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    Kiron,
    Prompt,
    Response,
}

pub struct TodoItem {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kiron: Option<KironMeta>,
}

pub struct KironMeta {
    pub session_id: String,
}
```

## FocusState Addition

```rust
KiroTerminal {
    pty: Box<PtyTab>,
    kiron_fi: usize,
    kiron_path: Vec<usize>,
}
```

## Dependencies

- `portable-pty` — PTY management
- `vte` — terminal escape parser
- `uuid` — session IDs (already have)

## Execution Order

1. **12.001** — Kiron marking + model changes + PTY embedding
2. **12.003** — MCP server (parallel with 12.002)
3. **12.002** — Prompt/Response flow (depends on 12.001)

## Pre-requisites to Check

- [ ] Does `kiro-cli` support session persistence? How?
- [ ] Does `kiro-cli` support MCP server config via CLI/env?
- [ ] What are the actual trust/confirmation prompt patterns in kiro?
- [ ] Kiro command template should be in user config (XDG)
