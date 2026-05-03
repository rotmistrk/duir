/// Read a file from local filesystem or S3.
pub fn read_file(path: &str) -> Result<String, String> {
    if duir_core::s3_storage::S3Path::is_s3(path) {
        let s3path = duir_core::s3_storage::S3Path::parse(path).ok_or("Invalid S3 path")?;
        let s3 = duir_core::s3_storage::S3Storage::new().map_err(|e| format!("{e}"))?;
        let bytes = s3.read_bytes(&s3path.bucket, &s3path.key).map_err(|e| format!("{e}"))?;
        String::from_utf8(bytes).map_err(|e| format!("{e}"))
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("{e}"))
    }
}

/// Write bytes to local filesystem or S3.
pub fn write_file(path: &str, data: &[u8]) -> Result<(), String> {
    if duir_core::s3_storage::S3Path::is_s3(path) {
        let s3path = duir_core::s3_storage::S3Path::parse(path).ok_or("Invalid S3 path")?;
        let s3 = duir_core::s3_storage::S3Storage::new().map_err(|e| format!("{e}"))?;
        s3.write_bytes(&s3path.bucket, &s3path.key, data.to_vec())
            .map_err(|e| format!("{e}"))
    } else {
        std::fs::write(path, data).map_err(|e| format!("{e}"))
    }
}

pub fn find_available_path(base: &str) -> std::path::PathBuf {
    let path = std::path::PathBuf::from(base);
    if !path.exists() {
        return path;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("export");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
    for i in 1..100 {
        let candidate = std::path::PathBuf::from(format!("{stem}.{i}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    std::path::PathBuf::from(format!("{stem}.99.{ext}"))
}

/// Detect macOS terminal by checking `TERM_PROGRAM` env var.
pub(super) fn detect_mac_terminal() -> bool {
    std::env::var("TERM_PROGRAM")
        .map(|v| {
            let lower = v.to_lowercase();
            lower.contains("iterm") || lower.contains("apple_terminal") || lower.contains("terminal.app")
        })
        .unwrap_or(false)
        || std::env::var("LC_TERMINAL")
            .map(|v| v.to_lowercase().contains("iterm"))
            .unwrap_or(false)
}

use super::{App, app_kiron};

impl App {
    pub(crate) fn mark_modified(&mut self, fi: usize, path: &[usize]) {
        if let Some(file) = self.files.get_mut(fi) {
            file.modified = true;
        }
        for len in (1..=path.len()).rev() {
            if let Some(ancestor) = path.get(..len)
                && let Some(file) = self.files.get_mut(fi)
                && let Some(item) = duir_core::tree_ops::get_item_mut(&mut file.data, &ancestor.to_vec())
            {
                duir_core::crypto::invalidate_cipher(item);
            }
        }
        // Sync MCP snapshot if this edit is inside an active kiron's subtree
        if let Some(file) = self.files.get(fi) {
            let file_id = file.id;
            for (key, kiron) in &self.active_kirons {
                if key.0 != file_id {
                    continue;
                }
                if let Some(ref snapshot) = kiron.mcp_snapshot
                    && let Some(kiron_path) = duir_core::tree_ops::find_node_path(&file.data, &key.1)
                    && path.starts_with(&kiron_path)
                    && let Some(item) = duir_core::tree_ops::get_item(&file.data, &kiron_path)
                {
                    app_kiron::sync_mcp_snapshot(snapshot, item);
                }
            }
        }
    }

    pub(crate) fn mark_saved(&mut self, fi: usize) {
        if let Some(file) = self.files.get_mut(fi) {
            file.modified = false;
        }
    }

    pub(crate) fn mark_file_modified(&mut self, fi: usize) {
        if let Some(file) = self.files.get_mut(fi) {
            file.modified = true;
        }
    }
}
