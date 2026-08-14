use std::fs;
use std::process::ExitCode;

use cynic_protocol::{ExecutionAuthorization, PolicyAuthority, RequiredProof};

use crate::host_exec_cli::args::flag_value;
use crate::host_exec_cli::io::emit_json;
use crate::network_actuation::{
    execute_network_actuation_with_readback, NetworkActuationExecutor, NetworkActuationObservation,
    NetworkActuationObserver, NetworkActuationPlan, NetworkActuationRequest,
    NetworkActuationRuntime, admit_network_actuation_plan,
    build_network_actuation_commands, gate_network_actuation_plan,
    NETWORK_ACTUATION_REQUEST_SCHEMA,
};
use crate::registry::{
    ObservedManagementLink, ResolvedDirectManagementLink, ResolvedHostIdentity,
    resolve_observed_direct_management_link,
};
use crate::runtime_layout::current_runtime_layout;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};
use crate::execution::HostCommandExecution;

pub(crate) fn execute(args: &[String]) -> Result<ExitCode, String> {
    let layout = current_runtime_layout();
    let node_bin = std::env::var("CYNIC_NODE_BIN").unwrap_or_else(|_| "cynic-node".to_string());
    execute_with_runner(
        args,
        &LocalArgvRunner,
        &layout.network_actuator_policy_path(),
        &node_bin,
    )
}

fn execute_with_runner(
    args: &[String],
    runner: &impl ArgvRunner,
    policy_path: &str,
    node_bin: &str,
) -> Result<ExitCode, String> {
    let profile = flag_value(args, "--profile")
        .ok_or_else(|| "network-actuate requires --profile <profile>".to_string())?;
    let target = flag_value(args, "--target").unwrap_or_else(|| "local-host".to_string());
    let apply = args.iter().any(|arg| arg == "--apply");
    let registry_path = flag_value(args, "--registry");
    let bytes = fs::read(&policy_path)
        .map_err(|error| format!("read root-owned network policy {policy_path}: {error}"))?;
    let policy: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse root-owned network policy {policy_path}: {error}"))?;
    let allowed = policy
        .get("allowed_profiles")
        .and_then(|value| value.get(&profile))
        .cloned();
    let policy_allows = allowed.is_some();
    let observation_output = runner.run(&[node_bin, "network", "actuation-observation", "--json"]);
    let mut observation = if observation_output.success() {
        serde_json::from_str::<crate::network_actuation::NetworkActuationObservation>(
            observation_output.stdout.trim(),
        )
        .ok()
    } else {
        None
    };
    let mut projection_blockers = Vec::new();
    let mut identity_resolution: Option<ResolvedHostIdentity> = None;
    if profile == "direct-management-link" {
        if let Some(observed) = observation.as_mut() {
            match observe_local_management_link(runner, &observed.target.synapse_cidr) {
                Some(local_link) => match project_direct_management_target_from_registry(
                    observed,
                    Some(&local_link),
                    registry_path.as_deref(),
                ) {
                    Ok(identity) => identity_resolution = Some(identity),
                    Err(error) => {
                        projection_blockers.push(format!("registry_projection_failed:{error}"))
                    }
                },
                None => projection_blockers
                    .push("local_management_observation_unavailable".to_string()),
            }
        } else {
            projection_blockers.push("registry_projection_failed:observation_unavailable".to_string());
        }
    }
    let observation_allows = observation
        .as_ref()
        .is_some_and(|value| {
            projection_blockers.is_empty() && value.allows_profile(&profile)
        });
    let plan = if projection_blockers.is_empty() {
        observation.as_ref().and_then(|observed| {
            let layout = current_runtime_layout();
            let capabilities = layout.capabilities();
            let runtime = NetworkActuationRuntime {
                helper_path: layout.host_exec_helper_path(),
                sudoers_subject: "cynic",
                max_timeout_seconds: 60,
                service_manager_bin: capabilities.service_manager.command,
                wireguard_unit: layout.wg_quick_unit(),
                route_bin: capabilities.route_tool.command,
                wireguard_bin: capabilities.wireguard_tool.command,
                network_manager_bin: capabilities.network_manager.command,
            };
            build_network_actuation_commands(&profile, &observed.target, &runtime).map(
                |commands| NetworkActuationPlan {
                    schema: "cynic.host.network_actuation_plan.v1",
                    id: format!("network-actuate:{profile}:{target}"),
                    profile: profile.clone(),
                    target: target.clone(),
                    commands,
                    preconditions: vec![
                        "root_owned_policy_loaded".to_string(),
                        "local_observation_allows_profile".to_string(),
                    ],
                    rollback_intent: "restore the previously observed route and service state"
                        .to_string(),
                    expected_proofs: vec![RequiredProof {
                        gate: "postflight_network_observation".to_string(),
                        subject: target.clone(),
                    }],
                },
            )
        })
    } else {
        None
    };
    let actor = flag_value(args, "--actor").unwrap_or_else(|| "local-operator".to_string());
    let admission = plan
        .as_ref()
        .map(|value| admit_network_actuation_plan(value, actor.clone()));
    let authorization = ExecutionAuthorization::new(
        actor,
        PolicyAuthority::HumanOperator,
        apply,
        vec![
            "root_owned_policy_loaded".to_string(),
            "local_observation_allows_profile".to_string(),
        ],
    );
    let admission_blockers = plan
        .as_ref()
        .zip(admission.as_ref())
        .map(|(plan, admission)| gate_network_actuation_plan(plan, admission, &authorization))
        .unwrap_or_else(|| {
            if projection_blockers.is_empty() {
                vec!["canonical_plan_unavailable".to_string()]
            } else {
                projection_blockers.clone()
            }
        });
    let request = plan
        .clone()
        .zip(admission.clone())
        .map(|(plan, admission)| NetworkActuationRequest {
            schema: NETWORK_ACTUATION_REQUEST_SCHEMA.to_string(),
            plan,
            admission,
            authorization: authorization.clone(),
        });
    if apply {
        if let Some(plan) = plan.as_ref() {
            let mut proof = execute_network_actuation_with_readback(
                plan,
                admission_blockers,
                &RootInvoker { runner },
                &NetworkActuationObserverLocal {
                    runner,
                    observation_argv: vec![
                        node_bin.to_string(),
                        "network".to_string(),
                        "actuation-observation".to_string(),
                        "--json".to_string(),
                    ],
                },
            );
            proof.postflight_controls.insert(
                "firewall".to_string(),
                observe_root_firewall(runner),
            );
            return emit_json(&proof, proof.status != "proved");
        }
    }

    let status = if !apply {
        "blocked"
    } else if !policy_allows || !observation_allows || !admission_blockers.is_empty() {
        "blocked"
    } else {
        "planned"
    };

    let blockers = if !apply {
        vec!["explicit_apply_required".to_string()]
    } else if !policy_allows {
        vec!["profile_not_allowed_by_local_policy".to_string()]
    } else if !observation_allows {
        vec!["profile_not_allowed_by_local_observation".to_string()]
    } else {
        admission_blockers.clone()
    };

    emit_json(
        &serde_json::json!({
            "schema": "cynic.host.network_actuate.preflight.v1",
            "status": status,
            "mutates": false,
            "profile": profile,
            "policy_path": policy_path,
            "apply_requested": apply,
            "allowed_profile": allowed,
            "observation": observation,
            "identity_resolution": identity_resolution,
            "plan": plan,
            "admission": admission,
            "authorization": authorization,
            "request": request,
            "admission_blockers": admission_blockers,
            "blockers": blockers,
            "next": "canonical topology-bound plan admitted locally; execution proceeds through helper root with postflight proof"
        }),
        status == "blocked",
    )
}

fn observe_root_firewall(runner: &impl ArgvRunner) -> serde_json::Value {
    let nft_bin = std::env::var("CYNIC_NFT_BIN").unwrap_or_else(|_| "nft".to_string());
    let output = runner.run(&[nft_bin.as_str(), "list", "ruleset"]);
    serde_json::json!({
        "schema": "cynic.host.firewall_observation.v1",
        "status": if output.success() { "observed" } else { "blocked" },
        "mutates": false,
        "command": nft_bin,
        "args": ["list", "ruleset"],
        "exit_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": stderr_with_spawn_error(output.stderr, output.spawn_error),
    })
}

fn stderr_with_spawn_error(mut stderr: String, spawn_error: Option<String>) -> String {
    if let Some(error) = spawn_error {
        if stderr.is_empty() {
            stderr = error;
        } else {
            stderr = format!("{error}: {stderr}");
        }
    }
    stderr
}

struct RootInvoker<'a> {
    runner: &'a dyn ArgvRunner,
}

impl<'a> NetworkActuationExecutor for RootInvoker<'a> {
    fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution {
        let command_argv = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&command_argv);
        HostCommandExecution {
            role: role.to_string(),
            argv: argv.to_vec(),
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: stderr_with_spawn_error(output.stderr, output.spawn_error),
        }
    }
}

struct NetworkActuationObserverLocal<'a> {
    runner: &'a dyn ArgvRunner,
    observation_argv: Vec<String>,
}

impl<'a> NetworkActuationObserver for NetworkActuationObserverLocal<'a> {
    fn observe(&self) -> Result<NetworkActuationObservation, String> {
        let observation_argv = self
            .observation_argv
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let output = self.runner.run(&observation_argv);
        if output.spawn_error.is_some() {
            return Err(format!(
                "postflight observation command failed to spawn: {:?}",
                output.spawn_error
            ));
        }
        if output.exit_code != Some(0) {
            return Err(format!(
                "postflight observation command returned exit code {:?}",
                output.exit_code
            ));
        }
        serde_json::from_str(&output.stdout)
            .map_err(|error| format!("postflight observation command returned invalid JSON: {error}"))
    }
}

fn project_direct_management_target_from_registry(
    observation: &mut crate::network_actuation::NetworkActuationObservation,
    local_link: Option<&ObservedManagementLink>,
    registry_path: Option<&str>,
) -> Result<ResolvedHostIdentity, String> {
    let hostname = std::env::var("HOSTNAME")
        .map_err(|_| "hostname_observation_unavailable".to_string())?;
    let identity = resolve_observed_direct_management_link(
        &hostname,
        &observation.target.synapse_cidr,
        local_link,
        registry_path,
    )?;
    apply_direct_management_projection(observation, &identity.link);
    Ok(identity)
}

fn observe_local_management_link(
    runner: &impl ArgvRunner,
    synapse_cidr: &str,
) -> Option<ObservedManagementLink> {
    let capabilities = current_runtime_layout().capabilities();
    let nmcli = capabilities.network_manager.command;
    let route_bin = capabilities.route_tool.command;
    let nmcli_argv = [
        nmcli.as_str(),
        "-t",
        "-f",
        "GENERAL.CONNECTION,GENERAL.DEVICE,IP4.ADDRESS",
        "device",
        "show",
    ];
    let nmcli_output = runner.run(&nmcli_argv);
    if !nmcli_output.success() {
        return None;
    }
    let route_argv = [route_bin.as_str(), "route", "show", synapse_cidr];
    let route_output = runner.run(&route_argv);
    if !route_output.success() {
        return None;
    }
    let (route_interface, gateway) = route_output
        .stdout
        .lines()
        .find_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            let via = fields.iter().position(|field| *field == "via")?;
            let dev = fields.iter().position(|field| *field == "dev")?;
            Some((fields.get(dev + 1)?.to_string(), fields.get(via + 1)?.to_string()))
        })?;
    let mut connection = None;
    let mut interface = None;
    let mut address = None;
    for line in nmcli_output.stdout.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key {
            "GENERAL.CONNECTION" => connection = Some(value.to_string()),
            "GENERAL.DEVICE" => interface = Some(value.to_string()),
            key if key.starts_with("IP4.ADDRESS") => address = Some(value.to_string()),
            _ => {}
        }
        if interface.as_deref() == Some(route_interface.as_str()) && address.is_some() {
            return Some(ObservedManagementLink {
                interface: route_interface,
                connection: connection.unwrap_or_default(),
                address: address.unwrap_or_default(),
                gateway,
            });
        }
    }
    None
}

fn apply_direct_management_projection(
    observation: &mut crate::network_actuation::NetworkActuationObservation,
    resolved: &ResolvedDirectManagementLink,
) {
    observation.target.management_interface = Some(resolved.interface.clone());
    observation.target.management_connection = Some(resolved.connection.clone());
    observation.target.management_address = Some(resolved.address.clone());
    observation.target.management_gateway = Some(resolved.gateway.clone());
}

#[cfg(test)]
mod tests {
    use super::{execute_with_runner, project_direct_management_target_from_registry};
    use crate::network_actuation::NetworkActuationObservation;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};
    use std::fs;

    #[test]
    fn preflight_requires_policy_and_observation_to_allow_profile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy = dir.path().join("policy.json");
        std::fs::write(
            &policy,
            r#"{"allowed_profiles":{"synapse-route":{"command_prefixes":[]}}}"#,
        )
        .expect("write policy");
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "/portable/cynic-node".into(),
                    "network".into(),
                    "actuation-observation".into(),
                    "--json".into(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: r#"{"provider_fallback_active":true,"synapse_route_active":false,"wireguard_active":true,"target":{"provider_fallback_cidr":"10.0.0.0/24","provider_fallback_gateway":null,"synapse_cidr":"10.10.10.0/24","mesh_interface":"wg0"}}"#.into(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let result = execute_with_runner(
            &["--profile".into(), "synapse-route".into()],
            &runner,
            policy.to_str().expect("policy path"),
            "/portable/cynic-node",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn direct_management_projection_overrides_target_from_registry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = dir.path().join("registry.json");
        fs::write(
            &registry,
            r#"{
              "network_policy": {
                "planes": {
                  "synapse": {"cidr":"10.10.10.0/24"}
                }
              },
              "nodes": [
                {
                  "name":"cynic-host-cortex",
                  "role":"cortex_host",
                  "aliases":["ubuntu"],
                  "network_links":[
                    {
                      "plane":"management",
                      "provider":"network-manager",
                      "interface":"enp42s0",
                      "connection":"Connexion filaire 1",
                      "address":"192.168.100.2/24",
                      "gateway":"192.168.100.1",
                      "routes":["10.10.10.0/24"],
                      "default_route":false
                    }
                  ]
                }
              ]
            }"#,
        )
        .expect("write registry fixture");
        unsafe {
            std::env::set_var("HOSTNAME", "cynic-host-cortex");
        }
        let mut observation = NetworkActuationObservation {
            provider_fallback_active: true,
            synapse_route_active: false,
            wireguard_active: false,
            management_link_active: false,
            target: crate::network_actuation::NetworkActuationTarget {
                provider_fallback_cidr: "192.168.100.0/24".to_string(),
                provider_fallback_gateway: Some("192.168.100.1".to_string()),
                synapse_cidr: "10.10.10.0/24".to_string(),
                mesh_interface: "wg0".to_string(),
                management_interface: None,
                management_connection: None,
                management_address: None,
                management_gateway: None,
            },
        };

        project_direct_management_target_from_registry(
            &mut observation,
            None,
            Some(registry.to_str().expect("registry path")),
        )
        .expect("projection succeeds");

        assert_eq!(
            observation.target.management_interface.as_deref(),
            Some("enp42s0")
        );
        assert_eq!(
            observation.target.management_connection.as_deref(),
            Some("Connexion filaire 1")
        );
        assert_eq!(
            observation.target.management_address.as_deref(),
            Some("192.168.100.2/24")
        );
        assert_eq!(
            observation.target.management_gateway.as_deref(),
            Some("192.168.100.1")
        );
    }

    #[test]
    fn direct_management_projection_fails_without_matching_registry_link() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = dir.path().join("registry.json");
        fs::write(
            &registry,
            r#"{
              "nodes":[{"name":"cynic-host-cortex","aliases":["ubuntu"],"network_links":[]}]
            }"#,
        )
        .expect("write registry fixture");
        unsafe {
            std::env::set_var("HOSTNAME", "cynic-host-cortex");
        }
        let mut observation = NetworkActuationObservation {
            provider_fallback_active: true,
            synapse_route_active: false,
            wireguard_active: false,
            management_link_active: false,
            target: crate::network_actuation::NetworkActuationTarget {
                provider_fallback_cidr: "192.168.100.0/24".to_string(),
                provider_fallback_gateway: Some("192.168.100.1".to_string()),
                synapse_cidr: "10.10.10.0/24".to_string(),
                mesh_interface: "wg0".to_string(),
                management_interface: None,
                management_connection: None,
                management_address: None,
                management_gateway: None,
            },
        };

        let error = project_direct_management_target_from_registry(
            &mut observation,
            None,
            Some(registry.to_str().expect("registry path")),
        )
        .expect_err("projection fails");

        assert_eq!(error, "direct_management_link_not_declared_in_registry");
    }

    #[test]
    fn network_metadata_overrides_stale_hostname_alias() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = dir.path().join("registry.json");
        fs::write(
            &registry,
            r#"{
              "network_policy": {"planes": {"synapse": {"cidr":"10.10.10.0/24"}}},
              "nodes": [
                {"name":"local-builder","aliases":["ubuntu"],"role":"build_runner"},
                {
                  "name":"cynic-host-cortex",
                  "role":"cortex_host",
                  "network_links":[{
                    "plane":"management",
                    "provider":"network-manager",
                    "interface":"enp42s0",
                    "connection":"Connexion filaire 1",
                    "address":"192.168.100.2/24",
                    "gateway":"192.168.100.1",
                    "routes":["10.10.10.0/24"]
                  }]
                }
              ]
            }"#,
        )
        .expect("write registry fixture");
        unsafe {
            std::env::set_var("HOSTNAME", "ubuntu");
        }
        let mut observation = NetworkActuationObservation {
            provider_fallback_active: true,
            synapse_route_active: false,
            wireguard_active: false,
            management_link_active: false,
            target: crate::network_actuation::NetworkActuationTarget {
                provider_fallback_cidr: "192.168.100.0/24".to_string(),
                provider_fallback_gateway: Some("192.168.100.1".to_string()),
                synapse_cidr: "10.10.10.0/24".to_string(),
                mesh_interface: "wg0".to_string(),
                management_interface: None,
                management_connection: None,
                management_address: None,
                management_gateway: None,
            },
        };
        let identity = project_direct_management_target_from_registry(
            &mut observation,
            Some(&crate::registry::ObservedManagementLink {
                interface: "enp42s0".to_string(),
                connection: "Connexion filaire 1".to_string(),
                address: "192.168.100.2/24".to_string(),
                gateway: "192.168.100.1".to_string(),
            }),
            Some(registry.to_str().expect("registry path")),
        )
        .expect("network identity resolves");

        assert_eq!(identity.node, "cynic-host-cortex");
        assert_eq!(identity.match_source, "management_network");
        assert!(!identity.hostname_match);
        assert!(identity.network_match);
        assert!(identity
            .metadata
            .contains(&"hostname_registry_alias_mismatch".to_string()));
    }
}
