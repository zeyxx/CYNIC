use std::path::Path;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum BuilderCommand {
    /// Detect ownership drift in builder refs
    DriftCheck {
        #[arg(
            long,
            default_value = "/var/lib/cynic/source/repositories/CYNIC/.git/refs/remotes/forgejo/"
        )]
        refs_path: String,
        #[arg(long, default_value = "cynic")]
        expected_owner: String,
        #[arg(long)]
        json: bool,
    },
    /// Repair ownership of drifted builder refs
    RepairRefs {
        #[arg(
            long,
            default_value = "/var/lib/cynic/source/repositories/CYNIC/.git/refs/remotes/forgejo/"
        )]
        refs_path: String,
        #[arg(long, default_value = "cynic")]
        target_owner: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_builder_command(cmd: BuilderCommand) -> std::process::ExitCode {
    match cmd {
        BuilderCommand::DriftCheck {
            refs_path,
            expected_owner,
            json,
        } => {
            let path = Path::new(&refs_path);
            if !path.exists() {
                if json {
                    println!(
                        r#"{{"status":"ok","path":"{refs_path}","message":"path does not exist — no drift"}}"#
                    );
                } else {
                    println!("path does not exist: {refs_path} — no drift to detect");
                }
                return std::process::ExitCode::SUCCESS;
            }
            let reports = match cynic_host::builder_drift::detect::detect_drift(path, &expected_owner) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("drift detection error: {e}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            if json {
                let count = reports.len();
                let paths: Vec<String> = reports.iter().map(|r| r.ref_path.clone()).collect();
                println!(
                    r#"{{"status":"{}","count":{},"drifted_paths":[{}]}}"#,
                    if reports.is_empty() { "ok" } else { "drift_detected" },
                    count,
                    paths.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join(",")
                );
            } else if reports.is_empty() {
                println!("no drift detected — all refs owned by '{expected_owner}'");
            } else {
                println!("drift detected in {} refs:", reports.len());
                for r in &reports {
                    println!("  {}: expected={}, actual={}", r.ref_path, r.expected_owner, r.actual_owner);
                }
            }
            std::process::ExitCode::SUCCESS
        }
        BuilderCommand::RepairRefs {
            refs_path,
            target_owner,
            dry_run,
            json,
        } => {
            let path = Path::new(&refs_path);
            if !path.exists() {
                if json {
                    println!(r#"{{"status":"ok","repaired":0,"message":"path does not exist"}}"#);
                } else {
                    println!("path does not exist: {refs_path} — nothing to repair");
                }
                return std::process::ExitCode::SUCCESS;
            }
            let reports = match cynic_host::builder_drift::detect::detect_drift(path, &target_owner) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("drift detection error: {e}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            if reports.is_empty() {
                if json {
                    println!(r#"{{"status":"ok","repaired":0,"message":"no drift"}}"#);
                } else {
                    println!("no drift detected — nothing to repair");
                }
                return std::process::ExitCode::SUCCESS;
            }
            if dry_run {
                if json {
                    println!(r#"{{"status":"dry_run","drifted":{},"repaired":0}}"#, reports.len());
                } else {
                    println!("dry run: {} refs would be repaired (no changes made)", reports.len());
                }
                return std::process::ExitCode::SUCCESS;
            }
            let repair_reports = match cynic_host::builder_drift::repair::repair_drift(&reports) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("repair error: {e}");
                    return std::process::ExitCode::FAILURE;
                }
            };
            let repaired = repair_reports.iter().filter(|r| r.repaired).count();
            if json {
                println!(r#"{{"status":"repaired","repaired":{repaired},"total":{}}}"#, repair_reports.len());
            } else {
                println!("repaired {} of {} refs to '{}'", repaired, repair_reports.len(), target_owner);
                for r in &repair_reports {
                    if let Some(err) = &r.error {
                        eprintln!("  FAILED {}: {}", r.ref_path, err);
                    }
                }
            }
            std::process::ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_command_compiles() {
        let _ = BuilderCommand::RepairRefs {
            refs_path: "/tmp/does_not_exist".to_string(),
            target_owner: "cynic".to_string(),
            dry_run: true,
            json: false,
        };
    }

    #[test]
    fn drift_check_command_compiles() {
        let _ = BuilderCommand::DriftCheck {
            refs_path: "/tmp/does_not_exist".to_string(),
            expected_owner: "cynic".to_string(),
            json: true,
        };
    }
}
