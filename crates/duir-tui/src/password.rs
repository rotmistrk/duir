use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

/// Password prompt modal state.
pub struct PasswordPrompt {
    pub title: String,
    pub input: String,
    pub callback: PasswordAction,
}

/// What to do after password is entered.
pub enum PasswordAction {
    Decrypt { file_index: usize, path: Vec<usize> },
    Encrypt { file_index: usize, path: Vec<usize> },
    ChangePassword { file_index: usize, path: Vec<usize> },
}

/// Result of handling a key in the password prompt.
pub enum PromptResult {
    /// Still typing.
    Pending,
    /// User pressed Enter with this password.
    Submitted(String),
    /// User pressed Esc.
    Cancelled,
}

impl PasswordPrompt {
    #[must_use]
    pub fn new(title: &str, callback: PasswordAction) -> Self {
        Self {
            title: title.to_owned(),
            input: String::new(),
            callback,
        }
    }

    /// Handle a key event.
    pub fn handle_key(&mut self, key: KeyEvent) -> PromptResult {
        match key.code {
            KeyCode::Enter => PromptResult::Submitted(self.input.clone()),
            KeyCode::Esc => PromptResult::Cancelled,
            KeyCode::Backspace => {
                self.input.pop();
                PromptResult::Pending
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                PromptResult::Pending
            }
            _ => PromptResult::Pending,
        }
    }

    /// Render the password prompt.
    pub fn render(&self, frame: &mut ratatui::Frame, area: Rect) {
        let w = 50.min(area.width.saturating_sub(4));
        let h = 6;
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let popup = Rect::new(x, y, w, h);

        frame.render_widget(Clear, popup);

        let masked: String = "•".repeat(self.input.len());
        let lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Password: "),
                Span::styled(format!("{masked}▏"), Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::styled(
                "  Enter to confirm, Esc to cancel",
                Style::default().fg(Color::DarkGray),
            ),
            Line::raw(""),
        ];

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        frame.render_widget(paragraph, popup);
    }
}
