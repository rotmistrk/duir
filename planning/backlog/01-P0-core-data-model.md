# Epic: Core Data Model and Storage

**ID**: 01
**Priority**: P0
**Status**: backlog

## Goal

Implement the core library: data model, JSON persistence, markdown import/export,
filter/search, and legacy XML import. This is the foundation everything else builds on.

## Acceptance Criteria

- [ ] TodoFile/TodoItem model with full serde round-trip (JSON primary, YAML export)
- [ ] Load/save JSON files from local filesystem
- [ ] Export subtree as markdown
- [ ] Import markdown as subtree
- [ ] Filter items by title and note content
- [ ] Compute completion percentages recursively
- [ ] Import legacy `.todo` XML files, converting HTML notes to markdown
- [ ] All public API has doc comments and unit tests

## Stories

- [ ] 01.001 — JSON serialization and file I/O
- [ ] 01.002 — Tree operations (add, delete, move, clone, sort)
- [ ] 01.003 — Completion percentage computation
- [ ] 01.004 — Markdown export
- [ ] 01.005 — Markdown import
- [ ] 01.006 — Filter and search
- [ ] 01.007 — Legacy XML import

## Notes

- Model structs already scaffolded in `crates/omela-core/src/model.rs`
- Storage trait should abstract over local files and S3 (S3 is a later epic)
- Markdown export maps tree depth to heading levels + checkbox lists
- Legacy import is one-time migration, doesn't need to be fast
