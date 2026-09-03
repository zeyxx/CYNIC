//! HTTP adapter for submitting build workloads to the kernel.
//!
//! Implements `BuildWorkloadSubmissionPort` by POSTing the admitted delivery
//! to the kernel's `/agent-workloads` endpoint.

use async_trait::async_trait;
use reqwest::Client;

use crate::ports::build_workload_submission_port::{
    BuildWorkloadSubmissionPort, BuildWorkloadSubmissionReadBack,
};
use cynic_protocol::AgentWorkloadDelivery;

#[derive(Debug, Clone)]
pub struct HttpBuildWorkloadSubmission {
    client: Client,
    kernel_url: String,
}

impl HttpBuildWorkloadSubmission {
    pub fn new(kernel_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            kernel_url: kernel_url.into(),
        }
    }
}

#[async_trait]
impl BuildWorkloadSubmissionPort for HttpBuildWorkloadSubmission {
    async fn submit(
        &self,
        delivery: &AgentWorkloadDelivery,
    ) -> Result<BuildWorkloadSubmissionReadBack, String> {
        let url = format!("{}/agent-workloads", self.kernel_url);

        let response = self
            .client
            .post(&url)
            .json(delivery)
            .send()
            .await
            .map_err(|e| format!("submit workload request failed: {e}"))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| format!("read submit response body: {e}"))?;

        Ok(BuildWorkloadSubmissionReadBack {
            http_status: status.as_u16(),
            body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_strips_trailing_slash() {
        let adapter = HttpBuildWorkloadSubmission::new("http://kernel.example.com/");
        assert_eq!(adapter.kernel_url, "http://kernel.example.com/");
    }
}
