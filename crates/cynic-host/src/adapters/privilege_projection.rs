use crate::ports::HostCommandResult;
use crate::runtime_layout::current_runtime_layout;

use super::local_command_runner::run_command;

pub(super) fn install_owned_file(
    source: &str,
    target: &str,
    user: &str,
    group: &str,
    mode: u32,
) -> HostCommandResult {
    let mode = format!("{mode:04o}");
    run_command(
        "install",
        &["-o", user, "-g", group, "-m", &mode, source, target],
    )
}

pub(super) fn validate_projection(provider: &str, path: &str) -> HostCommandResult {
    match provider {
        "sudoers" => run_command(&current_runtime_layout().visudo_bin(), &["-cf", path]),
        _ => unsupported_provider(provider, path),
    }
}

fn unsupported_provider(provider: &str, path: &str) -> HostCommandResult {
    HostCommandResult {
        command: format!("validate_privilege_projection provider={provider} path={path}"),
        success: false,
        exit_code: Some(2),
        stdout: String::new(),
        stderr: format!("unsupported privilege projection provider `{provider}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_projection;

    #[test]
    fn unknown_provider_fails_closed_without_shelling_out() {
        let result = validate_projection("unknown-provider", "/tmp/projection");

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(2));
        assert!(result.stderr.contains("unsupported privilege projection"));
    }
}
