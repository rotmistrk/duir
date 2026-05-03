# 15-P1: Post-User-Testing Roadmap

**Priority**: P1
**Status**: backlog
**Prerequisite**: 1-2 weeks of real-world user testing

## Epics (to be detailed after user testing stabilizes the UI)

### Architecture
- [ ] **duir-app crate extraction** (P1) — move AppState, Action enum, command parser, completer, vim editor state, TermBuf, TreeRow flattening from duir-tui to shared crate. Both TUI and GUI reuse it.

### GUI
- [ ] **egui desktop app** (P2) — native window via eframe, reuses duir-core + duir-app. Tree view, note editor, kiro terminal. ~12-16 sessions.

### Integrations
- [ ] **OneDrive storage** (P2) — Microsoft Graph API, OAuth2 browser flow. Enables corporate file sync. OAuth2 plumbing unlocks all MS integrations.
- [ ] **MS To Do export/import** (P3) — flat task lists via Graph API. Depends on OAuth2 from OneDrive.
- [ ] **CalDAV/VTODO export** (P3) — standards-based, works with Apple Reminders, Thunderbird, Google Tasks.
- [ ] **Quip export/import** (P3) — REST API, OAuth2. Salesforce shops only.

### Kairn convergence
- [ ] **duir-core as kairn library** (P2) — planning panel in kairn powered by duir-core, rendered via txv-widgets. duir-tui stays standalone.
- [ ] **Evaluate txv for duir-tui** (P3) — after kairn proves the txv stack, consider porting duir-tui rendering from ratatui to txv.
- [ ] **Rusticle scripting** (P3) — scriptable commands, custom keybindings, hooks. When duir needs user-extensible behavior beyond static config.

## Notes

- Priorities will shift based on user testing feedback
- Detailed stories written when an epic moves to ondeck
- Architecture extraction (duir-app) is prerequisite for GUI work
