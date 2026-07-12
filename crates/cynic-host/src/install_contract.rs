use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallOwnership {
    pub user: Cow<'static, str>,
    pub group: Cow<'static, str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallMode {
    pub octal: Cow<'static, str>,
    pub unix: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallPathPolicy {
    pub owner: InstallOwnership,
    pub mode: InstallMode,
}

pub const ROOT_OWNERSHIP: InstallOwnership = InstallOwnership {
    user: Cow::Borrowed("root"),
    group: Cow::Borrowed("root"),
};

pub const MODE_0755: InstallMode = InstallMode {
    octal: Cow::Borrowed("0755"),
    unix: 0o755,
};
pub const MODE_0644: InstallMode = InstallMode {
    octal: Cow::Borrowed("0644"),
    unix: 0o644,
};
pub const MODE_0440: InstallMode = InstallMode {
    octal: Cow::Borrowed("0440"),
    unix: 0o440,
};

pub const fn root_owned_path(mode: InstallMode) -> InstallPathPolicy {
    InstallPathPolicy {
        owner: ROOT_OWNERSHIP,
        mode,
    }
}

pub fn owner_string(owner: InstallOwnership) -> String {
    format!("{}:{}", owner.user, owner.group)
}

pub fn mode_map(entries: &[(&'static str, InstallMode)]) -> BTreeMap<&'static str, String> {
    entries
        .iter()
        .map(|(name, mode)| (*name, mode.octal.to_string()))
        .collect()
}

pub fn render_install_dir(path: &str, policy: InstallPathPolicy) -> String {
    format!(
        "install -d -o {} -g {} -m {} {}",
        policy.owner.user, policy.owner.group, policy.mode.octal, path
    )
}

pub fn render_install_file(source: &str, target: &str, policy: InstallPathPolicy) -> String {
    format!(
        "install -o {} -g {} -m {} {} {}",
        policy.owner.user, policy.owner.group, policy.mode.octal, source, target
    )
}
