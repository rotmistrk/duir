//! Apply palette/color overrides from Tcl variables after init.tcl loads.

use std::sync::Arc;

use txv_core::cell::{Color, Style};
use txv_core::palette::{self, DerivedPalette, StyleId};

use super::ScriptEngine;

/// Style IDs configurable via `set color.X "fg bg"` in init.tcl.
const COLOR_ENTRIES: &[(&str, StyleId)] = &[
    ("color.chrome.status_bar", StyleId::StatusBar),
    ("color.chrome.status_bar_modal", StyleId::StatusBarModal),
    ("color.chrome.bar", StyleId::ChromeBar),
    ("color.chrome.tab_focused", StyleId::TabFocused),
    ("color.chrome.tab_active", StyleId::TabActive),
    ("color.interactive.cursor_focused", StyleId::CursorFocused),
    ("color.interactive.input_cursor", StyleId::InputCursor),
    ("color.interactive.search_match", StyleId::SearchMatch),
];

/// Read Tcl variables and apply as palette overrides.
pub fn apply_palette_from_config(engine: &ScriptEngine) {
    let base = palette::palette();
    let mut derived = DerivedPalette::new(base);
    let mut has_overrides = false;

    for &(var_name, style_id) in COLOR_ENTRIES {
        if let Some(val) = engine.get_var(var_name)
            && let Some(style) = parse_style(&val)
        {
            derived = derived.with_override(style_id, style);
            has_overrides = true;
        }
    }

    if has_overrides {
        palette::set_palette(Arc::new(derived));
    }
}

/// Parse "fg bg" or "fg" color spec. Colors: ansi names, #hex, or numbers.
fn parse_style(spec: &str) -> Option<Style> {
    let parts: Vec<&str> = spec.split_whitespace().collect();
    let fg = parse_color(parts.first()?)?;
    let bg = parts.get(1).and_then(|s| parse_color(s)).unwrap_or(Color::Reset);
    Some(Style::new(fg, bg))
}

fn parse_color(s: &str) -> Option<Color> {
    match s.to_lowercase().as_str() {
        "reset" | "default" => Some(Color::Reset),
        "black" => Some(Color::Ansi(0)),
        "red" => Some(Color::Ansi(1)),
        "green" => Some(Color::Ansi(2)),
        "yellow" => Some(Color::Ansi(3)),
        "blue" => Some(Color::Ansi(4)),
        "magenta" => Some(Color::Ansi(5)),
        "cyan" => Some(Color::Ansi(6)),
        "white" => Some(Color::Ansi(7)),
        "gray" | "grey" => Some(Color::Ansi(8)),
        s if s.starts_with('#') && s.len() == 7 => {
            let r = u8::from_str_radix(&s[1..3], 16).ok()?;
            let g = u8::from_str_radix(&s[3..5], 16).ok()?;
            let b = u8::from_str_radix(&s[5..7], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        s => s.parse::<u8>().ok().map(Color::Ansi),
    }
}
