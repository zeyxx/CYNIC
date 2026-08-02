use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use crate::agent_access::{
    AGENT_ACCESS_OBSERVATION_SCHEMA, AgentAccessObservation, AgentAccessPolicy,
};

pub fn observe_agent_access(policy: &AgentAccessPolicy) -> AgentAccessObservation {
    observe_agent_access_from(policy, Path::new("/etc/group"))
}

pub fn observe_linux_uid(principal: &str) -> Result<u32, String> {
    let database = std::fs::read_to_string("/etc/passwd")
        .map_err(|error| format!("principal database is unreadable: {error}"))?;
    parse_passwd_uid(&database, principal)
        .ok_or_else(|| format!("principal `{principal}` is not observable"))
}

fn observe_agent_access_from(
    policy: &AgentAccessPolicy,
    group_database: &Path,
) -> AgentAccessObservation {
    let mut unknowns = Vec::new();
    let group = match std::fs::read_to_string(group_database) {
        Ok(content) => content
            .lines()
            .find_map(|line| parse_group(line, &policy.access_group)),
        Err(error) => {
            unknowns.push(format!("agent_access_group_database_unreadable:{error}"));
            None
        }
    };
    let (access_group_present, access_group_id, observed_members) = match group {
        Some((group_id, members)) => (Some(true), Some(group_id), members),
        None if unknowns.is_empty() => (Some(false), None, Vec::new()),
        None => (None, None, Vec::new()),
    };
    let runtime_directory_present = Some(Path::new(&policy.runtime_directory).is_dir());
    let socket_metadata = std::fs::symlink_metadata(&policy.socket_path).ok();
    let socket_present = Some(socket_metadata.is_some());
    let socket_group_id = socket_metadata.as_ref().map(MetadataExt::gid);
    let socket_mode = socket_metadata
        .as_ref()
        .map(|metadata| metadata.permissions().mode() & 0o777);

    AgentAccessObservation {
        schema: AGENT_ACCESS_OBSERVATION_SCHEMA.to_string(),
        access_group_present,
        access_group_id,
        observed_members,
        runtime_directory_present,
        socket_present,
        socket_group_id,
        socket_mode,
        evidence_refs: vec![
            format!("group_database:{}", group_database.display()),
            format!("kernel_socket:{}", policy.socket_path),
        ],
        unknowns,
    }
}

fn parse_group(line: &str, expected: &str) -> Option<(u32, Vec<String>)> {
    let mut fields = line.split(':');
    let name = fields.next()?;
    let _password = fields.next()?;
    let group_id = fields.next()?.parse::<u32>().ok()?;
    let members = fields.next().unwrap_or_default();
    (name == expected).then(|| {
        (
            group_id,
            members
                .split(',')
                .filter(|member| !member.is_empty())
                .map(ToString::to_string)
                .collect(),
        )
    })
}

fn parse_passwd_uid(database: &str, expected: &str) -> Option<u32> {
    database.lines().find_map(|line| {
        let mut fields = line.split(':');
        let name = fields.next()?;
        let _password = fields.next()?;
        let uid = fields.next()?.parse::<u32>().ok()?;
        (name == expected).then_some(uid)
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_group, parse_passwd_uid};

    #[test]
    fn group_observation_reads_only_the_declared_access_group() {
        assert_eq!(
            parse_group("cynic-agent:x:1901:cynic,operator", "cynic-agent"),
            Some((1901, vec!["cynic".to_string(), "operator".to_string()]))
        );
        assert_eq!(parse_group("cynic:x:1900:cynic", "cynic-agent"), None);
    }

    #[test]
    fn principal_uid_observation_requires_an_exact_name() {
        let database =
            "root:x:0:0:root:/root:/bin/sh\noperator:x:1000:1000::/home/operator:/bin/sh\n";
        assert_eq!(parse_passwd_uid(database, "operator"), Some(1000));
        assert_eq!(parse_passwd_uid(database, "oper"), None);
    }
}
