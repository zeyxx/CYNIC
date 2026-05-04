//! ProxyProbe — test connectivity to mitmproxy HTTP proxy on localhost:8888.
//! Verifies that the proxy process is listening and accepting connections.

use crate::domain::probe::{ProbeDetails, ProbeError, ProbeResult, ProbeStatus};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct ProxyProbe;

#[async_trait]
impl crate::domain::probe::Probe for ProxyProbe {
    fn name(&self) -> &str {
        "proxy"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        // Try to connect to localhost:8888 (mitmproxy) with a 2s timeout.
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            tokio::net::TcpStream::connect("127.0.0.1:8888"),
        )
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        let status = match result {
            Ok(Ok(_)) => ProbeStatus::Ok,
            Ok(Err(_)) | Err(_) => ProbeStatus::Unavailable,
        };

        Ok(ProbeResult {
            name: "proxy".to_string(),
            status,
            details: ProbeDetails::Empty,
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn proxy_probe_returns_result() {
        let probe = ProxyProbe;
        let result = probe.sense().await.expect("should not error");
        // Result depends on whether mitmproxy is running.
        // Just verify we get a ProbeResult with a name.
        assert_eq!(result.name, "proxy");
    }
}
