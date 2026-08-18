use crate::process_observation::capture_command;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Observe the high-value subset of process/storage causality that can be
/// collected without privilege: deleted files which remain open.
///
/// This is deliberately a partial observer. It does not claim a complete
/// process-to-file graph and never mutates the host.
pub fn observe_deleted_open_files() -> Result<Value, String> {
    let output = capture_command("lsof", &["-nP", "+L1", "-F", "pcnst"])
        .map_err(|error| format!("lsof observation unavailable: {error}"))?;
    if !output.success {
        return Err(format!(
            "lsof observation failed (exit_code={:?}): {}",
            output.exit_code,
            if output.stderr.is_empty() {
                "no stderr"
            } else {
                output.stderr.as_str()
            }
        ));
    }

    let mut processes: BTreeMap<String, ProcessEntry> = BTreeMap::new();
    let mut current_pid = String::new();
    let mut current_command = String::new();
    let mut current_file: Option<OpenFileEntry> = None;
    for line in output.stdout.lines() {
        let Some(kind) = line.chars().next() else {
            continue;
        };
        let value = &line[kind.len_utf8()..];
        match kind {
            'p' => {
                flush_file(&mut processes, &current_pid, &mut current_file);
                current_pid = value.to_owned();
            }
            'c' => current_command = value.to_owned(),
            't' => {
                if let Some(file) = current_file.as_mut() {
                    file.file_type = Some(value.to_owned());
                }
            }
            'n' => {
                flush_file(&mut processes, &current_pid, &mut current_file);
                current_file = Some(OpenFileEntry {
                    name: value.to_owned(),
                    file_type: None,
                    size_bytes: None,
                });
            }
            's' => {
                if let Some(file) = current_file.as_mut() {
                    file.size_bytes = value.parse().ok();
                }
            }
            _ => {}
        }
        if !current_pid.is_empty() {
            processes
                .entry(current_pid.clone())
                .or_insert_with(|| ProcessEntry {
                    pid: current_pid.clone(),
                    command: current_command.clone(),
                    files: Vec::new(),
                })
                .command = current_command.clone();
        }
    }
    flush_file(&mut processes, &current_pid, &mut current_file);

    Ok(json!({
        "schema": "cynic.host.storage_process_inventory.v1",
        "scope": "deleted_open_files",
        "processes": processes.into_values().collect::<Vec<_>>(),
        "complete": false,
        "residual_unknowns": [
            "ordinary process-to-file attribution requires a broader observer and privileges"
        ]
    }))
}

#[derive(Debug)]
struct ProcessEntry {
    pid: String,
    command: String,
    files: Vec<OpenFileEntry>,
}

impl serde::Serialize for ProcessEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ProcessEntry", 3)?;
        state.serialize_field("pid", &self.pid)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("files", &self.files)?;
        state.end()
    }
}

#[derive(Debug, serde::Serialize)]
struct OpenFileEntry {
    name: String,
    file_type: Option<String>,
    size_bytes: Option<u64>,
}

fn flush_file(
    processes: &mut BTreeMap<String, ProcessEntry>,
    pid: &str,
    file: &mut Option<OpenFileEntry>,
) {
    let Some(file) = file.take() else { return };
    if let Some(process) = processes.get_mut(pid) {
        process.files.push(file);
    }
}
