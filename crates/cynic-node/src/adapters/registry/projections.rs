use std::collections::{BTreeMap, BTreeSet};

use super::model::{Registry, RegistryNode};

pub(super) fn canonical_names_for(
    dns_zone: &str,
    primary_name: &str,
    aliases: &[String],
    reserved_host: Option<String>,
) -> Vec<String> {
    let mut names = BTreeSet::new();
    names.insert(format!("{primary_name}.{dns_zone}"));
    for alias in aliases {
        if !alias.is_empty() {
            names.insert(format!("{alias}.{dns_zone}"));
        }
    }
    if let Some(host) = reserved_host.filter(|value| !value.is_empty()) {
        names.insert(host);
    }
    names.into_iter().collect()
}

pub(super) fn preferred_node_host(
    dns_zone: &str,
    node: &RegistryNode,
    reserved_host: Option<String>,
    local_ip: &str,
) -> String {
    if let Some(host) = reserved_host {
        return host;
    }
    if let Some(name) = node.name.as_deref() {
        return format!("{name}.{dns_zone}");
    }
    local_ip.to_string()
}

pub(super) fn vm_aliases_by_name(nodes: &[RegistryNode]) -> BTreeMap<String, Vec<String>> {
    let mut aliases = BTreeMap::new();
    for node in nodes {
        for vm in node.vms.as_deref().unwrap_or(&[]) {
            let Some(name) = vm.name.clone().filter(|value| !value.is_empty()) else {
                continue;
            };
            let entry = aliases.entry(name).or_insert_with(Vec::new);
            for alias in vm.aliases.clone().unwrap_or_default() {
                if !alias.is_empty() && !entry.contains(&alias) {
                    entry.push(alias);
                }
            }
        }
    }
    aliases
}

pub(super) fn reservation_hosts_by_name(
    registry: &Registry,
    dns_zone: &str,
) -> BTreeMap<String, String> {
    let mut hosts = BTreeMap::new();
    for reservation in registry
        .network_policy
        .as_ref()
        .and_then(|policy| policy.reservations.as_ref())
        .into_iter()
        .flatten()
    {
        let Some(name) = reservation.name.clone().filter(|value| !value.is_empty()) else {
            continue;
        };
        let preferred = reservation
            .dns
            .clone()
            .unwrap_or_default()
            .into_iter()
            .find(|value| value.ends_with(dns_zone));
        if let Some(host) = preferred {
            hosts.insert(name, host);
        }
    }
    hosts
}
