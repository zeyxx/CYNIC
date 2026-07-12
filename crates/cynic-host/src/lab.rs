use serde_json::Value;

pub mod bundle;
pub mod context;
pub mod image;
pub mod registry;
pub mod topology;
pub mod types;

pub use bundle::write_lab_bundle;
use context::lab_context;
use image::image_preflight;
pub use image::{build_lab_image_report, resolve_lab_image_sha256};
use registry::read_registry;
use topology::{
    execution_suite, guest_bootstrap_preflight, isolation_plan, provider_plan, provision_blockers,
    provision_steps, vm_plan,
};
pub use types::{
    LabBundleReport, LabGuestBootstrapBundleReport, LabImageReport, LabImageResolveReport,
    LabPlanReport, LabProvisionReport,
};

pub fn build_lab_plan(registry_path: &str) -> Result<LabPlanReport, String> {
    let registry = read_registry(registry_path)?;
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes must be an array".to_string())?;
    let network_policy = registry.get("network_policy");
    let provider = provider_plan();
    let isolation = isolation_plan(network_policy);
    let vms = nodes.iter().map(vm_plan).collect::<Result<Vec<_>, _>>()?;

    Ok(LabPlanReport {
        schema: "cynic.lab.plan.v1",
        context: lab_context(),
        status: "planned",
        mutates: false,
        can_apply: false,
        registry_path: registry_path.to_string(),
        provider,
        isolation,
        vm_count: vms.len(),
        vms,
        execution_suite: execution_suite(registry_path),
        blockers: vec![
            "base image path and checksum are not declared in registry",
            "guest bootstrap transport is not declared",
            "closed lab network must be created before mutating guest routes",
        ],
        safety_rules: vec![
            "host routing table must not be mutated by lab mesh apply",
            "host firewall and DNS must not be changed by lab mesh apply",
            "mutating mesh profiles require --closed-environment inside guests only",
            "all runtime proof must be read back from guests after apply",
        ],
    })
}

pub fn build_lab_provision_report(
    registry_path: &str,
    apply: bool,
) -> Result<LabProvisionReport, String> {
    let registry = read_registry(registry_path)?;
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "registry nodes must be an array".to_string())?;
    let network_policy = registry.get("network_policy");
    let provider = provider_plan();
    let isolation = isolation_plan(network_policy);
    let image = image_preflight(&registry);
    let guest_bootstrap = guest_bootstrap_preflight(&registry);
    let vms = nodes.iter().map(vm_plan).collect::<Result<Vec<_>, _>>()?;
    let blockers = provision_blockers(&provider, &image, &guest_bootstrap);

    Ok(LabProvisionReport {
        schema: "cynic.lab.provision.v1",
        context: lab_context(),
        status: if blockers.is_empty() {
            "ready"
        } else if apply {
            "refused"
        } else {
            "blocked"
        },
        mutates: false,
        requested_apply: apply,
        can_apply: false,
        registry_path: registry_path.to_string(),
        provider,
        isolation,
        image,
        guest_bootstrap,
        vms,
        planned_mutations: provision_steps(registry_path),
        blockers,
        safety_rules: vec![
            "provision preflight must not create libvirt networks or domains",
            "closed lab network must be isolated from the host default route",
            "base images must be declared with checksum before clone",
            "guest bootstrap must target guests only, never host network state",
        ],
    })
}

pub fn write_guest_bootstrap_bundle(
    output_dir: &str,
) -> Result<LabGuestBootstrapBundleReport, String> {
    crate::framework_bootstrap::write_framework_bootstrap_bundle(output_dir)
}

#[cfg(test)]
mod tests {
    use super::image::{build_lab_image_report, parse_sha256_sums, parse_sha256sum_stdout};
    use super::{
        build_lab_plan, build_lab_provision_report, write_guest_bootstrap_bundle, write_lab_bundle,
    };

    #[test]
    fn lab_plan_is_non_mutating_and_registry_driven() {
        let registry = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let plan = build_lab_plan(&registry).expect("lab plan");

        assert_eq!(plan.schema, "cynic.lab.plan.v1");
        assert!(!plan.mutates);
        assert!(!plan.can_apply);
        assert_eq!(plan.vm_count, 3);
        assert!(plan.vms.iter().any(|vm| vm.name == "lab-kernel"));
        assert_eq!(plan.isolation.host_routing_mutation, false);
    }

    #[test]
    fn lab_provision_refuses_until_image_contract_is_satisfied() {
        let registry = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let report = build_lab_provision_report(&registry, true).expect("provision report");

        assert_eq!(report.schema, "cynic.lab.provision.v1");
        assert_eq!(report.status, "refused");
        assert!(!report.mutates);
        assert!(!report.can_apply);
        assert!(
            report
                .blockers
                .iter()
                .any(|blocker| *blocker == "registry lab.base_image.path does not exist on host")
        );
        assert!(report.blockers.iter().all(|blocker| *blocker
            != "registry lab.base_image.sha256 must be a 64-character lowercase hex digest"));
    }

    #[test]
    fn lab_bundle_writes_libvirt_and_cloud_init_artifacts() {
        let registry = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let out = std::env::temp_dir().join(format!(
            "cynic-lab-bundle-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let report = write_lab_bundle(&registry, out.to_str().expect("path")).expect("bundle");

        assert_eq!(report.schema, "cynic.lab.bundle.v1");
        assert!(!report.mutates);
        assert_eq!(report.vm_count, 3);
        assert!(out.join("manifest.json").is_file());
        assert!(out.join("network.xml").is_file());
        assert!(out.join("domains/lab-kernel.xml").is_file());
        assert!(out.join("cloud-init/lab-kernel-user-data.yaml").is_file());
        std::fs::remove_dir_all(out).expect("cleanup");
    }

    #[test]
    fn guest_bootstrap_bundle_writes_security_kit() {
        let out = std::env::temp_dir().join(format!(
            "cynic-guest-bootstrap-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));

        let report = write_guest_bootstrap_bundle(out.to_str().expect("path"))
            .expect("guest bootstrap bundle");

        assert_eq!(report.status, "written");
        assert!(out.join("host-exec-bundle/cynic-host-exec").is_file());
        assert!(
            out.join("host-exec-bundle/cynic-host-exec.sudoers")
                .is_file()
        );
        assert!(out.join("install.sh").is_file());
        assert!(out.join("verify.sh").is_file());
        assert!(out.join("security-maintenance.sh").is_file());
        assert!(out.join("security-policy.json").is_file());
        std::fs::remove_dir_all(out).expect("cleanup");
    }

    #[test]
    fn lab_image_fetch_is_ready_after_checksum_is_pinned() {
        let registry = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let report = build_lab_image_report(&registry, false).expect("image report");

        assert_eq!(report.schema, "cynic.lab.image.v1");
        assert_eq!(report.status, "planned");
        assert!(!report.mutates);
        assert!(report.can_fetch);
        assert!(report.image.source_url.is_some());
        assert!(report.image.checksum_valid_shape);
        assert!(
            report
                .commands
                .iter()
                .any(|command| command.role == "download_base_image")
        );
        assert!(report.executions.is_empty());
        assert!(report.blockers.is_empty());
    }

    #[test]
    fn parses_target_image_from_sha256_sums() {
        let sums = "\
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa *other.img
0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef *noble-server-cloudimg-amd64.img
";
        let digest = parse_sha256_sums(sums, "noble-server-cloudimg-amd64.img").expect("digest");

        assert_eq!(
            digest,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn parses_sha256sum_stdout() {
        let digest = parse_sha256sum_stdout(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  image.img\n",
        )
        .expect("digest");

        assert_eq!(
            digest,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }
}
