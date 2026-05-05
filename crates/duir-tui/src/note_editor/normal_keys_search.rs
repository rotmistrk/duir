use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::NoteEditor;

impl NoteEditor<'_> {
    pub(crate) fn handle_search_keys(&mut self, key: KeyEvent) -> Option<bool> {
        match key.code {
            KeyCode::Char('n') => {
                self.textarea.search_forward(false);
                Some(true)
            }
            KeyCode::Char('N') => {
                self.textarea.search_back(false);
                Some(true)
            }
            KeyCode::Char('*') => {
                self.search_word_under_cursor(true);
                Some(true)
            }
            KeyCode::Char('#') => {
                self.search_word_under_cursor(false);
                Some(true)
            }
            KeyCode::Char(';') => {
                if let Some((cmd, ch)) = self.last_find {
                    self.execute_find(cmd, ch);
                }
                Some(true)
            }
            KeyCode::Char(',') => {
                if let Some((cmd, ch)) = self.last_find {
                    let rev = match cmd {
                        'f' => 'F',
                        'F' => 'f',
                        't' => 'T',
                        'T' => 't',
                        _ => cmd,
                    };
                    self.execute_find(rev, ch);
                }
                Some(true)
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.open_url_at_cursor();
                Some(true)
            }
            _ => None,
        }
    }
}
