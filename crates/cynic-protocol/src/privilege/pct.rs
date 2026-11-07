//! Shared admission for `pct` argv — the single source of truth.
//!
//! Moved here from `cynic-lxc` (and formerly duplicated by
//! `pct-bounded-exec.sh` and `proxmox_clone.rs`) so every producer of a
//! `/usr/sbin/pct` argv consults one validator. Pure: no I/O, no process.

/// Decide whether an inner argv (the command run via `pct exec <vmid> -- …`)
/// is in the LXC bridge catalog, and whether it mutates.
pub fn admitted_operation(argv: &[String]) -> Result<bool, String> {
    let Some(program) = argv.first().map(String::as_str) else {
        return Err("argv_empty".to_string());
    };
    if argv
        .iter()
        .any(|value| value.is_empty() || value.contains('\0'))
    {
        return Err("argv_invalid".to_string());
    }
    match program {
        "/bin/true" | "/usr/bin/true" if argv.len() == 1 => Ok(false),
        "/usr/bin/id" if argv.len() <= 2 && argv.iter().skip(1).all(|v| safe_name(v)) => {
            Ok(false)
        }
        "/usr/bin/getent"
            if argv.len() == 3
                && matches!(argv[1].as_str(), "passwd" | "group" | "hosts")
                && safe_name(&argv[2]) =>
        {
            Ok(false)
        }
        "/usr/bin/stat" if argv.len() == 2 && safe_absolute_path(&argv[1]) => Ok(false),
        "/usr/bin/test"
            if argv.len() == 3
                && matches!(argv[1].as_str(), "-e" | "-f" | "-d")
                && safe_absolute_path(&argv[2]) =>
        {
            Ok(false)
        }
        "/usr/bin/ss" if argv.len() == 2 && matches!(argv[1].as_str(), "-ltn" | "-ltnp") => {
            Ok(false)
        }
        "/usr/bin/systemctl" => admit_systemctl(argv),
        _ => Err("operation_not_in_lxc_bridge_catalog".to_string()),
    }
}

pub fn admit_systemctl(argv: &[String]) -> Result<bool, String> {
    if argv.len() < 3 || !safe_unit(&argv[2]) {
        return Err("systemctl_argv_invalid".to_string());
    }
    match argv[1].as_str() {
        "is-active" | "is-enabled" | "status" if argv.len() == 3 => Ok(false),
        "show"
            if argv.iter().skip(3).all(|value| {
                value == "--no-pager"
                    || value.strip_prefix("--property=").is_some_and(safe_property)
            }) =>
        {
            Ok(false)
        }
        "start" | "stop" | "restart" | "enable" | "disable" if argv.len() == 3 => Ok(true),
        _ => Err("systemctl_operation_not_admitted".to_string()),
    }
}

pub fn safe_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.contains("..")
        && !value.contains('\n')
        && !value.contains('\r')
}

pub fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

pub fn safe_unit(value: &str) -> bool {
    safe_name(value) && value.ends_with(".service")
}

pub fn safe_property(value: &str) -> bool {
    safe_name(value)
}

/// Shared alias used by callers that want a single admission entry point.
pub fn admit_pct_inner_argv(argv: &[String]) -> Result<bool, String> {
    admitted_operation(argv)
}

/// Validate an outer `pct` command shape. Accepts the known proxmox
/// operations produced by the adapters; rejects anything else.
pub fn admit_pct_command(argv: &[String]) -> Result<(), String> {
    // argv is expected to be the remote command vector starting at `pct`
    // or the full ssh-wrapped vector; find the pct token.
    let pct_index = argv.iter().position(|v| v == "/usr/bin/pct" || v == "pct");
    let Some(idx) = pct_index else {
        // Not a pct command; nothing to validate here.
        return Ok(());
    };
    let rest = &argv[idx + 1..];
    let Some(op) = rest.first().map(String::as_str) else {
        return Err("pct_operation_missing".to_string());
    };
    match op {
        "list" | "status" | "clone" | "destroy" | "template" | "resize" | "exec" => {}
        _ => return Err(format!("pct_operation_not_admitted:{op}")),
    }
    // For exec, validate the inner argv too.
    if op == "exec" {
        // pct exec <vmid> -- <inner...>
        if let Some(sep) = rest.iter().position(|v| v == "--") {
            let inner: Vec<String> = rest[sep + 1..].to_vec();
            if !inner.is_empty() {
                admitted_operation(&inner).map(|_| ())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admitted_operation_blocks_shell() {
        assert!(admitted_operation(&["/bin/sh".to_string(), "-c".to_string(), "id".to_string()])
            .is_err());
    }

    #[test]
    fn admit_pct_command_accepts_clone() {
        let argv = vec![
            "ssh".to_string(),
            "/usr/bin/pct".to_string(),
            "clone".to_string(),
            "105".to_string(),
            "900105".to_string(),
        ];
        assert!(admit_pct_command(&argv).is_ok());
    }
}
