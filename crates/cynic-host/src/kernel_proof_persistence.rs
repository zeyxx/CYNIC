use cynic_protocol::RuntimeProofPersistenceRecord;
use serde::Serialize;
use serde_json::Value;

use crate::update_lifecycle::ArgvRunner;

pub const KERNEL_PROOF_PERSISTENCE_REPORT_SCHEMA: &str =
    "cynic.host.kernel_proof_persistence_report.v1";

#[derive(Debug, Clone, Serialize)]
pub struct KernelProofPersistenceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub endpoint: String,
    pub target: String,
    pub post_success: bool,
    pub read_back_success: bool,
    pub commands: Vec<KernelPersistenceCommandProof>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KernelPersistenceCommandProof {
    pub operation: &'static str,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait KernelProofPersistencePort {
    fn persist(&self, record: &RuntimeProofPersistenceRecord) -> KernelProofPersistenceReport;
}

pub struct CurlKernelProofPersistenceAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    endpoint: String,
    bearer_token: Option<String>,
}

impl<'a, R: ArgvRunner> CurlKernelProofPersistenceAdapter<'a, R> {
    pub fn new(runner: &'a R, endpoint: impl Into<String>, bearer_token: Option<String>) -> Self {
        Self {
            runner,
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            bearer_token,
        }
    }

    fn headers(&self) -> Vec<String> {
        let mut headers = vec!["Content-Type: application/json".to_string()];
        if let Some(token) = &self.bearer_token {
            headers.push(format!("Authorization: Bearer {token}"));
        }
        headers
    }

    fn curl(&self, operation: &'static str, args: &[String]) -> KernelPersistenceCommandProof {
        let mut owned = vec!["curl".to_string(), "-sS".to_string()];
        owned.extend(args.iter().cloned());
        let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        KernelPersistenceCommandProof {
            operation,
            success: output.success(),
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.spawn_error.unwrap_or(output.stderr),
        }
    }
}

impl<'a, R: ArgvRunner> KernelProofPersistencePort for CurlKernelProofPersistenceAdapter<'a, R> {
    fn persist(&self, record: &RuntimeProofPersistenceRecord) -> KernelProofPersistenceReport {
        let target = format!("runtime-proof/{}/{}", record.node, record.revision);
        let body = serde_json::json!({
            "tool": "cynic_runtime_proof",
            "target": target,
            "domain": "cynic.runtime.proof",
            "status": record.status,
            "context": serde_json::to_string(record).unwrap_or_default(),
            "project": "CYNIC",
            "agent_id": "cynic-host",
            "tags": ["runtime", "proof", "conformance"],
            "value": record,
            "confidence": "observed"
        });
        let body = serde_json::to_string(&body).unwrap_or_default();
        let mut post_args = vec!["-X".to_string(), "POST".to_string()];
        for header in self.headers() {
            post_args.extend(["-H".to_string(), header]);
        }
        post_args.extend([
            "--data-raw".to_string(),
            body,
            format!("{}/observe", self.endpoint),
        ]);
        let post = self.curl("post", &post_args);
        let mut blockers = Vec::new();
        if !post.success {
            blockers.push("kernel_proof_post_failed".to_string());
        } else if !post.stdout.contains("\"status\":\"observed\"") {
            blockers.push("kernel_proof_post_ack_missing".to_string());
        }

        let mut get_args = vec!["-G".to_string(), format!("{}/observations", self.endpoint)];
        get_args.extend([
            "--data-urlencode".to_string(),
            "domain=cynic.runtime.proof".to_string(),
        ]);
        get_args.extend(["--data-urlencode".to_string(), format!("target={target}")]);
        get_args.extend(["--data-urlencode".to_string(), "limit=100".to_string()]);
        for header in self.headers() {
            get_args.extend(["-H".to_string(), header]);
        }
        let read_back = self.curl("read_back", &get_args);
        let read_back_success =
            read_back.success && contains_persisted_record(&read_back.stdout, record);
        if !read_back_success {
            blockers.push("kernel_proof_read_back_missing".to_string());
        }
        KernelProofPersistenceReport {
            schema: KERNEL_PROOF_PERSISTENCE_REPORT_SCHEMA,
            status: if blockers.is_empty() {
                "proved"
            } else {
                "blocked"
            },
            endpoint: self.endpoint.clone(),
            target,
            post_success: post.success,
            read_back_success,
            commands: vec![post, read_back],
            blockers,
        }
    }
}

fn contains_persisted_record(raw: &str, record: &RuntimeProofPersistenceRecord) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    contains_record_value(&value, record)
}

fn contains_record_value(value: &Value, record: &RuntimeProofPersistenceRecord) -> bool {
    if value.get("schema").and_then(Value::as_str) == Some(record.schema.as_str())
        && value.get("node").and_then(Value::as_str) == Some(record.node.as_str())
        && value.get("revision").and_then(Value::as_str) == Some(record.revision.as_str())
    {
        return true;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .any(|value| contains_record_value(value, record)),
        Value::Object(values) => values
            .values()
            .any(|value| contains_record_value(value, record)),
        _ => false,
    }
}

/// Port for chaining a host proof submission into the kernel Proof-of-History ledger.
pub trait KernelProofHistoryPort {
    fn persist_poh(
        &self,
        submission: &cynic_protocol::HostProofSubmission,
    ) -> KernelProofPersistenceReport;
}

pub struct CurlKernelProofHistoryAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    endpoint: String,
    bearer_token: Option<String>,
}

impl<'a, R: ArgvRunner> CurlKernelProofHistoryAdapter<'a, R> {
    pub fn new(runner: &'a R, endpoint: impl Into<String>, bearer_token: Option<String>) -> Self {
        Self {
            runner,
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            bearer_token,
        }
    }

    fn headers(&self) -> Vec<String> {
        let mut headers = vec!["Content-Type: application/json".to_string()];
        if self.bearer_token.is_some() {
            headers.push("Authorization: Bearer ***".to_string());
        }
        headers
    }

    fn curl(&self, operation: &'static str, args: &[String]) -> KernelPersistenceCommandProof {
        let mut owned = vec!["curl".to_string(), "-sS".to_string()];
        owned.extend(args.iter().cloned());
        let refs = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        KernelPersistenceCommandProof {
            operation,
            success: output.success(),
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.spawn_error.unwrap_or(output.stderr),
        }
    }
}

impl<'a, R: ArgvRunner> KernelProofHistoryPort for CurlKernelProofHistoryAdapter<'a, R> {
    fn persist_poh(
        &self,
        submission: &cynic_protocol::HostProofSubmission,
    ) -> KernelProofPersistenceReport {
        let body = serde_json::json!({
            "submission": submission,
        });
        let body = serde_json::to_string(&body).unwrap_or_default();
        let mut post_args = vec!["-X".to_string(), "POST".to_string()];
        for header in self.headers() {
            post_args.extend(["-H".to_string(), header]);
        }
        post_args.extend([
            "--data-raw".to_string(),
            body,
            format!("{}/proof-of-history", self.endpoint),
        ]);
        let post = self.curl("post", &post_args);
        let mut blockers = Vec::new();
        if !post.success {
            blockers.push("kernel_proof_history_post_failed".to_string());
        } else if !post.stdout.contains("\"status\":\"recorded\"") {
            blockers.push("kernel_proof_history_ack_missing".to_string());
        }

        let mut get_args = vec![
            "-G".to_string(),
            format!("{}/proof-of-history", self.endpoint),
        ];
        get_args.extend(["--data-urlencode".to_string(), "limit=1".to_string()]);
        for header in self.headers() {
            get_args.extend(["-H".to_string(), header]);
        }
        let read_back = self.curl("read_back", &get_args);
        let read_back_success = read_back.success
            && read_back
                .stdout
                .contains(&format!("\"event_type\":\"{}\"", submission.event_type));
        if !read_back_success {
            blockers.push("kernel_proof_history_read_back_missing".to_string());
        }

        KernelProofPersistenceReport {
            schema: KERNEL_PROOF_PERSISTENCE_REPORT_SCHEMA,
            status: if blockers.is_empty() {
                "proved"
            } else {
                "blocked"
            },
            endpoint: self.endpoint.clone(),
            target: submission.event_type.clone(),
            post_success: post.success,
            read_back_success,
            commands: vec![post, read_back],
            blockers,
        }
    }
}
