use std::error::Error;
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::Duration;

use crate::ports::network_interfaces_port::{
    ProviderFallbackProbePort, ProviderFallbackRoute, ProviderFallbackRouteStatus,
    ProviderServiceCheck, ProviderServiceConnectivity,
};

#[derive(Debug, Clone, Default)]
pub struct LocalProviderFallbackProbe;

impl LocalProviderFallbackProbe {
    fn command_output(command: &str, args: &[&str]) -> (Option<String>, Option<String>) {
        match Command::new(command).args(args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if output.status.success() {
                    (Some(stdout), (!stderr.is_empty()).then_some(stderr))
                } else {
                    (None, (!stderr.is_empty()).then_some(stderr))
                }
            }
            Err(error) => (None, Some(error.to_string())),
        }
    }
}

impl ProviderFallbackProbePort for LocalProviderFallbackProbe {
    fn observe_route(
        &self,
        route: &ProviderFallbackRoute,
    ) -> Result<ProviderFallbackRouteStatus, Box<dyn Error>> {
        let cidr = route.cidr.as_str();
        let gateway = route.gateway.clone().unwrap_or_default();

        let (route_stdout, route_error) = Self::command_output("ip", &["route", "show", cidr]);
        let (active_connections, nmcli_active_error) =
            Self::command_output("nmcli", &["-t", "-f", "NAME", "connection", "show", "--active"]);

        let mut nmcli_outputs = Vec::new();
        let mut nmcli_errors = Vec::new();
        if let Some(stderr) = nmcli_active_error {
            nmcli_errors.push(stderr);
        }
        for connection_name in active_connections.unwrap_or_default().lines() {
            let connection_name = connection_name.trim();
            if connection_name.is_empty() {
                continue;
            }
            let (routes_stdout, routes_error) =
                Self::command_output("nmcli", &["-g", "ipv4.routes", "connection", "show", connection_name]);
            if let Some(stdout) = routes_stdout.filter(|value| !value.is_empty()) {
                nmcli_outputs.push(stdout);
            }
            if let Some(stderr) = routes_error {
                nmcli_errors.push(format!("{connection_name}: {stderr}"));
            }
        }

        let observed_route = route_stdout.unwrap_or_default();
        let observed_nmcli_routes = nmcli_outputs.join("\n");
        let expected = format!("{cidr} {gateway}");
        let active = !gateway.is_empty()
            && observed_route.contains(cidr)
            && observed_route.contains(&gateway);
        let persistent = !gateway.is_empty() && observed_nmcli_routes.contains(&expected);

        Ok(ProviderFallbackRouteStatus {
            active,
            persistent,
            observed_route,
            observed_nmcli_routes,
            ip_route_error: route_error,
            nmcli_error: (!nmcli_errors.is_empty()).then(|| nmcli_errors.join("\n")),
        })
    }

    fn observe_connectivity(
        &self,
        checks: &[ProviderServiceCheck],
    ) -> Result<Vec<ProviderServiceConnectivity>, Box<dyn Error>> {
        let mut results = Vec::new();
        for check in checks {
            let address: SocketAddr = format!("{}:{}", check.host, check.port).parse().or_else(|_| {
                format!("{}:{}", check.ip, check.port).parse()
            })?;

            let result = match TcpStream::connect_timeout(&address, Duration::from_secs(2)) {
                Ok(stream) => {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    ProviderServiceConnectivity {
                        node: check.node.clone(),
                        service: check.service.clone(),
                        host: check.host.clone(),
                        ip: check.ip.clone(),
                        port: check.port,
                        status: "open".to_string(),
                        network_status: "reachable".to_string(),
                        error: None,
                    }
                }
                Err(error) => {
                    let error_text = error.to_string();
                    ProviderServiceConnectivity {
                        node: check.node.clone(),
                        service: check.service.clone(),
                        host: check.host.clone(),
                        ip: check.ip.clone(),
                        port: check.port,
                        status: "closed".to_string(),
                        network_status: if error_text.to_lowercase().contains("timed out") {
                            "blocked".to_string()
                        } else {
                            "reachable".to_string()
                        },
                        error: Some(error_text),
                    }
                }
            };
            results.push(result);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connectivity_payload_retains_check_identity() {
        let probe = LocalProviderFallbackProbe;
        let result = probe
            .observe_connectivity(&[ProviderServiceCheck {
                node: "forge".to_string(),
                service: "ssh".to_string(),
                host: "127.0.0.1".to_string(),
                ip: "127.0.0.1".to_string(),
                port: 9,
            }])
            .expect("connectivity");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].node, "forge");
        assert_eq!(result[0].service, "ssh");
        assert_eq!(result[0].host, "127.0.0.1");
        assert_eq!(result[0].port, 9);
    }
}
