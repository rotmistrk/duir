# Epic: Editor Polish and Bug Fixes

**ID**: 08
**Priority**: P1
**Status**: backlog

## Issues

### 08.001 — Cursor visibility in normal mode
- No visible cursor in markdown-highlighted normal mode (we render Paragraph, not textarea)
- Need to show cursor position (underline or reverse on the character)
- Cursor position must be preserved across mode switches and session

### 08.002 — URL opening unreliable
- URL detection regex is too simple, misses edge cases
- Cursor must be ON the URL for Shift+Enter to work — hard to tell without visible cursor
- Depends on 08.001 (cursor visibility)

### 08.003 — Tab/Backspace indent behavior
- Tab should insert spaces to next tab stop (not just 4 spaces from current position)
- Backspace on leading spaces should delete back to previous tab stop
- Tab width configurable (default 4)

### 08.004 — Encrypt/decrypt visual feedback
- scrypt KDF is intentionally slow (~1-2 seconds) — this is security, not a bug
- But need a "Working..." indicator during encrypt/decrypt
- Could show a brief status message or spinner
- Consider: is the work factor configurable? (age crate default is fine for security)

## Notes
- 08.001 is the most impactful — affects all normal-mode editing
- 08.003 is standard editor behavior, should be straightforward
- 08.004 is UX polish — the slowness is correct behavior
