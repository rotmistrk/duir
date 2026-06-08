//! Test harness for duir-tui integration/scenario tests.

use std::path::Path;

use tempfile::TempDir;
use txv_core::clipboard_ring::new_clipboard;
use txv_core::event::{KeyCode, KeyMod};
use txv_core::program::Program;
use txv_core::run::MockBackend;

/// Create a temp project dir with .duir/todo.todo.json.
pub fn temp_project(items_json: &str) -> TempDir {
    let dir = TempDir::new().expect("tmp dir");
    let duir_dir = dir.path().join(".duir");
    std::fs::create_dir_all(&duir_dir).expect("create .duir");
    std::fs::write(duir_dir.join("todo.todo.json"), items_json).expect("write todo");
    dir
}

/// Default todo file with a few items.
pub fn default_todo() -> &'static str {
    r#"{"version":"2.0","title":"Test","items":[
        {"id":"a","title":"First item","completed":"Open","note":"hello"},
        {"id":"b","title":"Second item","completed":"Open","items":[
            {"id":"c","title":"Child item","completed":"Open"}
        ]},
        {"id":"d","title":"Done item","completed":"Done"}
    ]}"#
}

pub struct TestHarness {
    pub program: Program,
    pub backend: MockBackend,
}

impl TestHarness {
    pub fn new(root_dir: &Path) -> Self {
        Self::with_size(root_dir, 120, 30)
    }

    pub fn with_size(root_dir: &Path, width: u16, height: u16) -> Self {
        use duir_tui::build_desktop::build_workspace;
        use duir_tui::status::build_status_bar;

        let clipboard = new_clipboard(20);
        let desktop = build_workspace(root_dir, clipboard.clone());
        let bar = build_status_bar(&desktop, clipboard);
        let program = Program::new(Box::new(bar), Box::new(desktop));
        let backend = MockBackend::new(width, height);
        Self { program, backend }
    }

    pub fn inject_key(&mut self, code: KeyCode, mods: KeyMod) {
        self.backend.inject_key(code, mods);
    }

    pub fn inject_str(&mut self, s: &str) {
        self.backend.inject_str(s);
    }

    pub fn run_cycles(&mut self, n: usize) {
        self.program.run_cycles(
            &mut self.backend,
            &mut |ctx| {
                duir_tui::handler::handle_command(ctx);
            },
            n,
        );
    }

    pub fn screen_text(&self) -> String {
        self.backend.screen_text()
    }

    pub fn contains(&self, text: &str) -> bool {
        self.screen_text().contains(text)
    }
}
