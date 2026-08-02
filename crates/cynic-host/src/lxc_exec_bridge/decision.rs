use super::types::{LxcBridgeDecision, LxcBridgeRequest, REQUEST_SCHEMA};

pub fn admit(request: &LxcBridgeRequest, pct_bin: &str) -> LxcBridgeDecision {
    let mut blockers = Vec::new();
    if request.schema != REQUEST_SCHEMA {
        blockers.push("request_schema_mismatch".to_string());
    }
    if !(100..=999_999_999).contains(&request.vmid) {
        blockers.push("vmid_out_of_range".to_string());
    }
    if request.purpose.trim().is_empty() || request.purpose.len() > 256 {
        blockers.push("purpose_invalid".to_string());
    }
    if request.agent_id.trim().is_empty() || request.agent_id.len() > 256 {
        blockers.push("agent_id_invalid".to_string());
    }
    if !(1..=60).contains(&request.timeout_seconds) {
        blockers.push("timeout_out_of_range".to_string());
    }
    if !safe_absolute_path(pct_bin) {
        blockers.push("pct_adapter_path_invalid".to_string());
    }

    let mutates = match admitted_operation(&request.argv) {
        Ok(value) => value,
        Err(blocker) => {
            blockers.push(blocker);
            false
        }
    };
    if mutates && !request.apply {
        blockers.push("mutation_requires_apply".to_string());
    }
    if !blockers.is_empty() {
        return LxcBridgeDecision::Blocked { blockers };
    }

    let mut pct_argv = vec![
        pct_bin.to_string(),
        "exec".to_string(),
        request.vmid.to_string(),
        "--".to_string(),
    ];
    pct_argv.extend(request.argv.clone());
    if mutates {
        LxcBridgeDecision::Apply { pct_argv }
    } else {
        LxcBridgeDecision::Observe { pct_argv }
    }
}

fn admitted_operation(argv: &[String]) -> Result<bool, String> {
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
        "/usr/bin/id" if argv.len() <= 2 && argv.iter().skip(1).all(|v| safe_name(v)) => Ok(false),
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

fn admit_systemctl(argv: &[String]) -> Result<bool, String> {
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

fn safe_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.contains("..")
        && !value.contains('\n')
        && !value.contains('\r')
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn safe_unit(value: &str) -> bool {
    safe_name(value) && value.ends_with(".service")
}

fn safe_property(value: &str) -> bool {
    safe_name(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lxc_exec_bridge::{LxcBridgeAuthority, LxcBridgeRequest};

    fn request(argv: &[&str], apply: bool) -> LxcBridgeRequest {
        LxcBridgeRequest {
            schema: REQUEST_SCHEMA.to_string(),
            vmid: 103,
            purpose: "storage health".to_string(),
            agent_id: "agent-a".to_string(),
            authority: LxcBridgeAuthority::RuntimeOperator,
            argv: argv.iter().map(|value| value.to_string()).collect(),
            timeout_seconds: 10,
            apply,
        }
    }

    #[test]
    fn read_only_service_status_is_admitted_without_apply() {
        assert!(matches!(
            admit(
                &request(
                    &["/usr/bin/systemctl", "is-active", "surrealdb.service"],
                    false
                ),
                "/usr/sbin/pct"
            ),
            LxcBridgeDecision::Observe { .. }
        ));
    }

    #[test]
    fn mutation_requires_apply_and_shells_are_never_admitted() {
        assert!(matches!(
            admit(
                &request(
                    &["/usr/bin/systemctl", "restart", "surrealdb.service"],
                    false
                ),
                "/usr/sbin/pct"
            ),
            LxcBridgeDecision::Blocked { .. }
        ));
        assert!(matches!(
            admit(&request(&["/bin/sh", "-c", "id"], true), "/usr/sbin/pct"),
            LxcBridgeDecision::Blocked { .. }
        ));
    }
}
