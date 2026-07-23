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

    // Dream Server / ODS layout (v2.5+): repo-root install.sh → dream-server/install.sh.
    // Legacy nested ods/install.sh kept as fallback for older pins.
    let install_sh = if src.join("install.sh").exists() {
        src.join("install.sh")
    } else if src.join("dream-server/install.sh").exists() {
        src.join("dream-server/install.sh")
    } else if src.join("ods/install.sh").exists() {
        src.join("ods/install.sh")
    } else {
        // Pre-clone: prefer root entrypoint documented by upstream.
        src.join("install.sh")
    };
    let install_cwd = install_sh
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| src.clone());
    // Non-interactive full stack: installer owns docker compose + optional services.
    // Flags are product-side; CYNIC only invokes — no ODS logic reimplementation.
    let mut install_argv = vec![
        "bash".into(),
        install_sh.display().to_string(),
        "--non-interactive".into(),
        "--skip-docker".into(), // host docker already present (CYNIC ops host)
        "--all".into(),
    ];
    if let Ok(extra) = std::env::var("CYNIC_ODS_INSTALL_FLAGS") {
        for tok in extra.split_whitespace() {
            if !tok.is_empty() {
                install_argv.push(tok.to_string());
            }
        }
    }
    steps.push(ApplyStep {
        role: "ods_install".into(),
        argv: install_argv,
        cwd: Some(install_cwd),
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    // Symlink CLI into principal bin when product exposes dream / ods / dream-server.
    steps.push(ApplyStep {
        role: "ods_link_cli".into(),
        argv: vec![
            "bash".into(),
            "-c".into(),
            format!(
                r#"set -euo pipefail
BIN="{bin}"
SRC="{src}"
PRINCIPAL_HOME="{home}"
mkdir -p "$BIN"
for name in dream ods dream-server; do
  for cand in \
    "$SRC/dream-server/bin/$name" \
    "$SRC/dream-server/cli/$name" \
    "$SRC/bin/$name" \
    "$PRINCIPAL_HOME/.local/bin/$name" \
    "/usr/local/bin/$name"; do
    if [ -x "$cand" ]; then
      ln -sfn "$cand" "$BIN/$name"
      break
    fi
  done
done
# always leave a discoverable shim that re-enters install dir
if [ ! -e "$BIN/ods" ]; then
  cat > "$BIN/ods" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
ROOT="${{CYNIC_ODS_ROOT:-$PRINCIPAL_HOME/.local/share/cynic/src/ODS}}"
if [ -x "$ROOT/dream-server/install.sh" ]; then
  exec bash "$ROOT/dream-server/install.sh" "$@"
fi
if [ -x "$ROOT/install.sh" ]; then
  exec bash "$ROOT/install.sh" "$@"
fi
echo "ods shim: no install entry at $ROOT" >&2
exit 127
SH
  chmod +x "$BIN/ods"
fi
command -v docker >/dev/null
"#,
                bin = layout.bin_dir.display(),
                src = src.display(),
                home = layout.home_dir.display(),
            ),
        ],
        cwd: None,
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
            format!(
                "test -x {}/ods || command -v ods || test -f {}/install.sh",
                layout.bin_dir.display(),
                src.display()
            ),
            "cynic runtime inference-discover --json".into(),
            "cynic runtime inference-discover --json --apply".into(),
        ],
        summary_commands: vec![
            format!("# principal={} pin={}", layout.runtime_user, pin_tag),
            "cynic deps upgrade --yes --only ods".into(),
            "cynic runtime inference-discover --json".into(),
            "cynic runtime inference-discover --json --apply".into(),
            "cynic chat --ods".into(),
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
