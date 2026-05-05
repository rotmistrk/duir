use std::path::PathBuf;
use std::sync::mpsc;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Events sent from the file watcher to the app.
#[derive(Debug)]
pub struct FileChanged {
    pub path: PathBuf,
}

/// Spawns a file watcher on the given directories.
/// Returns a receiver for change events, or None if watching fails.
pub fn spawn(dirs: &[PathBuf]) -> Option<mpsc::Receiver<FileChanged>> {
    let (tx, rx) = mpsc::channel();

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        if let Ok(event) = res {
            let event: notify::Event = event;
            let dominated = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if dominated {
                for path in event.paths {
                    if path.extension().and_then(|e| e.to_str()) == Some("json")
                        && path.to_string_lossy().contains(".todo.json")
                    {
                        let _ = tx.send(FileChanged { path });
                    }
                }
            }
        }
    })
    .ok()?;

    for dir in dirs {
        let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
    }

    // Keep watcher alive by leaking it (lives for the process lifetime)
    std::mem::forget(watcher);

    Some(rx)
}
