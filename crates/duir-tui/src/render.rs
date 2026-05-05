use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::{App, FocusState, StatusLevel};
use crate::completer::Completer;
use crate::tree_view::TreeView;

const ACTIVE_BORDER: Color = Color::Rgb(0, 255, 255); // bright cyan
const INACTIVE_BORDER: Color = Color::DarkGray;

fn panel_block(title: Line<'static>, focused: bool) -> Block<'static> {
    let color = if focused { ACTIVE_BORDER } else { INACTIVE_BORDER };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
}

// Layout chunk indices match the number of constraints provided to Layout::split.
pub fn render_frame(frame: &mut ratatui::Frame, app: &mut App) {
    let size = frame.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(size);

    if app.flags.zoomed() {
        // Fullscreen: show only the focused panel, no border
        let area = main_chunks.first().copied().unwrap_or_default();

        if app.is_note_focused() || app.is_kiro_focused() {
            render_note_panel(frame, app, area, true);
        } else {
            render_tree_panel(frame, app, area, true);
        }
    } else {
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100 - app.note_panel_pct),
                Constraint::Percentage(app.note_panel_pct),
            ])
            .split(main_chunks.first().copied().unwrap_or_default());

        render_tree_panel(frame, app, content_chunks.first().copied().unwrap_or_default(), false);
        render_note_panel(frame, app, content_chunks.get(1).copied().unwrap_or_default(), false);
    }

    // Status bar
    let status = build_status_line(app);

    frame.render_widget(Paragraph::new(status), main_chunks.get(1).copied().unwrap_or_default());

    // Command palette popup (above status bar)
    if app.is_command_active() && !app.completer.matches.is_empty() {
        render_palette(frame, &app.completer, main_chunks.get(1).copied().unwrap_or_default());
    }

    // Overlays
    if app.is_about_shown() {
        crate::help::render_about(frame, size);
    }

    if let FocusState::Help { scroll, ref search } = app.state {
        crate::help::render_help(frame, size, scroll, search);
    }

    if let Some(prompt) = &app.password_prompt {
        prompt.render(frame, size);
    }
}

fn render_tree_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect, zoomed: bool) {
    let mut label = "Tree".to_owned();

    if app.has_unsaved() {
        label.push_str(" (*)");
    }

    if !app.filter_committed_text.is_empty() && !app.is_filter_active() {
        use std::fmt::Write;
        let _ = write!(label, " [/{}]", app.filter_committed_text);
    }

    let tree_focused = app.is_tree_focused() || app.is_editing_title();
    let title = crate::tab_style::panel_title(&label, tree_focused);

    let tree_block = if zoomed {
        Block::default().title(title).borders(Borders::NONE)
    } else {
        panel_block(title, tree_focused)
    };

    frame.render_stateful_widget(TreeView::new().block(tree_block), area, app);
}

// Layout chunk indices match the number of constraints provided to Layout::split.
fn render_note_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect, zoomed: bool) {
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
                render_termbuf(frame, &kiron.pty.termbuf, inner);

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

pub fn render_palette(frame: &mut ratatui::Frame, completer: &Completer, status_area: Rect) {
    let matches = &completer.matches;

    let max_visible = 10.min(matches.len());

    let height = u16::try_from(max_visible).unwrap_or(u16::MAX);

    let popup = Rect::new(
        status_area.x + 1,
        status_area.y.saturating_sub(height),
        30.min(status_area.width),
        height,
    );

    frame.render_widget(Clear, popup);

    let lines: Vec<Line<'_>> = matches
        .iter()
        .take(max_visible)
        .enumerate()
        .map(|(i, cmd)| {
            let style = if completer.selected == Some(i) {
                Style::default().bg(Color::DarkGray).fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 30))
            };
            Line::styled(format!(" {cmd}"), style)
        })
        .collect();

    let block = Block::default().borders(Borders::NONE);
    let paragraph = Paragraph::new(lines).block(block);

    frame.render_widget(paragraph, popup);
}

/// Render a `TermBuf` into a ratatui frame area (direct buffer write).
// col is bounded by `col >= cells.len()` break above; buf[(x,y)] is ratatui's own indexing.
pub fn render_termbuf(frame: &mut ratatui::Frame<'_>, termbuf: &crate::termbuf::TermBuf, area: Rect) {
    let buf = frame.buffer_mut();

    for row in 0..area.height as usize {
        if row >= termbuf.rows() {
            break;
        }

        let cells = termbuf.visible_row(row);

        for col in 0..area.width as usize {
            if col >= cells.len() {
                break;
            }

            let x = area.x + u16::try_from(col).unwrap_or(u16::MAX);

            let y = area.y + u16::try_from(row).unwrap_or(u16::MAX);

            if x < area.right() && y < area.bottom() {
                let Some(cell) = cells.get(col) else { break };
                let buf_cell = &mut buf[(x, y)];

                buf_cell.set_char(cell.ch);
                buf_cell.set_style(cell.style);
            }
        }
    }
}

pub fn build_status_line(app: &App) -> Line<'_> {
    if let FocusState::Command { ref buffer, .. } = app.state {
        Line::from(vec![
            Span::raw(":"),
            Span::styled(format!("{buffer}▏"), Style::default().add_modifier(Modifier::BOLD)),
        ])
    } else if let FocusState::Filter { ref text, .. } = app.state {
        Line::from(vec![
            Span::raw("Filter: "),
            Span::styled(format!("{text}▏"), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  [Enter] apply  [Esc] cancel"),
        ])
    } else if app.is_editing_title() {
        Line::from(vec![
            Span::raw("Editing: "),
            Span::styled(
                "[Enter] confirm  [Esc] cancel",
                Style::default().add_modifier(Modifier::DIM),
            ),
        ])
    } else {
        let bold = Style::default().add_modifier(Modifier::BOLD);

        let mut spans = vec![
            Span::styled(" q", bold),
            Span::raw("uit "),
            Span::styled("e", bold),
            Span::raw("dit "),
            Span::styled("n", bold),
            Span::raw("ew "),
            Span::styled("b", bold),
            Span::raw("ranch "),
            Span::styled("d", bold),
            Span::raw("el "),
            Span::styled("c", bold),
            Span::raw("lone "),
            Span::styled("!", bold),
            Span::raw("imp "),
            Span::styled("HJKL", bold),
            Span::raw(" move "),
            Span::styled("S", bold),
            Span::raw("ort "),
            Span::styled("/", bold),
            Span::raw("filter "),
            Span::styled("^S", bold),
            Span::raw("ave "),
        ];

        if app.active_kiron_for_cursor().is_some() {
            spans.push(Span::styled("⏎", bold));
            spans.push(Span::raw("send "));
        }

        spans.extend_from_slice(&[Span::styled(":", bold), Span::raw("cmd "), Span::styled(":help", bold)]);

        if !app.status_message.is_empty() {
            let color = match app.status_level {
                StatusLevel::Info => Color::DarkGray,
                StatusLevel::Success => Color::Green,
                StatusLevel::Warning => Color::Yellow,
                StatusLevel::Error => Color::Red,
            };

            spans.push(Span::styled(
                format!("  │ {}", app.status_message),
                Style::default().fg(color),
            ));
        }

        Line::from(spans)
    }
}
