use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One parsed Host entry from `~/.ssh/config`.
#[derive(Debug, Clone)]
pub struct SshConfigEntry {
    pub host_pattern: String,
    pub host_name: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<String>,
    pub proxy_jump: Option<String>,
    pub identities_only: Option<bool>,
    pub strict_host_key_checking: Option<String>,
    pub batch_mode: Option<bool>,
    pub connect_timeout: Option<u32>,
    /// All other fields not parsed into named fields.
    pub extra: BTreeMap<String, String>,
}

/// Pure parser result for `~/.ssh/config` style files.
#[derive(Debug, Clone, Default)]
pub struct SshConfigParse {
    pub entries: Vec<SshConfigEntry>,
    pub parse_errors: Vec<String>,
}

/// Parse an `~/.ssh/config`-style file into structured entries.
///
/// Pure function — no I/O, no path resolution. Tests can feed it any
/// string. The caller owns reading the file.
pub fn parse_ssh_config(content: &str) -> SshConfigParse {
    let mut result = SshConfigParse::default();
    let mut current: Option<SshConfigEntry> = None;

    for (line_no, raw_line) in content.lines().enumerate() {
        let line_no = line_no + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(2, |c: char| c.is_whitespace()).collect();
        if parts.len() != 2 {
            result.parse_errors.push(format!(
                "line {line_no}: expected directive, got '{trimmed}'"
            ));
            continue;
        }
        let key = parts[0].to_lowercase();
        let value = parts[1].trim();

        if key == "host" {
            if let Some(prev) = current.take() {
                result.entries.push(prev);
            }
            current = Some(SshConfigEntry {
                host_pattern: value.to_string(),
                host_name: None,
                user: None,
                port: None,
                identity_file: None,
                proxy_jump: None,
                identities_only: None,
                strict_host_key_checking: None,
                batch_mode: None,
                connect_timeout: None,
                extra: BTreeMap::new(),
            });
            continue;
        }

        let entry = match current.as_mut() {
            Some(e) => e,
            None => {
                result.parse_errors.push(format!(
                    "line {line_no}: directive '{key}' outside any Host block"
                ));
                continue;
            }
        };

        match key.as_str() {
            "hostname" => entry.host_name = Some(value.to_string()),
            "user" => entry.user = Some(value.to_string()),
            "port" => match value.parse() {
                Ok(p) => entry.port = Some(p),
                Err(_) => result
                    .parse_errors
                    .push(format!("line {line_no}: invalid port '{value}'")),
            },
            "identityfile" => entry.identity_file = Some(value.to_string()),
            "proxyjump" => entry.proxy_jump = Some(value.to_string()),
            "identitiesonly" => {
                entry.identities_only = Some(value.eq_ignore_ascii_case("yes"));
            }
            "stricthostkeychecking" => {
                entry.strict_host_key_checking = Some(value.to_string());
            }
            "batchmode" => {
                entry.batch_mode = Some(value.eq_ignore_ascii_case("yes"));
            }
            "connecttimeout" => match value.parse() {
                Ok(s) => entry.connect_timeout = Some(s),
                Err(_) => result
                    .parse_errors
                    .push(format!("line {line_no}: invalid connecttimeout '{value}'")),
            },
            _ => {
                entry.extra.insert(key.to_string(), value.to_string());
            }
        }
    }

    if let Some(last) = current.take() {
        result.entries.push(last);
    }
    result
}

/// Parse `~/.ssh/config` from disk.
///
/// Returns `Ok(None)` if the file does not exist — a missing config is an
/// explicit unknown, not a converged state.
pub fn parse_ssh_config_file(config_path: &Path) -> Result<Option<SshConfigParse>, String> {
    if !config_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("failed to read {}: {e}", config_path.display()))?;
    Ok(Some(parse_ssh_config(&content)))
}

/// Look up an exact Host alias in the parsed config.
///
/// Glob patterns (`*`, `?`) are NOT expanded — only literal alias matches
/// count. A declared capability alias is always a literal, so this is the
/// right semantics for correlation checks.
pub fn find_entry<'a>(parse: &'a SshConfigParse, host_alias: &str) -> Option<&'a SshConfigEntry> {
    parse.entries.iter().find(|e| {
        e.host_pattern
            .split_whitespace()
            .any(|tok| tok == host_alias)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_host_block() {
        let content = "Host test-builder\n  HostName 10.0.0.5\n  User cynic\n";
        let parsed = parse_ssh_config(content);
        assert_eq!(parsed.entries.len(), 1);
        let entry = &parsed.entries[0];
        assert_eq!(entry.host_pattern, "test-builder");
        assert_eq!(entry.host_name.as_deref(), Some("10.0.0.5"));
        assert_eq!(entry.user.as_deref(), Some("cynic"));
        assert!(parsed.parse_errors.is_empty());
    }

    #[test]
    fn parses_extra_fields_into_bucket() {
        let content = "Host probe\n  HostName 192.168.1.1\n  ForwardAgent yes\n";
        let parsed = parse_ssh_config(content);
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(
            parsed.entries[0]
                .extra
                .get("forwardagent")
                .map(String::as_str),
            Some("yes")
        );
    }

    #[test]
    fn rejects_directive_outside_host_block() {
        let content = "HostName 10.0.0.1\n";
        let parsed = parse_ssh_config(content);
        assert_eq!(parsed.entries.len(), 0);
        assert!(!parsed.parse_errors.is_empty());
    }

    #[test]
    fn find_entry_matches_literal_alias() {
        let content = "Host cynic-builder-ssh\n  HostName 10.10.10.119\n";
        let parsed = parse_ssh_config(content);
        assert!(find_entry(&parsed, "cynic-builder-ssh").is_some());
        assert!(find_entry(&parsed, "nonexistent").is_none());
    }
}
