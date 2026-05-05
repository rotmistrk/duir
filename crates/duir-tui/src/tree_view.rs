use duir_core::Completion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::app::{App, FocusState};

/// Renders the tree pane.
pub struct TreeView<'a> {
    block: Option<Block<'a>>,
}

impl<'a> TreeView<'a> {
    #[must_use]
    pub const fn new() -> Self {
        Self { block: None }
    }

    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

impl StatefulWidget for TreeView<'_> {
    type State = App;

    // ri is bounded by state.scroll_offset..end where end <= state.rows.len().
    // chars slicing is safe: pos = cursor.min(chars.len()).
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = self.block.as_ref().map_or(area, |block| {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        });

        let visible_height = inner.height as usize;
        if state.rows.is_empty() || visible_height == 0 {
            return;
        }

        // Adjust scroll to keep cursor visible
        if state.cursor < state.scroll_offset {
            state.scroll_offset = state.cursor;
        }
        if state.cursor >= state.scroll_offset + visible_height {
            state.scroll_offset = state.cursor - visible_height + 1;
        }

        let end = (state.scroll_offset + visible_height).min(state.rows.len());
        let ready_paths = state.response_ready_paths();

        for (vi, ri) in (state.scroll_offset..end).enumerate() {
            let Some(row) = state.rows.get(ri) else { break };
            let y = inner.y + u16::try_from(vi).unwrap_or(u16::MAX);
            let is_selected = ri == state.cursor;
            render_row(row, is_selected, &ready_paths, state, inner, y, buf);
        }
    }
}

fn render_row(
    row: &crate::app::TreeRow,
    is_selected: bool,
    ready_paths: &[(usize, Vec<usize>)],
    state: &App,
    inner: Rect,
    y: u16,
    buf: &mut Buffer,
) {
    let prefix = build_prefix(row, ready_paths);
    let prefix_width = prefix.chars().count();

    let title = if is_selected && let FocusState::EditingTitle { ref buffer, cursor, .. } = state.state {
        let chars: Vec<char> = buffer.chars().collect();
        let pos = cursor.min(chars.len());
        let before: String = chars.get(..pos).unwrap_or(&[]).iter().collect();
        let after: String = chars.get(pos..).unwrap_or(&[]).iter().collect();
        format!("{before}▏{after}")
    } else {
        row.title.clone()
    };

    let stats_label = if row.stats_text.is_empty() {
        String::new()
    } else {
        format!(" {}", row.stats_text)
    };

    let imp = if row.flags.important() { " !" } else { "" };

    let mut style = Style::default();
    if row.completed == Completion::Done {
        style = style.add_modifier(Modifier::CROSSED_OUT | Modifier::DIM);
    } else if row.flags.important() && !row.flags.has_children() {
        style = style.add_modifier(Modifier::BOLD);
    }

    let content = format!("{title}{stats_label}{imp}");
    let content_width = content.chars().count();
    let total_width = inner.width as usize;
    let pad_len = total_width.saturating_sub(prefix_width + content_width);
    let padding = " ".repeat(pad_len);

    let line = if is_selected {
        build_selected_line(&prefix, &content, &padding, style, state)
    } else {
        Line::from(vec![
            Span::raw(prefix),
            Span::styled(title, style),
            Span::styled(
                format!("{stats_label}{imp}"),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ])
    };
    buf.set_line(inner.x, y, &line, inner.width);
}

fn build_prefix(row: &crate::app::TreeRow, ready_paths: &[(usize, Vec<usize>)]) -> String {
    let indent = "  ".repeat(row.depth);
    let arrow = if row.flags.has_children() {
        if row.flags.expanded() { "▼ " } else { "▶ " }
    } else {
        "  "
    };
    let lock_icon = if row.flags.locked() {
        "🔒"
    } else if row.flags.encrypted() {
        "🔓"
    } else if row.flags.has_encrypted_children() && !row.flags.expanded() {
        "🔐"
    } else {
        ""
    };
    let kiron_icon = if row.flags.kiro_active() {
        "🤖▶"
    } else if row.flags.is_kiron() {
        "🤖"
    } else {
        ""
    };
    let response_icon = if ready_paths
        .iter()
        .any(|(fi, kp)| *fi == row.file_index && kp.starts_with(&row.path))
    {
        "💬"
    } else {
        ""
    };
    let checkbox = if row.flags.is_file_root() {
        match row.file_source {
            Some(crate::app::FileSource::Local) => "📁 ".to_owned(),
            _ => "🏠 ".to_owned(),
        }
    } else {
        match row.completed {
            Completion::Open => "☐ ".to_owned(),
            Completion::Done => "☑ ".to_owned(),
            Completion::Partial => "◐ ".to_owned(),
        }
    };
    format!("{indent}{arrow}{checkbox}{lock_icon}{kiron_icon}{response_icon}")
}

fn build_selected_line(prefix: &str, content: &str, padding: &str, style: Style, state: &App) -> Line<'static> {
    let is_editing = matches!(state.state, FocusState::EditingTitle { .. });
    let select_all = matches!(state.state, FocusState::EditingTitle { select_all: true, .. });
    let title_style = if select_all {
        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
    } else if is_editing {
        Style::default().fg(Color::White).add_modifier(Modifier::UNDERLINED)
    } else {
        style.fg(Color::LightCyan).add_modifier(Modifier::UNDERLINED)
    };
    let trail_style = if select_all {
        Style::default().bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::UNDERLINED)
    };
    Line::from(vec![
        Span::raw(prefix.to_owned()),
        Span::styled(content.to_owned(), title_style),
        Span::styled(padding.to_owned(), trail_style),
    ])
}
