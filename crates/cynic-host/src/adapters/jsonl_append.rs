//! Unified JSONL append for CYNIC.
//!
//! WHY THIS EXISTS.
//!
//! Before this module, 73 sites in CYNIC wrote JSONL files with:
//!
//! ```rust
//! let mut file = OpenOptions::new().create(true).append(true).open(path)?;
//! writeln!(file, "{line}")?;
//! ```
//!
//! The same pattern was copy-pasted across `build_proof_log.rs`,
//! `build_runner_adapter.rs`, and others. Each site had its own error handling,
//! its own directory creation, its own logging. Adding fsync, rotation, or
//! quotas required touching every site.
//!
//! This module is the single point where CYNIC appends to JSONL files. All 73
//! sites should migrate here.
//!
//! DESIGN.
//!
//! - `JsonlAppend::append(path, line)` appends a line to a JSONL file, creating
//!   parent directories if needed.
//! - `JsonlAppend::append_with_fsync(path, line)` appends and fsyncs, for
//!   durability-critical logs (e.g., proof logs).
//! - Errors are returned as `String` for simplicity.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Unified JSONL append for CYNIC.
pub struct JsonlAppend;

impl JsonlAppend {
    /// Append a line to a JSONL file.
    ///
    /// Creates parent directories if they don't exist. Creates the file if it
    /// doesn't exist. Appends if it does.
    ///
    /// # Arguments
    ///
    /// * `path` — Path to the JSONL file.
    /// * `line` — The line to append (should be valid JSON).
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(String)` on failure.
    pub fn append(path: &Path, line: &str) -> Result<(), String> {
        Self::append_internal(path, line, false)
    }

    /// Append a line to a JSONL file and fsync.
    ///
    /// Use this for durability-critical logs (e.g., proof logs, audit logs)
    /// where a crash must not lose the last entry.
    pub fn append_with_fsync(path: &Path, line: &str) -> Result<(), String> {
        Self::append_internal(path, line, true)
    }

    /// Internal implementation.
    fn append_internal(path: &Path, line: &str, fsync: bool) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create parent directory {}: {e}", parent.display()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("cannot open {}: {e}", path.display()))?;

        writeln!(file, "{line}").map_err(|e| format!("cannot write to {}: {e}", path.display()))?;

        if fsync {
            file.sync_all()
                .map_err(|e| format!("cannot fsync {}: {e}", path.display()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn append_creates_file_and_appends() {
        let path = Path::new("/tmp/test-jsonl-append.jsonl");
        let _ = fs::remove_file(path);

        JsonlAppend::append(path, r#"{"key":"value1"}"#).unwrap();
        JsonlAppend::append(path, r#"{"key":"value2"}"#).unwrap();

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains(r#""key":"value1""#));
        assert!(content.contains(r#""key":"value2""#));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn append_creates_parent_directories() {
        let path = Path::new("/tmp/test-jsonl-append/dir1/dir2/test.jsonl");
        let _ = fs::remove_dir_all("/tmp/test-jsonl-append");

        JsonlAppend::append(path, r#"{"key":"value"}"#).unwrap();

        assert!(path.exists());

        let _ = fs::remove_dir_all("/tmp/test-jsonl-append");
    }

    #[test]
    fn append_with_fsync_succeeds() {
        let path = Path::new("/tmp/test-jsonl-fsync.jsonl");
        let _ = fs::remove_file(path);

        JsonlAppend::append_with_fsync(path, r#"{"key":"value"}"#).unwrap();

        assert!(path.exists());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn append_to_invalid_path_fails() {
        let path = Path::new("/proc/invalid/path/test.jsonl");
        let result = JsonlAppend::append(path, r#"{"key":"value"}"#);
        assert!(result.is_err());
    }
}
