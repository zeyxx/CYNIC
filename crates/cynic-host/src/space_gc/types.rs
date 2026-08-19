use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GcScope {
    Repo,
    Devops,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcTarget {
    pub path: String,
    pub bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceGcObservation {
    pub schema: &'static str,
    pub scope: GcScope,
    pub total_bytes: u64,
    pub file_count: u64,
    pub targets: Vec<GcTarget>,
    pub timestamp_unix_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcSummary {
    pub scope: GcScope,
    pub total_bytes: u64,
    pub file_count: u64,
    pub reclaimable_bytes: u64,
    pub targets: Vec<GcTarget>,
}

impl GcSummary {
    pub fn human_readable(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
