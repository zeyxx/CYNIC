use crate::domain::mesh::MeshTopology;
use crate::ports::mesh_port::MeshAdapter;
use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
use std::error::Error;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct WireguardAdapter {
    interface_name: String,
    listen_port: u16,
}

impl WireguardAdapter {
    pub fn new(interface_name: &str) -> Self {
        Self {
            interface_name: interface_name.to_string(),
            listen_port: 44196,
        }
    }

    pub fn with_listen_port(mut self, port: u16) -> Self {
        self.listen_port = port;
        self
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

        // 4. Set listen port
        self.execute(
            &wg_bin,
            &[
                "set",
                self.interface_name.as_str(),
                "listen-port",
                &self.listen_port.to_string(),
            ],
        )?;

        // 5. Bring interface up
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
            let internal_ip_str = format!("{}/32", peer.internal_ip);

            // wg CLI requires allowed-ips LAST; order: peer <key> [endpoint] [persistent-keepalive] allowed-ips
            let mut args = vec![
                "set".to_string(),
                self.interface_name.clone(),
                "peer".to_string(),
                peer.public_key.clone(),
            ];

            if let Some(ep) = &peer.endpoint {
                if !ep.is_empty() {
                    args.push("endpoint".to_string());
                    args.push(ep.clone());
                }
            }

            if let Some(ka) = peer.keepalive_s {
                args.push("persistent-keepalive".to_string());
                args.push(ka.to_string());
            }

            args.push("allowed-ips".to_string());
            args.push(internal_ip_str);

            let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            self.execute(&wg_bin, &args_ref)?;
        }

        Ok(())
    }
}
