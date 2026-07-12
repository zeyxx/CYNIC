use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::command_surface::{HostCommandInvocation, HostCommandSurface};
use crate::execution::{HostCommandExecution, execute_host_commands};

use super::context::lab_context;
use super::registry::read_registry;
use super::types::{
    LabImageCommand, LabImageExecution, LabImagePreflight, LabImageReport, LabImageResolveReport,
};

pub fn build_lab_image_report(registry_path: &str, apply: bool) -> Result<LabImageReport, String> {
    let registry = read_registry(registry_path)?;
    let image = image_preflight(&registry);
    let blockers = image_fetch_blockers(&image);
    let commands = image_fetch_commands(&image);
    let executions = if apply && blockers.is_empty() {
        run_image_commands(&commands)
    } else {
        Vec::new()
    };
    let observed_sha256 = executions
        .iter()
        .find(|execution| execution.role == "verify_base_image")
        .and_then(|execution| parse_sha256sum_stdout(&execution.stdout));
    let checksum_matches = image
        .sha256
        .as_ref()
        .is_some_and(|expected| observed_sha256.as_ref() == Some(expected));
    let execution_success = executions
        .iter()
        .all(|execution| execution.exit_code == Some(0));

    Ok(LabImageReport {
        schema: "cynic.lab.image.v1",
        context: lab_context(),
        status: if !blockers.is_empty() {
            if apply { "refused" } else { "blocked" }
        } else if !apply {
            "planned"
        } else if execution_success && checksum_matches {
            "completed"
        } else if executions.is_empty() {
            "ready"
        } else if apply {
            "failed"
        } else {
            "planned"
        },
        mutates: apply && blockers.is_empty(),
        requested_apply: apply,
        can_fetch: blockers.is_empty(),
        commands,
        executions,
        observed_sha256,
        checksum_matches,
        image,
        blockers,
        safety_rules: vec![
            "image fetch may only write to registry lab.base_image.path",
            "image fetch requires a pinned 64-character lowercase sha256 in registry",
            "download and checksum verification must use argv commands, never shell",
            "VM provisioning remains blocked until image exists and checksum is valid",
        ],
    })
}

pub fn resolve_lab_image_sha256(registry_path: &str) -> Result<LabImageResolveReport, String> {
    let registry = read_registry(registry_path)?;
    let image = image_preflight(&registry);
    let image_name = image_name(&image)?;
    let mut blockers = Vec::new();
    let resolved_sha256 = match image.sha256_url.as_deref() {
        Some(url) => {
            let sums = fetch_text(url)?;
            match parse_sha256_sums(&sums, &image_name) {
                Some(digest) => Some(digest),
                None => {
                    blockers.push("image name was not found in sha256 sums");
                    None
                }
            }
        }
        None => {
            blockers.push("registry lab.base_image.sha256_url is missing");
            None
        }
    };
    let registry_matches_resolved = image
        .sha256
        .as_ref()
        .is_some_and(|registry_sha| resolved_sha256.as_ref() == Some(registry_sha));
    let proposed_registry_patch = resolved_sha256.as_ref().map(|sha256| {
        serde_json::json!({
            "op": "replace",
            "path": "/lab/base_image/sha256",
            "value": sha256
        })
    });

    Ok(LabImageResolveReport {
        schema: "cynic.lab.image_resolve.v1",
        context: lab_context(),
        status: if blockers.is_empty() {
            "resolved"
        } else {
            "blocked"
        },
        mutates: false,
        image_name,
        sha256_url: image.sha256_url.clone(),
        resolved_sha256,
        registry_sha256: image.sha256.clone(),
        registry_matches_resolved,
        proposed_registry_patch,
        blockers,
        safety_rules: vec![
            "resolve fetches SHA256SUMS only and does not download the image",
            "resolve never edits the registry automatically",
            "operator must review and apply the proposed registry patch explicitly",
        ],
    })
}

pub(crate) fn image_preflight(registry: &Value) -> LabImagePreflight {
    let image = registry.get("lab").and_then(|lab| lab.get("base_image"));
    let path = image
        .and_then(|image| image.get("path"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let sha256 = image
        .and_then(|image| image.get("sha256"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    LabImagePreflight {
        declared: image.is_some(),
        source_url: image
            .and_then(|image| image.get("source_url"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        sha256_url: image
            .and_then(|image| image.get("sha256_url"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        sha256: sha256.clone(),
        checksum_declared: image
            .and_then(|image| image.get("sha256"))
            .and_then(Value::as_str)
            .is_some(),
        checksum_valid_shape: sha256.as_deref().map(is_sha256_hex).unwrap_or(false),
        exists_on_host: path
            .as_deref()
            .map(|path| Path::new(path).is_file())
            .unwrap_or(false),
        path,
    }
}

fn fetch_text(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--proto", "=https", url])
        .output()
        .map_err(|error| format!("failed to execute curl: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "curl failed for {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| format!("curl output was not utf-8: {error}"))
}

fn image_name(image: &LabImagePreflight) -> Result<String, String> {
    image
        .source_url
        .as_deref()
        .and_then(|url| url.rsplit('/').next())
        .filter(|name| !name.trim().is_empty())
        .or_else(|| {
            image
                .path
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|name| name.to_str())
        })
        .map(ToString::to_string)
        .ok_or_else(|| "cannot derive image filename from lab.base_image".to_string())
}

fn image_fetch_blockers(image: &LabImagePreflight) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !image.declared {
        blockers.push("registry lab.base_image is missing");
    }
    if image.path.is_none() {
        blockers.push("registry lab.base_image.path is missing");
    }
    if image.source_url.is_none() {
        blockers.push("registry lab.base_image.source_url is missing");
    }
    if image.sha256_url.is_none() {
        blockers.push("registry lab.base_image.sha256_url is missing");
    }
    if !image.checksum_declared {
        blockers.push("registry lab.base_image.sha256 is missing");
    }
    if image.checksum_declared && !image.checksum_valid_shape {
        blockers.push("registry lab.base_image.sha256 must be a 64-character lowercase hex digest");
    }
    blockers
}

fn image_fetch_commands(image: &LabImagePreflight) -> Vec<LabImageCommand> {
    let Some(path) = image.path.as_ref() else {
        return Vec::new();
    };
    let Some(url) = image.source_url.as_ref() else {
        return Vec::new();
    };
    let parent = Path::new(path)
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| ".".to_string());

    vec![
        LabImageCommand {
            role: "create_image_dir",
            kind: "argv",
            argv: vec![
                "install".to_string(),
                "-d".to_string(),
                "-m".to_string(),
                "0755".to_string(),
                parent,
            ],
        },
        LabImageCommand {
            role: "download_base_image",
            kind: "argv",
            argv: vec![
                "curl".to_string(),
                "-fL".to_string(),
                "--proto".to_string(),
                "=https".to_string(),
                "-o".to_string(),
                path.clone(),
                url.clone(),
            ],
        },
        LabImageCommand {
            role: "verify_base_image",
            kind: "argv",
            argv: vec!["sha256sum".to_string(), path.clone()],
        },
    ]
}

fn run_image_commands(commands: &[LabImageCommand]) -> Vec<LabImageExecution> {
    let surfaces = commands
        .iter()
        .map(image_command_surface)
        .collect::<Vec<_>>();
    execute_host_commands(&surfaces, true, true)
        .executions
        .into_iter()
        .map(image_execution)
        .collect()
}

fn image_command_surface(command: &LabImageCommand) -> HostCommandSurface {
    HostCommandSurface {
        role: command.role.to_string(),
        invocation: HostCommandInvocation::Argv {
            argv: command.argv.clone(),
        },
    }
}

fn image_execution(execution: HostCommandExecution) -> LabImageExecution {
    LabImageExecution {
        role: execution.role,
        argv: execution.argv,
        exit_code: execution.exit_code,
        stdout: execution.stdout,
        stderr: execution.stderr,
    }
}

pub(crate) fn parse_sha256_sums(sums: &str, image_name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let digest = parts.next()?;
        let path = parts.next()?;
        let normalized = path.trim_start_matches('*').trim_start_matches("./");
        if normalized == image_name && is_sha256_hex(digest) {
            Some(digest.to_string())
        } else {
            None
        }
    })
}

pub(crate) fn parse_sha256sum_stdout(stdout: &str) -> Option<String> {
    stdout.split_whitespace().next().and_then(|digest| {
        if is_sha256_hex(digest) {
            Some(digest.to_string())
        } else {
            None
        }
    })
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
