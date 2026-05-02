# Epic: Stable Identity — FileId + NodeId Refactor

**ID**: 13
**Priority**: P0
**Status**: backlog

## Problem

The codebase uses positional indices (`usize` file_index, `Vec<usize>` path)
as identity keys in HashMaps, FocusState, PendingResponse, and active_kirons.
These go stale on any insertion, deletion, or reorder — causing:

- Editor writes to wrong node (FocusState captures stale path)
- Passwords map to wrong files after file close
- Active kiron sessions reference wrong files/nodes
- Pending responses insert at wrong position
- Cipher cache lookup fails on stale paths → data loss on save

14 issues identified in audit, 6 eliminated by this refactor.

## Solution

Replace positional indices with stable, monotonic identifiers:

```rust
/// Stable file identity — never reused, survives reorder/close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(u64);

/// Stable node identity — persisted in JSON, survives tree mutations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);
```

## Stories

### 13.001 — NodeId on TodoItem

Add `id: NodeId` field to `TodoItem`. Every node gets a UUID on creation.

- `TodoItem::new()` generates a UUID
- Deserialization: if `id` is missing (legacy files), generate on load
- `NodeId` implements Display, FromStr
- Serialized in JSON/YAML as `"id": "uuid"`
- Markdown export/import: preserve in HTML comment `<!-- id:uuid -->`

**Tests:**
- Serialization round-trip preserves NodeId
- Legacy files without id get one on load
- Two calls to `TodoItem::new()` produce different ids

### 13.002 — FileId on LoadedFile

Add `id: FileId` field to `LoadedFile`. Monotonic counter, never reused.

- `App` holds a `next_file_id: u64` counter
- `add_file()` assigns `FileId(next_file_id++)` 
- `files` stays as `Vec<LoadedFile>` (order matters for display)
- Add `fn file_by_id(&self, id: FileId) -> Option<&LoadedFile>`
- Add `fn file_by_id_mut(&mut self, id: FileId) -> Option<&mut LoadedFile>`
- Add `fn file_index_for_id(&self, id: FileId) -> Option<usize>` (for tree_ops that need index)

**Tests:**
- FileId is unique across add/close/add cycles
- file_by_id returns correct file after reorder

### 13.003 — Rekey passwords HashMap

Change `passwords: HashMap<(usize, Vec<usize>), String>`
to `passwords: HashMap<(FileId, NodeId), String>`.

- `cmd_encrypt`/`cmd_decrypt` use FileId + NodeId
- `lock_for_save` looks up by FileId + NodeId
- `collapse_current` uses FileId + NodeId
- Password prompt callback carries FileId + NodeId

**Tests:**
- Password survives tree mutation (add sibling above)
- Password survives file close/reopen of different file

### 13.004 — Rekey active_kirons and pending_responses

Change `active_kirons: HashMap<(usize, Vec<usize>), ActiveKiron>`
to `active_kirons: HashMap<(FileId, NodeId), ActiveKiron>`.

Change `PendingResponse` to use FileId + NodeId.

- `active_kiron_for_cursor` walks up from current node checking NodeIds
- `process_mcp_mutations` resolves FileId + NodeId to current path
- `kiro_start`/`kiro_stop` use FileId + NodeId

**Tests:**
- Active kiron survives sibling insertion above
- Pending response finds correct node after tree mutation

### 13.005 — FocusState::Note uses FileId + NodeId

Change `FocusState::Note { file_index, path }` to `{ file_id: FileId, node_id: NodeId }`.

- `save_editor` resolves FileId + NodeId to current path, writes content
- `focus_note` captures FileId + NodeId from current item
- If resolution fails (node deleted), editor is discarded with warning

**Tests:**
- Editor saves to correct node after sibling insertion
- Editor saves to correct node after file reorder
- Editor discarded gracefully if node was deleted

### 13.006 — Private modified flag + mark_modified as sole entry point

- Make `LoadedFile.modified` private (pub(crate) or behind accessor)
- All 21 call sites that set `modified = true` directly → call `mark_modified`
- `mark_modified(FileId, NodeId)` invalidates cipher on ancestors
- Add `fn is_modified(&self, file_id: FileId) -> bool` accessor

**Tests:**
- Cipher invalidated when child of encrypted node is modified via any path
- Cannot set modified without going through mark_modified

### 13.007 — Remaining quick wins

After the refactor, fix the remaining non-index issues:

1. **#4**: Call `save_editor()` before any encryption operation
2. **#6**: Check encrypt result in `collapse_current`, don't remove password on failure
3. **#7**: Update MCP snapshot after user mutations on kiron subtree
4. **#9**: `save_all` continues on error, collects and reports all failures
5. **#14**: Bounds-check in `process_mcp_mutations`, clean up on file close

## Execution Order

```
13.001 (NodeId)  ──→  13.003 (passwords)  ──→  13.006 (private modified)
                  ──→  13.004 (kirons)     ──→  13.007 (quick wins)
13.002 (FileId)  ──→  13.005 (FocusState)
```

13.001 and 13.002 are independent foundations.
13.003-13.006 depend on both.
13.007 is cleanup after the refactor.

## Migration

- Existing JSON files without `id` fields: NodeId generated on load
- FileId is runtime-only (not persisted)
- No breaking changes to file format (id field is additive)

## LOE

| Story | Estimate | Risk |
|-------|----------|------|
| 13.001 | 1-2h | Low — additive field |
| 13.002 | 1h | Low — runtime only |
| 13.003 | 2h | Medium — many call sites |
| 13.004 | 1-2h | Medium — path resolution |
| 13.005 | 2h | Medium — FocusState is central |
| 13.006 | 1-2h | Medium — 21 call sites |
| 13.007 | 1h | Low — straightforward fixes |

**Total: 9-12 hours**
