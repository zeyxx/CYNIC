//! Freebox OS network surface commands.
//!
//! Exposes complete Freebox router supervision and mutation under `cynic network freebox`.
//! Mutations (`dmz write`, `nat add`, `nat delete`) enforce the anti-implicit doctrine:
//! they require explicit `--yes` confirmation before applying.

use std::process::ExitCode;
use clap::Subcommand;
use serde_json::json;

use cynic_host::adapters::http_freebox_probe_adapter::HttpFreeboxProbeAdapter;
use cynic_network::{FreeboxNatRuleSpec, FreeboxPort};

use crate::network_surface::NetworkCommand;
use crate::runtime_env::ResolvedRuntimePaths;

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum NetworkFreeboxCommand {
    /// Observe Freebox router state (LAN topology + WAN connection + DMZ + NAT rules).
    Observe {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        json: bool,
    },
    /// Inspect or configure the DMZ.
    Dmz {
        #[command(subcommand)]
        command: FreeboxDmzSubcommand,
    },
    /// List, create, or delete NAT port forwarding rules.
    Nat {
        #[command(subcommand)]
        command: FreeboxNatSubcommand,
    },
    /// Query live WAN connection status (IPv4, IPv6, throughput).
    Wan {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum FreeboxDmzSubcommand {
    /// Read current DMZ state (enabled flag + target IP).
    Read {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        json: bool,
    },
    /// Configure DMZ (requires explicit --yes consent).
    Write {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        enabled: bool,
        #[arg(long)]
        ip: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum FreeboxNatSubcommand {
    /// List all NAT port-forwarding rules.
    List {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        json: bool,
    },
    /// Add a NAT port-forwarding rule (requires explicit --yes consent).
    Add {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        ip: String,
        #[arg(long, default_value = "tcp")]
        proto: String,
        #[arg(long)]
        wan_port: u16,
        #[arg(long)]
        wan_port_end: Option<u16>,
        #[arg(long)]
        lan_port: Option<u16>,
        #[arg(long, default_value = "cynic")]
        comment: String,
        #[arg(long, default_value_t = true)]
        enabled: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Delete a NAT port-forwarding rule by ID (requires explicit --yes consent).
    Delete {
        #[arg(long, default_value = "http://mafreebox.freebox.fr")]
        endpoint: String,
        #[arg(long)]
        rule_id: u32,
        #[arg(long)]
        yes: bool,
    },
}

pub(crate) fn try_execute_freebox(
    cmd: NetworkCommand,
    _runtime: &ResolvedRuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    let command = match cmd {
        NetworkCommand::Freebox { command } => command,
        other => return Err(other),
    };

    let adapter = HttpFreeboxProbeAdapter::default();

    match command {
        NetworkFreeboxCommand::Observe { endpoint, json } => {
            let lan = adapter.probe_lan(&endpoint);
            let wan = adapter.read_wan(&endpoint);
            let dmz = adapter.read_dmz(&endpoint);
            let nat = adapter.list_nat_rules(&endpoint);

            if json {
                let out = json!({
                    "endpoint": endpoint,
                    "lan": lan.ok(),
                    "wan": wan.ok(),
                    "dmz": dmz.ok(),
                    "nat_rules": nat.ok(),
                });
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            } else {
                println!("=== Freebox Overview ({endpoint}) ===");
                match wan {
                    Ok(w) => println!("  WAN: state={}, ipv4={:?}, ipv6={:?}", w.state, w.ipv4, w.ipv6),
                    Err(e) => println!("  WAN: error ({e})"),
                }
                match dmz {
                    Ok(d) => println!("  DMZ: enabled={}, ip={}", d.enabled, d.ip),
                    Err(e) => println!("  DMZ: error ({e})"),
                }
                match nat {
                    Ok(rules) => println!("  NAT: {} active rules", rules.len()),
                    Err(e) => println!("  NAT: error ({e})"),
                }
                match lan {
                    Ok(l) => println!("  LAN: {} hosts discovered", l.hosts.len()),
                    Err(e) => println!("  LAN: error ({e})"),
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        NetworkFreeboxCommand::Wan { endpoint, json } => {
            match adapter.read_wan(&endpoint) {
                Ok(wan) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&wan).unwrap_or_default());
                    } else {
                        println!("=== Freebox WAN Status ===");
                        println!("  State:       {}", wan.state);
                        println!("  IPv4:        {}", wan.ipv4.as_deref().unwrap_or("none"));
                        println!("  IPv6:        {}", wan.ipv6.as_deref().unwrap_or("none"));
                        println!("  Down (bps):  {}", wan.rate_down_bps.unwrap_or(0));
                        println!("  Up (bps):    {}", wan.rate_up_bps.unwrap_or(0));
                    }
                    Ok(ExitCode::SUCCESS)
                }
                Err(e) => {
                    eprintln!("error: failed to read Freebox WAN: {e}");
                    Ok(ExitCode::FAILURE)
                }
            }
        }
        NetworkFreeboxCommand::Dmz { command } => match command {
            FreeboxDmzSubcommand::Read { endpoint, json } => {
                match adapter.read_dmz(&endpoint) {
                    Ok(dmz) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&dmz).unwrap_or_default());
                        } else {
                            println!("=== Freebox DMZ Configuration ===");
                            println!("  Enabled: {}", dmz.enabled);
                            println!("  Target:  {}", dmz.ip);
                        }
                        Ok(ExitCode::SUCCESS)
                    }
                    Err(e) => {
                        eprintln!("error: failed to read DMZ: {e}");
                        Ok(ExitCode::FAILURE)
                    }
                }
            }
            FreeboxDmzSubcommand::Write {
                endpoint,
                enabled,
                ip,
                yes,
            } => {
                if !yes {
                    eprintln!("error: updating DMZ is a mutation; pass --yes to confirm.");
                    return Ok(ExitCode::FAILURE);
                }
                match adapter.write_dmz(&endpoint, enabled, &ip) {
                    Ok(dmz) => {
                        println!("DMZ updated successfully: enabled={}, target={}", dmz.enabled, dmz.ip);
                        Ok(ExitCode::SUCCESS)
                    }
                    Err(e) => {
                        eprintln!("error: failed to update DMZ: {e}");
                        Ok(ExitCode::FAILURE)
                    }
                }
            }
        },
        NetworkFreeboxCommand::Nat { command } => match command {
            FreeboxNatSubcommand::List { endpoint, json } => {
                match adapter.list_nat_rules(&endpoint) {
                    Ok(rules) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&rules).unwrap_or_default());
                        } else {
                            println!("=== Freebox NAT Rules ({} total) ===", rules.len());
                            for r in rules {
                                println!(
                                    "  [{:>3}] {:<4} WAN:{}-{} -> LAN:{}:{} ({}) [enabled={}]",
                                    r.id,
                                    r.protocol.to_uppercase(),
                                    r.wan_port_start,
                                    r.wan_port_end,
                                    r.ip,
                                    r.lan_port,
                                    r.comment,
                                    r.enabled
                                );
                            }
                        }
                        Ok(ExitCode::SUCCESS)
                    }
                    Err(e) => {
                        eprintln!("error: failed to list NAT rules: {e}");
                        Ok(ExitCode::FAILURE)
                    }
                }
            }
            FreeboxNatSubcommand::Add {
                endpoint,
                ip,
                proto,
                wan_port,
                wan_port_end,
                lan_port,
                comment,
                enabled,
                yes,
            } => {
                if !yes {
                    eprintln!("error: creating a NAT rule is a mutation; pass --yes to confirm.");
                    return Ok(ExitCode::FAILURE);
                }
                let spec = FreeboxNatRuleSpec {
                    enabled,
                    ip,
                    protocol: proto,
                    wan_port_start: wan_port,
                    wan_port_end: wan_port_end.unwrap_or(wan_port),
                    lan_port: lan_port.unwrap_or(wan_port),
                    comment,
                };
                match adapter.add_nat_rule(&endpoint, &spec) {
                    Ok(rule) => {
                        println!(
                            "NAT rule #{} created: {} WAN:{} -> {}:{} ({})",
                            rule.id, rule.protocol, rule.wan_port_start, rule.ip, rule.lan_port, rule.comment
                        );
                        Ok(ExitCode::SUCCESS)
                    }
                    Err(e) => {
                        eprintln!("error: failed to add NAT rule: {e}");
                        Ok(ExitCode::FAILURE)
                    }
                }
            }
            FreeboxNatSubcommand::Delete {
                endpoint,
                rule_id,
                yes,
            } => {
                if !yes {
                    eprintln!("error: deleting a NAT rule is destructive; pass --yes to confirm.");
                    return Ok(ExitCode::FAILURE);
                }
                match adapter.delete_nat_rule(&endpoint, rule_id) {
                    Ok(()) => {
                        println!("NAT rule #{rule_id} deleted successfully.");
                        Ok(ExitCode::SUCCESS)
                    }
                    Err(e) => {
                        eprintln!("error: failed to delete NAT rule #{rule_id}: {e}");
                        Ok(ExitCode::FAILURE)
                    }
                }
            }
        },
    }
}
