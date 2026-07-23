use std::path::PathBuf;

use cynic_host::update_lifecycle::{LocalArgvRunner, default_state_path};

pub(crate) fn runner() -> LocalArgvRunner {
    LocalArgvRunner
}

pub(crate) fn state_path(override_path: Option<String>) -> PathBuf {
    override_path
        .map(PathBuf::from)
        .unwrap_or_else(default_state_path)
}
