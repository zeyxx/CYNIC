use std::path::Path;
use std::process::Command;

use super::types::*;
use crate::credential::CredentialResolver;

pub fn execute_plan(root: &Path, plan: &SyncPlan) -> SyncProof {
    let mut steps_completed = Vec::new();
    let mut success = true;

    for step in &plan.steps {
        let result = execute_step(root, step, None);
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

pub fn execute_plan_with_enrollment(
    root: &Path,
    plan: &SyncPlan,
    credential_ref: Option<&str>,
    secret_type: Option<crate::credential::CredentialType>,
) -> SyncProof {
    let resolver = crate::credential::SecretFileResolver::new(
        crate::credential::SecretFileResolver::default_path(),
    );

    let credential = if let Some(ref_path) = credential_ref {
        match resolver.resolve(ref_path) {
            Ok(cred) => Some(cred),
            Err(_) => {
                if let Some(s) = secret_type {
                    Some(resolver.prompt_and_save(ref_path, s).unwrap_or_else(|e| {
                        eprintln!("enrollment failed: {e}");
                        std::process::exit(2);
                    }))
                } else {
                    eprintln!("credential not found: {ref_path}");
                    std::process::exit(2);
                }
            }
        }
    } else {
        None
    };

    let mut steps_completed = Vec::new();
    let mut success = true;

    for step in &plan.steps {
        let result = execute_step(root, step, credential.as_ref());
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

fn execute_step(
    root: &Path,
    step: &SyncStep,
    credential: Option<&crate::credential::Credential>,
) -> SyncStepResult {
    match step {
        SyncStep::GitCommand { argv, layer: _ } => {
            let mut cmd = Command::new(&argv[0]);
            cmd.args(&argv[1..]).current_dir(root);

            if let Some(cred) = credential {
                match cred {
                    crate::credential::Credential::SshKey(key_path) => {
                        cmd.env(
                            "GIT_SSH_COMMAND",
                            format!(
                                "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes",
                                key_path.display()
                            ),
                        );
                    }
                    crate::credential::Credential::Token(token_path) => {
                        cmd.env("GIT_ASKPASS", token_path.display().to_string());
                    }
                }
            }

            let output = cmd.output();

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
        SyncStep::Archive {
            branch,
            archive_ref,
        } => {
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
        temp_dir.push("cynic_test_sync_3");
        let _ = std::fs::create_dir_all(&temp_dir);

        let step = SyncStep::WriteFile {
            path: temp_dir.join("test.txt").to_string_lossy().to_string(),
            content: "hello".to_string(),
        };
        let result = execute_step(&temp_dir, &step, None);
        assert!(result.success);
    }
}
