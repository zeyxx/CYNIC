use std::path::Path;
use std::process::Command;

use super::types::*;

pub fn execute_plan(root: &Path, plan: &SyncPlan) -> SyncProof {
    let mut steps_completed = Vec::new();
    let mut success = true;

    for step in &plan.steps {
        let result = execute_step(root, step);
        if !result.success {
            success = false;
            steps_completed.push(result);
            break;
        }
        steps_completed.push(result);
    }

    if !success {
        execute_rollback(root, &plan.rollback);
    }

    SyncProof {
        schema: "cynic.host.source_sync_proof.v1",
        plan: plan.clone(),
        executed_at_unix_s: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        steps_completed,
        success,
        residual_unknowns: Vec::new(),
    }
}

fn execute_step(root: &Path, step: &SyncStep) -> SyncStepResult {
    match step {
        SyncStep::GitCommand { argv, layer: _ } => {
            let output = Command::new(&argv[0])
                .args(&argv[1..])
                .current_dir(root)
                .output();

            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    SyncStepResult {
                        step: step.clone(),
                        success: o.status.success(),
                        output: Some(format!("{}\n{}", stdout, stderr)),
                        error: if o.status.success() {
                            None
                        } else {
                            Some(format!("exit code: {:?}", o.status.code()))
                        },
                    }
                }
                Err(e) => SyncStepResult {
                    step: step.clone(),
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                },
            }
        }
        SyncStep::WriteFile { path, content } => {
            let write_result = std::fs::write(path, content);
            SyncStepResult {
                step: step.clone(),
                success: write_result.is_ok(),
                output: None,
                error: write_result.err().map(|e| e.to_string()),
            }
        }
        SyncStep::Archive { branch, archive_ref } => {
            let output = Command::new("git")
                .args(["branch", archive_ref, branch])
                .current_dir(root)
                .output();
            match output {
                Ok(o) => SyncStepResult {
                    step: step.clone(),
                    success: o.status.success(),
                    output: Some(String::from_utf8_lossy(&o.stdout).to_string()),
                    error: if o.status.success() {
                        None
                    } else {
                        Some(String::from_utf8_lossy(&o.stderr).to_string())
                    },
                },
                Err(e) => SyncStepResult {
                    step: step.clone(),
                    success: false,
                    output: None,
                    error: Some(e.to_string()),
                },
            }
        }
    }
}

fn execute_rollback(root: &Path, rollback: &RollbackIntent) {
    match rollback {
        RollbackIntent::None => {}
        RollbackIntent::ResetCortex { to_sha } => {
            let _ = Command::new("git")
                .args(["reset", "--hard", to_sha])
                .current_dir(root)
                .output();
        }
        RollbackIntent::ResetForgejo { to_sha: _ } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_file_step_succeeds() {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push("cynic_test_sync");
        let _ = std::fs::create_dir_all(&temp_dir);

        let step = SyncStep::WriteFile {
            path: temp_dir.join("test.txt").to_string_lossy().to_string(),
            content: "hello".to_string(),
        };
        let result = execute_step(&temp_dir, &step);
        assert!(result.success);
    }
}
