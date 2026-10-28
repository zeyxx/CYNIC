use cynic_host::runtime_layout::current_runtime_layout;
use cynic_network::actuator::membrane_admission as pure_membrane_admission;

pub(crate) use cynic_network::actuator::{
    NETWORK_ACTUATOR_MANIFEST_FILENAME, NETWORK_ACTUATOR_MEMBRANE_CONTRACT,
    NETWORK_ACTUATOR_POLICY_FILENAME, NETWORK_ACTUATOR_SUDOERS_SUBJECT,
    bundle_required_files,
};

/// Executor of network actuation: the basket-managed `cynic` binary.
/// There is no `cynic-network-actuator` binary anymore; the capability
/// `network-actuate` runs through `cynic privilege exec` under sudo admission.
pub(crate) fn executor_path() -> String {
    current_runtime_layout().privilege_executor_path()
}

pub(crate) fn policy_path() -> String {
    current_runtime_layout().network_actuator_policy_path()
}

/// The membrane file the bridge installs. The node actuator never writes
/// sudoers itself: there is exactly one membrane, `/etc/sudoers.d/cynic`,
/// owned by the bridge install flow.
pub(crate) fn membrane_path() -> String {
    current_runtime_layout().privilege_sudoers_path()
}

/// Admission class the membrane must contain for network actuation.
pub(crate) fn membrane_admission() -> String {
    pure_membrane_admission(&executor_path())
}

#[cfg(test)]
mod tests {
    use super::{bundle_required_files, membrane_admission};

    #[test]
    fn bundle_requires_only_manifest_and_policy() {
        assert_eq!(bundle_required_files(), ["manifest.json", "network-actuator.policy.json"]);
    }

    #[test]
    fn membrane_admission_names_the_exec_capability() {
        let admission = membrane_admission();
        assert!(
            admission.ends_with("privilege exec network-actuate"),
            "admission must name the capability: {admission}"
        );
        assert!(
            !admission.contains("cynic-network-actuator"),
            "no helper binary in the admission: {admission}"
        );
    }
}
