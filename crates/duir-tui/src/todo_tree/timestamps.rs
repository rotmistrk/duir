//! Timestamp formatting — age-adaptive 5-char columns.

use std::time::{SystemTime, UNIX_EPOCH};

use super::data::TodoTreeData;

impl TodoTreeData {
    /// Get formatted timestamp cell for a node. col: 0=created, 1=started, 2=last-action.
    #[must_use]
    pub fn timestamp_cell(&self, id: usize, col: usize) -> &str {
        self.timestamps
            .get(id)
            .map_or("     ", |ts| ts.get(col).map_or("     ", String::as_str))
    }

    /// Rebuild all timestamp strings.
    pub fn rebuild_timestamps(&mut self) {
        self.timestamps.clear();
        for i in 0..self.nodes.len() {
            let ts = self.item_at(i).map_or_else(
                || [String::from("     "), String::from("     "), String::from("     ")],
                |item| {
                    [
                        item.created_at.map_or_else(|| "     ".to_owned(), format_ts),
                        item.started_at.map_or_else(|| "     ".to_owned(), format_ts),
                        item.updated_at.map_or_else(|| "     ".to_owned(), format_ts),
                    ]
                },
            );
            self.timestamps.push(ts);
        }
    }
}

/// Format a UTC epoch timestamp into 5-char age-adaptive string.
fn format_ts(epoch_secs: u64) -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let age_secs = now.saturating_sub(epoch_secs);

    if age_secs < 86400 {
        // Today: hh:mm
        let total_mins = (epoch_secs % 86400) / 60;
        let h = total_mins / 60;
        let m = total_mins % 60;
        format!("{h:02}:{m:02}")
    } else if age_secs < 365 * 86400 {
        // This year: MM/DD
        let days_since_epoch = epoch_secs / 86400;
        let (month, day) = month_day_from_epoch_days(days_since_epoch);
        format!("{month:02}/{day:02}")
    } else {
        // Older: YY/MM
        let days_since_epoch = epoch_secs / 86400;
        let (year, month, _) = ymd_from_epoch_days(days_since_epoch);
        format!("{:02}/{month:02}", year % 100)
    }
}

fn month_day_from_epoch_days(days: u64) -> (u64, u64) {
    let (_, m, d) = ymd_from_epoch_days(days);
    (m, d)
}

/// Approximate year/month/day from days since Unix epoch.
fn ymd_from_epoch_days(days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    let mut remaining = days;
    loop {
        let diy = if is_leap(y) { 366 } else { 365 };
        if remaining < diy {
            break;
        }
        remaining -= diy;
        y += 1;
    }
    let months: [u64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u64;
    for dm in months {
        if remaining < dm {
            break;
        }
        remaining -= dm;
        m += 1;
    }
    (y, m, remaining + 1)
}

const fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}
