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

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::too_many_lines)]
    // ri is bounded by state.scroll_offset..end where end <= state.rows.len().
    // chars slicing is safe: pos = cursor.min(chars.len()).
    #[allow(clippy::indexing_slicing)]
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

        // Collect kiron paths with response_ready for bubble-up indicator
        let ready_paths = state.response_ready_paths();

        for (vi, ri) in (state.scroll_offset..end).enumerate() {
            let row = &state.rows[ri];
            let y = inner.y + vi as u16;
            let is_selected = ri == state.cursor;

            let indent = "  ".repeat(row.depth);

            let arrow = if row.has_children {
                if row.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };

            let lock_icon = if row.locked {
                "🔒"
            } else if row.encrypted {
                "🔓"
            } else if row.has_encrypted_children && !row.expanded {
                "🔐"
            } else {
                ""
            };

            let kiron_icon = if row.kiro_active {
                "🤖▶"
            } else if row.is_kiron {
                "🤖"
            } else {
                ""
            };

            // Show 💬 if this row is (or is an ancestor of) a kiron with response_ready
            let response_icon = if ready_paths
                .iter()
                .any(|(fi, kp)| *fi == row.file_index && kp.starts_with(&row.path))
            {
                "💬"
            } else {
                ""
            };

            let checkbox = if row.is_file_root {
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

            let prefix = format!("{indent}{arrow}{checkbox}{lock_icon}{kiron_icon}{response_icon}");
            let prefix_width = prefix.chars().count();

            let title = if is_selected && let FocusState::EditingTitle { ref buffer, cursor, .. } = state.state {
                let chars: Vec<char> = buffer.chars().collect();
                let pos = cursor.min(chars.len());
                let before: String = chars[..pos].iter().collect();
                let after: String = chars[pos..].iter().collect();
                format!("{before}▏{after}")
            } else {
                row.title.clone()
            };

            let stats_label = if row.stats_text.is_empty() {
                String::new()
            } else {
                format!(" {}", row.stats_text)
            };

            let imp = if row.important { " !" } else { "" };

            let mut style = Style::default();
            if row.completed == Completion::Done {
                style = style.add_modifier(Modifier::CROSSED_OUT | Modifier::DIM);
            } else if row.important && !row.has_children {
                style = style.add_modifier(Modifier::BOLD);
            }

            // Content after prefix: title + stats + padding to fill line
            let content = format!("{title}{stats_label}{imp}");
            let content_width = content.chars().count();
            let total_width = inner.width as usize;
            let pad_len = total_width.saturating_sub(prefix_width + content_width);
            let padding = " ".repeat(pad_len);

            if is_selected {
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

                let line = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(content, title_style),
                    Span::styled(padding, trail_style),
                ]);
                buf.set_line(inner.x, y, &line, inner.width);
            } else {
                let line = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(title, style),
                    Span::styled(
                        format!("{stats_label}{imp}"),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                ]);
                buf.set_line(inner.x, y, &line, inner.width);
            }
        }
    }
}
