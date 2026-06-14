//! Item lifecycle — archive done items, trash old archived items.

use crate::model::{Completion, TodoFile, TodoItem};

/// Lifecycle engine configuration.
pub struct Lifecycle {
    /// Hours after which done items are moved to archive (0 = disabled).
    pub archive_after_hours: u64,
    /// Days after which archived items are moved to trash (0 = disabled).
    pub delete_after_days: u64,
}

impl Lifecycle {
    #[must_use]
    pub const fn new(archive_after_hours: u64, delete_after_days: u64) -> Self {
        Self {
            archive_after_hours,
            delete_after_days,
        }
    }

    /// Run lifecycle: move stale done items to archive, stale archived to trash.
    pub fn run(&self, main: &mut TodoFile, archive: &mut TodoFile, trash: &mut TodoFile) {
        let now = now_epoch();

        // Phase 1: archive → trash (old archived items)
        if self.delete_after_days > 0 {
            let mut archive_keep = Vec::new();
            for item in archive.items.drain(..) {
                if item_age_secs(&item, now) >= self.delete_after_days * 24 * 3600 {
                    trash.items.push(item);
                } else {
                    archive_keep.push(item);
                }
            }
            archive.items = archive_keep;
        }

        // Phase 2: main → archive (old done items)
        if self.archive_after_hours > 0 {
            let mut keep = Vec::new();
            for item in main.items.drain(..) {
                if should_archive(&item, now, self.archive_after_hours) {
                    archive.items.push(item);
                } else {
                    keep.push(item);
                }
            }
            main.items = keep;
        }
    }

    /// Restore item from source file to main (appends at end).
    pub fn restore(source: &mut TodoFile, path: &[usize], dest: &mut TodoFile) {
        let Some(&idx) = path.first() else { return };
        if path.len() != 1 || idx >= source.items.len() {
            return;
        }
        let item = source.items.remove(idx);
        dest.items.push(item);
    }
}

fn should_archive(item: &TodoItem, now: u64, hours: u64) -> bool {
    if item.completed != Completion::Done {
        return false;
    }
    item_age_secs(item, now) >= hours * 3600
}

fn item_age_secs(item: &TodoItem, now: u64) -> u64 {
    let ts = item.updated_at.or(item.created_at).unwrap_or(0);
    now.saturating_sub(ts)
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
