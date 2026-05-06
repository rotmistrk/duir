use ratatui::layout::Rect;

use crate::app::App;

use super::render::panel_block;

/// Render the kiro panel as a standalone panel (not tabbed with note).
pub fn render_kiro_panel(frame: &mut ratatui::Frame, app: &mut App, area: Rect) {
    let active_kiron_key = app.active_kiron_for_cursor();

    let kiro_focused = app.is_kiro_focused();
    let title = crate::tab_style::panel_title("🤖 Kiro", kiro_focused);
    let kiro_block = panel_block(title, kiro_focused);
    let inner = kiro_block.inner(area);

    if let Some(ref key) = active_kiron_key
        && let Some(kiron) = app.active_kirons.get_mut(key)
    {
        if kiron.pty.termbuf.cols() != inner.width as usize || kiron.pty.termbuf.rows() != inner.height as usize {
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
    } else {
        // No active kiro session — show placeholder
        let placeholder = ratatui::widgets::Paragraph::new("No active kiro session")
            .style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray))
            .block(kiro_block);
        frame.render_widget(placeholder, area);
    }
}
