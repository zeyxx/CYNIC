use std::process::ExitCode;

use clap::Subcommand;
use cynic_network::{MAX_REGISTRY_SYNC_BACKUPS, RegistrySyncReport, apply_registry_sync};

use crate::network_surface::NetworkCommand;
use crate::runtime_paths::RuntimePaths;

#[derive(Subcommand)]
pub(crate) enum NetworkRegistryCommand {
    Observe {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        apply: bool,
    },
    Sync {
        /// Declared registry file (the repo copy). Explicit: the binary
        /// cannot know the checkout location, so there is no default.
        #[arg(long)]
        source: String,
        /// Runtime copy to refresh. Defaults to the system registry path.
        #[arg(long)]
        target: Option<String>,
        /// Write. Without it, the merge is computed and reported only.
        #[arg(long)]
        yes: bool,
    },
}

pub(crate) fn try_execute_registry(
    command: NetworkCommand,
    _runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    let NetworkCommand::Registry { command } = command else {
        return Err(command);
    };

    let exit = match command {
        NetworkRegistryCommand::Observe {
            json,
            registry,
            observation,
            apply,
        } => execute_registry_observe(json, registry, &observation, apply),
        NetworkRegistryCommand::Sync {
            source,
            target,
            yes,
        } => execute_registry_sync(&source, target.as_deref(), yes),
    };
    Ok(exit)
}

pub(crate) fn execute_registry_observe(
    json: bool,
    registry: Option<String>,
    observation: &str,
    apply: bool,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let registry = crate::registry_path(registry);
    match cynic_host::registry_observation::project_registry_observation(
        &registry,
        observation,
        apply,
    ) {
        Ok(report) => crate::emit_json(&report, report.status != "applied"),
        Err(error) => crate::fail(error),
    }
}

fn file_digest(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content);
    hex::encode(hasher.finalize())
}

fn execute_registry_sync(source: &str, target: Option<&str>, yes: bool) -> ExitCode {
    let target_path = crate::registry_path(target.map(str::to_string));
    let source_content = match std::fs::read(source) {
        Ok(content) => content,
        Err(_) => {
            eprintln!("registry sync: cannot read source {source}");
            return ExitCode::FAILURE;
        }
    };
    let source_value: serde_json::Value = match serde_json::from_slice(&source_content) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("registry sync: source {source} is not valid JSON");
            return ExitCode::FAILURE;
        }
    };
    let target_before = std::fs::read(&target_path).unwrap_or_default();
    let mut merged: serde_json::Value = if target_before.is_empty() {
        serde_json::Value::Null
    } else {
        match serde_json::from_slice(&target_before) {
            Ok(value) => value,
            Err(_) => {
                eprintln!("registry sync: target {target_path} is not valid JSON");
                return ExitCode::FAILURE;
            }
        }
    };
    let report: RegistrySyncReport = match apply_registry_sync(&source_value, &mut merged) {
        Ok(report) => report,
        Err(reason) => {
            eprintln!("registry sync refused: {reason}");
            return ExitCode::FAILURE;
        }
    };
    let digest_before = file_digest(&target_before);
    let merged_content = match serde_json::to_string_pretty(&merged) {
        Ok(content) => content,
        Err(_) => {
            eprintln!("registry sync: merged registry is not serializable");
            return ExitCode::FAILURE;
        }
    };
    let digest_after = file_digest(merged_content.as_bytes());
    if digest_before == digest_after {
        return crate::emit_json(
            &serde_json::json!({
                "schema": "cynic.network.registry_sync.v1",
                "source": source,
                "target": target_path,
                "written": false,
                "noop": true,
                "digest": digest_after,
                "report": report,
            }),
            false,
        );
    }
    if !yes {
        return crate::emit_json(
            &serde_json::json!({
                "schema": "cynic.network.registry_sync.v1",
                "source": source,
                "target": target_path,
                "written": false,
                "noop": false,
                "digest_before": digest_before,
                "digest_after": digest_after,
                "report": report,
            }),
            false,
        );
    }
    if !target_before.is_empty() {
        let backup = format!(
            "{target_path}.bak-{}",
            file_digest(&source_content)[..12].to_string()
        );
        if let Err(_) = std::fs::write(&backup, &target_before) {
            eprintln!("registry sync: cannot write backup {backup}");
            return ExitCode::FAILURE;
        }
        prune_sync_backups(&target_path);
    }
    if let Err(_) = std::fs::write(&target_path, merged_content) {
        eprintln!("registry sync: cannot write target {target_path}");
        return ExitCode::FAILURE;
    }
    crate::emit_json(
        &serde_json::json!({
            "schema": "cynic.network.registry_sync.v1",
            "source": source,
            "target": target_path,
            "written": true,
            "noop": false,
            "digest_before": digest_before,
            "digest_after": digest_after,
            "report": report,
        }),
        false,
    )
}

fn prune_sync_backups(target_path: &str) {
    let parent = std::path::Path::new(target_path)
        .parent()
        .map(|parent| parent.to_path_buf());
    let file_name = std::path::Path::new(target_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string());
    let (Some(parent), Some(file_name)) = (parent, file_name) else {
        return;
    };
    let prefix = format!("{file_name}.bak-");
    let mut backups = std::fs::read_dir(&parent)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
                .filter_map(|entry| {
                    entry
                        .metadata()
                        .ok()
                        .and_then(|metadata| metadata.modified().ok())
                        .map(|modified| (modified, entry.path()))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    backups.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, path) in backups.into_iter().skip(MAX_REGISTRY_SYNC_BACKUPS) {
        let _ = std::fs::remove_file(path);
    }
}
