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
}
