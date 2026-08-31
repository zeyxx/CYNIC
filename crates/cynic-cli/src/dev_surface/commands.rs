use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct CheckArgs {
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) stop_on_first_failure: bool,
    #[arg(long)]
    pub(crate) architecture_only: bool,
    #[arg(long)]
    pub(crate) layered: bool,
    #[arg(long)]
    pub(crate) target_root: Option<PathBuf>,
}

#[derive(Args)]
pub(crate) struct BuildAdmissionArgs {
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Args)]
pub(crate) struct InferenceMatrixArgs {
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) model: PathBuf,
    #[arg(long)]
    pub(crate) params_billions: f32,
    #[arg(long)]
    pub(crate) context_filter: Option<String>,
    #[arg(long)]
    pub(crate) execute: bool,
    #[arg(long)]
    pub(crate) sample: Option<usize>,
    #[arg(long)]
    pub(crate) ledger: Option<PathBuf>,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
}

#[derive(Args)]
pub(crate) struct InferenceOptimalArgs {
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) model: String,
    #[arg(long)]
    pub(crate) context: Option<String>,
    #[arg(long)]
    pub(crate) goal: String,
    #[arg(long)]
    pub(crate) ledger: Option<PathBuf>,
    #[arg(long)]
    pub(crate) export_toml: Option<PathBuf>,
}

#[derive(Args)]
pub(crate) struct AuditArgs {
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long)]
    pub(crate) persist: bool,
}

#[derive(Args)]
pub(crate) struct OpportunitiesArgs {
    #[arg(long)]
    pub(crate) json: bool,
    #[arg(long)]
    pub(crate) frame: Option<String>,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
}
