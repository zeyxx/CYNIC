use super::{FleetNodeObservation, ProviderError};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::process::Command;

pub(crate) struct KubernetesInventoryProvider {
    kubectl_bin: String,
    namespace: Option<String>,
    context: Option<String>,
    discover_nodes: bool,
}

impl KubernetesInventoryProvider {
    pub(crate) fn from_env() -> Option<Self> {
        let kubectl_bin = std::env::var("CYNIC_KUBECTL_BIN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "kubectl".to_string());
        if kubectl_bin.trim().is_empty() {
            return None;
        }

        Some(Self {
            kubectl_bin,
            namespace: std::env::var("CYNIC_K8S_NAMESPACE").ok(),
            context: std::env::var("CYNIC_K8S_CONTEXT").ok(),
            discover_nodes: super::super::bool_flag("CYNIC_K8S_DISCOVER_NODES").unwrap_or(true),
        })
    }

    async fn run_kubectl_json(&self, args: &[&str]) -> Result<Value, ProviderError> {
        let mut command = Command::new(&self.kubectl_bin);
        if let Some(context) = &self.context {
            command.arg("--context").arg(context);
        }
        if let Some(namespace) = &self.namespace {
            command.arg("--namespace").arg(namespace);
        }
        command.args(args).arg("-o").arg("json");

        let output = command
            .output()
            .await
            .map_err(|err| kubectl_error(&self.kubectl_bin, args, &err))?;

        if !output.status.success() {
            return Err(kubectl_status_error(&self.kubectl_bin, args, &output));
        }

        parse_kubectl_stdout(&output)
    }

    fn parse_nodes(body: &Value) -> Vec<FleetNodeObservation> {
        body.get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|node| {
                        let name = node.get("metadata")?.get("name")?.as_str()?;
                        let phase = node
                            .get("status")
                            .and_then(|s| s.get("phase"))
                            .and_then(Value::as_str);
                        let conditions = node
                            .get("status")
                            .and_then(|s| s.get("conditions"))
                            .and_then(Value::as_array)
                            .map(|items| Value::Array(items.clone()));

                        let status = Self::parse_node_status(conditions.as_ref(), phase);
                        let liveness = Self::node_liveness_from_status(&status).to_string();
                        let ip = node
                            .get("status")
                            .and_then(|s| s.get("addresses"))
                            .and_then(Value::as_array)
                            .and_then(|addresses| {
                                addresses.iter().find_map(|entry| {
                                    match entry.get("type").and_then(Value::as_str) {
                                        Some("InternalIP") | Some("ExternalIP") => entry
                                            .get("address")
                                            .and_then(Value::as_str)
                                            .map(ToString::to_string),
                                        _ => None,
                                    }
                                })
                            });

                        Some(FleetNodeObservation {
                            provider: "kubernetes".to_string(),
                            // A Kubernetes node is its own execution host;
                            // the adapter, not the protocol, supplies that
                            // substrate-specific relationship.
                            provider_node: Some(name.to_string()),
                            node_id: format!("k8s/node/{name}"),
                            name: name.to_string(),
                            status,
                            liveness,
                            ip,
                            desired: false,
                            observed: true,
                            meta: Some(BTreeMap::from([
                                ("provider".to_string(), "kubernetes".to_string()),
                                ("api".to_string(), "kubectl".to_string()),
                                ("resource".to_string(), "nodes".to_string()),
                            ])),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_node_status(conditions: Option<&Value>, phase: Option<&str>) -> String {
        if let Some(conditions) = conditions.and_then(Value::as_array)
            && conditions.iter().any(|condition| {
                condition.get("type").and_then(Value::as_str) == Some("Ready")
                    && condition
                        .get("status")
                        .and_then(Value::as_str)
                        .is_some_and(|status| status == "True")
            })
        {
            return "ready".to_string();
        }

        phase.unwrap_or("unknown").to_string()
    }

    fn node_liveness_from_status(status: &str) -> &str {
        match status {
            "ready" | "Running" | "up" => "up",
            _ => "down",
        }
    }
}

impl KubernetesInventoryProvider {
    pub(crate) async fn collect_nodes(
        &self,
        _declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
        if !self.discover_nodes {
            tracing::debug!("kubernetes provider has node discovery disabled");
            return Ok(Vec::new());
        }

        let nodes = self.run_kubectl_json(&["get", "nodes"]).await?;
        Ok(Self::parse_nodes(&nodes))
    }
}

fn parse_kubectl_stdout(output: &std::process::Output) -> Result<Value, ProviderError> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(serde_json::json!({
            "kind": "List",
            "apiVersion": "v1",
            "items": []
        }));
    }

    serde_json::from_str::<Value>(&stdout).map_err(|error| {
        format!(
            "kubectl returned invalid JSON: {error} (stderr: {})",
            String::from_utf8_lossy(&output.stderr),
        )
    })
}

fn kubectl_error(bin: &str, args: &[&str], err: &std::io::Error) -> String {
    format!("{bin} {args:?}: {err}")
}

fn kubectl_status_error(bin: &str, args: &[&str], output: &std::process::Output) -> String {
    format!(
        "{bin} {args:?} exited {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )
}
