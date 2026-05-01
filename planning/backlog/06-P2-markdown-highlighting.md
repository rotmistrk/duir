# Epic: Markdown Syntax Highlighting

**ID**: 06
**Priority**: P2
**Status**: backlog

## Goal

Syntax-highlight markdown in the note editor, including fenced code blocks
with language-specific highlighting.

## Design

### Markdown Elements

| Element | Style |
|---------|-------|
| `# Heading` | Bold + cyan |
| `**bold**` | Bold |
| `*italic*` | Italic |
| `` `inline code` `` | Green / dim background |
| `- list item` | Bullet colored |
| `> blockquote` | Dim / indented |
| `[link](url)` | Underlined blue |
| `---` | Horizontal rule (dim line) |

### Fenced Code Blocks

````
```rust
fn main() {}
```
````

- Detect language from the info string after ``` (e.g., `rust`, `python`, `go`)
- If no language specified, attempt auto-detection from content heuristics
- Apply language-specific syntax highlighting using `syntect`
- Code block background slightly different (dim) to visually separate

### Implementation

- Use `syntect` crate for code block highlighting (ships with common language grammars)
- Embed default theme (e.g., `base16-ocean.dark`) in binary
- Markdown structure parsing: simple line-by-line state machine
  (not a full AST — just enough for highlighting)
- Apply styles as `tui-textarea` doesn't support per-line styling natively,
  so this may require rendering the note as a custom widget when unfocused
  (read-only rendered markdown) vs raw text when focused (editing)

### Two-Mode Rendering

- **Unfocused** (viewing): Rendered markdown with full highlighting
- **Focused** (editing): Raw text with syntax-aware coloring
  (headings colored, code blocks dimmed, but no rendered formatting)

## Acceptance Criteria

- [ ] Markdown headings, bold, italic, code styled in editor
- [ ] Fenced code blocks highlighted with language-specific colors
- [ ] Language auto-detection for untagged code blocks
- [ ] Embedded syntect themes (no external files)
- [ ] Performance acceptable for notes up to 1000 lines
- [ ] Unfocused note pane shows rendered markdown

## Stories

- [ ] 06.001 — Markdown line-by-line style parser
- [ ] 06.002 — Integrate syntect for fenced code blocks
- [ ] 06.003 — Auto-detect language for untagged blocks
- [ ] 06.004 — Two-mode rendering (view vs edit) — separate epic/story

## Notes

- `syntect` adds ~2MB to binary (embedded grammars + themes)
- Alternative: `tree-sitter-highlight` — lighter but more complex setup
- Could make syntect optional via cargo feature flag
- The unfocused rendered view could reuse the existing `NoteView` widget
  with enhanced markdown parsing
