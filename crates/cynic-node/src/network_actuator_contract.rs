use cynic_host::bounded_helper::BoundedHelperProjection;
use cynic_host::runtime_layout::current_runtime_layout;

pub(crate) const NETWORK_ACTUATOR_SUDOERS_SUBJECT: &str = "cynic";
pub(crate) const NETWORK_ACTUATOR_HELPER_FILENAME: &str = "cynic-network-actuator";
pub(crate) const NETWORK_ACTUATOR_POLICY_FILENAME: &str = "network-actuator.policy.json";
pub(crate) const NETWORK_ACTUATOR_SUDOERS_FILENAME: &str = "cynic-network-actuator.sudoers";
pub(crate) const NETWORK_ACTUATOR_MANIFEST_FILENAME: &str = "manifest.json";
pub(crate) const NETWORK_ACTUATOR_HELPER_CONTRACT: &str =
    "single root-owned cynic-network-actuator binary; no shell; bounded profile execution only";

#[derive(Debug, Clone)]
struct NetworkActuatorContractContext {
    helper_path: String,
    policy_path: String,
    sudoers_path: String,
}

impl NetworkActuatorContractContext {
    fn from_current_runtime() -> Self {
        let runtime_layout = current_runtime_layout();
        Self {
            helper_path: runtime_layout.network_actuator_helper_path(),
            policy_path: runtime_layout.network_actuator_policy_path(),
            sudoers_path: runtime_layout.network_actuator_sudoers_path(),
        }
    }
}

pub(crate) fn helper_path() -> String {
    NetworkActuatorContractContext::from_current_runtime().helper_path
}

pub(crate) fn policy_path() -> String {
    NetworkActuatorContractContext::from_current_runtime().policy_path
}

pub(crate) fn sudoers_path() -> String {
    NetworkActuatorContractContext::from_current_runtime().sudoers_path
}

pub(crate) fn sudoers_rule() -> String {
    sudoers_rule_with_context(&NetworkActuatorContractContext::from_current_runtime())
}

pub(crate) fn sudoers_projection() -> String {
    sudoers_projection_with_context(&NetworkActuatorContractContext::from_current_runtime())
}

fn sudoers_rule_with_context(context: &NetworkActuatorContractContext) -> String {
    bounded_helper_projection(context).sudoers_rule()
}

fn sudoers_projection_with_context(context: &NetworkActuatorContractContext) -> String {
    bounded_helper_projection(context).sudoers_projection()
}

pub(crate) fn bundle_required_files() -> [&'static str; 4] {
    [
        NETWORK_ACTUATOR_MANIFEST_FILENAME,
        NETWORK_ACTUATOR_HELPER_FILENAME,
        NETWORK_ACTUATOR_POLICY_FILENAME,
        NETWORK_ACTUATOR_SUDOERS_FILENAME,
    ]
}

fn bounded_helper_projection(context: &NetworkActuatorContractContext) -> BoundedHelperProjection {
    BoundedHelperProjection::new(
        NETWORK_ACTUATOR_SUDOERS_SUBJECT,
        context.helper_path.clone(),
        context.sudoers_path.clone(),
        "Allows the cynic service user to invoke only the root-owned network actuator helper.",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkActuatorContractContext, sudoers_projection_with_context, sudoers_rule_with_context,
    };

    fn test_context() -> NetworkActuatorContractContext {
        NetworkActuatorContractContext {
            helper_path: "/portable/bin/cynic-network-actuator".to_string(),
            policy_path: "/portable/etc/cynic/network-actuator.policy.json".to_string(),
            sudoers_path: "/portable/etc/sudoers.d/cynic-network-actuator".to_string(),
        }
    }

    #[test]
    fn contract_paths_render_from_injected_context() {
        let context = test_context();
        assert_eq!(
            context.policy_path,
            "/portable/etc/cynic/network-actuator.policy.json"
        );
        assert_eq!(
            sudoers_rule_with_context(&context),
            "cynic ALL=(root) NOPASSWD: /portable/bin/cynic-network-actuator"
        );
    }

    #[test]
    fn sudoers_projection_contains_rendered_rule() {
        let projection = sudoers_projection_with_context(&test_context());
        assert!(projection.contains("/portable/bin/cynic-network-actuator"));
        assert!(projection.contains("Allows the cynic service user"));
    }
}
