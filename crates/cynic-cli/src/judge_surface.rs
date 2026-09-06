use crate::cli_json::emit_pretty_json;
use clap::Subcommand;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Subcommand)]
pub(crate) enum JudgeCommand {
    /// Judge an extracted patch (diff vs origin/main) and emit CynicJudgment.
    Patch {
        #[arg(long)]
        branch: String,
        #[arg(long, default_value = ".")]
        root: String,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_judge(command: JudgeCommand) -> ExitCode {
    match command {
        JudgeCommand::Patch { branch, root, json } => {
            let obs = cynic_host::judge_patch_observer::observe_patch(Path::new(&root), &branch);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let input = cynic_protocol::judge_patch::PatchJudgmentInput {
                observation: obs.clone(),
                judged_at_unix_s: now,
            };
            let judgment = cynic_protocol::judge_patch::judge_patch(&input);
            if json {
                #[derive(serde::Serialize)]
                struct Out<'a> {
                    observation: &'a cynic_protocol::judge_patch::PatchObservation,
                    judgment: &'a cynic_protocol::judgment::CynicJudgment,
                }
                emit_pretty_json(&Out {
                    observation: &obs,
                    judgment: &judgment,
                })
            } else {
                println!(
                    "judge patch {}: actionable={} confidence={:.2} domain={:?} unknowns={:?}",
                    branch,
                    judgment.actionable,
                    judgment.confidence,
                    judgment.cynefin_domain,
                    judgment.residual_unknowns
                );
                println!(
                    "  diff {} bytes hash {}..",
                    obs.diff_bytes_len,
                    &obs.diff_hash[..8.min(obs.diff_hash.len())]
                );
                ExitCode::SUCCESS
            }
        }
    }
}
