# Omela — Agent-Driven Development Process

## Overview

Development is organized as a hierarchy of **epics → stories → tasks**, tracked as
markdown files in this directory. Agents (human or AI) pick up work items, execute
them, and move completed items to `done/`.

## Directory Structure

```
planning/
├── ondeck/      ← current sprint: items actively being worked on
├── backlog/     ← prioritized queue: ready to start, not yet active
├── done/        ← completed items (kept for audit trail)
└── README.md    ← this file
```

## File Naming Convention

Hierarchy is encoded in the filename:

| Level | Pattern | Example |
|-------|---------|---------|
| Epic | `NN-PRIORITY-description.md` | `01-P0-core-data-model.md` |
| Story | `NN.SSS-PRIORITY-description.md` | `01.001-P0-json-serialization.md` |
| Task | `NN.SSS.TT-PRIORITY-description.md` | `01.001.01-P0-define-structs.md` |

- `NN` = epic number (01-99)
- `SSS` = story number within epic (001-999)
- `TT` = task number within story (01-99)
- `PRIORITY` = P0 (critical), P1 (high), P2 (medium), P3 (low)

## Priorities

- **P0**: Blocks everything, do immediately
- **P1**: Required for current milestone
- **P2**: Important but not blocking
- **P3**: Nice to have, do when time permits

## Workflow

### For Humans

1. Create epics in `backlog/`
2. Move epic to `ondeck/` when starting work
3. Break epic into stories, stories into tasks
4. Execute tasks, check off exit criteria
5. When all tasks in a story pass exit criteria → story is done
6. When all stories in an epic pass exit criteria → epic is done
7. Move completed items to `done/`

### For AI Agents

1. **Orchestrator agent** reads an epic from `ondeck/`
2. Orchestrator creates stories (if not already present) in `ondeck/`
3. Orchestrator spawns **sub-agents**, one per story
4. Each sub-agent:
   a. Reads its story file
   b. Creates task files in `ondeck/`
   c. Executes each task (writes code, runs tests)
   d. Checks exit criteria after each task
   e. Moves completed tasks to `done/`
   f. When all tasks pass → marks story complete, moves to `done/`
5. Orchestrator checks all stories → marks epic complete, moves to `done/`

### Rules

- A task file is the **atomic unit of work** — one clear action, one clear result
- Stories have **exit criteria** — measurable conditions that must all pass
- Epics have **acceptance criteria** — high-level conditions validated by stories
- Items reference their parent (bottom-up) and children (top-down)
- An agent MUST NOT modify files outside its assigned story scope
- An agent MUST run `cargo clippy` and `cargo test` before marking a task done

## Templates

### Epic Template

```markdown
# Epic: [Title]

**ID**: NN
**Priority**: P0/P1/P2/P3
**Status**: backlog | ondeck | done

## Goal

[One paragraph describing what this epic achieves]

## Acceptance Criteria

- [ ] [Criterion 1]
- [ ] [Criterion 2]

## Stories

- [ ] NN.001 — [Story title]
- [ ] NN.002 — [Story title]

## Notes

[Any context, constraints, or design decisions]
```

### Story Template

```markdown
# Story: [Title]

**ID**: NN.SSS
**Epic**: NN — [Epic title]
**Priority**: P0/P1/P2/P3
**Status**: backlog | ondeck | done

## Goal

[What this story delivers]

## Exit Criteria

- [ ] [Criterion 1 — must be testable/verifiable]
- [ ] [Criterion 2]

## Tasks

- [ ] NN.SSS.01 — [Task title]
- [ ] NN.SSS.02 — [Task title]

## Notes

[Technical approach, constraints, dependencies]
```

### Task Template

```markdown
# Task: [Title]

**ID**: NN.SSS.TT
**Story**: NN.SSS — [Story title]
**Priority**: P0/P1/P2/P3
**Status**: backlog | ondeck | done

## Action

[Specific action to take — one clear thing]

## Done When

- [ ] [Verifiable condition]

## Files Modified

- [list of files created/modified]

## Notes

[Implementation details, edge cases]
```

## Conventions

- One file per item — never combine multiple epics/stories/tasks
- Keep descriptions concise — the code is the source of truth
- Exit criteria must be **objectively verifiable** (test passes, file exists, etc.)
- When moving to `done/`, update Status field in the file to `done`
- Prefix commit messages with the item ID: `[01.001.01] Define core structs`
