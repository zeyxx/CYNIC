use crate::ports::{HostCommandResult, PrincipalLoginShell};
use crate::process_observation::capture_command;
use crate::runtime_layout::current_runtime_layout;

use super::local_command_runner::run_command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrincipalManagerKind {
    Shadow,
    BusyBox,
}

pub(super) fn ensure_system_group(group: &str) -> HostCommandResult {
    if identity_exists("group", group) {
        return idempotent_identity_result("group", group);
    }
    if !has_root_authority() {
        return missing_root_authority("group", group);
    }
    match detected_manager() {
        Some(PrincipalManagerKind::Shadow) => {
            run_command("groupadd", &["--system", "--force", group])
        }
        Some(PrincipalManagerKind::BusyBox) => run_command("addgroup", &["-S", group]),
        None => unsupported_principal_manager("group", group),
    }
}

pub(super) fn ensure_system_user(
    user: &str,
    group: &str,
    home: &str,
    shell: &PrincipalLoginShell,
) -> HostCommandResult {
    if identity_exists("passwd", user) {
        return idempotent_identity_result("user", user);
    }
    if !has_root_authority() {
        return missing_root_authority("user", user);
    }
    let shell = resolve_login_shell(shell);
    match detected_manager() {
        Some(PrincipalManagerKind::Shadow) => run_command(
            "useradd",
            &[
                "--system",
                "--home-dir",
                home,
                "--create-home",
                "--gid",
                group,
                "--shell",
                &shell,
                user,
            ],
        ),
        Some(PrincipalManagerKind::BusyBox) => run_command(
            "adduser",
            &["-S", "-D", "-h", home, "-G", group, "-s", &shell, user],
        ),
        None => unsupported_principal_manager("user", user),
    }
}

pub(super) fn ensure_owned_dir(
    path: &str,
    user: &str,
    group: &str,
    mode: u32,
) -> HostCommandResult {
    if !has_root_authority() {
        return missing_root_authority("owned_dir", path);
    }
    let mode = format!("{mode:04o}");
    run_command(
        "install",
        &["-d", "-o", user, "-g", group, "-m", &mode, path],
    )
}

fn detected_manager() -> Option<PrincipalManagerKind> {
    select_manager(
        command_available("groupadd"),
        command_available("useradd"),
        command_available("addgroup"),
        command_available("adduser"),
    )
}

fn has_root_authority() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn select_manager(
    groupadd: bool,
    useradd: bool,
    addgroup: bool,
    adduser: bool,
) -> Option<PrincipalManagerKind> {
    if groupadd && useradd {
        Some(PrincipalManagerKind::Shadow)
    } else if addgroup && adduser {
        Some(PrincipalManagerKind::BusyBox)
    } else {
        None
    }
}

fn identity_exists(database: &str, identity: &str) -> bool {
    capture_command(
        &current_runtime_layout().getent_bin(),
        &[database, identity],
    )
    .is_ok_and(|output| output.success)
}

fn command_available(command: &str) -> bool {
    capture_command(&current_runtime_layout().which_bin(), &[command])
        .is_ok_and(|output| output.success)
}

fn resolve_login_shell(shell: &PrincipalLoginShell) -> String {
    match shell {
        PrincipalLoginShell::NoLogin => ["/usr/sbin/nologin", "/sbin/nologin"]
            .into_iter()
            .find(|path| std::path::Path::new(path).is_file())
            .unwrap_or("/usr/sbin/nologin")
            .to_string(),
    }
}

fn idempotent_identity_result(kind: &str, identity: &str) -> HostCommandResult {
    HostCommandResult {
        command: format!("ensure_system_{kind} {identity}"),
        success: true,
        exit_code: Some(0),
        stdout: format!("{kind} already present"),
        stderr: String::new(),
    }
}

fn unsupported_principal_manager(kind: &str, identity: &str) -> HostCommandResult {
    HostCommandResult {
        command: format!("ensure_system_{kind} {identity}"),
        success: false,
        exit_code: Some(2),
        stdout: String::new(),
        stderr: "no supported principal manager capability detected".to_string(),
    }
}

fn missing_root_authority(kind: &str, target: &str) -> HostCommandResult {
    HostCommandResult {
        command: format!("ensure_system_{kind} {target}"),
        success: false,
        exit_code: Some(126),
        stdout: String::new(),
        stderr: "root authority is required; refusing user-namespace ownership as proof".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{PrincipalManagerKind, select_manager};

    #[test]
    fn capability_selection_supports_shadow_and_busybox_without_distro_names() {
        assert_eq!(
            select_manager(true, true, false, false),
            Some(PrincipalManagerKind::Shadow)
        );
        assert_eq!(
            select_manager(false, false, true, true),
            Some(PrincipalManagerKind::BusyBox)
        );
        assert_eq!(select_manager(true, false, true, false), None);
    }
}
