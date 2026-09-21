//! Principal install layout resolution (runtime user `cynic` by default).

use std::path::{Path, PathBuf};

use super::authority::{LocalPrincipalAuthority, PrincipalAuthorityPort};
use super::types::{InstallPrincipalKind, PrincipalInstallLayout, RunAs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePrincipalLayout {
    pub runtime_user: String,
    pub runtime_group: String,
    pub home_dir: PathBuf,
    pub state_root: PathBuf,
    pub log_root: PathBuf,
}

pub fn default_runtime_principal_layout() -> RuntimePrincipalLayout {
    RuntimePrincipalLayout {
        runtime_user: "cynic".to_string(),
        runtime_group: "cynic".to_string(),
        home_dir: PathBuf::from("/var/lib/cynic"),
        state_root: PathBuf::from("/var/lib/cynic"),
        log_root: PathBuf::from("/var/log/cynic"),
    }
}

/// Resolve the runtime principal install layout (user `cynic` by default).
///
/// Override:
/// - `CYNIC_RUNTIME_USER` — principal username
/// - `CYNIC_PRINCIPAL_HOME` — force home root
/// - `CYNIC_ALLOW_LOCAL_DEV_BUILD` — dev fallback to operator `$HOME`
pub fn resolve_principal_install_layout() -> PrincipalInstallLayout {
    let authority = LocalPrincipalAuthority;
    resolve_principal_install_layout_with_authority(&authority)
}

pub fn resolve_principal_install_layout_with_authority(
    authority: &dyn PrincipalAuthorityPort,
) -> PrincipalInstallLayout {
    let declared = default_runtime_principal_layout();
    let runtime_user =
        std::env::var("CYNIC_RUNTIME_USER").unwrap_or_else(|_| declared.runtime_user.clone());
    let allow_local_dev = std::env::var("CYNIC_ALLOW_LOCAL_DEV_BUILD").is_ok();

    let home_dir = if let Ok(forced) = std::env::var("CYNIC_PRINCIPAL_HOME") {
        PathBuf::from(forced)
    } else if allow_local_dev {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
        } else {
            declared.home_dir.clone()
        }
    } else {
        declared.home_dir.clone()
    };

    let bin_dir = home_dir.join(".local/bin");
    let share_dir = home_dir.join(".local/share/cynic");
    let src_dir = share_dir.join("src");

    let current = authority.current_username();
    let mut notes = Vec::new();
    notes.push(format!(
        "ecosystem packages install under principal user `{runtime_user}` layout"
    ));
    notes.push(format!("bin_dir={}", bin_dir.display()));
    notes.push(format!("src_dir={}", src_dir.display()));

    let (run_as, writable) = if current.as_deref() == Some(runtime_user.as_str()) {
        notes.push("run_as=current (already principal)".into());
        (RunAs::Current, true)
    } else if authority.can_run_as(&runtime_user) == Some(true) {
        notes.push(format!("run_as=sudo -u {runtime_user}"));
        (RunAs::SudoUser(runtime_user.clone()), true)
    } else if path_is_writable(&bin_dir) || path_is_writable(bin_dir.parent().unwrap_or(&bin_dir)) {
        notes.push(
            "principal sudo unavailable; using current user write access to principal paths".into(),
        );
        (RunAs::Current, true)
    } else {
        notes.push(format!(
            "blocked: cannot write as `{runtime_user}` — run as that user or enable passwordless sudo -u {runtime_user}"
        ));
        (RunAs::SudoUser(runtime_user.clone()), false)
    };

    let principal = if allow_local_dev && std::env::var("CYNIC_PRINCIPAL_HOME").is_err() {
        InstallPrincipalKind::OperatorLocal
    } else {
        InstallPrincipalKind::RuntimeUser
    };

    PrincipalInstallLayout {
        principal,
        runtime_user,
        home_dir,
        bin_dir,
        src_dir,
        share_dir,
        run_as,
        writable,
        notes,
    }
}

fn path_is_writable(path: &Path) -> bool {
    if path.is_dir() {
        let probe = path.join(".cynic-write-probe");
        match std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&probe)
        {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                true
            }
            Err(_) => false,
        }
    } else if let Some(parent) = path.parent() {
        if parent.exists() {
            path_is_writable(parent)
        } else {
            false
        }
    } else {
        false
    }
}
