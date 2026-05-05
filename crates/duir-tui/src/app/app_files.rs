use super::{App, FileSource};

impl App {
    pub(crate) fn cmd_files(&mut self) {
        let lines: Vec<String> = self
            .files
            .iter()
            .map(|f| {
                let icon = match f.source {
                    FileSource::Central => "🏠",
                    FileSource::Local => "📁",
                };
                let modified = if f.modified { " [+]" } else { "" };
                format!("{icon} {}{modified}", f.name)
            })
            .collect();
        self.status_message = lines.join(" │ ");
    }

    pub fn apply_file_order(&mut self, order: &[String]) {
        self.files
            .sort_by_key(|f| order.iter().position(|n| n == &f.name).unwrap_or(usize::MAX));
        self.rebuild_rows();
    }
}
