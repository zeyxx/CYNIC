use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SourceSnapshotPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub repository_root: String,
    pub source_commit: String,
    pub output_dir: String,
    pub format: &'static str,
    pub planned_argv: Vec<Vec<String>>,
    pub blockers: Vec<String>,
    pub stop_condition: &'static str,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSnapshotReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub source_commit: String,
    pub snapshot_digest: Option<String>,
    pub snapshot_format: &'static str,
    pub snapshot_path: Option<String>,
    pub commit_read_back: Option<String>,
    pub executed_argv: Vec<Vec<String>>,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}
