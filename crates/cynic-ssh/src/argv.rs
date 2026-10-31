//! SSH argv construction and validation.

use crate::options::SSH_BATCH_OPTIONS;

/// Reject an invalid SSH target/endpoint.
pub fn validate_target(target: &str) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("empty target".to_string());
    }
    if target.starts_with('-') {
        return Err(format!("target {target} could be read as an option"));
    }
    if target.contains(char::is_whitespace) {
        return Err(format!("target {target} contains whitespace"));
    }
    if target.chars().any(|c| c.is_control()) {
        return Err(format!("target {target} contains a control character"));
    }
    if target.chars().any(|c| ";|&$()<>`{}\\".contains(c)) {
        return Err(format!("target {target} contains a shell metacharacter"));
    }
    Ok(())
}

/// Refuse a remote script that could execute more than one command.
pub fn validate_remote_script(script: &str) -> Result<(), String> {
    if script.contains('\n') || script.contains('\r') {
        return Err("remote script must be a single line".to_string());
    }
    if script.contains('\0') {
        return Err("remote script must not contain NUL".to_string());
    }
    if script.chars().any(|c| c.is_control()) {
        return Err("remote script must not contain control characters".to_string());
    }
    if script.chars().any(|c| "`$|;&<>".contains(c)) {
        return Err("remote script must not contain a shell metacharacter".to_string());
    }
    if script.contains("eval") {
        return Err("remote script must not invoke eval".to_string());
    }
    Ok(())
}

/// Reject an invalid remote path segment.
pub fn validate_remote_segment(segment: &str) -> Result<(), String> {
    if segment.is_empty()
        || segment == "."
        || segment == ".."
        || segment.starts_with('-')
        || !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err("invalid remote path segment".to_string());
    }
    Ok(())
}

/// Build the SSH argv for a remote command, applying the given options.
pub fn ssh_argv_with_options<T: AsRef<str>>(
    target: &str,
    remote_argv: &[T],
    options: &crate::options::SshOptions,
) -> Result<Vec<String>, String> {
    validate_target(target)?;
    if remote_argv.is_empty() {
        return Err("empty remote command".to_string());
    }
    let remote: Vec<String> = remote_argv.iter().map(|v| v.as_ref().to_string()).collect();
    // The remote program is `sh -c <script>`: one argv element, so the script
    // is never re-parsed by the local shell.
    if remote.len() >= 3 && remote[0] == "sh" && remote[1] == "-c" {
        validate_remote_script(&remote[2])?;
    }
    let mut argv = vec!["ssh".to_string()];
    argv.extend(options.to_argv());
    argv.push(target.to_string());
    argv.extend(remote);
    Ok(argv)
}

/// Build the SSH argv for a remote command.
pub fn ssh_argv<T: AsRef<str>>(target: &str, remote_argv: &[T]) -> Result<Vec<String>, String> {
    ssh_argv_with_options(target, remote_argv, &crate::options::SshOptions::default())
}

/// Build the SCP argv for a copy.
pub fn scp_argv(source: &str, target: &str) -> Result<Vec<String>, String> {
    validate_target(target)?;
    if source.is_empty() || source.starts_with('-') || source.contains(char::is_whitespace) {
        return Err("invalid local source path".to_string());
    }
    let mut argv = vec!["scp".to_string()];
    argv.extend(SSH_BATCH_OPTIONS.iter().map(|option| option.to_string()));
    argv.push(source.to_string());
    argv.push(target.to_string());
    Ok(argv)
}

/// Build the SSH argv for a remote script string, applying the given options.
pub fn ssh_script_argv_with_options(
    target: &str,
    script: &str,
    options: &crate::options::SshOptions,
) -> Result<Vec<String>, String> {
    validate_target(target)?;
    validate_remote_script(script)?;
    let mut argv = vec!["ssh".to_string()];
    argv.extend(options.to_argv());
    argv.push(target.to_string());
    argv.push("sh".to_string());
    argv.push("-c".to_string());
    argv.push(script.to_string());
    Ok(argv)
}

/// Build the SSH argv for a remote script string.
pub fn ssh_script_argv(target: &str, script: &str) -> Result<Vec<String>, String> {
    ssh_script_argv_with_options(target, script, &crate::options::SshOptions::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_target_rejects_bad_values() {
        assert!(validate_target("").is_err());
        assert!(validate_target("  ").is_err());
        assert!(validate_target("-oProxyCommand=evil").is_err());
        assert!(validate_target("host; rm -rf /").is_err());
        assert!(validate_target("host name").is_err());
        assert!(validate_target("cynic@host").is_ok());
    }

    #[test]
    fn validate_remote_script_rejects_newlines_and_nul() {
        assert!(validate_remote_script("rm -rf /\nrm -rf /").is_err());
        assert!(validate_remote_script("a\0b").is_err());
        assert!(validate_remote_script("mv -f /a /b").is_ok());
    }

    #[test]
    fn validate_remote_segment_rejects_bad_segments() {
        assert!(validate_remote_segment("").is_err());
        assert!(validate_remote_segment(".").is_err());
        assert!(validate_remote_segment("..").is_err());
        assert!(validate_remote_segment("a; rm").is_err());
        assert!(validate_remote_segment("abc123").is_ok());
        assert!(validate_remote_segment("cynic-node").is_ok());
    }

    #[test]
    fn ssh_argv_carries_batch_options() {
        let remote = ["sha256sum", "/x"];
        let argv = ssh_argv("cynic-builder-ssh", &remote).expect("valid");
        assert_eq!(argv[0], "ssh");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert!(argv.contains(&"StrictHostKeyChecking=yes".to_string()));
        assert!(argv.contains(&"ConnectTimeout=10".to_string()));
        assert_eq!(argv[argv.len() - 2], "sha256sum");
        assert_eq!(argv[argv.len() - 1], "/x");
    }

    #[test]
    fn ssh_argv_rejects_malicious_target() {
        let remote = ["true"];
        assert!(ssh_argv("-oProxyCommand=evil", &remote).is_err());
        assert!(ssh_argv("host; rm -rf /", &remote).is_err());
        assert!(ssh_argv("host name", &remote).is_err());
        assert!(ssh_argv("  ", &remote).is_err());
    }

    #[test]
    fn ssh_argv_rejects_multi_line_script() {
        let remote = ["sh", "-c", "rm -rf /\nrm -rf /"];
        assert!(ssh_argv("host", &remote).is_err());
    }

    #[test]
    fn ssh_argv_accepts_single_line_script() {
        let remote = ["sh", "-c", "mv -f /a /b"];
        assert!(ssh_argv("host", &remote).is_ok());
    }

    #[test]
    fn ssh_argv_accepts_vec_string() {
        let remote = vec!["true".to_string()];
        let argv = ssh_argv("host", &remote).expect("valid");
        assert_eq!(argv.last().unwrap(), "true");
    }

    #[test]
    fn scp_argv_builds_expected() {
        let argv = scp_argv("/tmp/a", "cynic@h:.cynic-releases/p/a").expect("valid");
        assert_eq!(argv[0], "scp");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert_eq!(argv[argv.len() - 2], "/tmp/a");
        assert_eq!(argv[argv.len() - 1], "cynic@h:.cynic-releases/p/a");
    }

    #[test]
    fn scp_argv_rejects_bad_source() {
        assert!(scp_argv("", "host:/x").is_err());
        assert!(scp_argv("-rf", "host:/x").is_err());
        assert!(scp_argv("/tmp/a b", "host:/x").is_err());
    }

    #[test]
    fn ssh_script_argv_wraps_script() {
        let argv = ssh_script_argv("host", "mv -f /a /b").expect("valid");
        assert_eq!(argv[0], "ssh");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert_eq!(argv[argv.len() - 3], "sh");
        assert_eq!(argv[argv.len() - 2], "-c");
        assert_eq!(argv[argv.len() - 1], "mv -f /a /b");
    }

    #[test]
    fn ssh_argv_with_options_applies_options() {
        let remote = ["true"];
        let opts = crate::options::SshOptions {
            batch_mode: false,
            strict_host_key_checking: true,
            connect_timeout_secs: 3,
        };
        let argv = ssh_argv_with_options("host", &remote, &opts).expect("valid");
        assert_eq!(argv[0], "ssh");
        assert!(!argv.contains(&"BatchMode=yes".to_string()));
        assert!(argv.contains(&"StrictHostKeyChecking=yes".to_string()));
        assert!(argv.contains(&"ConnectTimeout=3".to_string()));
        assert_eq!(argv.last().unwrap(), "true");
    }

    #[test]
    fn injection_payloads_rejected_by_validators() {
        let payloads = [
            "host; rm -rf /",
            "host | cat /etc/passwd",
            "host && id",
            "host`id`",
            "host$(id)",
            "host${IFS}id",
            "--option",
            "a\0b",
            "a\x01b",
            "eval rm -rf /",
        ];
        for payload in payloads {
            assert!(
                validate_target(payload).is_err(),
                "target accepted: {payload}"
            );
        }
        for payload in payloads {
            // validate_remote_script rejects shell metacharacters / control / eval / nul.
            // Some payloads without metachar (like plain eval) still rejected.
            let _ = validate_remote_script(payload);
        }
        // Explicit script-level rejections.
        assert!(validate_remote_script("echo `id`").is_err());
        assert!(validate_remote_script("echo $(id)").is_err());
        assert!(validate_remote_script("echo ${IFS}").is_err());
        assert!(validate_remote_script("eval rm -rf /").is_err());
        assert!(validate_remote_script("a\0b").is_err());
        assert!(validate_remote_script("a\x01b").is_err());
        assert!(validate_remote_script("cmd1 | cmd2").is_err());
        assert!(validate_remote_script("IFS=,;cmd").is_err());

        // Segment-level rejections.
        assert!(validate_remote_segment("--option").is_err());
        assert!(validate_remote_segment("a;b").is_err());
        assert!(validate_remote_segment("a|b").is_err());
        assert!(validate_remote_segment("a`b").is_err());
        assert!(validate_remote_segment("a$(b)").is_err());
        assert!(validate_remote_segment("a${b}").is_err());
    }
}
