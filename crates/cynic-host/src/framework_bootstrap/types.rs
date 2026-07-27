use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkBootstrapBundleReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub output_dir: String,
    pub files: Vec<String>,
    pub required_role: String,
    pub missing_artifacts: Vec<String>,
    pub blockers: Vec<String>,
    pub seed_id: String,
    pub seed_manifest: String,
    pub entrypoint: String,
    pub operator_command: String,
    pub maintenance_command: String,
    pub target_runtime: &'static str,
    pub security_invariants: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkSeedManifest {
    pub schema: &'static str,
    pub seed_id: String,
    pub status: &'static str,
    pub artifacts: Vec<FrameworkSeedArtifact>,
    pub enrollment: FrameworkSeedEnrollment,
    pub proof_contract: Vec<&'static str>,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkSeedArtifact {
    pub path: String,
    pub sha256: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkSeedEnrollment {
    pub required: bool,
    pub target_role: String,
    pub identity_class: &'static str,
    pub token_material: &'static str,
    pub admission_status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct StandaloneCliInstallReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub source: String,
    pub target: String,
    pub command: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub proof: Vec<&'static str>,
}
