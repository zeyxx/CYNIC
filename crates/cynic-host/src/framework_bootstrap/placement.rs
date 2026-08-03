use serde_json::Value;

pub(super) fn unique_role_node<'a>(
    nodes: &'a [Value],
    role: &str,
    blocker_prefix: &str,
) -> Result<&'a Value, String> {
    let matches = nodes
        .iter()
        .filter(|node| node.get("role").and_then(Value::as_str) == Some(role))
        .collect::<Vec<_>>();
    let [node] = matches.as_slice() else {
        return Err(format!("{blocker_prefix}_node_not_unique"));
    };
    Ok(node)
}

pub(super) fn ensure_local_placement(
    nodes: &[Value],
    role_node: &Value,
    observed_hostname: &str,
    blocker_prefix: &str,
) -> Result<(), String> {
    let selector = role_node
        .get("host_node_id")
        .and_then(Value::as_str)
        .or_else(|| role_node.get("id").and_then(Value::as_str))
        .or_else(|| role_node.get("name").and_then(Value::as_str))
        .ok_or_else(|| format!("{blocker_prefix}_host_placement_missing"))?;
    let placements = nodes
        .iter()
        .filter(|node| node_matches(node, selector))
        .collect::<Vec<_>>();
    let [placement] = placements.as_slice() else {
        return Err(format!("{blocker_prefix}_host_placement_not_unique"));
    };
    if observed_hostname.trim().is_empty() || !node_matches(placement, observed_hostname) {
        return Err(format!("{blocker_prefix}_host_identity_mismatch"));
    }
    Ok(())
}

fn node_matches(node: &Value, value: &str) -> bool {
    node.get("id").and_then(Value::as_str) == Some(value)
        || node.get("name").and_then(Value::as_str) == Some(value)
        || node
            .get("aliases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|alias| alias.as_str() == Some(value))
}
