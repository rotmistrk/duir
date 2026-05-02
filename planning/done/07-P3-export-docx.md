# Epic: Export to Word Document

**ID**: 07
**Priority**: P3
**Status**: backlog

## Goal

Export subtree as a .docx Word document, mirroring the markdown export
structure with proper headings, checkboxes, and formatting.

## Design

- Use `docx-rs` or `rust-docx` crate to generate .docx
- Same structure as markdown export: headings for depth, checkboxes for leaves
- Encrypted nodes redacted (same as :yank safe export)
- Command: `:export docx [file.docx]`

## Acceptance Criteria

- [ ] `:export docx` generates a valid .docx file
- [ ] Headings map to Word heading styles (Heading 1, 2, 3)
- [ ] Checkboxes rendered as bullet lists with ☐/☑ symbols
- [ ] Important items bold
- [ ] Completed items strikethrough
- [ ] Notes rendered as body text under their item
- [ ] Encrypted nodes show as 🔒 placeholder
- [ ] Default output to temp dir, optional filename argument

## Stories

- [ ] 07.001 — docx generation with heading/checkbox structure
- [ ] 07.002 — Formatting (bold, strikethrough, notes)
- [ ] 07.003 — Encryption-safe export

## Notes

- Low priority — markdown covers most use cases
- Useful for sharing with non-technical stakeholders
- Consider also PDF export via the same intermediate representation
