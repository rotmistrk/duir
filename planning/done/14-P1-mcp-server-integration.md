# 14-P1: MCP Server Integration for Kiro — Design

## Problem

duir has an MCP server implementation in `duir-core` (10 tools: read_node, list_children,
list_subtree, search, add_child, add_sibling, mark_done, mark_important, reorder, get_context).
It is fully tested but **never wired up** — kiro-cli cannot access it.

## Hard Constraints

1. **Multi-instance safe**: Multiple duir processes must coexist without conflicts
2. **No config ownership**: duir must NOT overwrite kiro's MCP config files
   - `~/.kiro/settings/mcp.json` and `.kiro/settings/mcp.json` belong to the user
   - Overwriting destroys other MCP servers (mcp-kit, mcp-lint, etc.)
   - Removing entries on stop breaks other running duir instances
3. **Real-time state**: MCP server needs live access to the tree, not stale file snapshots
4. **Bidirectional**: Mutations from kiro must reach duir immediately
5. **Clean lifecycle**: Starting/stopping duir must not break anything else

## kiro-cli MCP Architecture (from docs research)

### Transport
- **Local servers**: stdio subprocess — kiro-cli spawns a command, communicates
  via JSON-RPC on stdin/stdout
- **Remote servers**: HTTP/SSE endpoint with optional OAuth
- No support for pre-connected file descriptors or Unix sockets directly

### Configuration Locations (merged, workspace takes precedence)
- **User level**: `~/.kiro/settings/mcp.json` — global across all workspaces
- **Workspace level**: `.kiro/settings/mcp.json` — project-specific
- **Per-agent**: `mcpServers` field in agent JSON config
- **`includeMcpJson`**: Agent field controlling whether to also load from mcp.json

### Environment Variable Expansion
MCP config supports `${ENV_VAR}` syntax in env values. This means we can pass
dynamic values (like socket paths) via environment variables set in the PTY.

### Custom Agents
Agents can define their OWN `mcpServers` inline — **separate** from mcp.json.
This is the key insight: a `.kiro/agents/duir.json` agent config can include
its own MCP server definition without touching any shared config.

Agent config supports:
- `mcpServers` — agent-specific MCP servers
- `includeMcpJson: true` — ALSO load user's other MCP servers (mcp-kit, etc.)
- `resources` — file:// and skill:// URIs loaded into context
- `prompt` — system prompt, can reference file:// URI
- `tools` — whitelist of available tools (`["*"]` = all)
- `allowedTools` — auto-approved tools (no confirmation prompt)

---

## Chosen Design: Option A — Per-Session Unix Socket + Static Agent

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│ duir process                                            │
│                                                         │
│  ┌──────────┐    Arc<Mutex<TodoFile>>    ┌───────────┐  │
│  │ Main     │◄──────────────────────────►│ MCP Server│  │
│  │ Thread   │    mpsc::channel           │ Thread    │  │
│  │ (TUI)    │◄───── McpMutation ─────────│           │  │
│  └──────────┘                            └─────┬─────┘  │
│                                                │        │
│                                    Unix Socket │        │
│                                   (per-session)│        │
└────────────────────────────────────────────────┼────────┘
                                                 │
                              ┌──────────────────┼──────────────────┐
                              │ duir --mcp-connect (stdio bridge)   │
                              │ reads DUIR_MCP_SOCKET env var       │
                              │ stdin/stdout ↔ Unix socket          │
                              └──────────────────┬──────────────────┘
                                                 │
                                          stdin/stdout
                                                 │
                              ┌──────────────────┼──────────────────┐
                              │ kiro-cli chat --agent duir          │
                              │ spawns "duir --mcp-connect" as MCP  │
                              │ inherits DUIR_MCP_SOCKET from PTY   │
                              └─────────────────────────────────────┘
```

### Key Design Decisions

**Single static agent file**: `.kiro/agents/duir.json`
- One file, shared by all duir instances in this workspace
- No PID in filename, no cleanup needed
- The dynamic part is the `DUIR_MCP_SOCKET` env var, not the config file
- Multiple duir instances use the same agent file; each kiro-cli PTY inherits
  a different socket path from its own PTY environment

**Binary discovery**: `std::env::current_exe()` — same binary for TUI and
`--mcp-connect` mode. No separate binary needed.

**Socket location**: `$XDG_RUNTIME_DIR/duir-mcp-{session_id}.sock`
- `XDG_RUNTIME_DIR` is per-user, tmpfs, cleaned on reboot
- Fallback: `/tmp/duir-{uid}/mcp-{session_id}.sock`
- Session ID is the kiron's UUID (already exists in `KironMeta.session_id`)

**Lifecycle**:
- Socket created on `:kiro start`
- Socket removed on `:kiro stop` or duir process exit
- Orphaned sockets from crashes: harmless, `connect()` returns "connection refused"
- `XDG_RUNTIME_DIR` cleaned on reboot anyway

**Multi-instance safety**:
- Each duir instance creates its own socket with unique session ID
- Session ID comes from `KironMeta.session_id` (per-kiron, stored in JSON)
- If two duir instances open the same file and try to start the same kiron,
  the second gets "address already in use" on socket bind — **natural mutex**.
  duir detects this and reports "Kiron already active in another duir instance."
- Two duirs editing the same kiron is a general data conflict problem (not
  MCP-specific) — they'd overwrite each other on save regardless. Future work:
  file locking or CRDT.
- Each kiro-cli PTY inherits its own `DUIR_MCP_SOCKET` value
- No shared mutable state between instances
- Agent file is read-only, identical content for all instances

### Agent Config

`.kiro/agents/duir.json`:
```json
{
  "name": "duir",
  "description": "AI assistant with access to duir task tree via MCP",
  "mcpServers": {
    "duir": {
      "command": "duir",
      "args": ["--mcp-connect"],
      "env": { "DUIR_MCP_SOCKET": "${DUIR_MCP_SOCKET}" },
      "autoApprove": ["*"]
    }
  },
  "includeMcpJson": true,
  "tools": ["*"],
  "allowedTools": ["@duir"],
  "prompt": "file://.kiro/agents/duir-prompt.md"
}
```
The `command` field uses `duir` which must be in PATH. Alternatively, duir
can write the absolute path from `std::env::current_exe()` when generating
the agent file.

**Future**: If duir is launched with `--agent`, it could read the parent kiro
agent config and generate `duir.json` based on it (inheriting model, prompt
style, etc.). Not P0.

### stdio Bridge (`duir --mcp-connect`)

Minimal mode — pure byte pump, no JSON parsing or MCP logic:
1. Read `DUIR_MCP_SOCKET` env var (fail fast if unset or empty)
2. Connect to Unix socket with read/write timeouts (30s)
3. Spawn two threads: stdin→socket, socket→stdout
4. When either direction closes, `shutdown(Both)` unblocks the other thread
5. Errors reported to stderr (visible in kiro-cli logs)

No JSON parsing, no MCP logic.

---

## Implementation Plan

### Phase 1: Unix socket MCP server (in-process)
- On `:kiro start`, create `Arc<Mutex<TodoFile>>` snapshot of kiron subtree
- Spawn MCP server thread listening on Unix socket
- `McpServer::run()` already handles JSON-RPC — just need socket accept loop
- On `:kiro stop`, drop listener, remove socket file

### Phase 2: stdio bridge
- Add `duir --mcp-connect` CLI mode
- Read `DUIR_MCP_SOCKET` env var
- Connect to Unix socket, bridge stdin/stdout ↔ socket
- Exit when either side closes

### Phase 3: Agent + PTY integration
- On first `:kiro start`, create `.kiro/agents/duir.json` if it doesn't exist
- Add `envs` parameter to `PtyTab::spawn`
- Spawn kiro-cli with `--agent duir` and `DUIR_MCP_SOCKET=<path>` env var

### Phase 4: Mutation channel
- MCP server sends `McpMutation` via `mpsc::channel` to main thread
- Main thread applies mutations in `poll_kirons`
- Main thread updates `Arc<Mutex<TodoFile>>` snapshot after user edits

### Phase 5: Bidirectional sync
- When user edits tree in duir, update the MCP snapshot
- When MCP mutation arrives, apply to real tree and rebuild rows
- Conflict resolution: last-write-wins (MCP mutations are append-only by design)

---

## Rejected Options

### Option B: duir-mcp-proxy Daemon
Separate daemon process. Rejected: lifecycle complexity, extra binary, extra hop,
session routing logic. Overengineered for the problem.

### Option C: No MCP — File-based
Export tree as file, kiro reads/writes via fs_read/fs_write. Rejected as primary
approach: no structured tools, error-prone JSON manipulation, race conditions.
Could work as a read-only Phase 0 stepping stone.

### Option D: HTTP MCP Server
Localhost HTTP server. Rejected: port conflicts, no authentication, firewall
issues, HTTP overhead for local IPC. Unix socket is simpler and more secure
for same-machine communication.
