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
        .filter_map(|line| {
            // Skip separator rows
            if line.starts_with('|') && line.contains("---") {
                return None;
            }
            Some(render_help_line(line))
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

fn render_help_line(line: &str) -> Line<'_> {
    if let Some(h) = line.strip_prefix("### ") {
        return Line::styled(h, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    }
    if let Some(h) = line.strip_prefix("## ") {
        return Line::from(vec![
            Span::raw(""),
            Span::styled(h, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        ]);
    }
    if let Some(h) = line.strip_prefix("# ") {
        return Line::styled(
            h,
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        );
    }
    if line.starts_with('|') {
        return render_table_row(line);
    }
    render_inline_markup(line)
}

fn render_table_row(line: &str) -> Line<'_> {
    let cells: Vec<&str> = line.split('|').collect();
    // cells[0] is empty (before first |), cells[1] is key, cells[2] is action
    if cells.len() >= 3 {
        let key = cells[1].trim();
        let action = cells[2].trim();
        let key_width = 22;
        Line::from(vec![
            Span::styled(format!("  {key:<key_width$}"), Style::default().fg(Color::Green)),
            Span::raw(action.to_owned()),
        ])
    } else {
        Line::raw(line)
    }
}

fn render_inline_markup(line: &str) -> Line<'_> {
    let mut spans = Vec::new();
    let mut rest = line;

    while let Some(pos) = rest.find('`') {
        if pos > 0 {
            spans.push(Span::raw(&rest[..pos]));
        }
        let after = &rest[pos + 1..];
        if let Some(end) = after.find('`') {
            spans.push(Span::styled(&after[..end], Style::default().fg(Color::Green)));
            rest = &after[end + 1..];
        } else {
            spans.push(Span::raw(&rest[pos..]));
            rest = "";
            break;
        }
    }
    if !rest.is_empty() {
        spans.push(Span::raw(rest));
    }
    Line::from(spans)
}
