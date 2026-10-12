//! Storage CLI surface — multi-model projection, SurrealDB and storage capabilities.

use clap::Subcommand;
use std::process::ExitCode;

use crate::{emit_json, fail};

#[derive(Subcommand, Debug)]
pub enum StorageCommand {
    /// Probe SurrealDB health and connectivity via cynic-surreal.
    Ping {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Execute a SurrealQL query via cynic-surreal authenticated client.
    Query {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        sql: String,
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Discover active storage runtime, engine, transport and capability proof levels.
    Capabilities {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        url: Option<String>,
    },
    /// Evaluate storage credential sync status.
    Credential {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        endpoint: Option<String>,
    },
}

pub fn execute_storage(command: StorageCommand) -> ExitCode {
    match command {
        StorageCommand::Ping { json: _, endpoint } => {
            let ep = endpoint.unwrap_or_else(|| "http://10.10.10.103:8000".to_string());
            let config = cynic_surreal::SurrealConfig::new(ep, "cynic", "runtime", "root");
            let client = cynic_surreal::SurrealHttpClient::new(config);
            match cynic_surreal::SurrealStoragePort::check_health(&client) {
                Ok(report) => emit_json(&report, !report.healthy),
                Err(e) => fail(e),
            }
        }
        StorageCommand::Query {
            json: _,
            sql,
            endpoint,
        } => {
            let ep = endpoint.unwrap_or_else(|| "http://10.10.10.103:8000".to_string());
            let config = cynic_surreal::SurrealConfig::new(ep, "cynic", "runtime", "root");
            let client = cynic_surreal::SurrealHttpClient::new(config);
            match cynic_surreal::SurrealStoragePort::execute_sql(&client, &sql) {
                Ok(results) => emit_json(&results, false),
                Err(e) => fail(e),
            }
        }
        StorageCommand::Capabilities { json, url } => {
            match cynic_host::surreal_client::execute_storage_capabilities(json, url.as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            }
        }
        StorageCommand::Credential { json, endpoint } => {
            match crate::runtime_surface::execute_storage_credential_sync(json, endpoint.as_deref())
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => fail(e),
            }
        }
    }
}
