use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, FocusState};

use super::render::panel_block;

// Layout chunk indices match the number of constraints provided to Layout::split.
pub fn render_note_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect, zoomed: bool) {
    let active_kiron_key = app.active_kiron_for_cursor();
    let has_kiron = active_kiron_key.is_some();

    if has_kiron && !matches!(app.state, FocusState::Note { .. }) {
        let kiro_focused = app.is_kiro_focused();

        if app.flags.kiro_tab_focused() {
            if let Some(ref key) = active_kiron_key
                && let Some(kiron) = app.active_kirons.get_mut(key)
            {
                let title = crate::tab_style::tab_title(&[("📝 Note", false), ("🤖 Kiro", kiro_focused)]);

                let kiro_block = if zoomed {
                    Block::default().title(title).borders(Borders::NONE)
                } else {
                    panel_block(title, kiro_focused)
                };
                let inner = kiro_block.inner(area);

                if kiron.pty.termbuf.cols() != inner.width as usize || kiron.pty.termbuf.rows() != inner.height as usize
                {
                    kiron.pty.resize(inner.width, inner.height);
                }

                frame.render_widget(kiro_block, area);
                super::render::render_termbuf(frame, &kiron.pty.termbuf, inner);

                if kiro_focused && kiron.pty.termbuf.cursor_visible {
                    let (crow, ccol) = kiron.pty.termbuf.cursor();

                    frame.set_cursor_position((
                        inner.x + u16::try_from(ccol).unwrap_or(u16::MAX),
                        inner.y + u16::try_from(crow).unwrap_or(u16::MAX),
                    ));
                }
            }
        } else {
            let note_content = app.current_note();
            let note_focused = app.is_note_focused();

            let title = crate::tab_style::tab_title(&[("📝 Note", note_focused), ("🤖 Kiro", false)]);

            let note_block = if zoomed {
                Block::default().title(title).borders(Borders::NONE)
            } else {
                panel_block(title, note_focused)
            };

            let lines =
                crate::markdown_view::highlight_lines_with_syntax(&note_content, usize::MAX, 0, Some(&app.highlighter));

            let paragraph = Paragraph::new(lines).block(note_block);
            frame.render_widget(paragraph, area);
        }
    } else if let FocusState::Note { ref mut editor, .. } = app.state {
        let title_line = if has_kiron {
            crate::tab_style::tab_title(&[("📝 Note", true), ("🤖 Kiro", false)])
        } else {
            crate::tab_style::panel_title("📝 Note", true)
        };

        let has_cmdline = matches!(
            editor.mode,
            crate::note_editor::EditorMode::Command | crate::note_editor::EditorMode::Search
        );

        if has_cmdline {
            let note_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)])
                .split(area);

            editor.set_block(title_line.clone(), true, zoomed);
            editor.render(
                frame,
                note_chunks.first().copied().unwrap_or_default(),
                &app.highlighter,
            );

            let cmd_line = editor.status_line();
            frame.render_widget(
                Paragraph::new(cmd_line),
                note_chunks.get(1).copied().unwrap_or_default(),
            );
        } else {
            editor.set_block(title_line, true, zoomed);
            editor.render(frame, area, &app.highlighter);
        }
    } else {
        let note_content = app.current_note();

        let title = crate::tab_style::panel_title("📝 Note", false);

        let note_block = if zoomed {
            Block::default().title(title).borders(Borders::NONE)
        } else {
            panel_block(title, false)
        };

        let lines =
            crate::markdown_view::highlight_lines_with_syntax(&note_content, usize::MAX, 0, Some(&app.highlighter));

        let paragraph = Paragraph::new(lines).block(note_block);
        frame.render_widget(paragraph, area);
    }
}
