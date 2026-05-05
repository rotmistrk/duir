use crossterm::event::{KeyCode, KeyModifiers};
use duir_core::{FileStorage, TodoStorage};

use crate::app::App;

/// Convert a crossterm `KeyEvent` to bytes for PTY input.
pub fn key_to_bytes(key: crossterm::event::KeyEvent) -> Vec<u8> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                let ctrl = (c as u8).wrapping_sub(b'a').wrapping_add(1);
                vec![ctrl]
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                s.as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        _ => Vec::new(),
    }
}

pub fn save_file_order(app: &App, _config: &duir_core::config::Config) {
    let order: Vec<String> = app.files.iter().map(|f| f.name.clone()).collect();
    let state = duir_core::config::AppState { file_order: order };
    state.save();
}

pub fn handle_file_changed(app: &mut App, storage_dir: &std::path::Path, path: &std::path::Path) {
    let name = match path.file_stem().and_then(|s| s.to_str()) {
        Some(n) => n.strip_suffix(".todo").unwrap_or(n).to_owned(),
        None => return,
    };
    let Some(fi) = app.files.iter().position(|f| f.name == name) else {
        return;
    };
    let Ok(storage) = FileStorage::new(storage_dir) else {
        return;
    };
    let new_mtime = storage.mtime(&name);
    let Some(file) = app.files.get_mut(fi) else { return };

    if file.is_modified() {
        file.conflicted = true;
        app.set_status(
            &format!("⚠ {name} changed on disk (conflicted)"),
            crate::app::StatusLevel::Warning,
        );
    } else if let Ok(data) = storage.load(&name) {
        file.data = data;
        file.disk_mtime = new_mtime;
        app.rebuild_rows();
        app.set_status(
            &format!("↻ {name} reloaded from disk"),
            crate::app::StatusLevel::Success,
        );
    }
}
