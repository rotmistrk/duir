//! Markdown syntax highlighting — returns style per character for a line.

use txv_core::cell::{Attrs, Color, Style};

/// Compute the style for a line of markdown text.
pub fn line_style(line: &str) -> Style {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        Style {
            fg: Color::Ansi(3), // yellow
            attrs: Attrs {
                bold: true,
                ..Attrs::default()
            },
            ..Style::default()
        }
    } else if trimmed.starts_with("```") {
        Style {
            fg: Color::Ansi(8),
            ..Style::default()
        } // gray
    } else if trimmed.starts_with('>') {
        Style {
            fg: Color::Ansi(6),
            ..Style::default()
        } // cyan
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        Style {
            fg: Color::Ansi(2),
            ..Style::default()
        } // green
    } else {
        Style::default()
    }
}
