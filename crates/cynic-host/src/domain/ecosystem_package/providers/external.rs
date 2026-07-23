//! External-channel providers — vendor self-update CLIs (hf, npm, rustup, …).

use crate::domain::ecosystem_package::port::EcosystemPackageProvider;
use crate::domain::ecosystem_package::types::{
    ApplyStep, EcosystemPackagePlan, InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel,
};

/// Static external provider (argv owned as &'static).
pub struct ExternalCliProvider {
    pub id: &'static str,
    pub argv: &'static [&'static str],
    pub as_principal: bool,
}

impl EcosystemPackageProvider for ExternalCliProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn channel(&self) -> ProviderChannel {
        ProviderChannel::External
    }

    fn principal_kind(&self) -> InstallPrincipalKind {
        if self.as_principal {
            InstallPrincipalKind::RuntimeUser
        } else {
            InstallPrincipalKind::OperatorLocal
        }
    }

    fn plan(&self, reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackagePlan {
        plan_external(
            self.id,
            reason,
            self.argv.iter().map(|s| (*s).to_string()).collect(),
            layout,
            self.as_principal,
        )
    }
}

/// External-channel plan: single vendor argv (hf, npm, claude, …).
pub fn plan_external(
    dep_id: &str,
    reason: &str,
    argv: Vec<String>,
    layout: &PrincipalInstallLayout,
    as_principal: bool,
) -> EcosystemPackagePlan {
    let steps = vec![ApplyStep {
        role: format!("upgrade_{dep_id}"),
        argv: argv.clone(),
        cwd: None,
        mutates: true,
        privilege_required: false,
        as_principal,
    }];
    let mut summary = vec![argv.join(" ")];
    if as_principal && !layout.writable {
        summary.push(format!(
            "# blocked until writable as {}: sudo -u {} true",
            layout.runtime_user, layout.runtime_user
        ));
    }
    EcosystemPackagePlan {
        dep_id: dep_id.into(),
        channel: ProviderChannel::External,
        principal: if as_principal {
            layout.principal
        } else {
            InstallPrincipalKind::OperatorLocal
        },
        layout: layout.clone(),
        reason: reason.to_string(),
        steps,
        auto_applicable: !as_principal || layout.writable,
        proofs: vec![format!("re-probe {dep_id} via cynic deps")],
        summary_commands: summary,
    }
}

pub static HUGGINGFACE_CLI: ExternalCliProvider = ExternalCliProvider {
    id: "huggingface-cli",
    argv: &["hf", "update"],
    as_principal: true,
};

pub static UV: ExternalCliProvider = ExternalCliProvider {
    id: "uv",
    argv: &["uv", "self", "update"],
    as_principal: false,
};

pub static HERMES_AGENT: ExternalCliProvider = ExternalCliProvider {
    id: "hermes-agent",
    argv: &["uv", "tool", "install", "--force", "hermes-agent"],
    as_principal: true,
};

pub static CODEX_CLI: ExternalCliProvider = ExternalCliProvider {
    id: "codex-cli",
    argv: &["npm", "i", "-g", "@openai/codex@latest"],
    as_principal: false,
};

pub static CLAUDE_CODE: ExternalCliProvider = ExternalCliProvider {
    id: "claude-code",
    argv: &["claude", "update"],
    as_principal: false,
};

pub static RUSTC: ExternalCliProvider = ExternalCliProvider {
    id: "rustc",
    argv: &["rustup", "update"],
    as_principal: false,
};

pub static CARGO: ExternalCliProvider = ExternalCliProvider {
    id: "cargo",
    argv: &["rustup", "update"],
    as_principal: false,
};
