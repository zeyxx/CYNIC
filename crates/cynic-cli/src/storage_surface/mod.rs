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
            let ep = endpoint.map(|s| s.to_string()).or_else(|| std::env::var("CYNIC_SURREALDB_URL").ok()).or_else(|| std::env::var("SURREALDB_URL").ok()).unwrap_or_else(|| crate::runtime_surface::resolve_surrealdb_endpoint());
            let config = cynic_storage::SurrealConfig::new(ep, "cynic", "runtime", "root");
            let client = cynic_storage::SurrealHttpClient::new(config);
            match cynic_storage::SurrealStoragePort::check_health(&client) {
                Ok(report) => emit_json(&report, !report.healthy),
                Err(e) => fail(e),
            }
        }
        StorageCommand::Query {
            json: _,
            sql,
            endpoint,
        } => {
            let ep = endpoint.map(|s| s.to_string()).or_else(|| std::env::var("CYNIC_SURREALDB_URL").ok()).or_else(|| std::env::var("SURREALDB_URL").ok()).unwrap_or_else(|| crate::runtime_surface::resolve_surrealdb_endpoint());
            let config = cynic_storage::SurrealConfig::new(ep, "cynic", "runtime", "root");
            let client = cynic_storage::SurrealHttpClient::new(config);
            match cynic_storage::SurrealStoragePort::execute_sql(&client, &sql) {
                Ok(results) => emit_json(&results, false),
                Err(e) => fail(e),
            }
        }
        StorageCommand::Capabilities { json, url } => {
            if !json {
                eprintln!("storage capabilities require --json");
                return ExitCode::FAILURE;
            }
            let endpoint = url
                .or_else(|| std::env::var("SURREALDB_URL").ok())
                .unwrap_or_else(|| "http://localhost:8000".into());
            let report = cynic_host::storage_capabilities::observe_storage_capabilities(&endpoint);
            emit_json(&report, report.status == "blocked")
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
