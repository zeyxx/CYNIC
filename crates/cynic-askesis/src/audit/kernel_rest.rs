use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use crate::audit::AuditEngine;
use crate::log::ObservationEvent;
use crate::reflection::{Reflection, Verdict};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize)]
struct JudgeRequest {
    content: String,
    context: Option<String>,
    domain: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JudgeResponse {
    verdict: String,
    q_score: QScore,
    reasoning: Reasoning,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QScore {
    total: f32,
    phi: f32,
}

#[derive(Debug, Deserialize)]
struct Reasoning {
    phi: String,
    culture: String,
}

#[derive(Debug)]
pub struct KernelRestAudit {
    client: Client,
    endpoint: String,
    api_key: String,
}

impl Default for KernelRestAudit {
    fn default() -> Self {
        let host =
            env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "http://localhost:3030".to_string());
        let api_key = env::var("CYNIC_API_KEY").unwrap_or_default();
        Self {
            client: Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .unwrap_or_default(),
            endpoint: format!("{host}/judge"),
            api_key,
        }
    }
}

impl KernelRestAudit {
    fn build_content(logs: &[ObservationEvent], questions: &[&str]) -> String {
        let mut p = String::new();
        p.push_str("=== LOGS ===\n");
        for e in logs {
            p.push_str(&format!(
                "[{}] domain={}: {}\n",
                chrono::DateTime::from_timestamp_millis(e.timestamp)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| e.timestamp.to_string()),
                e.domain.as_deref().unwrap_or("(none)"),
                e.content
            ));
        }
        p.push_str("\n=== DIRECTIVES ===\n");
        p.push_str("Base your reflection on these axiomatic directives:\n");
        for q in questions {
            p.push_str(&format!("- {q}\n"));
        }
        p
    }
}

#[async_trait]
impl AuditEngine for KernelRestAudit {
    async fn audit(
        &self,
        logs: &[ObservationEvent],
        questions: &[&str],
    ) -> crate::Result<Reflection> {
        let content = Self::build_content(logs, questions);

        let req = JudgeRequest {
            content,
            context: Some("Askesis periodic audit reflection".to_string()),
            domain: Some("askesis-reflection".to_string()),
        };

        let mut request_builder = self.client.post(&self.endpoint).json(&req);
        if !self.api_key.is_empty() {
            request_builder = request_builder.bearer_auth(&self.api_key);
        }

        let resp = match request_builder.send().await {
            Ok(r) => r,
            Err(e) => return Ok(Reflection::degraded(format!("REST call failed: {e}"))),
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Ok(Reflection::degraded(format!(
                "REST returned {status}: {text}"
            )));
        }

        let judge_resp: JudgeResponse = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                return Ok(Reflection::degraded(format!(
                    "Failed to parse response: {e}"
                )));
            }
        };

        let verdict = match judge_resp.verdict.to_uppercase().as_str() {
            "HOWL" => Verdict::Howl,
            "WAG" => Verdict::Wag,
            "GROWL" => Verdict::Growl,
            "BARK" => Verdict::Bark,
            _ => Verdict::Degraded,
        };

        let mut prose = String::new();
        prose.push_str(&format!(
            "**Phi Reasoning:** {}\n\n",
            judge_resp.reasoning.phi
        ));
        prose.push_str(&format!(
            "**Culture Reasoning:** {}\n",
            judge_resp.reasoning.culture
        ));

        Ok(Reflection {
            verdict,
            prose,
            patterns_detected: vec!["Pattern extraction delegated to kernel".to_string()],
            kenosis_candidate: None,
            confidence: judge_resp.q_score.total.min(0.618),
        })
    }
}
