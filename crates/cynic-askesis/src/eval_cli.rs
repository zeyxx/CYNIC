use std::path::{Path, PathBuf};

use clap::Args;
use cynic_askesis::eval::dataset::load_jsonl_dataset;
use cynic_askesis::eval::openai::OpenAiCompletionClient;
use cynic_askesis::eval::runner::run_agent_eval;

#[derive(Debug, Args)]
pub(crate) struct EvalArgs {
    /// Versioned JSONL dataset
    #[arg(value_name = "DATASET")]
    dataset: PathBuf,
    /// OpenAI-compatible base URL, including /v1
    #[arg(long)]
    endpoint: String,
    /// Model identifier sent to the endpoint
    #[arg(long)]
    model: String,
    /// Optional API-key file; secret values are never reported
    #[arg(long)]
    api_key_file: Option<PathBuf>,
    /// Aggregate score required for a green report
    #[arg(long, default_value_t = 80.0)]
    target_percent: f64,
    /// Repetitions per case for empirical variance
    #[arg(long, default_value_t = 3)]
    repetitions: u32,
    /// Seed increment between repetitions
    #[arg(long, default_value_t = 1)]
    seed_stride: i64,
    /// Optional machine-readable proof output path
    #[arg(long)]
    output: Option<PathBuf>,
    /// Print the complete machine-readable report
    #[arg(long)]
    json: bool,
}

pub(crate) async fn run(args: EvalArgs) -> Result<(), Box<dyn std::error::Error>> {
    let dataset = load_jsonl_dataset(&args.dataset)?;
    let api_key = resolve_api_key(args.api_key_file.as_deref())?;
    let client = OpenAiCompletionClient::new(args.endpoint, args.model, api_key)?;
    let report = run_agent_eval(
        &dataset,
        &client,
        args.target_percent,
        args.repetitions,
        args.seed_stride,
    )
    .await?;
    let proof = serde_json::to_string_pretty(&report)?;
    if let Some(path) = args.output {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &proof)?;
    }
    if args.json {
        println!("{proof}");
    } else {
        println!(
            "agent-eval: model={} score={:?}% target={}% passed={}",
            report.model, report.score_percent, report.target_percent, report.passed
        );
    }
    Ok(())
}

fn resolve_api_key(path: Option<&Path>) -> Result<Option<String>, std::io::Error> {
    if let Ok(value) = std::env::var("CYNIC_CHAT_API_KEY")
        && !value.trim().is_empty()
    {
        return Ok(Some(value.trim().to_string()));
    }
    let path = path.map(PathBuf::from).or_else(|| {
        std::env::var("CYNIC_CHAT_API_KEY_FILE")
            .ok()
            .map(PathBuf::from)
    });
    path.map(std::fs::read_to_string)
        .transpose()
        .map(|value| value.map(|secret| secret.trim().to_string()))
}
