use crate::domain::mesh::MeshTopology;
use crate::ports::mesh_port::MeshAdapter;
use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
use std::error::Error;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct WireguardAdapter {
    interface_name: String,
}

impl WireguardAdapter {
    pub fn new(interface_name: &str) -> Self {
        Self {
            interface_name: interface_name.to_string(),
        }
    }

    fn execute(&self, cmd: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
        let output = capture_command(cmd, args)?;
        if !output.success {
            let err_msg = if output.stderr.is_empty() {
                format!(
                    "Command '{} {:?}' failed with exit code {:?}",
                    cmd, args, output.exit_code
                )
            } else {
                format!(
                    "Command '{} {:?}' failed with exit code {:?}: {}",
                    cmd, args, output.exit_code, output.stderr
                )
            };
            error!("{}", err_msg);
            return Err(err_msg.into());
        }
        Ok(())
    }

    fn create_interface_if_missing(&self, ip_bin: &str) {
        let args = [
            "link",
            "add",
            "dev",
            self.interface_name.as_str(),
            "type",
            "wireguard",
        ];
        // Creation is idempotent at this boundary: a pre-existing interface is
        // reconciled by the following address/key/up operations.
        let _ = capture_command(ip_bin, &args);
    }

    fn install_private_key(&self, wg_bin: &str, private_key: &str) -> Result<(), Box<dyn Error>> {
        use std::io::Write;

        let mut temp_file = tempfile::NamedTempFile::new()?;
        write!(temp_file, "{}", private_key)?;
        let temp_key_path = temp_file.path().to_string_lossy().to_string();
        self.execute(
            wg_bin,
            &[
                "set",
                self.interface_name.as_str(),
                "private-key",
                &temp_key_path,
            ],
        )
    }
}

impl MeshAdapter for WireguardAdapter {
    fn initialize_interface(
        &self,
        local_ip: &str,
        private_key: &str,
    ) -> Result<(), Box<dyn Error>> {
        info!(
            "Initializing WireGuard interface {} with IP {}",
            self.interface_name, local_ip
        );
        let capabilities = current_runtime_layout().capabilities();
        let ip_bin = capabilities.route_tool.command;
        let wg_bin = capabilities.wireguard_tool.command;

        // 1. Create interface (ignore error if it already exists)
        self.create_interface_if_missing(&ip_bin);

        // 2. Assign IP
        self.execute(
            &ip_bin,
            &["address", "replace", local_ip, "dev", &self.interface_name],
        )?;

        // 3. Set private key through a short-lived file.
        self.install_private_key(&wg_bin, private_key)?;

        // 4. Bring interface up
        self.execute(&ip_bin, &["link", "set", "up", "dev", &self.interface_name])?;

        Ok(())
    }

    fn sync_topology(&self, topology: &MeshTopology) -> Result<(), Box<dyn Error>> {
        info!(
            "Synchronizing WireGuard topology on {}",
            self.interface_name
        );
        let wg_bin = current_runtime_layout()
            .capabilities()
            .wireguard_tool
            .command;

        for peer in &topology.peers {
            let mut endpoint_args = Vec::new();
            if let Some(ref ep) = peer.endpoint {
                if !ep.is_empty() {
                    endpoint_args.push("endpoint".to_string());
                    endpoint_args.push(ep.to_string());
                }
            }

            let internal_ip_str = peer.internal_ip.to_string();

            let mut args = vec![
                "set".to_string(),
                self.interface_name.clone(),
                "peer".to_string(),
                peer.public_key.clone(),
                "allowed-ips".to_string(),
                internal_ip_str,
            ];
            args.extend(endpoint_args);

            let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            self.execute(&wg_bin, &args_ref)?;
        }

        Ok(())
    }
}

/// Generate a standard WireGuard configuration file content for Synapse mesh
pub fn generate_wireguard_config(
    local_private_key: &str,
    local_ip: &str,
    listen_port: u16,
    peers: &[crate::domain::mesh::PeerIdentity],
) -> String {
    let mut config = String::new();
    config.push_str("[Interface]\n");
    config.push_str(&format!("PrivateKey = {}\n", local_private_key.trim()));
    config.push_str(&format!("Address = {}/24\n", local_ip.trim()));
    config.push_str(&format!("ListenPort = {}\n\n", listen_port));

    for peer in peers {
        config.push_str("[Peer]\n");
        config.push_str(&format!("PublicKey = {}\n", peer.public_key.trim()));
        config.push_str(&format!("AllowedIPs = {}/32\n", peer.internal_ip));
        if let Some(endpoint) = &peer.endpoint {
            config.push_str(&format!("Endpoint = {}\n", endpoint.trim()));
        }
        config.push_str("PersistentKeepalive = 25\n\n");
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_generate_wireguard_config() {
        let peers = vec![
            crate::domain::mesh::PeerIdentity {
                public_key: "pubkey-peer-alpha-123456789=".to_string(),
                internal_ip: IpAddr::V4(Ipv4Addr::new(10, 10, 10, 101)),
                endpoint: Some("192.168.1.100:51820".to_string()),
                node_id: "node-alpha".to_string(),
            },
            crate::domain::mesh::PeerIdentity {
                public_key: "pubkey-peer-beta-987654321=".to_string(),
                internal_ip: IpAddr::V4(Ipv4Addr::new(10, 10, 10, 102)),
                endpoint: None,
                node_id: "node-beta".to_string(),
            },
        ];

        let conf = generate_wireguard_config(
            "privkey-local-0000000000000000000=",
            "10.10.10.1",
            51820,
            &peers,
        );

        assert!(conf.contains("[Interface]"));
        assert!(conf.contains("PrivateKey = privkey-local-0000000000000000000="));
        assert!(conf.contains("Address = 10.10.10.1/24"));
        assert!(conf.contains("ListenPort = 51820"));
        assert!(conf.contains("PublicKey = pubkey-peer-alpha-123456789="));
        assert!(conf.contains("AllowedIPs = 10.10.10.101/32"));
        assert!(conf.contains("Endpoint = 192.168.1.100:51820"));
        assert!(conf.contains("PublicKey = pubkey-peer-beta-987654321="));
        assert!(conf.contains("AllowedIPs = 10.10.10.102/32"));
        assert!(conf.contains("PersistentKeepalive = 25"));
    }
}
