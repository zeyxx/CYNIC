use std::fs;
use std::path::Path;

use cynic_protocol::hardening::{FirewallObservedState, SshdObservedState};

pub fn observe_sshd_state(base_config_path: Option<&str>) -> SshdObservedState {
    let main_path = base_config_path.unwrap_or("/etc/ssh/sshd_config");
    let mut password_auth = true;
    let mut permit_root = "yes".to_string();
    let mut pubkey_auth = true;
    let mut listen_addrs = Vec::new();
    let mut port = 22;

    if let Ok(content) = fs::read_to_string(main_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0].to_lowercase().as_str() {
                    "passwordauthentication" => {
                        password_auth = parts[1].to_lowercase() == "yes";
                    }
                    "permitrootlogin" => {
                        permit_root = parts[1].to_string();
                    }
                    "pubkeyauthentication" => {
                        pubkey_auth = parts[1].to_lowercase() == "yes";
                    }
                    "listenaddress" => {
                        listen_addrs.push(parts[1].to_string());
                    }
                    "port" => {
                        if let Ok(p) = parts[1].parse() {
                            port = p;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Check drop-ins in /etc/ssh/sshd_config.d/
    let drop_in_dir = Path::new("/etc/ssh/sshd_config.d");
    if let Ok(entries) = fs::read_dir(drop_in_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') {
                        continue;
                    }
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        match parts[0].to_lowercase().as_str() {
                            "passwordauthentication" => {
                                password_auth = parts[1].to_lowercase() == "yes";
                            }
                            "permitrootlogin" => {
                                permit_root = parts[1].to_string();
                            }
                            "listenaddress" => {
                                listen_addrs.push(parts[1].to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    SshdObservedState {
        config_file_path: main_path.to_string(),
        password_auth_enabled: password_auth,
        permit_root_login: permit_root,
        pubkey_auth_enabled: pubkey_auth,
        listen_addresses: listen_addrs,
        port,
    }
}

pub fn observe_firewall_state() -> FirewallObservedState {
    #[cfg(target_os = "linux")]
    {
        let nft_exists = Path::new("/usr/sbin/nft").exists() || Path::new("/sbin/nft").exists();
        let iptables_exists = Path::new("/usr/sbin/iptables").exists();

        let backend = if nft_exists {
            "nftables".to_string()
        } else if iptables_exists {
            "iptables".to_string()
        } else {
            "none".to_string()
        };

        FirewallObservedState {
            backend_detected: backend,
            default_input_policy: "unknown".to_string(),
            wireguard_port_allowed: true,
            direct_wan_ssh_blocked: false,
        }
    }

    #[cfg(target_os = "windows")]
    {
        FirewallObservedState {
            backend_detected: "windows_firewall".to_string(),
            default_input_policy: "block_inbound_by_default".to_string(),
            wireguard_port_allowed: true,
            direct_wan_ssh_blocked: true,
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        FirewallObservedState {
            backend_detected: "unsupported".to_string(),
            default_input_policy: "unknown".to_string(),
            wireguard_port_allowed: false,
            direct_wan_ssh_blocked: false,
        }
    }
}
