//! llama.cpp provider — principal-local git + cmake → llama-server.

use std::path::Path;

use crate::domain::ecosystem_package::port::EcosystemPackageProvider;
use crate::domain::ecosystem_package::probe::which;
use crate::domain::ecosystem_package::types::{
    ApplyStep, EcosystemPackagePlan, InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel,
};

pub struct LlamaCppProvider;

impl EcosystemPackageProvider for LlamaCppProvider {
    fn id(&self) -> &'static str {
        "llama.cpp"
    }

    fn channel(&self) -> ProviderChannel {
        ProviderChannel::Scripted
    }

    fn principal_kind(&self) -> InstallPrincipalKind {
        InstallPrincipalKind::RuntimeUser
    }

    fn plan(&self, reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackagePlan {
        plan_llama_cpp(reason, layout)
    }
}

/// Build a principal-local plan for llama.cpp (git + cmake → {home_dir}/.local/bin/llama-server).
pub fn plan_llama_cpp(reason: &str, layout: &PrincipalInstallLayout) -> EcosystemPackagePlan {
    let src = layout.src_dir.join("llama.cpp");
    let bin = layout.bin_dir.join("llama-server");
    let build = src.join("build");
    let built = build.join("bin/llama-server");

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
            role: "git_pull".into(),
            argv: vec![
                "git".into(),
                "-C".into(),
                src.display().to_string(),
                "pull".into(),
                "--ff-only".into(),
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
                "--depth".into(),
                "1".into(),
                "https://github.com/ggml-org/llama.cpp".into(),
                src.display().to_string(),
            ],
            cwd: None,
            mutates: true,
            privilege_required: false,
            as_principal: true,
        });
    }

    let mut cmake_cfg = vec![
        "cmake".into(),
        "-S".into(),
        src.display().to_string(),
        "-B".into(),
        build.display().to_string(),
        "-DCMAKE_BUILD_TYPE=Release".into(),
        "-DLLAMA_BUILD_SERVER=ON".into(),
        "-DGGML_NATIVE=ON".into(),
    ];
    if Path::new("/usr/include/vulkan/vulkan.h").exists() || Path::new("/usr/share/vulkan").exists()
    {
        cmake_cfg.push("-DGGML_VULKAN=ON".into());
    }
    steps.push(ApplyStep {
        role: "cmake_configure".into(),
        argv: cmake_cfg,
        cwd: None,
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    let jobs = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".into());
    steps.push(ApplyStep {
        role: "cmake_build".into(),
        argv: vec![
            "cmake".into(),
            "--build".into(),
            build.display().to_string(),
            "--target".into(),
            "llama-server".into(),
            "-j".into(),
            jobs,
        ],
        cwd: None,
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    steps.push(ApplyStep {
        role: "install_bin".into(),
        argv: vec![
            "install".into(),
            "-m".into(),
            "0755".into(),
            built.display().to_string(),
            bin.display().to_string(),
        ],
        cwd: None,
        mutates: true,
        privilege_required: false,
        as_principal: true,
    });

    let auto = layout.writable && which("cmake").is_some() && which("git").is_some();
    let mut summary = vec![
        format!(
            "# principal={} bin={}",
            layout.runtime_user,
            layout.bin_dir.display()
        ),
        "cynic deps upgrade --yes --only llama.cpp".into(),
    ];
    if !layout.writable {
        summary.push(format!(
            "# blocked until writable as {}: sudo -u {} true",
            layout.runtime_user, layout.runtime_user
        ));
    }

    EcosystemPackagePlan {
        dep_id: "llama.cpp".into(),
        channel: ProviderChannel::Scripted,
        principal: layout.principal,
        layout: layout.clone(),
        reason: reason.to_string(),
        steps,
        auto_applicable: auto,
        proofs: vec![format!("{} --version", bin.display()), "cynic deps".into()],
        summary_commands: summary,
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
    fn llama_plan_is_scripted_under_principal() {
        let plan = plan_llama_cpp("test", &test_layout());
        assert_eq!(plan.channel, ProviderChannel::Scripted);
        assert_eq!(plan.principal, InstallPrincipalKind::RuntimeUser);
        assert!(plan.auto_applicable);
        assert!(
            plan.steps
                .iter()
                .any(|s| s.role == "cmake_build" && s.as_principal)
        );
        assert!(
            plan.steps
                .iter()
                .any(|s| s.argv.iter().any(|a| a.contains("llama-server")))
        );
    }
}
