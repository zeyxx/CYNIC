//! Reproducible build cache reclaim execution.

use cynic_node::ports::build_runner_adapter::{CommandRunner, LocalCommandRunner};

pub(crate) fn reclaim_build_cache_local(
    node: &str,
    registry: &str,
    state_root: &str,
    temporary_root: &str,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    use cynic_host::adapters::local_build_cache::LocalBuildCacheAdapter;
    use cynic_host::domain::disk::build_runner::{
        load_build_cache_desired_state, reconcile_build_cache_resources,
    };

    let desired = load_build_cache_desired_state(
        std::path::Path::new(registry),
        node,
        std::path::Path::new(state_root),
        std::path::Path::new(temporary_root),
    )?;

    let report = reconcile_build_cache_resources(
        &desired,
        apply,
        &LocalBuildCacheAdapter,
        &LocalBuildCacheAdapter,
    );

    if json {
        let text = serde_json::to_string_pretty(&report)
            .map_err(|error| format!("reclaim report serialization failed: {error}"))?;
        println!("{text}");
    } else {
        println!(
            "build cache reclaim — node={} status={}",
            report.status, report.status
        );
        println!("decision: {:?}", report.plan.decision);
        println!("available_bytes: {:?}", report.before.available_bytes);
        println!(
            "minimum_available_bytes: {}",
            report.plan.minimum_available_bytes
        );
        println!("reclaim_profile: {}", report.plan.target_profile);
        println!("preserved_profiles: {:?}", report.plan.preserved_profiles);
        if !report.before.observed_noncanonical_cache_paths.is_empty() {
            println!(
                "non-canonical caches present: {:?}",
                report.before.observed_noncanonical_cache_paths
            );
        }
        for line in &report.evidence {
            println!("evidence: {line}");
        }
        for blocker in &report.blockers {
            eprintln!("blocker: {blocker}");
        }
        if !apply {
            println!("(dry run — pass --apply to execute)");
        }
    }

    if report.status == "blocked" {
        return Err(format!("reclaim blocked: {}", report.blockers.join(", ")));
    }
    Ok(())
}

/// Registry path the builder must read for a remote reclaim.
/// The local default is repo-relative (cortex checkout) and does not exist
/// on the builder; only the untouched default is remapped, an explicit
/// override is forwarded verbatim.
pub(crate) fn remote_reclaim_registry_for(registry: &str) -> &str {
    const DEFAULT_LOCAL_REGISTRY: &str = "infra/registry.json";
    const BUILDER_CANONICAL_REGISTRY: &str = "/etc/cynic/registry.json";
    if registry == DEFAULT_LOCAL_REGISTRY {
        BUILDER_CANONICAL_REGISTRY
    } else {
        registry
    }
}

pub(crate) fn reclaim_build_cache_remote(
    node: &str,
    registry: &str,
    state_root: &str,
    temporary_root: &str,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    let transport = cynic_host::registry::resolve::resolve_build_runner_transport(Some(registry))
        .map_err(|error| {
            format!("build runner unresolvable in registry {registry}: {error}")
        })?;

    let remote_registry = remote_reclaim_registry_for(registry);

    let mut argv: Vec<String> = vec![
        "cynic".to_string(),
        "build".to_string(),
        "reclaim".to_string(),
        "--local".to_string(),
        "--node".to_string(),
        node.to_string(),
        "--registry".to_string(),
        remote_registry.to_string(),
        "--state-root".to_string(),
        state_root.to_string(),
        "--temporary-root".to_string(),
        temporary_root.to_string(),
    ];
    if apply {
        argv.push("--apply".to_string());
    }
    if json {
        argv.push("--json".to_string());
    }

    let mut full: Vec<String> = vec!["ssh".to_string(), transport.ssh_target.clone()];
    full.extend(argv);

    let borrowed: Vec<&str> = full.iter().map(String::as_str).collect();
    let runner = LocalCommandRunner;
    let output = runner.run(&borrowed);

    if let Some(error) = &output.spawn_error {
        return Err(format!(
            "reclaim dispatch to {} failed: {error}",
            transport.ssh_target
        ));
    }
    if !output.stdout.is_empty() {
        println!("{}", output.stdout);
    }
    if !output.stderr.is_empty() {
        eprintln!("{}", output.stderr);
    }
    if !output.success() {
        return Err(format!(
            "reclaim on {} exited {:?}",
            transport.ssh_target, output.exit_code
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::remote_reclaim_registry_for;

    #[test]
    fn default_local_registry_remaps_to_builder_canonical() {
        assert_eq!(
            remote_reclaim_registry_for("infra/registry.json"),
            "/etc/cynic/registry.json"
        );
    }

    #[test]
    fn explicit_registry_override_forwards_verbatim() {
        assert_eq!(
            remote_reclaim_registry_for("/srv/custom/registry.json"),
            "/srv/custom/registry.json"
        );
    }
}
