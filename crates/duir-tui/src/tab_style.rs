use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const ACTIVE_BG: Color = Color::Rgb(36, 52, 89); // dark blue
const ACTIVE_FG: Color = Color::Rgb(0, 255, 255); // bright cyan
const INACTIVE_BG: Color = Color::Rgb(58, 58, 58); // dark gray
const INACTIVE_FG: Color = Color::White;
const PANEL_BG: Color = Color::Reset;

/// Powerline round separators (Nerd Font).
const LEFT: &str = "\u{E0B6}"; //  left round
const RIGHT: &str = "\u{E0B4}"; //  right round

/// Build a yazi-style rounded tab bar.
pub fn tab_title(tabs: &[(&str, bool)]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (label, active) in tabs {
        let (bg, fg) = if *active {
            (ACTIVE_BG, ACTIVE_FG)
        } else {
            (INACTIVE_BG, INACTIVE_FG)
        };
        spans.push(Span::styled(LEFT, Style::default().fg(bg).bg(PANEL_BG)));
        spans.push(Span::styled(format!(" {label} "), Style::default().fg(fg).bg(bg)));
        spans.push(Span::styled(RIGHT, Style::default().fg(bg).bg(PANEL_BG)));
    }
    Line::from(spans)
}

/// Build a single-tab title (for tree panel).
pub fn panel_title(label: &str, focused: bool) -> Line<'static> {
    tab_title(&[(label, focused)])
}
