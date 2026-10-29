//! Storage CLI surface — multi-model projection, SurrealDB and storage capabilities.

pub mod sync;

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
    /// Synchronize knowledge graph, documents and chunks into SurrealDB.
    Sync {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        embed: bool,
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

fn get_client(endpoint: Option<String>) -> (String, cynic_persistence::SurrealHttpClient) {
    let ep = endpoint
        .or_else(|| std::env::var("CYNIC_SURREALDB_URL").ok())
        .or_else(|| std::env::var("SURREALDB_URL").ok())
        .unwrap_or_else(crate::runtime_surface::resolve_surrealdb_endpoint);
    let config = cynic_persistence::SurrealConfig::new(ep.clone(), "cynic", "runtime", "root");
    let mut client = cynic_persistence::SurrealHttpClient::new(config);
    if std::env::var("CYNIC_STORAGE_PASSWORD").is_err()
        && std::env::var("SURREALDB_PASSWORD").is_err()
    {
        client = client.with_password("cynic");
    }
    (ep, client)
}

pub fn execute_storage(command: StorageCommand) -> ExitCode {
    std::thread::spawn(move || match command {
        StorageCommand::Ping { json: _, endpoint } => {
            let (_ep, client) = get_client(endpoint);
            match cynic_persistence::SurrealStoragePort::check_health(&client) {
                Ok(report) => emit_json(&report, !report.healthy),
                Err(e) => fail(e),
            }
        }
        StorageCommand::Sync {
            json: _,
            endpoint,
            embed,
        } => {
            let (ep, client) = get_client(endpoint);
            match sync::execute_storage_sync(&client, &ep, embed) {
                Ok(report) => emit_json(&report, false),
                Err(e) => fail(e),
            }
        }
        StorageCommand::Query {
            json: _,
            sql,
            endpoint,
        } => {
            let (_ep, client) = get_client(endpoint);
            match cynic_persistence::SurrealStoragePort::execute_sql(&client, &sql) {
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
    })
    .join()
    .unwrap_or_else(|_| {
        eprintln!("storage thread panicked");
        ExitCode::FAILURE
    })
}
