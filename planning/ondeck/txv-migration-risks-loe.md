# TXV Migration — Risks & LOE Assessment

## Level of Effort

### Summary

| Phase | Description | Effort | Cumulative |
|-------|-------------|--------|------------|
| 0 | Foundation (feature flag, skeleton) | 4h | 4h |
| 1 | Tree Panel | 12h | 16h |
| 2 | Note Panel (vim editor) | 20h | 36h |
| 3 | Kiro Panel (PTY) | 8h | 44h |
| 4 | Layout & Composition | 8h | 52h |
| 5 | Commands & Status Bar | 8h | 60h |
| 6 | Overlays & Dialogs | 4h | 64h |
| 7 | Integration & Cleanup | 12h | 76h |

**Total estimate: 76 hours (~10 working days)**

### Effort Breakdown by Activity

| Activity | Hours | % |
|----------|-------|---|
| View implementation | 32h | 42% |
| Test writing | 20h | 26% |
| Integration/wiring | 12h | 16% |
| Debugging/fixing | 8h | 11% |
| Cleanup/removal | 4h | 5% |

### Confidence Level

- Phases 0, 3, 4, 6: **High confidence** (well-understood, txv has direct equivalents)
- Phases 1, 5: **Medium confidence** (straightforward but many details)
- Phase 2: **Low confidence** (vim editor is the largest unknown — tui-textarea removal)
- Phase 7: **Medium confidence** (integration issues always surface late)

---

## Risks

### HIGH RISK

#### 1. Vim Editor Rewrite (Phase 2)
- **What:** tui-textarea is ratatui-specific. Must rewrite text rendering and cursor management.
- **Impact:** 1,583 lines of vim logic. If the new rendering doesn't match tui-textarea's behavior exactly, subtle editing bugs appear.
- **Probability:** High (70%)
- **Mitigation:** Keep vim logic (motions, operators) as pure functions. Only rewrite the rendering layer. Extensive test coverage for every motion/operator.
- **Contingency:** If too complex, create a thin adapter that renders tui-textarea output to txv Surface (ugly but functional).

#### 2. Feature Regression During Migration
- **What:** Subtle behavior differences between old and new code paths.
- **Impact:** Users hit bugs in features that "used to work."
- **Probability:** Medium (50%)
- **Mitigation:** Feature flag allows A/B comparison. Screen-based tests catch rendering differences. Keep old path until Phase 7.
- **Contingency:** Revert to old path for affected features, fix in isolation.

#### 3. Performance Regression
- **What:** Retained-mode overhead (dirty tracking, surface allocation) slower than ratatui's simple immediate-mode for small terminals.
- **Impact:** Noticeable lag on low-end machines or over SSH.
- **Probability:** Low (20%) — txv's diff-flush should be faster for partial updates.
- **Mitigation:** Profile early (Phase 1). Compare frame times old vs new.
- **Contingency:** Optimize hot paths in txv (Surface allocation, diff algorithm).

### MEDIUM RISK

#### 4. txv Framework Gaps
- **What:** txv may lack features duir needs (see gap list in migration guide).
- **Known gaps:**
  - No multi-line text editor widget
  - No markdown/rich-text rendering
  - No async event injection from background threads
  - Limited mouse support
- **Impact:** Must build missing pieces, adding to LOE.
- **Probability:** High (80%) — gaps are known, but effort to fill them is uncertain.
- **Mitigation:** You control the txv repo. Add features as needed during migration.
- **Contingency:** Accept some features (mouse, rich markdown) as post-migration improvements.

#### 5. MCP Server Integration
- **What:** MCP server runs on a background thread with Unix socket. Events must flow into the txv event loop.
- **Impact:** If event injection is awkward, mutations from kiro may lag or deadlock.
- **Probability:** Medium (40%)
- **Mitigation:** Add a channel-based event source to MockBackend/Backend that polls alongside terminal events.
- **Contingency:** Use a polling approach on Tick (check channel every 50ms) — simple, slightly laggy.

#### 6. Test Migration Effort Underestimated
- **What:** Porting 166 existing tests to screen-based assertions may take longer than expected.
- **Impact:** Phase 7 balloons from 12h to 20h+.
- **Probability:** Medium (40%)
- **Mitigation:** Port tests incrementally during each phase (not all at the end). Each phase includes its own test targets.
- **Contingency:** Accept some tests as "ported but simplified" — cover the same scenario with fewer assertions.

### LOW RISK

#### 7. Dependency Conflicts
- **What:** txv uses crossterm internally; version mismatch with duir's crossterm.
- **Impact:** Compilation errors, API incompatibilities.
- **Probability:** Low (15%) — you control both repos.
- **Mitigation:** Pin same crossterm version in both. During transition, only one code path uses crossterm directly.

#### 8. File Watcher Integration
- **What:** notify crate fires events on a background thread. Must inject into txv event loop.
- **Impact:** File change detection stops working.
- **Probability:** Low (20%) — same pattern as MCP (channel → poll on Tick).
- **Mitigation:** Solve once for MCP, reuse for file watcher.

#### 9. Clipboard/OSC 52
- **What:** OSC 52 writes directly to terminal. txv's Backend abstraction may not expose raw write.
- **Impact:** Clipboard copy breaks.
- **Probability:** Low (10%) — Backend likely has raw write or can be extended.
- **Mitigation:** Add `raw_write(bytes)` to Backend trait if needed.

---

## Decision Points

### Before Starting

1. **tui-textarea strategy** — Rewrite from scratch or adapter layer?
   - Recommendation: Rewrite. The adapter would be fragile and temporary.
   - Impact on LOE: +8h if rewrite is harder than expected.

2. **Parallel development or sequential?**
   - Recommendation: Sequential phases. Feature flag allows shipping intermediate states.
   - Alternative: Parallel (tree + editor simultaneously) — faster but riskier.

3. **txv enhancements — upstream first or inline?**
   - Recommendation: Add to txv repo as needed. Don't fork or inline.
   - Needed: async event channel, multi-line text widget (or build in duir).

### During Migration

4. **Phase 2 checkpoint** — After implementing basic insert/normal mode, evaluate:
   - Is the vim editor working well enough?
   - Are there tui-textarea behaviors we can't replicate?
   - Decision: Continue rewrite or fall back to adapter.

5. **Phase 4 checkpoint** — After layout works:
   - Is performance acceptable?
   - Does the three-panel layout feel right?
   - Decision: Proceed to Phase 5 or optimize first.

---

## Success Criteria

1. All existing user-visible features work identically
2. 170+ screen-based integration tests pass
3. No ratatui/tui-textarea/vte dependencies remain
4. Frame time ≤ 16ms (60fps) on standard terminal
5. Code is more maintainable: each view owns its state, no god-object
6. New features (e.g., mouse support, split editors) become easier to add

---

## Recommendation

**Proceed with migration.** The benefits (testability, maintainability, composition)
justify the ~76h investment. The highest risk (vim editor rewrite) is mitigable
because the logic already exists — only the rendering layer changes.

**Start with Phase 0+1** (tree panel) to validate the approach with minimal risk.
If Phase 1 goes smoothly, the rest is mechanical. If it reveals fundamental issues
with txv, you'll know after ~16h investment, not 76h.
