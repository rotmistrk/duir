use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

const HELP_TEXT: &str = include_str!("../../../HELP.md");

const OAK: &str = r#"
        &&& &&  & &&
      && &\/&\|& ()|/ @, &&
      &\/(/&/&||/& /_/)_&/_&
   &() &\/&|()|/&\/ '%" & ()
  &_\_&&_\ |& |&&/&__%_/_& &&
&&   && & &| &| /& & % ()& /&&
 ()&_---()&\&\|&&-&&--%---()~
     &&     \|||
             |||
             |||
             |||
         , -=-~  .-^- _
"#;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Render the `:about` overlay.
pub fn render_about(frame: &mut ratatui::Frame, area: Rect) {
    let popup = centered_rect(60, 80, area);
    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line<'_>> = Vec::new();

    for oak_line in OAK.lines() {
        lines.push(Line::styled(oak_line, Style::default().fg(Color::Green)));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  duir",
            Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("  v{VERSION}")),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "  Irish for \"oak\" · root of \"druid\" · sounds like \"do it\"",
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::raw(""));
    lines.push(Line::raw("  Hierarchical todo tree with vim-like editor"));
    lines.push(Line::raw("  Named after duir — the oak in the Ogham tree alphabet"));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "  (c) CyR 2025 · MIT License",
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "  Press any key to close",
        Style::default().add_modifier(Modifier::DIM),
    ));

    let block = Block::default()
        .title(" About Duir ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightCyan));

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup);
}

/// Render the `:help` overlay.
pub fn render_help(frame: &mut ratatui::Frame, area: Rect, scroll: u16) {
    let popup = centered_rect(80, 90, area);
    frame.render_widget(Clear, popup);

    let lines: Vec<Line<'_>> = HELP_TEXT
        .lines()
        .map(|line| {
            if line.starts_with("## ") {
                Line::styled(line, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD))
            } else if line.starts_with("### ") {
                Line::styled(line, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else if line.starts_with("# ") {
                Line::styled(
                    line,
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )
            } else if line.starts_with('|') && line.contains("---") {
                Line::styled(line, Style::default().fg(Color::DarkGray))
            } else if line.starts_with('|') {
                // Table row: highlight the key column
                let parts: Vec<&str> = line.splitn(3, '|').collect();
                if parts.len() >= 3 {
                    Line::from(vec![
                        Span::raw("|"),
                        Span::styled(parts[1], Style::default().fg(Color::Green)),
                        Span::raw("|"),
                        Span::raw(parts[2]),
                    ])
                } else {
                    Line::raw(line)
                }
            } else {
                Line::raw(line)
            }
        })
        .collect();

    let block = Block::default()
        .title(" Help — :help (↑↓ scroll, q/Esc close) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::LightCyan));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(paragraph, popup);
}

const fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let w = area.width * pct_x / 100;
    let h = area.height * pct_y / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
