//! Hermes Agent Runs API adapter.
//!
//! This intentionally starts with submit + bounded polling. SSE tool events
//! are a subsequent proof layer; no event is fabricated here.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use std::time::Duration;

use crate::application::agent_harness::{HarnessExecutor, InferenceReply, ScenarioSpec, TaskSpec};

#[derive(Debug, Clone)]
pub struct HermesRunExecutor {
    client: Client,
    base_url: String,
    api_key: String,
    poll_interval: Duration,
    max_wait: Duration,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HermesRunEvent {
    pub event: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HermesRuntimeObservation {
    pub endpoint: String,
    pub health_ok: bool,
    pub capabilities_ok: bool,
    pub model_count: usize,
    pub advertised_model: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HermesReadinessReport {
    pub observation: HermesRuntimeObservation,
    pub transition: cynic_protocol::CycleTransitionRecord,
}

/// Read-only projection of a pre-existing Hermes run. Raw output is excluded;
/// only execution state, bounded usage counters, and event-kind evidence cross
/// the adapter boundary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HermesRunObservation {
    pub run_id: String,
    pub http_status: Option<u16>,
    pub status: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub event_count: Option<u32>,
    pub event_kinds: Vec<String>,
    pub event_stream_error: Option<String>,
    pub error: Option<String>,
}

impl HermesRunExecutor {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        max_wait: Duration,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build()?,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            poll_interval: Duration::from_millis(500),
            max_wait,
        })
    }

    /// Drain the run's SSE event stream into structured evidence. The server
    /// may expire detached buffers; an empty result is therefore explicit and
    /// not interpreted as proof that no tools ran.
    pub async fn collect_run_events(&self, run_id: &str) -> Result<Vec<HermesRunEvent>, String> {
        let response = self
            .client
            .get(format!("{}/v1/runs/{run_id}/events", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("hermes_events_http:{}", response.status()));
        }
        let mut events = Vec::new();
        let mut buffer = String::new();
        let mut current_event = None;
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find('\n') {
                let line = buffer[..index].trim_end_matches('\r').to_string();
                buffer.drain(..=index);
                collect_sse_line(&line, &mut current_event, &mut events)?;
            }
        }
        if !buffer.trim_end_matches('\r').is_empty() {
            collect_sse_line(
                buffer.trim_end_matches('\r'),
                &mut current_event,
                &mut events,
            )?;
        }
        Ok(events)
    }

    /// Observe an existing run without submitting work or waiting for it to
    /// finish. Invalid identifiers are rejected before becoming URL path data.
    pub async fn observe_run(&self, run_id: &str) -> HermesRunObservation {
        if !valid_run_id(run_id) {
            return HermesRunObservation {
                run_id: run_id.into(),
                http_status: None,
                status: None,
                prompt_tokens: None,
                output_tokens: None,
                event_count: None,
                event_kinds: Vec::new(),
                event_stream_error: None,
                error: Some("invalid_hermes_run_id".into()),
            };
        }
        let response = match self
            .client
            .get(format!("{}/v1/runs/{run_id}", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => return unavailable_run(run_id, error.to_string()),
        };
        let http_status = response.status().as_u16();
        let state: Value = match response.json().await {
            Ok(value) => value,
            Err(error) => return unavailable_run(run_id, format!("hermes_run_json:{error}")),
        };
        if http_status >= 400 {
            return HermesRunObservation {
                run_id: run_id.into(),
                http_status: Some(http_status),
                status: state["status"].as_str().map(str::to_string),
                prompt_tokens: None,
                output_tokens: None,
                event_count: None,
                event_kinds: Vec::new(),
                event_stream_error: None,
                error: Some("hermes_run_http_failure".into()),
            };
        }
        let events = self.collect_run_events(run_id).await;
        HermesRunObservation {
            run_id: run_id.into(),
            http_status: Some(http_status),
            status: state["status"].as_str().map(str::to_string),
            prompt_tokens: state["usage"]["input_tokens"].as_u64(),
            output_tokens: state["usage"]["output_tokens"].as_u64(),
            event_count: events.as_ref().ok().map(|events| events.len() as u32),
            event_kinds: events
                .as_ref()
                .map(|events| {
                    events
                        .iter()
                        .filter_map(|event| event.event.clone())
                        .collect()
                })
                .unwrap_or_default(),
            event_stream_error: events.err(),
            error: None,
        }
    }

    pub async fn observe_readiness(&self) -> HermesRuntimeObservation {
        let health = self
            .client
            .get(format!("{}/health", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await;
        let health_ok = health
            .as_ref()
            .is_ok_and(|response| response.status().is_success());
        let capabilities = self
            .client
            .get(format!("{}/v1/capabilities", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await;
        let capabilities_ok = capabilities
            .as_ref()
            .is_ok_and(|response| response.status().is_success());
        let models = match self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await
        {
            Ok(response) => response.json::<Value>().await.ok(),
            Err(_) => None,
        };
        let model_rows = models.as_ref().and_then(|value| value["data"].as_array());
        HermesRuntimeObservation {
            endpoint: self.base_url.clone(),
            health_ok,
            capabilities_ok,
            model_count: model_rows.map_or(0, Vec::len),
            advertised_model: model_rows
                .and_then(|rows| rows.first())
                .and_then(|row| row["id"].as_str())
                .map(str::to_string),
            message: (!health_ok || !capabilities_ok).then(|| "hermes_runtime_not_ready".into()),
        }
    }

    pub async fn observe_readiness_with_transition(
        &self,
        correlation_id: impl Into<String>,
    ) -> HermesReadinessReport {
        let observation = self.observe_readiness().await;
        let ready =
            observation.health_ok && observation.capabilities_ok && observation.model_count > 0;
        let transition = cynic_protocol::CycleTransitionRecord::planned(
            correlation_id,
            serde_json::json!({"endpoint": &observation.endpoint, "runtime": "hermes"}),
            serde_json::to_value(&observation).unwrap_or_default(),
            serde_json::json!({"decision": if ready { "admit_agent_runtime" } else { "block_not_ready" }}),
        )
        .with_action(serde_json::json!({"action":"observe_only"}));
        let transition = if ready {
            transition.with_proof(serde_json::json!({"readiness":"converged"}))
        } else {
            transition
        };
        HermesReadinessReport {
            observation,
            transition,
        }
    }
}

fn valid_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id.len() <= 256
        && run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn unavailable_run(run_id: &str, error: String) -> HermesRunObservation {
    HermesRunObservation {
        run_id: run_id.into(),
        http_status: None,
        status: None,
        prompt_tokens: None,
        output_tokens: None,
        event_count: None,
        event_kinds: Vec::new(),
        event_stream_error: None,
        error: Some(error),
    }
}

fn parse_sse_event(explicit_event: Option<String>, data: &str) -> Result<HermesRunEvent, String> {
    let value: serde_json::Value =
        serde_json::from_str(data).map_err(|error| format!("hermes_event_json:{error}"))?;
    let event = explicit_event.or_else(|| value["event"].as_str().map(str::to_string));
    Ok(HermesRunEvent { event, data: value })
}

fn collect_sse_line(
    line: &str,
    current_event: &mut Option<String>,
    events: &mut Vec<HermesRunEvent>,
) -> Result<(), String> {
    if line.is_empty() {
        current_event.take();
        return Ok(());
    }
    if let Some(name) = line.strip_prefix("event:") {
        *current_event = Some(name.trim().to_string());
        return Ok(());
    }
    if let Some(data) = line.strip_prefix("data:") {
        let data = data.trim();
        if data == "[DONE]" {
            return Ok(());
        }
        events.push(parse_sse_event(current_event.take(), data)?);
    }
    Ok(())
}

#[async_trait]
impl HarnessExecutor for HermesRunExecutor {
    async fn execute(&self, scenario: &ScenarioSpec, task: &TaskSpec) -> InferenceReply {
        let started = tokio::time::Instant::now();
        let mut submit = self.client.post(format!("{}/v1/runs", self.base_url)).bearer_auth(&self.api_key).json(&json!({
            "input": task.prompt,
            "model": scenario.model.parameters.model_id,
            "session_id": scenario.runtime_id,
            "instructions": format!("CYNIC benchmark task {}. Stop after the declared task; do not exceed {} agent steps.", task.task_id, scenario.max_agent_steps.unwrap_or(1)),
        }));
        if let Some(correlation_id) = &scenario.correlation_id {
            submit = submit.header("X-CYNIC-Correlation-ID", correlation_id);
        }
        let submit = submit.send().await;
        let response = match submit {
            Ok(response) => response,
            Err(error) => return failed(None, error.to_string()),
        };
        let status = response.status().as_u16();
        let created: Value = match response.json().await {
            Ok(value) if status < 400 => value,
            Ok(value) => return failed(Some(status), value.to_string()),
            Err(error) => return failed(Some(status), error.to_string()),
        };
        let Some(run_id) = created["run_id"].as_str() else {
            return failed(Some(status), "missing_run_id".into());
        };
        let deadline = tokio::time::Instant::now() + self.max_wait;
        loop {
            if tokio::time::Instant::now() >= deadline {
                return failed(Some(408), format!("hermes_run_timeout:{run_id}"));
            }
            let response = match self
                .client
                .get(format!("{}/v1/runs/{run_id}", self.base_url))
                .bearer_auth(&self.api_key)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => return failed(None, error.to_string()),
            };
            let http_status = response.status().as_u16();
            let state: Value = match response.json().await {
                Ok(value) => value,
                Err(error) => return failed(Some(http_status), error.to_string()),
            };
            match state["status"].as_str().unwrap_or("unknown") {
                "completed" => {
                    let mut result = reply(http_status, &state, started.elapsed());
                    result.runtime_run_id = Some(run_id.to_string());
                    match self.collect_run_events(run_id).await {
                        Ok(events) => {
                            result.event_count = Some(events.len() as u32);
                            result.event_kinds =
                                events.into_iter().filter_map(|event| event.event).collect();
                        }
                        Err(error) => result.event_stream_error = Some(error),
                    }
                    return result;
                }
                "failed" | "cancelled" => {
                    return failed(
                        Some(http_status),
                        state["error"]
                            .as_str()
                            .unwrap_or("hermes_run_failed")
                            .into(),
                    );
                }
                _ => tokio::time::sleep(self.poll_interval).await,
            }
        }
    }
}

fn reply(status: u16, state: &Value, elapsed: Duration) -> InferenceReply {
    let usage = &state["usage"];
    let output_tokens = usage["output_tokens"].as_u64();
    let wall_time_ms = elapsed.as_millis() as u64;
    InferenceReply {
        runtime_run_id: None,
        http_status: Some(status),
        prompt_tokens: usage["input_tokens"].as_u64(),
        output_tokens,
        prompt_tokens_per_second: None,
        output_tokens_per_second: output_tokens
            .filter(|_| wall_time_ms > 0)
            .map(|value| value as f32 * 1000.0 / wall_time_ms as f32),
        wall_time_ms: Some(wall_time_ms),
        event_count: None,
        event_kinds: Vec::new(),
        event_stream_error: None,
        text_response: None,
        error: None,
    }
}

fn failed(status: Option<u16>, error: String) -> InferenceReply {
    InferenceReply {
        runtime_run_id: None,
        http_status: status,
        prompt_tokens: None,
        output_tokens: None,
        prompt_tokens_per_second: None,
        output_tokens_per_second: None,
        wall_time_ms: None,
        event_count: None,
        event_kinds: Vec::new(),
        event_stream_error: None,
        text_response: None,
        error: Some(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{collect_sse_line, parse_sse_event, valid_run_id};

    #[test]
    fn sse_data_event_type_is_preserved() {
        let event = parse_sse_event(None, r#"{"event":"run.completed","run_id":"r1"}"#)
            .expect("valid event");
        assert_eq!(event.event.as_deref(), Some("run.completed"));
    }

    #[test]
    fn sse_final_data_line_without_newline_is_collected() {
        let mut events = Vec::new();
        let mut event_name = None;

        collect_sse_line("event: run.completed", &mut event_name, &mut events)
            .expect("set event name");
        collect_sse_line(
            r#"data: {"event":"run.completed","run_id":"r-final"}"#,
            &mut event_name,
            &mut events,
        )
        .expect("collect final data line");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("run.completed"));
        assert_eq!(events[0].data["run_id"].as_str(), Some("r-final"));
    }

    #[test]
    fn sse_done_marker_is_ignored_without_emitting_event() {
        let mut events = Vec::new();
        let mut event_name = Some("run.completed".to_string());
        collect_sse_line("data: [DONE]", &mut event_name, &mut events)
            .expect("consume done marker");
        assert!(events.is_empty());
    }

    #[test]
    fn run_id_cannot_escape_the_read_only_endpoint_path() {
        assert!(valid_run_id("run_01-a"));
        assert!(!valid_run_id("../run"));
        assert!(!valid_run_id("run?secret=value"));
    }
}
