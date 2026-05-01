# Epic: Terminal UI

**ID**: 02
**Priority**: P0
**Status**: backlog

## Goal

Build the ratatui-based TUI with tree view, note pane, keyboard navigation,
and all tree operations wired up. Multi-file support as first-class feature.

## Acceptance Criteria

- [ ] Split layout: tree pane (left) + note pane (right/bottom)
- [ ] Tree renders with checkboxes, importance, completion %, strikethrough
- [ ] Full keyboard navigation matching design spec
- [ ] Inline title editing
- [ ] Note viewing with basic markdown rendering
- [ ] Note editing (toggle between view/edit)
- [ ] Multi-file: top-level nodes are files, open/create/save per file
- [ ] Cross-file item move (cut/paste)
- [ ] Filter toolbar
- [ ] Status bar with help hints
- [ ] Autosave support
- [ ] Command-line args: open files, export, import

## Stories

- [ ] 02.001 — App shell and layout
- [ ] 02.002 — Tree rendering
- [ ] 02.003 — Keyboard navigation and tree operations
- [ ] 02.004 — Note pane (view + edit)
- [ ] 02.005 — Multi-file management
- [ ] 02.006 — Filter UI
- [ ] 02.007 — CLI arguments and batch operations

## Notes

- Depends on Epic 01 (core library)
- Key bindings from design doc in chat history
- Layout: tree left, note right, status bar bottom
