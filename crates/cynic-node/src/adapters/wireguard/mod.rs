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

    /// Build the `wg set` argv for one peer.
    ///
    /// Pure: no spawn, no privilege, no host mutation. Extracted so the argv this
    /// adapter would issue is assertable in a test — previously the whole adapter
    /// was untestable because every path ran a real command, which is why it had
    /// zero tests despite implementing the canonical transport port.
    fn peer_set_args(&self, peer: &crate::domain::mesh::PeerIdentity) -> Vec<String> {
        let mut args = vec![
            "set".to_string(),
            self.interface_name.clone(),
            "peer".to_string(),
            peer.public_key.clone(),
            "allowed-ips".to_string(),
            peer.internal_ip.to_string(),
        ];
        // A roaming/NAT peer has no endpoint; emitting an empty one would make wg
        // reject the whole command, so absence must stay absence.
        if let Some(ep) = peer.endpoint.as_ref().filter(|e| !e.is_empty()) {
            args.push("endpoint".to_string());
            args.push(ep.to_string());
        }
        args
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
            let args = self.peer_set_args(peer);
            let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            self.execute(&wg_bin, &args_ref)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mesh::PeerIdentity;
    use cynic_protocol::network::{MeshRole, canonical_provider_for_role};

    fn peer(key: &str, ip: &str, endpoint: Option<&str>) -> PeerIdentity {
        PeerIdentity {
            public_key: key.to_string(),
            internal_ip: ip.parse().expect("ip"),
            endpoint: endpoint.map(|e| e.to_string()),
        }
    }

    #[test]
    fn a_peer_with_an_endpoint_gets_one_argument_pair() {
        let args = WireguardAdapter::new("wg0").peer_set_args(&peer(
            "pubkey1",
            "10.20.0.3",
            Some("203.0.113.1:51820"),
        ));
        assert_eq!(
            args,
            vec![
                "set",
                "wg0",
                "peer",
                "pubkey1",
                "allowed-ips",
                "10.20.0.3",
                "endpoint",
                "203.0.113.1:51820"
            ]
        );
    }

    /// A roaming peer has no endpoint. Emitting an empty `endpoint` argument
    /// would make wg reject the command for every peer in the batch.
    #[test]
    fn a_roaming_peer_emits_no_endpoint_argument() {
        for missing in [None, Some("")] {
            let args = WireguardAdapter::new("wg0").peer_set_args(&peer("k", "10.20.0.4", missing));
            assert!(
                !args.iter().any(|a| a == "endpoint"),
                "endpoint leaked for {missing:?}: {args:?}"
            );
            assert_eq!(args.len(), 6);
        }
    }

    /// The argv must address the injected interface, never a hardcoded one.
    #[test]
    fn the_interface_name_is_the_one_the_adapter_was_given() {
        let args = WireguardAdapter::new("synapse0").peer_set_args(&peer("k", "10.20.0.5", None));
        assert_eq!(args[1], "synapse0");
    }

    /// This adapter is the declared owner of Transport; if the provider table
    /// ever names someone else, these two have diverged.
    #[test]
    fn wireguard_is_the_canonical_transport_owner() {
        let owner = canonical_provider_for_role(MeshRole::Transport).expect("transport owner");
        assert_eq!(owner.provider, "wireguard");
    }
}
