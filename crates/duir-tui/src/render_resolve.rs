use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};

use crate::app::ConflictState;
use duir_core::conflict::{ConflictKind, Resolution};

pub fn render_resolve_overlay(frame: &mut Frame, size: Rect, state: &ConflictState) {
    let width = size.width.saturating_sub(4).min(80);
    let count = u16::try_from(state.conflicts.len()).unwrap_or(u16::MAX);
    let height = size.height.saturating_sub(4).min(count + 4);
    let x = (size.width.saturating_sub(width)) / 2;
    let y = (size.height.saturating_sub(height)) / 2;
    let area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Resolve Conflicts (m=mine t=theirs b=both Enter=apply Esc=cancel) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = state
        .conflicts
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let marker = match state.resolutions.get(i).and_then(|r| r.as_ref()) {
                Some(Resolution::KeepMine) => "[mine]  ",
                Some(Resolution::KeepTheirs) => "[theirs]",
                Some(Resolution::KeepBoth) => "[both]  ",
                None => "[?]     ",
            };
            let kind = match c.kind {
                ConflictKind::Modified => "~",
                ConflictKind::DeletedLocally => "+disk",
                ConflictKind::DeletedOnDisk => "-disk",
            };
            let title = if c.title_mine.is_empty() {
                &c.title_theirs
            } else {
                &c.title_mine
            };
            let selected = i == state.cursor;
            let style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(kind, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(title.to_owned(), style),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}
