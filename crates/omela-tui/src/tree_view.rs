use omela_core::Completion;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, StatefulWidget, Widget};

use crate::app::App;

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

        for (vi, ri) in (state.scroll_offset..end).enumerate() {
            let row = &state.rows[ri];
            let y = inner.y + vi as u16;
            let is_selected = ri == state.cursor;

            let indent = "  ".repeat(row.depth);

            let arrow = if row.is_file_root {
                if row.has_children {
                    if row.expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                }
            } else if row.has_children {
                if row.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };

            let checkbox = if row.is_file_root {
                String::new()
            } else {
                match row.completed {
                    Completion::Open => "☐ ".to_owned(),
                    Completion::Done => "☑ ".to_owned(),
                    Completion::Partial => "◐ ".to_owned(),
                }
            };

            let title = if is_selected && state.editing_title {
                format!("{}▏", state.edit_buffer)
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

            if is_selected {
                style = style.add_modifier(Modifier::REVERSED);
            }

            let line = Line::from(vec![
                Span::raw(format!("{indent}{arrow}{checkbox}")),
                Span::styled(title, style),
                Span::styled(
                    format!("{stats_label}{imp}"),
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ]);

            let width = inner.width as usize;
            buf.set_line(inner.x, y, &line, width as u16);
        }
    }
}
