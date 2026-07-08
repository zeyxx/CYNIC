fn network_mesh_dns_activation_plan_args() -> Vec<String> {
    vec!["dns".to_string(), "plan".to_string()]
}

fn network_mesh_dns_activation_bundle_args(
    params: &NetworkMeshDnsActivationBundleParams,
) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "dns".to_string(),
        "bundle".to_string(),
        "--registry".to_string(),
        registry_ref_arg(&params.registry_ref)?,
        "--output-dir".to_string(),
        tmp_output_dir_arg(&params.output_dir, "/tmp/cynic-synapse-dns", "output_dir")?,
    ])
}

fn network_mesh_provider_fallback_hosts_args(
    params: &NetworkMeshProviderFallbackHostsParams,
) -> Result<Vec<String>, McpError> {
    Ok(vec![
        "fallback-hosts".to_string(),
        "--registry".to_string(),
        registry_ref_arg(&params.registry_ref)?,
        "--output-dir".to_string(),
        tmp_output_dir_arg(&params.output_dir, "/tmp/cynic-provider-fallback", "output_dir")?,
    ])
}

fn network_mesh_provider_fallback_status_args(
    params: &NetworkMeshProviderFallbackStatusParams,
) -> Result<Vec<String>, McpError> {
    let mut args = vec![
        "fallback-status".to_string(),
        "--registry".to_string(),
        registry_ref_arg(&params.registry_ref)?,
        "--hosts-path".to_string(),
        params
            .hosts_path
            .clone()
            .unwrap_or_else(|| "/etc/hosts".to_string()),
    ];
    if params.check_connectivity.unwrap_or(false) {
        args.push("--check-connectivity".to_string());
    }
    Ok(args)
}

fn network_mesh_dns_activation_status_args(
    params: &NetworkMeshDnsActivationStatusParams,
) -> Vec<String> {
    let mut args = vec!["dns".to_string(), "activation-status".to_string()];
    if let Some(value) = &params.hosts_path {
        args.extend(["--hosts-path".to_string(), value.clone()]);
    }
    if let Some(value) = &params.corefile_path {
        args.extend(["--corefile-path".to_string(), value.clone()]);
    }
    if let Some(value) = &params.service_path {
        args.extend(["--service-path".to_string(), value.clone()]);
    }
    if let Some(value) = &params.resolved_dropin_path {
        args.extend(["--resolved-dropin-path".to_string(), value.clone()]);
    }
    if params.resolve_dns.unwrap_or(true) {
        args.push("--resolve-dns".to_string());
    }
    args
}

fn run_network_mesh_dns_activation_plan(
    _params: &NetworkMeshDnsActivationPlanParams,
) -> Result<serde_json::Value, McpError> {
    run_network_surface_json(
        &network_mesh_dns_activation_plan_args(),
        "network.mesh.dns.activation_plan",
    )
}

fn run_network_mesh_dns_activation_bundle(
    params: &NetworkMeshDnsActivationBundleParams,
) -> Result<serde_json::Value, McpError> {
    run_network_surface_json(
        &network_mesh_dns_activation_bundle_args(params)?,
        "network.mesh.dns.activation_bundle",
    )
}

fn run_network_mesh_provider_fallback_hosts(
    params: &NetworkMeshProviderFallbackHostsParams,
) -> Result<serde_json::Value, McpError> {
    run_network_surface_json(
        &network_mesh_provider_fallback_hosts_args(params)?,
        "network.mesh.provider_fallback_hosts",
    )
}

fn run_network_mesh_provider_fallback_status(
    params: &NetworkMeshProviderFallbackStatusParams,
) -> Result<serde_json::Value, McpError> {
    run_network_surface_json(
        &network_mesh_provider_fallback_status_args(params)?,
        "network.mesh.provider_fallback_status",
    )
}

fn run_network_mesh_dns_activation_status(
    params: &NetworkMeshDnsActivationStatusParams,
) -> Result<serde_json::Value, McpError> {
    run_network_surface_json(
        &network_mesh_dns_activation_status_args(params),
        "network.mesh.dns.activation_status",
    )
}

fn synapse_cidr(registry: &serde_json::Value) -> Option<&str> {
    registry
        .get("network_policy")?
        .get("planes")?
        .get("synapse")?
        .get("cidr")?
        .as_str()
}

fn ipv4_to_u32(ip: &str) -> Option<u32> {
    let octets: Vec<u8> = ip
        .split('.')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if octets.len() != 4 {
        return None;
    }
    Some(
        ((octets[0] as u32) << 24)
            | ((octets[1] as u32) << 16)
            | ((octets[2] as u32) << 8)
            | (octets[3] as u32),
    )
}

fn ipv4_in_cidr(ip: &str, cidr: &str) -> bool {
    let Some((base, prefix)) = cidr.split_once('/') else {
        return false;
    };
    let Ok(prefix) = prefix.parse::<u32>() else {
        return false;
    };
    if prefix > 32 {
        return false;
    }
    let Some(ip_value) = ipv4_to_u32(ip) else {
        return false;
    };
    let Some(base_value) = ipv4_to_u32(base) else {
        return false;
    };
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (ip_value & mask) == (base_value & mask)
}

fn network_reservations(registry: &serde_json::Value) -> Vec<&serde_json::Value> {
    registry
        .get("network_policy")
        .and_then(|policy| policy.get("reservations"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .collect()
}

fn validate_network_policy(registry: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut violations = Vec::new();
    let Some(policy) = registry.get("network_policy") else {
        return vec![serde_json::json!({
            "kind": "missing_network_policy",
            "message": "registry missing network_policy"
        })];
    };
    if policy
        .get("canonical_service_plane")
        .and_then(|value| value.as_str())
        != Some("synapse")
    {
        violations.push(serde_json::json!({
            "kind": "invalid_canonical_service_plane",
            "expected": "synapse",
            "actual": policy.get("canonical_service_plane")
        }));
    }
    let Some(cidr) = synapse_cidr(registry) else {
        violations.push(serde_json::json!({
            "kind": "missing_synapse_cidr",
            "message": "network_policy.planes.synapse.cidr is required"
        }));
        return violations;
    };

    let mut seen: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for node in registry_nodes(registry) {
        let Some(name) = node_name(node) else {
            continue;
        };
        let Some(synapse) = node
            .get("network")
            .and_then(|network| network.get("synapse"))
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        if synapse.starts_with('<') {
            continue;
        }
        if !ipv4_in_cidr(synapse, cidr) {
            violations.push(serde_json::json!({
                "kind": "synapse_ip_outside_cidr",
                "node": name,
                "synapse": synapse,
                "cidr": cidr
            }));
        }
        if let Some(previous) = seen.insert(synapse.to_string(), name.to_string()) {
            violations.push(serde_json::json!({
                "kind": "duplicate_synapse_ip",
                "synapse": synapse,
                "nodes": [previous, name]
            }));
        }
        match find_reservation(registry, name) {
            Some(reservation)
                if reservation.get("synapse").and_then(|value| value.as_str()) == Some(synapse) => {
            }
            Some(reservation) => violations.push(serde_json::json!({
                "kind": "reservation_mismatch",
                "node": name,
                "node_synapse": synapse,
                "reservation_synapse": reservation.get("synapse")
            })),
            None => violations.push(serde_json::json!({
                "kind": "missing_reservation",
                "node": name,
                "synapse": synapse
            })),
        }
    }
    violations
}

fn synapse_dns_names(
    node: &serde_json::Value,
    reservation: Option<&serde_json::Value>,
) -> Vec<String> {
    let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if let Some(reservation_names) = reservation
        .and_then(|value| value.get("dns"))
        .and_then(|value| value.as_array())
    {
        for name in reservation_names.iter().filter_map(|value| value.as_str()) {
            names.insert(name.to_string());
        }
    }
    if let Some(name) = node_name(node) {
        names.insert(format!("{name}.synapse.cynic"));
        names.insert(name.to_string());
    }
    for alias in node_aliases(node) {
        names.insert(format!("{alias}.synapse.cynic"));
        names.insert(alias.to_string());
    }
    names.into_iter().collect()
}

fn render_synapse_dns_records(
    registry: &serde_json::Value,
    include_short_names: bool,
) -> Vec<serde_json::Value> {
    registry_nodes(registry)
        .into_iter()
        .filter_map(|node| {
            let name = node_name(node)?;
            let synapse = node
                .get("network")
                .and_then(|network| network.get("synapse"))
                .and_then(|value| value.as_str())?;
            if synapse.starts_with('<') {
                return None;
            }
            let reservation = find_reservation(registry, name);
            let names: Vec<String> = synapse_dns_names(node, reservation)
                .into_iter()
                .filter(|dns| include_short_names || dns.ends_with(".synapse.cynic"))
                .collect();
            Some(serde_json::json!({
                "ip": synapse,
                "names": names,
                "node": name
            }))
        })
        .collect()
}

fn resolve_synapse_dns_record(record: &serde_json::Value) -> serde_json::Value {
    let expected_ip = record
        .get("ip")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let names = record
        .get("names")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .filter(|name| name.ends_with(".synapse.cynic"));
    let checks: Vec<serde_json::Value> = names
        .map(|name| {
            let resolved: Vec<String> = std::net::ToSocketAddrs::to_socket_addrs(&(name, 0))
                .map(|items| {
                    items
                        .map(|addr| addr.ip().to_string())
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default();
            let status = if resolved.iter().any(|ip| ip == expected_ip) {
                "resolved_expected_ip"
            } else if resolved.is_empty() {
                "unresolved"
            } else {
                "resolved_unexpected_ip"
            };
            serde_json::json!({
                "name": name,
                "expected_ip": expected_ip,
                "resolved_ips": resolved,
                "status": status,
            })
        })
        .collect();
    let status = if checks.is_empty() {
        "no_fqdn_names"
    } else if checks.iter().all(|check| {
        check.get("status").and_then(|value| value.as_str()) == Some("resolved_expected_ip")
    }) {
        "ok"
    } else {
        "violation"
    };
    serde_json::json!({
        "node": record.get("node"),
        "ip": expected_ip,
        "status": status,
        "checks": checks,
    })
}

fn network_mesh_status_report(
    registry: &serde_json::Value,
    resolve_dns: bool,
    include_records: bool,
) -> serde_json::Value {
    let violations = validate_network_policy(registry);
    let records = render_synapse_dns_records(registry, true);
    let dns_checks: Vec<serde_json::Value> = if resolve_dns {
        records.iter().map(resolve_synapse_dns_record).collect()
    } else {
        Vec::new()
    };
    let mut missing: Vec<serde_json::Value> = violations
        .iter()
        .map(|violation| {
            serde_json::json!({
                "kind": "network_policy_violation",
                "detail": violation
            })
        })
        .collect();
    if resolve_dns {
        for check in &dns_checks {
            if check.get("status").and_then(|value| value.as_str()) != Some("ok") {
                missing.push(serde_json::json!({
                    "kind": "synapse_dns_resolution",
                    "detail": check
                }));
            }
        }
    }
    let policy_status = if violations.is_empty() {
        "ok"
    } else {
        "violation"
    };
    let dns_status = if !resolve_dns {
        "not_checked"
    } else if dns_checks
        .iter()
        .all(|check| check.get("status").and_then(|value| value.as_str()) == Some("ok"))
    {
        "ok"
    } else {
        "violation"
    };
    serde_json::json!({
        "schema": "cynic.infrastructure_mcp.network_mesh_status.v1",
        "zone": "synapse.cynic",
        "registry_ref": "infra/registry.json",
        "policy_status": policy_status,
        "dns_status": dns_status,
        "runtime_status": "not_observed",
        "resolve_dns": resolve_dns,
        "peers": records.len(),
        "missing": missing,
        "offline": [],
        "dns_checks": dns_checks,
        "records": if include_records { serde_json::json!(records) } else { serde_json::Value::Null },
    })
}
