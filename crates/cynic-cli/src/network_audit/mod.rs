use clap::Subcommand;
use std::process::ExitCode;

mod render;
mod scan;
mod types;

pub use scan::build_network_audit;
pub use render::render_text;

#[derive(Subcommand)]
pub(crate) enum NetworkAuditCommand {
    /// Audit declared vs observed network state (DNS, WireGuard, services).
    #[command(name = "network-audit")]
    NetworkAudit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "infra/registry.json")]
        registry: Option<String>,
    },
}

pub(crate) fn execute_network_audit(command: NetworkAuditCommand) -> ExitCode {
    match command {
        NetworkAuditCommand::NetworkAudit { json, registry } => {
            let registry_path = registry.unwrap_or_else(|| "infra/registry.json".to_string());
            let report = scan::build_network_audit(&registry_path);
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                render::render_text(&report)
            }
        }
    }
}
