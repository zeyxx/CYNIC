//! Kernel-plane TCP membership transport adapter (L4 → L3 feed).
//!
//! Measures whether the node's coordination plane (`kernel_url` from its own
//! config) answers a connect probe, and projects the result as typed
//! [`TransportReadback`]s for the membership tick. This is the minimal honest
//! plane every node already has; the WireGuard observer
//! (`cynic_host::wireguard_readback`) composes alongside it where an overlay
//! is declared. No new config surface: the kernel URL is the one the node
//! already needs to run.
//!
//! Authority: `protocols/machine-enrollment-federation.md` step 3 wiring.

use cynic_protocol::{TRANSPORT_READBACK_SCHEMA, TransportKind, TransportReadback, TransportState};

use crate::usecases::federation_membership_uc::MembershipTransportPort;

pub const KERNEL_TCP_PLANE_NODE_ID: &str = "kernel-primary";

/// Connect-probe the configured kernel host:port once per observation.
/// Blocking std::net on purpose; callers wrap in `spawn_blocking` via the
/// loop's async boundary or inject this trait with fixtures in tests.
pub struct KernelTcpTransport {
    host: String,
    port: u16,
    connect_timeout_ms: u64,
}

impl KernelTcpTransport {
    pub fn new(kernel_url_host: String, port: u16, connect_timeout_ms: u64) -> Self {
        KernelTcpTransport {
            host: kernel_url_host,
            port,
            connect_timeout_ms,
        }
    }

    /// Parse `http://host:port/…` into a probe target. Pure helper exposed
    /// for tests and CLI projection.
    pub fn from_kernel_url(kernel_url: &str, connect_timeout_ms: u64) -> Result<Self, String> {
        let trimmed = kernel_url.trim();
        let without_scheme = trimmed
            .strip_prefix("http://")
            .or_else(|| trimmed.strip_prefix("https://"))
            .unwrap_or(trimmed);
        let authority = without_scheme.split('/').next().unwrap_or("");
        if authority.is_empty() {
            return Err(format!("kernel url has no authority: {kernel_url}"));
        }
        let (host, port_str) = match authority.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.to_string()),
            None => (
                authority.to_string(),
                // Scheme-aware default: https is 443, everything else 80.
                if trimmed.starts_with("https://") {
                    "443".to_string()
                } else {
                    "80".to_string()
                },
            ),
        };
        let port: u16 = port_str
            .parse()
            .map_err(|_| format!("kernel url port invalid: {kernel_url}"))?;
        Ok(KernelTcpTransport::new(host, port, connect_timeout_ms))
    }

    fn probe_once(&self, now_unix_ms: u64) -> TransportReadback {
        use std::net::ToSocketAddrs;
        let state = match (self.host.as_str(), self.port).to_socket_addrs() {
            Err(resolve_error) => TransportState::Unreachable {
                detail: format!("resolve_failed: {resolve_error}"),
            },
            Ok(addrs) => {
                let mut connected = false;
                let mut last_error = String::from("no addresses resolved");
                for addr in addrs {
                    match std::net::TcpStream::connect_timeout(
                        &addr,
                        std::time::Duration::from_millis(self.connect_timeout_ms),
                    ) {
                        Ok(_stream) => {
                            connected = true;
                            break;
                        }
                        Err(e) => last_error = e.to_string(),
                    }
                }
                if connected {
                    TransportState::Healthy
                } else {
                    TransportState::Unreachable {
                        detail: format!("connect_refused: {last_error}"),
                    }
                }
            }
        };
        TransportReadback {
            schema: TRANSPORT_READBACK_SCHEMA.to_string(),
            node_id: KERNEL_TCP_PLANE_NODE_ID.to_string(),
            kind: TransportKind::Tcp {
                host: self.host.clone(),
                port: self.port,
            },
            state,
            observed_at_unix_ms: now_unix_ms,
            evidence_refs: vec![format!("probe_target:{}:{}", self.host, self.port)],
        }
    }
}

impl MembershipTransportPort for KernelTcpTransport {
    fn observe_transport(&self) -> Vec<TransportReadback> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        vec![self.probe_once(now)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_url_parses_into_probe_target() {
        let t =
            KernelTcpTransport::from_kernel_url("http://192.168.100.2:3030", 2000).expect("parses");
        assert_eq!(t.host, "192.168.100.2");
        assert_eq!(t.port, 3030);
        let t2 = KernelTcpTransport::from_kernel_url("http://kernel.synapse.cynic", 1000)
            .expect("parses default port");
        assert_eq!(t2.port, 80);
        // scheme-aware defaults: https without explicit port is 443, not 80
        let t3 = KernelTcpTransport::from_kernel_url("https://kernel.synapse.cynic", 1000)
            .expect("parses https default");
        assert_eq!(t3.port, 443);
    }

    #[test]
    fn garbage_url_is_an_explicit_error_not_a_guess() {
        assert!(KernelTcpTransport::from_kernel_url("::::", 1000).is_err());
    }

    #[test]
    fn refused_connection_reports_unreachable_with_reason() {
        // bind then drop: a port that is definitely closed on loopback
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);
        let t = KernelTcpTransport::new(addr.ip().to_string(), addr.port(), 300);
        let rb = t.probe_once(42);
        assert!(!rb.healthy());
        assert!(matches!(rb.state, TransportState::Unreachable { .. }));
        assert_eq!(
            rb.kind,
            TransportKind::Tcp {
                host: addr.ip().to_string(),
                port: addr.port(),
            }
        );
    }

    #[test]
    fn open_port_reports_healthy() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let t = KernelTcpTransport::new(addr.ip().to_string(), addr.port(), 500);
        let rb = t.probe_once(7);
        assert!(rb.healthy());
        assert_eq!(rb.node_id, KERNEL_TCP_PLANE_NODE_ID);
        assert!(rb.validate().is_empty(), "{:?}", rb.validate());
        drop(listener);
    }
}
