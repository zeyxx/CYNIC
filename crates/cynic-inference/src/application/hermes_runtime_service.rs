//! Application service for the Hermes layer lifecycle.

use crate::adapters::inference::hermes_process_supervisor::ProcessHermesGateway;
use crate::domain::hermes_runtime::{HermesGatewayMutation, HermesGatewayStatus};
use crate::ports::hermes_runtime_port::{HermesGatewayRuntimeError, HermesGatewayRuntimePort};
use cynic_protocol::CycleTransitionRecord;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HermesGatewayConvergenceReport {
    pub mutation: HermesGatewayMutation,
    pub transition: CycleTransitionRecord,
}

#[derive(Debug)]
pub struct HermesGatewayService<R> {
    runtime: R,
}

impl HermesGatewayService<ProcessHermesGateway> {
    pub fn from_process() -> Self {
        Self {
            runtime: ProcessHermesGateway::from_env(),
        }
    }
}

impl<R: HermesGatewayRuntimePort> HermesGatewayService<R> {
    pub fn status(&self) -> Result<HermesGatewayStatus, HermesGatewayRuntimeError> {
        self.runtime.status()
    }

    pub fn status_with_transition(
        &self,
        correlation_id: impl Into<String>,
    ) -> Result<(HermesGatewayStatus, CycleTransitionRecord), HermesGatewayRuntimeError> {
        let status = self.runtime.status()?;
        let transition = CycleTransitionRecord::planned(
            correlation_id,
            serde_json::json!({"runtime":"hermes_gateway"}),
            serde_json::to_value(&status).unwrap_or_default(),
            serde_json::json!({"decision":"observe_hermes_gateway"}),
        );
        Ok((status, transition))
    }

    pub fn start_with_transition(
        &self,
        wait_for_ready: bool,
        correlation_id: impl Into<String>,
    ) -> Result<HermesGatewayConvergenceReport, HermesGatewayRuntimeError> {
        let desired = serde_json::json!({
            "runtime": "hermes_gateway",
            "wait_for_ready": wait_for_ready,
        });
        let mutation = self.runtime.start(wait_for_ready)?;
        let transition = CycleTransitionRecord::planned(
            correlation_id,
            desired,
            serde_json::to_value(&mutation.status).unwrap_or_default(),
            serde_json::json!({"decision":"hermes_gateway_start","runtime_id":&mutation.runtime_id}),
        )
        .with_action(serde_json::to_value(&mutation).unwrap_or_default());
        let transition = if mutation.ok {
            transition.with_proof(serde_json::json!({
                "status": &mutation.status,
                "message": &mutation.message,
            }))
        } else {
            transition
        };
        Ok(HermesGatewayConvergenceReport {
            mutation,
            transition,
        })
    }

    pub fn stop(&self) -> Result<HermesGatewayMutation, HermesGatewayRuntimeError> {
        self.runtime.stop()
    }
}
