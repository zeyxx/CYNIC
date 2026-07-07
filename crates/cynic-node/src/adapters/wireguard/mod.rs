use crate::domain::mesh::MeshTopology;
use crate::ports::mesh_port::MeshAdapter;
use std::error::Error;
use std::process::Command;
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
        let status = Command::new(cmd).args(args).status()?;
        if !status.success() {
            let err_msg = format!("Command '{} {:?}' failed with status {}", cmd, args, status);
            error!("{}", err_msg);
            return Err(err_msg.into());
        }
        Ok(())
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

        // 1. Create interface (ignore error if it already exists)
        let _ = Command::new("ip")
            .args(&[
                "link",
                "add",
                "dev",
                &self.interface_name,
                "type",
                "wireguard",
            ])
            .status();

        // 2. Assign IP
        self.execute(
            "ip",
            &["address", "replace", local_ip, "dev", &self.interface_name],
        )?;

        // 3. Set private key (using a temporary file for security is better, but this is a PoC)
        // In a real frictionless OS, we'd use netlink directly to avoid dropping keys to disk.
        // But for now, we'll pipe it using bash or a temp file.
        use std::io::Write;
        let mut temp_file = tempfile::NamedTempFile::new()?;
        write!(temp_file, "{}", private_key)?;

        self.execute(
            "wg",
            &[
                "set",
                &self.interface_name,
                "private-key",
                temp_file.path().to_str().unwrap(),
            ],
        )?;

        // 4. Bring interface up
        self.execute("ip", &["link", "set", "up", "dev", &self.interface_name])?;

        Ok(())
    }

    fn sync_topology(&self, topology: &MeshTopology) -> Result<(), Box<dyn Error>> {
        info!(
            "Synchronizing WireGuard topology on {}",
            self.interface_name
        );

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
            self.execute("wg", &args_ref)?;
        }

        Ok(())
    }
}
