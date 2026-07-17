//! Osmantic ODS provider — principal-local pin checkout + install.sh.

use crate::domain::ecosystem_package::port::EcosystemPackageProvider;
use crate::domain::ecosystem_package::probe::which;
use crate::domain::ecosystem_package::types::{
    ApplyStep, EcosystemPackagePlan, InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel,
};

pub struct OdsProvider;

impl EcosystemPackageProvider for OdsProvider {
    fn id(&self) -> &'static str {
        "ods"
    }

    fn channel(&self) -> ProviderChannel {
        ProviderChannel::Scripted
    }

    fn principal_kind(&self) -> InstallPrincipalKind {
        InstallPrincipalKind::RuntimeUser
    }

    fn plan(&self, reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackagePlan {
        let tag = std::env::var("CYNIC_ODS_PIN").unwrap_or_else(|_| "v2.5.3".into());
        plan_ods(reason, layout, &tag)
    }
}

/// Build a principal-local plan for Osmantic ODS (clone pin + install.sh).
pub fn plan_ods(
    reason: &str,
    layout: &PrincipalInstallLayout,
    pin_tag: &str,
) -> EcosystemPackagePlan {
    let src = layout.src_dir.join("ODS");
    let mut steps = Vec::new();
    steps.push(ApplyStep {
        role: "ensure_layout".into(),
        argv: vec![
            "mkdir".into(),
            "-p".into(),
            layout.bin_dir.display().to_string(),
            layout.src_dir.display().to_string(),
        ],
        cwd: None,
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    if src.join(".git").is_dir() {
        steps.push(ApplyStep {
            role: "git_fetch".into(),
            argv: vec![
                "git".into(),
                "-C".into(),
                src.display().to_string(),
                "fetch".into(),
                "--tags".into(),
                "--force".into(),
            ],
            cwd: None,
            mutates: true,
            privilege_required: false,
            as_principal: true,
        });
    } else {
        steps.push(ApplyStep {
            role: "git_clone".into(),
            argv: vec![
                "git".into(),
                "clone".into(),
                "https://github.com/Osmantic/ODS.git".into(),
                src.display().to_string(),
            ],
            cwd: None,
            mutates: true,
            privilege_required: false,
            as_principal: true,
        });
    }

    steps.push(ApplyStep {
        role: "git_checkout_pin".into(),
        argv: vec![
            "git".into(),
            "-C".into(),
            src.display().to_string(),
            "checkout".into(),
            pin_tag.to_string(),
        ],
        cwd: None,
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    // Official path: ODS/ods/install.sh (or repo-root install.sh). Prefer nested.
    let install_sh = if src.join("ods/install.sh").exists() || !src.exists() {
        src.join("ods/install.sh")
    } else {
        src.join("install.sh")
    };
    steps.push(ApplyStep {
        role: "ods_install".into(),
        argv: vec!["bash".into(), install_sh.display().to_string()],
        cwd: Some(src.join("ods")),
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    let auto = layout.writable && which("git").is_some() && which("bash").is_some();
    EcosystemPackagePlan {
        dep_id: "ods".into(),
        channel: ProviderChannel::Scripted,
        principal: layout.principal,
        layout: layout.clone(),
        reason: reason.to_string(),
        steps,
        auto_applicable: auto,
        proofs: vec![
            format!("test -x {}/ods || command -v ods", layout.bin_dir.display()),
            "cynic runtime inference-discover --json".into(),
        ],
        summary_commands: vec![
            format!("# principal={} pin={}", layout.runtime_user, pin_tag),
            "cynic deps upgrade --yes --only ods".into(),
            "cynic runtime inference-discover --json".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ecosystem_package::types::RunAs;
    use std::path::PathBuf;

    fn test_layout() -> PrincipalInstallLayout {
        PrincipalInstallLayout {
            principal: InstallPrincipalKind::RuntimeUser,
            runtime_user: "cynic".into(),
            home_dir: PathBuf::from("/home/cynic"),
            bin_dir: PathBuf::from("/home/cynic/.local/bin"),
            src_dir: PathBuf::from("/home/cynic/.local/share/cynic/src"),
            share_dir: PathBuf::from("/home/cynic/.local/share/cynic"),
            run_as: RunAs::SudoUser("cynic".into()),
            writable: true,
            notes: vec![],
        }
    }

    #[test]
    fn ods_plan_checkouts_pin() {
        let plan = plan_ods("missing", &test_layout(), "v2.5.3");
        assert_eq!(plan.channel, ProviderChannel::Scripted);
        let checkout = plan
            .steps
            .iter()
            .find(|s| s.role == "git_checkout_pin")
            .expect("checkout step");
        assert!(checkout.argv.iter().any(|a| a == "v2.5.3"));
    }
}
