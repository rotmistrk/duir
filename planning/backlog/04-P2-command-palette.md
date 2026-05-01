# Epic: Enhanced Command Completion

**ID**: 04
**Priority**: P2
**Status**: backlog

## Goal

Improve the command completion system to show a full palette of available
commands when the command line is empty or has few characters, collapsing
to prefix groups when space is limited.

## Acceptance Criteria

- [ ] Empty `:` shows full command palette (popup above status bar)
- [ ] If terminal width is too narrow, collapse to first-letter groups (e.g., `[a]bout [c]ollapse [e] [ex]port ...`)
- [ ] Palette updates as user types, narrowing matches
- [ ] Selected item highlighted, Tab/Shift+Tab cycles
- [ ] Works in both app and editor command modes
- [ ] Palette respects available height (scrollable if needed)

## Stories

- [ ] 04.001 — Popup widget for command palette
- [ ] 04.002 — Adaptive collapse for narrow terminals
- [ ] 04.003 — Integration with both command lines

## Notes

- Current completer shows inline matches in status bar — this extends it to a popup
- Consider showing command descriptions alongside names
- Could also show recent commands at the top
