# Federated Coordinator Architecture

## Overview

duir instances are independent processes, each owning their local todo tree.
A **coordinator** role emerges from running instances to provide cross-project
visibility and item routing. No daemon. No shared filesystem locking.

## Principles

1. **Local-first.** Each instance is authoritative for its own `.duir/todo.json`.
   No instance depends on the coordinator for its own data.
2. **Projects are isolated.** A project instance knows nothing about other projects.
   It only knows itself and the coordinator.
3. **Coordinator is a hub, not a master.** It owns `~/.duir/todo.json` (the daily
   planner) and can broker data between projects — but never overwrites project-local
   state without the project requesting it.
4. **Coordinator has private state.** Not everything in the coordinator tree is
   visible to projects. Namespaces control what gets shared.

## Topology

```
  ┌──────────┐       ┌──────────┐       ┌──────────┐
  │  duir    │       │  duir    │       │  duir    │
  │ project A│       │ project B│       │  $HOME   │
  └────┬─────┘       └────┬─────┘       └────┬─────┘
       │                   │                   │
       └───────────────────┴───────────────────┘
                           │
                  ┌────────┴────────┐
                  │  coordinator    │
                  │  (thread in one │
                  │   of the above) │
                  └─────────────────┘
```

Star topology. Projects connect only to the coordinator, never to each other.
The coordinator can relay/broadcast between them.

## Data Ownership

| Instance | Owns | Persists to |
|----------|------|-------------|
| Project A | Project A's tree | `~/workplace/projA/.duir/todo.json` |
| Project B | Project B's tree | `~/workplace/projB/.duir/todo.json` |
| $HOME | Daily planner tree | `~/.duir/todo.json` |

The coordinator thread has no separate storage — it operates on whichever
instance's tree is the $HOME tree (or a minimal in-memory routing table if
hosted by a project instance temporarily).

## Coordinator Lifecycle

### Startup

```
1. Instance starts.
2. Try connect(~/.duir/coordinator.sock)
   → success: register as peer, enter project mode.
   → ECONNREFUSED / ENOENT / timeout:
     a. Unlink stale socket if present.
     b. Spawn coordinator thread.
     c. Bind ~/.duir/coordinator.sock.
     d. Enter project mode (with coordinator co-hosted).
```

### $HOME instance claims coordinator

The `$HOME` instance is the "rightful" coordinator. When it starts:

```
1. Try connect(~/.duir/coordinator.sock)
   → success: send handover-request (I am $HOME).
     Current host: flush state, unbind socket, respond "released".
     $HOME: bind socket, accept peer reconnections.
   → failure: become coordinator directly (normal path).
```

### Coordinator-owning process exits

**Clean exit:**
- Broadcast `coordinator-leaving` to all peers.
- Unbind socket, unlink file.
- Peers detect disconnect, race to bind socket. First wins.

**Crash:**
- Peers detect EOF on their connection.
- Short backoff (100ms), then race to bind socket.
- Winner announces itself; others reconnect.

### Last instance exits

- Socket file goes stale on disk.
- Next launch detects it (connect fails), unlinks, starts fresh.

## Communication Protocol

Unix domain socket, line-delimited JSON messages (or msgpack — TBD).

### Registration

```json
{"type": "register", "project_id": "duir", "root": "/Users/me/workplace/duir"}
```

Coordinator responds with any items it wants to push to this project.

### Push item to coordinator

```json
{"type": "push", "item": {"id": "uuid", "title": "...", "priority": 5, ...}}
```

Project sends an item to the coordinator's tree. Coordinator decides
where to place it (inbox, or tagged namespace).

### Pull items from coordinator

```json
{"type": "pull", "filter": {"tag": "duir"}}
```

Project requests items the coordinator has tagged for it.

### Broadcast (coordinator → projects)

```json
{"type": "broadcast", "target": "projB", "items": [...]}
```

Coordinator pushes items to a specific project or all projects.
Receiving project can accept (merge into local tree) or ignore.

### Link / status sync

```json
{"type": "link", "local_id": "abc", "coordinator_id": "xyz"}
{"type": "status_sync", "id": "xyz", "status": "done"}
```

Bidirectional: either side can update status. Conflict resolution:
last-write-wins with timestamp (sufficient for single-user).

### Handover

```json
{"type": "handover_request", "reason": "home_instance"}
{"type": "handover_ack"}
```

## Coordinator Namespaces

The coordinator's tree has namespaces:

- **private** — coordinator's own items, never shared (personal reminders,
  thoughts, things not tied to any project).
- **shared** — items visible to projects that request them (tagged by project
  or broadcast).
- **inbox** — items pushed from projects, awaiting triage by the user.

Projects see only items explicitly sent to them. They never browse the
coordinator's full tree.

## Kiro Integration

MCP server gives full (controlled by agent configuration) access to everything user can do
in duir: item creation/deletion/rename/edit nodes, status change, sort, reorder, reparent, 
tag for coordinator.

In focused mode (project instance + kiro session):

- Todo items can be **prompts**: marked with a status indicating "ready to ask."
- A command sends the selected item (title + notes) to the active kiro session.
- Kiro's response is captured and attached to the item as a note or child subtree.
- Item status transitions: open → in-progress (sent) → done (evaluated).

Cross-project kiro flow:
- User works with kiro in project A. Kiro recommends something relevant to project B.
- User pushes that item to coordinator (`send-to-coordinator`).
- User tags it for project B.
- Next time project B connects, it pulls the item.

## Rusticle-tk / Plugin Integration

The coordinator protocol is the extension point:

- Custom Tcl scripts can listen for coordinator events (item pushed, status changed).
- Scripts can create synthetic items, automate tagging, generate summaries.
- A "dashboard" view (rusticle-tk panel) in the $HOME instance can render
  cross-project status using the coordinator's shared namespace.
- Project instances can register custom message types the coordinator routes.

## Implementation Phases

### Phase 1: Local-only (no coordinator)
- duir on txv with improved todo tree (priority, badges, LOE from kairn).
- Each instance standalone, owns `.duir/todo.json`.
- No cross-instance communication yet.

### Phase 2: Coordinator protocol
- Coordinator thread, socket binding, peer registration.
- Push/pull items between project and coordinator.
- Handover protocol.
- Status sync for linked items.

### Phase 3: Kiro integration rethink
- Prompt/response lifecycle in the todo model.
- Send-to-kiro command, response capture.
- Named sessions (from kairn's design, adapted).

### Phase 4: Rusticle-tk extensions
- Tcl hooks for coordinator events.
- Dashboard views for $HOME instance.
- Plugin API for custom message types.

## Open Questions

- Message encoding: JSON lines vs msgpack vs custom binary?
  - JSON lines
- Should coordinator persist a routing table (which project wants which tags)?
  Or is it purely reactive (projects pull what they want)?
  - do not overengineer but don't create one-way door.  project can create a 
    'pocket' on coordinator site, coordinator sees it as a subtree identified 
    with the project dir, can move in/out anything.  projects see the pocket as 
    'coordinator' (whatever name we choose)-tagged.
- Authentication: needed if we ever go multi-machine? Probably not now, but
  keep the socket protocol extensible enough to add TLS/auth later.
  - +1.  Not now. Don't make it a closed door though.
- Conflict resolution on link sync: timestamp LWW is fine for single-user.
  Multi-user would need CRDTs or explicit merge. Park this.
  - yep
