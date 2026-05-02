# Epic: Type-Safe State Machine for App Focus

**ID**: 10
**Priority**: P0
**Status**: backlog

## Goal

Replace the `focus: Focus` + `editor: Option<NoteEditor>` pattern with
a type-safe state machine where illegal states are unrepresentable.

## Current Problem

```rust
pub struct App {
    pub focus: Focus,                          // Tree or Note
    pub editor: Option<NoteEditor<'static>>,   // exists independently
    pub editor_file_index: usize,              // dangling when editor is None
    pub editor_path: Vec<usize>,               // dangling when editor is None
}
```

The compiler allows: `focus = Tree` + `editor = Some(...)` (bug we hit 4 times).

## Target Design

```rust
pub enum FocusState {
    Tree,
    Note {
        editor: NoteEditor<'static>,
        file_index: usize,
        path: Vec<usize>,
    },
    Command {
        buffer: String,
        history_index: Option<usize>,
    },
    Filter {
        text: String,
        saved: String,
    },
    PasswordPrompt {
        prompt: PasswordPrompt,
    },
    Help {
        scroll: u16,
    },
    About,
}
```

With this:
- `editor` only exists inside `FocusState::Note` — impossible to access from Tree
- `command_buffer` only exists inside `FocusState::Command`
- `filter_text` only exists inside `FocusState::Filter`
- Transitions are explicit match arms — compiler forces handling all cases

## Transitions

```
Tree → Note:     load editor from model, construct Note variant
Note → Tree:     save editor to model, destructure Note variant (editor dropped)
Tree → Command:  construct Command variant
Command → Tree:  execute command, destructure
Tree → Filter:   construct Filter variant
Filter → Tree:   apply/revert filter, destructure
```

Each transition is a method that consumes the old state and produces the new one.
The compiler enforces that you can't skip steps.

## Implementation Plan

1. Define `FocusState` enum
2. Move focus-specific fields from App into enum variants
3. Update all input handlers to match on `FocusState`
4. Update rendering to match on `FocusState`
5. Remove `debug_assert!` invariant checks (no longer needed — compiler enforces)
6. Update all tests

## Notes

- This is the Rust "typestate pattern"
- Eliminates the entire class of focus/editor desync bugs
- Also eliminates: command_active, filter_active, editing_title booleans
- All modal state becomes a single enum — one source of truth
- Estimated effort: 2-3 hours of careful refactoring
