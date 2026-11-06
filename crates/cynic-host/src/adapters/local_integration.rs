//! Local observation adapter for ecosystem integrations.
//!
//! Commands are argv-only and injected through [`ArgvRunner`]. The adapter
//! reports credential presence and account identity, never credential values.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::ecosystem_package::{
    ConnectionState, IntegrationApplyReport, IntegrationConnectReport,
    IntegrationObservationContext, IntegrationObservationPort, IntegrationPrincipal,
    IntegrationSpec, IntegrationsPlanReport, IntegrationsStatusReport, RawIntegrationObservation,
    connect_integration, observe_integrations, plan_integrations,
};
use crate::project_root::current_project_root;
use cynic_registry::runtime_authority::default_runtime_principal_layout;
use cynic_process::{ArgvOutput, ArgvRunner, LocalArgvRunner};

pub struct LocalIntegrationObservationAdapter<'a> {
    runner: &'a dyn ArgvRunner,
    project_root: Option<PathBuf>,
}

impl<'a> LocalIntegrationObservationAdapter<'a> {
    pub fn new(runner: &'a dyn ArgvRunner) -> Self {
        Self {
            runner,
            project_root: current_project_root(),
        }
    }

    #[cfg(test)]
    fn with_project_root(runner: &'a dyn ArgvRunner, project_root: PathBuf) -> Self {
        Self {
            runner,
            project_root: Some(project_root),
        }
    }

    pub fn context(&self) -> IntegrationObservationContext {
        let declared = default_runtime_principal_layout();
        let runtime_user =
            std::env::var("CYNIC_RUNTIME_USER").unwrap_or_else(|_| declared.runtime_user.clone());
        let runtime_home = std::env::var("CYNIC_PRINCIPAL_HOME")
            .unwrap_or_else(|_| declared.home_dir.to_string_lossy().into_owned());
        let operator_user = std::env::var("USER")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| {
                let output = self.runner.run(&["id", "-un"]);
                output.success().then(|| output.stdout.trim().to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        IntegrationObservationContext {
            runtime_user,
            runtime_home,
            operator_user,
        }
    }

    fn observe_forgejo(&self) -> RawIntegrationObservation {
        let Some(endpoint) = self.forgejo_endpoint() else {
            return RawIntegrationObservation::missing(
                "Forgejo endpoint is absent from CYNIC_FORGEJO_URL and infra/repo-remotes.json",
            );
        };
        let version_url = format!("{endpoint}/api/v1/version");
        let argv = ["curl", "-sS", "-m", "3", version_url.as_str()];
        let output = self.runner.run(&argv);
        if !output.success() {
            return RawIntegrationObservation {
                present: false,
                path_or_endpoint: Some(endpoint),
                version: None,
                observed_principal: Some(IntegrationPrincipal::SystemManaged),
                principal_name: Some("forgejo-service".to_string()),
                connection: ConnectionState::Unknown,
                account: None,
                credential_source: None,
                notes: vec![concise_failure("Forgejo health probe failed", &output)],
            };
        }

        RawIntegrationObservation {
            present: true,
            path_or_endpoint: Some(endpoint),
            version: json_string(&output.stdout, "version"),
            observed_principal: Some(IntegrationPrincipal::SystemManaged),
            principal_name: Some("forgejo-service".to_string()),
            connection: ConnectionState::NotApplicable,
            account: None,
            credential_source: None,
            notes: vec![
                "service reachability is proven; repository authentication is a separate adoption step"
                    .to_string(),
            ],
        }
    }

    fn observe_github(&self, context: &IntegrationObservationContext) -> RawIntegrationObservation {
        let Some(path) = self.operator_binary("gh") else {
            return RawIntegrationObservation::missing("GitHub CLI `gh` was not found on PATH");
        };
        let version_output = self.run_path(&path, &["--version"]);
        let auth_output = self.run_path(&path, &["auth", "status"]);
        let connected = auth_output.success();

        RawIntegrationObservation {
            present: true,
            path_or_endpoint: Some(path),
            version: first_nonempty_line(&version_output.stdout),
            observed_principal: Some(IntegrationPrincipal::OperatorLocal),
            principal_name: Some(context.operator_user.clone()),
            connection: if connected {
                ConnectionState::Connected
            } else {
                ConnectionState::Disconnected
            },
            account: connected
                .then(|| parse_github_account(&auth_output))
                .flatten(),
            credential_source: connected.then(|| "provider_native_store_or_env".to_string()),
            notes: (!connected)
                .then(|| concise_failure("GitHub authentication is unavailable", &auth_output))
                .into_iter()
                .collect(),
        }
    }

    fn observe_huggingface(
        &self,
        context: &IntegrationObservationContext,
    ) -> RawIntegrationObservation {
        let runtime_candidate = Path::new(&context.runtime_home).join(".local/bin/hf");
        if runtime_candidate.is_file() {
            return self.observe_runtime_hf(&runtime_candidate, context);
        }

        let Some(path) = self.operator_binary("hf") else {
            return RawIntegrationObservation::missing(
                "Hugging Face CLI `hf` was not found for the runtime principal or operator",
            );
        };
        let version_output = self.run_path(&path, &["version"]);
        let auth_output = self.run_path(&path, &["auth", "whoami", "--format", "json"]);
        let connected = auth_output.success();

        RawIntegrationObservation {
            present: true,
            path_or_endpoint: Some(path),
            version: parse_hf_version(&version_output.stdout),
            observed_principal: Some(IntegrationPrincipal::OperatorLocal),
            principal_name: Some(context.operator_user.clone()),
            connection: if connected {
                ConnectionState::Connected
            } else {
                ConnectionState::Disconnected
            },
            account: connected
                .then(|| parse_hf_account(&auth_output.stdout))
                .flatten(),
            credential_source: connected.then(|| "provider_native_store_or_env".to_string()),
            notes: vec![format!(
                "operator discovery does not satisfy runtime principal `{}`",
                context.runtime_user
            )],
        }
    }

    fn observe_runtime_hf(
        &self,
        path: &Path,
        context: &IntegrationObservationContext,
    ) -> RawIntegrationObservation {
        let path_text = path.to_string_lossy().into_owned();
        let (version_output, auth_output) = if context.operator_user == context.runtime_user {
            (
                self.run_path(&path_text, &["version"]),
                self.run_path(&path_text, &["auth", "whoami", "--format", "json"]),
            )
        } else {
            (
                self.run_as_runtime(&path_text, &["version"], context),
                self.run_as_runtime(&path_text, &["auth", "whoami", "--format", "json"], context),
            )
        };
        let executable_as_runtime = version_output.success();
        let connected = auth_output.success();
        let mut notes = Vec::new();
        if !executable_as_runtime {
            notes.push(concise_failure(
                "runtime-principal execution is unavailable",
                &version_output,
            ));
        }

        RawIntegrationObservation {
            present: true,
            path_or_endpoint: Some(path_text),
            version: parse_hf_version(&version_output.stdout),
            observed_principal: executable_as_runtime.then_some(IntegrationPrincipal::RuntimeUser),
            principal_name: executable_as_runtime.then(|| context.runtime_user.clone()),
            connection: if !executable_as_runtime {
                ConnectionState::Unknown
            } else if connected {
                ConnectionState::Connected
            } else {
                ConnectionState::Disconnected
            },
            account: connected
                .then(|| parse_hf_account(&auth_output.stdout))
                .flatten(),
            credential_source: connected.then(|| "provider_native_store_or_env".to_string()),
            notes,
        }
    }

    fn operator_binary(&self, name: &str) -> Option<String> {
        let output = self.runner.run(&["which", name]);
        if !output.success() {
            return None;
        }
        output
            .stdout
            .lines()
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn run_path(&self, path: &str, arguments: &[&str]) -> ArgvOutput {
        let mut argv = Vec::with_capacity(arguments.len() + 1);
        argv.push(path);
        argv.extend_from_slice(arguments);
        self.runner.run(&argv)
    }

    fn run_as_runtime(
        &self,
        path: &str,
        arguments: &[&str],
        context: &IntegrationObservationContext,
    ) -> ArgvOutput {
        let home = format!("HOME={}", context.runtime_home);
        let mut owned = vec!["sudo".to_string()];
        if std::env::var_os("HF_TOKEN").is_some() {
            owned.push("--preserve-env=HF_TOKEN".to_string());
        }
        owned.extend([
            "-n".to_string(),
            "-u".to_string(),
            context.runtime_user.clone(),
            "--".to_string(),
            "env".to_string(),
            home,
            path.to_string(),
        ]);
        owned.extend(arguments.iter().map(|argument| (*argument).to_string()));
        let borrowed = owned.iter().map(String::as_str).collect::<Vec<_>>();
        self.runner.run(&borrowed)
    }

    fn forgejo_endpoint(&self) -> Option<String> {
        std::env::var("CYNIC_FORGEJO_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .and_then(|value| base_url(&value))
            .or_else(|| {
                let root = self.project_root.as_deref()?;
                let raw = std::fs::read_to_string(root.join("infra/repo-remotes.json")).ok()?;
                let document: Value = serde_json::from_str(&raw).ok()?;
                let remote =
                    document.get("remotes")?.as_array()?.iter().find(|remote| {
                        remote.get("id").and_then(Value::as_str) == Some("forgejo")
                    })?;
                let service_ref: ServiceRef = serde_json::from_value(
                    remote.get("admin_service_ref")?.clone(),
                )
                .ok()?;
                resolve_service_ref(&root.join("infra/registry.json"), &service_ref)
                    .ok()
                    .map(|resolved| resolved.endpoint)
            })
    }
}

impl IntegrationObservationPort for LocalIntegrationObservationAdapter<'_> {
    fn observe(
        &self,
        spec: &IntegrationSpec,
        context: &IntegrationObservationContext,
    ) -> RawIntegrationObservation {
        match spec.id {
            "forgejo" => self.observe_forgejo(),
            "github" => self.observe_github(context),
            "huggingface" => self.observe_huggingface(context),
            other => RawIntegrationObservation::missing(format!(
                "no local integration adapter is registered for `{other}`"
            )),
        }
    }
}

pub fn observe_local_integrations() -> IntegrationsStatusReport {
    let runner = LocalArgvRunner;
    let adapter = LocalIntegrationObservationAdapter::new(&runner);
    observe_integrations(&adapter, &adapter.context())
}

pub fn plan_local_integrations() -> IntegrationsPlanReport {
    let runner = LocalArgvRunner;
    let adapter = LocalIntegrationObservationAdapter::new(&runner);
    plan_integrations(&adapter, &adapter.context())
}

pub fn connect_local_integration(
    id: &str,
    apply: bool,
) -> Result<IntegrationConnectReport, String> {
    let runner = LocalArgvRunner;
    let adapter = LocalIntegrationObservationAdapter::new(&runner);
    connect_integration(id, apply, &runner, &adapter, &adapter.context())
}

pub fn apply_local_integration(id: &str, apply: bool) -> Result<IntegrationApplyReport, String> {
    let runner = LocalArgvRunner;
    let adapter = LocalIntegrationObservationAdapter::new(&runner);
    crate::ecosystem_package::integration_apply::apply_integration(
        id,
        apply,
        &runner,
        &adapter,
        &adapter.context(),
    )
}

fn base_url(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches('/');
    let (scheme, remainder) = trimmed.split_once("://")?;
    let authority = remainder.split('/').next()?.trim();
    (!scheme.is_empty() && !authority.is_empty()).then(|| format!("{scheme}://{authority}"))
}

fn json_string(raw: &str, key: &str) -> Option<String> {
    serde_json::from_str::<Value>(raw)
        .ok()?
        .get(key)?
        .as_str()
        .map(str::to_string)
}

fn first_nonempty_line(raw: &str) -> Option<String> {
    raw.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn parse_hf_version(raw: &str) -> Option<String> {
    json_string(raw, "version").or_else(|| {
        first_nonempty_line(raw).map(|line| {
            line.strip_prefix("version=")
                .unwrap_or(line.as_str())
                .to_string()
        })
    })
}

fn parse_hf_account(raw: &str) -> Option<String> {
    let document = serde_json::from_str::<Value>(raw).ok()?;
    document
        .get("user")
        .or_else(|| document.get("name"))?
        .as_str()
        .map(str::to_string)
}

fn parse_github_account(output: &ArgvOutput) -> Option<String> {
    output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .find_map(|line| {
            let (_, suffix) = line.split_once("account ")?;
            suffix.split_whitespace().next().map(str::to_string)
        })
}

fn concise_failure(prefix: &str, output: &ArgvOutput) -> String {
    let detail = output
        .spawn_error
        .clone()
        .or_else(|| first_nonempty_line(&output.stderr))
        .unwrap_or_else(|| format!("exit_code={:?}", output.exit_code));
    format!("{prefix}: {detail}")
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct ServiceRef {
    node: String,
    service: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct ResolvedServiceEndpoint {
    #[allow(dead_code)]
    service_ref: ServiceRef,
    #[allow(dead_code)]
    scheme: String,
    #[allow(dead_code)]
    host: String,
    #[allow(dead_code)]
    port: u16,
    endpoint: String,
}

#[derive(serde::Deserialize)]
struct Registry {
    network_policy: Option<NetworkPolicy>,
    nodes: Option<Vec<Node>>,
}

#[derive(serde::Deserialize)]
struct NetworkPolicy {
    reservations: Option<Vec<Reservation>>,
}

#[derive(serde::Deserialize)]
struct Reservation {
    name: Option<String>,
    dns: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct Node {
    id: Option<String>,
    name: Option<String>,
    services: Option<Vec<Service>>,
}

#[derive(serde::Deserialize)]
struct Service {
    name: Option<String>,
    scheme: Option<String>,
    port: Option<u16>,
}

fn resolve_service_ref(
    registry_path: &std::path::Path,
    reference: &ServiceRef,
) -> Result<ResolvedServiceEndpoint, String> {
    let raw = std::fs::read_to_string(registry_path)
        .map_err(|error| format!("read {}: {error}", registry_path.display()))?;
    let registry: Registry = serde_json::from_str(&raw)
        .map_err(|error| format!("parse {}: {error}", registry_path.display()))?;
    let nodes = registry.nodes.unwrap_or_default();
    let node = nodes
        .iter()
        .find(|node| {
            node.id.as_deref() == Some(reference.node.as_str())
                || node.name.as_deref() == Some(reference.node.as_str())
        })
        .ok_or_else(|| format!("service node not declared: {}", reference.node))?;
    let service = node
        .services
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|service| service.name.as_deref() == Some(reference.service.as_str()))
        .ok_or_else(|| {
            format!(
                "service not declared: {}/{}",
                reference.node, reference.service
            )
        })?;
    let scheme = required(service.scheme.clone(), "service.scheme")?;
    let port = service
        .port
        .ok_or_else(|| "service.port missing".to_string())?;
    let host = registry
        .network_policy
        .and_then(|policy| policy.reservations)
        .unwrap_or_default()
        .into_iter()
        .find(|reservation| reservation.name.as_deref() == Some(reference.node.as_str()))
        .and_then(|reservation| reservation.dns)
        .and_then(|names| names.into_iter().find(|name| !name.trim().is_empty()))
        .ok_or_else(|| format!("service DNS reservation missing: {}", reference.node))?;
    let endpoint = if default_port(&scheme) == Some(port) {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}:{port}")
    };
    Ok(ResolvedServiceEndpoint {
        service_ref: reference.clone(),
        scheme,
        host,
        port,
        endpoint,
    })
}

fn required(value: Option<String>, field: &str) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} missing"))
}

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        "ssh" => Some(22),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_process::MapArgvRunner;

    #[test]
    fn repository_url_is_reduced_to_service_endpoint() {
        assert_eq!(
            base_url("http://10.10.10.101:3000/owner/CYNIC.git").as_deref(),
            Some("http://10.10.10.101:3000")
        );
    }

    #[test]
    fn github_account_parser_discards_keyring_details() {
        let output = ArgvOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: "Logged in to github.com account zeyxx (keyring)\n".to_string(),
            spawn_error: None,
        };
        assert_eq!(parse_github_account(&output).as_deref(), Some("zeyxx"));
    }

    #[test]
    fn huggingface_version_prefix_is_normalized() {
        assert_eq!(
            parse_hf_version("version=1.17.0\n").as_deref(),
            Some("1.17.0")
        );
    }

    #[test]
    fn forgejo_endpoint_comes_from_runtime_remote_authority() {
        // Hermetic: the live workspace registry no longer declares a
        // forgejo-api-tls service, so reading the real registry makes this
        // test track infra drift instead of the authority wiring. Stage a
        // minimal authority pair and prove the endpoint is resolved from
        // it — never hardcoded, never None-by-accident.
        let root = tempfile::tempdir().expect("tempdir");
        let infra = root.path().join("infra");
        std::fs::create_dir_all(&infra).expect("infra dir");
        std::fs::write(
            infra.join("registry.json"),
            serde_json::json!({
                "nodes": [{
                    "id": "forgejo",
                    "services": [
                        {"name": "forgejo-api-tls", "scheme": "https", "port": 443},
                        {"name": "ssh", "scheme": "ssh", "port": 22}
                    ]
                }],
                "network_policy": {
                    "reservations": [
                        {"name": "forgejo", "dns": ["forgejo.service.cynic"]}
                    ]
                }
            })
            .to_string(),
        )
        .expect("registry fixture");
        std::fs::write(
            infra.join("repo-remotes.json"),
            serde_json::json!({
                "remotes": [{
                    "id": "forgejo",
                    "owner": "jeanterre13",
                    "repository": "CYNIC",
                    "git_user": "forgejo",
                    "machine_principal": "forgejo",
                    "repository_permission": "read",
                    "machine_key_fingerprint": "SHA256:test",
                    "admin_service_ref": {"node": "forgejo", "service": "forgejo-api-tls"},
                    "git_service_ref": {"node": "forgejo", "service": "ssh"}
                }]
            })
            .to_string(),
        )
        .expect("remotes fixture");
        let runner = MapArgvRunner::default();
        let adapter = LocalIntegrationObservationAdapter::with_project_root(
            &runner,
            root.path().to_path_buf(),
        );
        assert_eq!(
            adapter.forgejo_endpoint().as_deref(),
            Some("https://forgejo.service.cynic")
        );
    }
}
