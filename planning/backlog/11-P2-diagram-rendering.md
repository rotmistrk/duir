# Epic: Diagram Rendering in Export

**ID**: 11
**Priority**: P2
**Status**: backlog

## Goal

Support diagram rendering in document exports (docx, and potentially PDF).
Diagrams defined in markdown fenced blocks (mermaid, plantuml, dot/graphviz)
are rendered as images in the exported document.

## Design

### Supported Diagram Types

| Syntax | Engine | Example |
|--------|--------|---------|
| ```mermaid | mermaid-cli (mmdc) | Flowcharts, sequence, gantt |
| ```plantuml | plantuml.jar | Class, sequence, activity |
| ```dot / ```graphviz | graphviz (dot) | Directed/undirected graphs |

### Rendering Pipeline

1. Parse fenced code blocks with diagram language tags
2. Extract diagram source text
3. Render to SVG/PNG via external tool (mmdc, plantuml, dot)
4. Embed rendered image in docx (or inline in terminal as sixel/kitty)

### Terminal Preview (stretch goal)

- In the TUI note view, render diagrams as ASCII art approximation
- Or use sixel/kitty protocol for inline images (iTerm, kitty, WezTerm)

### Export Integration

- docx: embed as PNG image in the document
- md export: keep as fenced code block (passthrough)
- HTML export (future): embed as SVG

## Acceptance Criteria

- [ ] Detect mermaid/plantuml/dot fenced blocks
- [ ] Render to PNG via external tools
- [ ] Embed in docx export
- [ ] Graceful fallback if tool not installed (include source text)
- [ ] Configurable tool paths in config.toml

## Stories

- [ ] 11.001 — Diagram block detection and extraction
- [ ] 11.002 — External renderer integration (mermaid, plantuml, dot)
- [ ] 11.003 — PNG embedding in docx
- [ ] 11.004 — Terminal preview (ASCII or sixel)

## Notes

- External tools must be installed separately (mmdc, plantuml, dot)
- Consider bundling a simple ASCII diagram renderer for terminal
- mermaid-cli requires Node.js — document this dependency
- plantuml requires Java — document this
- graphviz (dot) is usually available via package manager
