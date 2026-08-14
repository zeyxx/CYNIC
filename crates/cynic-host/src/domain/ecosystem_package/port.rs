//! Universal provider port + argv apply engine.
//!
//! Adapters implement [`EcosystemPackageProvider`]. The engine only executes
//! planned argv steps (with principal wrap). No tool-specific recipes here.

use std::process::Command;

use super::types::{
    ApplyStep, EcosystemPackageApplyResult, EcosystemPackagePlan, InstallPrincipalKind,
    PrincipalInstallLayout, ProviderChannel, RunAs, StepExecution,
};

/// Universal interface every ecosystem dep provider satisfies.
pub trait EcosystemPackageProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn channel(&self) -> ProviderChannel;
    fn principal_kind(&self) -> InstallPrincipalKind;

    /// Build a portable plan (desired steps + proofs). Does not mutate.
    fn plan(&self, reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackagePlan;

    /// Apply via the shared argv engine unless the provider overrides.
    fn apply(&self, reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackageApplyResult {
        apply_package_plan(&self.plan(reason, layout))
    }
}

/// Execute a package plan step-by-step (argv only; principal wrap via sudo -u).
pub fn apply_package_plan(plan: &EcosystemPackagePlan) -> EcosystemPackageApplyResult {
    let requires_runtime_principal = plan.steps.iter().any(|step| step.as_principal)
        || matches!(plan.principal, InstallPrincipalKind::RuntimeUser);
    if requires_runtime_principal && !plan.layout.writable {
        return EcosystemPackageApplyResult {
            dep_id: plan.dep_id.clone(),
            attempted: false,
            ok: false,
            channel: plan.channel,
            principal: plan.principal,
            steps_run: vec![],
            message: format!(
                "principal `{}` not writable — run as that user or enable passwordless sudo -u {}",
                plan.layout.runtime_user, plan.layout.runtime_user
            ),
            proofs: plan.proofs.clone(),
        };
    }

    if !plan.auto_applicable {
        return EcosystemPackageApplyResult {
            dep_id: plan.dep_id.clone(),
            attempted: false,
            ok: false,
            channel: plan.channel,
            principal: plan.principal,
            steps_run: vec![],
            message: if !plan.layout.writable {
                format!(
                    "principal `{}` not writable — run as that user or enable sudo -u {}",
                    plan.layout.runtime_user, plan.layout.runtime_user
                )
            } else {
                "provider not auto_applicable".into()
            },
            proofs: plan.proofs.clone(),
        };
    }

    let mut steps_run = Vec::new();
    for step in &plan.steps {
        let argv = materialize_argv(step, &plan.layout);
        let mut cmd = Command::new(&argv[0]);
        if argv.len() > 1 {
            cmd.args(&argv[1..]);
        }
        if let Some(cwd) = &step.cwd {
            // cwd may not exist until clone — only set when present
            if cwd.is_dir() {
                cmd.current_dir(cwd);
            }
        }
        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                steps_run.push(StepExecution {
                    role: step.role.clone(),
                    argv: argv.clone(),
                    exit_code: None,
                    ok: false,
                    stdout_tail: String::new(),
                    stderr_tail: format!("spawn failed: {e}"),
                });
                return EcosystemPackageApplyResult {
                    dep_id: plan.dep_id.clone(),
                    attempted: true,
                    ok: false,
                    channel: plan.channel,
                    principal: plan.principal,
                    steps_run,
                    message: format!("failed at {}", step.role),
                    proofs: plan.proofs.clone(),
                };
            }
        };
        let ok = output.status.success();
        steps_run.push(StepExecution {
            role: step.role.clone(),
            argv: argv.clone(),
            exit_code: output.status.code(),
            ok,
            stdout_tail: tail(&String::from_utf8_lossy(&output.stdout), 1200),
            stderr_tail: tail(&String::from_utf8_lossy(&output.stderr), 1200),
        });
        if !ok {
            return EcosystemPackageApplyResult {
                dep_id: plan.dep_id.clone(),
                attempted: true,
                ok: false,
                channel: plan.channel,
                principal: plan.principal,
                steps_run,
                message: format!("step `{}` failed", step.role),
                proofs: plan.proofs.clone(),
            };
        }
    }

    EcosystemPackageApplyResult {
        dep_id: plan.dep_id.clone(),
        attempted: true,
        ok: true,
        channel: plan.channel,
        principal: plan.principal,
        steps_run,
        message: "upgraded".into(),
        proofs: plan.proofs.clone(),
    }
}

/// Materialize argv with principal wrap when required.
pub fn materialize_argv(step: &ApplyStep, layout: &PrincipalInstallLayout) -> Vec<String> {
    if !step.as_principal {
        return step.argv.clone();
    }
    match &layout.run_as {
        RunAs::Current => step.argv.clone(),
        RunAs::SudoUser(user) => {
            let mut out = vec!["sudo".into(), "-u".into(), user.clone(), "--".into()];
            out.extend(step.argv.iter().cloned());
            out
        }
    }
}

fn tail(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        t[t.len().saturating_sub(max)..].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_layout() -> PrincipalInstallLayout {
        PrincipalInstallLayout {
            principal: InstallPrincipalKind::RuntimeUser,
            runtime_user: "cynic".into(),
            home_dir: PathBuf::from("/var/lib/cynic"),
            bin_dir: PathBuf::from("/var/lib/cynic/.local/bin"),
            src_dir: PathBuf::from("/var/lib/cynic/.local/share/cynic/src"),
            share_dir: PathBuf::from("/var/lib/cynic/.local/share/cynic"),
            run_as: RunAs::SudoUser("cynic".into()),
            writable: true,
            notes: vec![],
        }
    }

    #[test]
    fn materialize_wraps_sudo_user() {
        let layout = test_layout();
        let step = ApplyStep {
            role: "t".into(),
            argv: vec!["true".into()],
            cwd: None,
            mutates: false,
            privilege_required: false,
            as_principal: true,
        };
        let argv = materialize_argv(&step, &layout);
        assert_eq!(argv, vec!["sudo", "-u", "cynic", "--", "true"]);
    }

    #[test]
    fn blocks_apply_when_principal_is_not_writable() {
        let mut layout = test_layout();
        layout.writable = false;
        let step = ApplyStep {
            role: "non_writable".into(),
            argv: vec!["true".into()],
            cwd: None,
            mutates: true,
            privilege_required: false,
            as_principal: true,
        };
        let plan = EcosystemPackagePlan {
            dep_id: "test".into(),
            channel: crate::domain::ecosystem_package::types::ProviderChannel::External,
            principal: layout.principal,
            layout: layout.clone(),
            reason: "test".into(),
            steps: vec![step],
            auto_applicable: true,
            proofs: vec![],
            summary_commands: vec!["test".into()],
        };

        let result = apply_package_plan(&plan);
        assert!(!result.ok);
        assert!(!result.attempted);
        assert_eq!(
            result.message,
            "principal `cynic` not writable — run as that user or enable passwordless sudo -u cynic"
        );
    }
}
