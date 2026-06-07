//! Badge computation — 5-char badge column.
//!
//! Layout: `[status][priority][effort][notes][type]`

#[allow(clippy::missing_const_for_fn)]
mod inner {
    use super::super::data::TodoTreeData;
    use super::super::model::{Completion, TodoItem};
    use duir_core::model::WorkStatus;

    impl TodoTreeData {
        /// Get the 5-char badge string for a node.
        pub fn badge_at(&self, id: usize) -> &str {
            self.badges.get(id).map_or("     ", String::as_str)
        }

        /// Rebuild all badge strings.
        pub fn rebuild_badges(&mut self) {
            self.badges.clear();
            for i in 0..self.nodes.len() {
                let badge = self.item_at(i).map_or_else(|| "     ".to_owned(), compute_badge);
                self.badges.push(badge);
            }
        }
    }

    fn compute_badge(item: &TodoItem) -> String {
        let mut buf = String::with_capacity(5);
        buf.push(status_char(item));
        buf.push(priority_char(item));
        buf.push(effort_char(item));
        buf.push(notes_char(item));
        buf.push(type_char(item));
        buf
    }

    fn status_char(item: &TodoItem) -> char {
        if item.is_locked() {
            return '🔒';
        }
        match (&item.completed, &item.work_status) {
            (Completion::Done, _) => '✓',
            (Completion::Partial, _) => '◐',
            (_, WorkStatus::InProgress) => '▶',
            (_, WorkStatus::Paused) => '⏸',
            _ => '○',
        }
    }

    fn priority_char(item: &TodoItem) -> char {
        const BRAILLE: &[char] = &[' ', '⠁', '⠃', '⠇', '⡇', '⣇', '⣧', '⣷', '⣿', '⣿'];
        let p = item.priority.unwrap_or(0).min(9) as usize;
        BRAILLE.get(p).copied().unwrap_or(' ')
    }

    fn effort_char(item: &TodoItem) -> char {
        match item.effort {
            None | Some(0) => ' ',
            Some(1) => '1',
            Some(2) => '2',
            Some(3) => '3',
            Some(5) => '5',
            Some(8) => '8',
            Some(13) => 'D',
            Some(21) => 'U',
            _ => '?',
        }
    }

    fn notes_char(item: &TodoItem) -> char {
        if item.note.is_empty() { ' ' } else { '♪' }
    }

    fn type_char(item: &TodoItem) -> char {
        match item.node_type.as_ref() {
            Some(duir_core::model::NodeType::Kiron) => '🤖',
            Some(duir_core::model::NodeType::Prompt) => '💬',
            Some(duir_core::model::NodeType::Response) => '📋',
            None => ' ',
        }
    }
}
