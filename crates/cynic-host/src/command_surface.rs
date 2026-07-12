use serde::{Deserialize, Serialize};

use crate::install_contract::InstallPathPolicy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostCommandSurface {
    pub role: String,
    #[serde(flatten)]
    pub invocation: HostCommandInvocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostCommandInvocation {
    Shell {
        command: String,
    },
    Argv {
        argv: Vec<String>,
    },
    Expectation {
        text: String,
    },
    PackagePresence {
        package: String,
        query_bin: String,
        rpm_query_bin: String,
    },
    InstallDir {
        path: String,
        policy: InstallPathPolicy,
    },
    InstallFile {
        source: String,
        target: String,
        policy: InstallPathPolicy,
    },
    ValidateSudoers {
        path: String,
    },
}

pub fn shell_command(role: &'static str, command: impl Into<String>) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::Shell {
            command: command.into(),
        },
    }
}

pub fn argv_command(role: &'static str, argv: Vec<String>) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::Argv { argv },
    }
}

pub fn expectation(role: &'static str, text: impl Into<String>) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::Expectation { text: text.into() },
    }
}

pub fn package_presence_command(
    role: &'static str,
    package: impl Into<String>,
    query_bin: impl Into<String>,
    rpm_query_bin: impl Into<String>,
) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::PackagePresence {
            package: package.into(),
            query_bin: query_bin.into(),
            rpm_query_bin: rpm_query_bin.into(),
        },
    }
}

pub fn install_dir_command(
    role: &'static str,
    path: impl Into<String>,
    policy: InstallPathPolicy,
) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::InstallDir {
            path: path.into(),
            policy,
        },
    }
}

pub fn install_file_command(
    role: &'static str,
    source: impl Into<String>,
    target: impl Into<String>,
    policy: InstallPathPolicy,
) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::InstallFile {
            source: source.into(),
            target: target.into(),
            policy,
        },
    }
}

pub fn validate_sudoers_command(role: &'static str, path: impl Into<String>) -> HostCommandSurface {
    HostCommandSurface {
        role: role.to_string(),
        invocation: HostCommandInvocation::ValidateSudoers { path: path.into() },
    }
}

pub fn render_command(command: &HostCommandSurface) -> String {
    render_invocation(&command.invocation)
}

pub fn render_invocation(invocation: &HostCommandInvocation) -> String {
    match invocation {
        HostCommandInvocation::Shell { command } => command.clone(),
        HostCommandInvocation::Argv { argv } => argv.join(" "),
        HostCommandInvocation::Expectation { text } => text.clone(),
        HostCommandInvocation::PackagePresence {
            package,
            query_bin,
            rpm_query_bin,
        } => format!(
            "command -v {package} || {query_bin} -s {package} || {rpm_query_bin} -q {package}"
        ),
        HostCommandInvocation::InstallDir { path, policy } => format!(
            "install -d -o {} -g {} -m {} {}",
            policy.owner.user, policy.owner.group, policy.mode.octal, path
        ),
        HostCommandInvocation::InstallFile {
            source,
            target,
            policy,
        } => format!(
            "install -o {} -g {} -m {} {} {}",
            policy.owner.user, policy.owner.group, policy.mode.octal, source, target
        ),
        HostCommandInvocation::ValidateSudoers { path } => format!("visudo -cf {path}"),
    }
}
