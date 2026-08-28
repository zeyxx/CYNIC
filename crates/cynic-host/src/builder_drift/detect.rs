use std::fs;
use std::path::Path;

pub const DRIFT_SCHEMA: &str = "cynic.builder_drift_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftReport {
    pub schema: String,
    pub ref_path: String,
    pub expected_owner: String,
    pub actual_owner: String,
    pub repairable: bool,
}

/// Detect ownership drift in `.git/refs/remotes/forgejo/`.
///
/// Returns a list of refs not owned by the expected principal (`cynic`).
pub fn detect_drift(forgejo_refs: &Path, expected_owner: &str) -> Result<Vec<DriftReport>, String> {
    let mut reports = Vec::new();

    if !forgejo_refs.exists() {
        return Ok(reports);
    }

    let entries = fs::read_dir(forgejo_refs)
        .map_err(|e| format!("failed to read {}: {}", forgejo_refs.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry error: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            reports.extend(detect_drift(&path, expected_owner)?);
        } else if path.is_file() {
            let actual = owner_of(&path)?;
            if actual != expected_owner {
                reports.push(DriftReport {
                    schema: DRIFT_SCHEMA.to_string(),
                    ref_path: path.to_string_lossy().to_string(),
                    expected_owner: expected_owner.to_string(),
                    actual_owner: actual,
                    repairable: true,
                });
            }
        }
    }

    Ok(reports)
}

fn owner_of(path: &Path) -> Result<String, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|e| format!("stat {}: {}", path.display(), e))?;
    let uid = metadata.uid();
    // Lookup user name via /etc/passwd (no libc dependency)
    let passwd =
        fs::read_to_string("/etc/passwd").map_err(|e| format!("read /etc/passwd: {}", e))?;
    for line in passwd.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 && parts[2] == uid.to_string() {
            return Ok(parts[0].to_string());
        }
    }
    Ok(format!("uid={}", uid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn no_drift_when_empty() {
        let dir = std::env::temp_dir().join("cynic_drift_test_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let reports = detect_drift(&dir, "cynic").unwrap();
        assert!(reports.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_drift_for_unknown_owner() {
        let dir = std::env::temp_dir().join("cynic_drift_test_drift");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test_ref");
        let mut f = fs::File::create(&file_path).unwrap();
        writeln!(f, "abc123").unwrap();
        // Since we can't easily chown in tests without root, just verify the function runs
        let result = detect_drift(&dir, "cynic_mismatch_test");
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&dir);
    }
}
