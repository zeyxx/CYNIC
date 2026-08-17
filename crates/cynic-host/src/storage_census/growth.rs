use super::types::{StorageCensusSnapshot, StorageFilesystemSnapshot, StorageGrowthObservation};
use crate::process_observation::capture_command;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub fn observe_snapshot() -> Result<StorageCensusSnapshot, String> {
    let output = capture_command("df", &["-B1", "--output=source,size,used,avail,target"])
        .map_err(|error| format!("df growth observation unavailable: {error}"))?;
    if !output.success {
        return Err(format!(
            "df growth observation failed (exit_code={:?}): {}",
            output.exit_code, output.stderr
        ));
    }
    let mut filesystems = Vec::new();
    for line in output.stdout.lines().skip(1) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 5 {
            continue;
        }
        let (Ok(total_bytes), Ok(used_bytes), Ok(available_bytes)) =
            (fields[1].parse(), fields[2].parse(), fields[3].parse())
        else {
            continue;
        };
        filesystems.push(StorageFilesystemSnapshot {
            scope: fields[4].to_owned(),
            source: fields[0].to_owned(),
            total_bytes,
            used_bytes,
            available_bytes,
        });
    }
    if filesystems.is_empty() {
        return Err("df returned no parseable filesystems".into());
    }
    Ok(StorageCensusSnapshot {
        schema: "cynic.host.storage_census_snapshot.v1".into(),
        observed_at_unix_ms: chrono::Utc::now().timestamp_millis().max(0) as u128,
        filesystems,
    })
}

pub fn read_previous_snapshot(path: &Path) -> Result<StorageCensusSnapshot, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("previous storage snapshot read failed: {error}"))?;
    serde_json::from_str::<StorageCensusSnapshot>(&raw)
        .map_err(|error| format!("previous storage snapshot JSON invalid: {error}"))
}

pub fn compare_snapshots(
    previous: &StorageCensusSnapshot,
    current: &StorageCensusSnapshot,
) -> Vec<StorageGrowthObservation> {
    let previous_by_scope = previous
        .filesystems
        .iter()
        .map(|filesystem| (filesystem.scope.as_str(), filesystem.used_bytes))
        .collect::<BTreeMap<_, _>>();
    current
        .filesystems
        .iter()
        .map(|filesystem| {
            let previous_bytes = previous_by_scope.get(filesystem.scope.as_str()).copied();
            StorageGrowthObservation {
                scope: filesystem.scope.clone(),
                previous_bytes,
                current_bytes: Some(filesystem.used_bytes),
                delta_bytes: previous_bytes
                    .map(|value| filesystem.used_bytes as i128 - value as i128)
                    .and_then(|delta| i64::try_from(delta).ok()),
                observed_at_unix_ms: current.observed_at_unix_ms,
                sample_count: 2,
            }
        })
        .collect()
}

pub fn load_previous_snapshot_from_env() -> Result<Option<StorageCensusSnapshot>, String> {
    let Some(path) = std::env::var_os("CYNIC_STORAGE_CENSUS_PREVIOUS_SNAPSHOT") else {
        return Ok(None);
    };
    read_previous_snapshot(Path::new(&path)).map(Some)
}

// Serialisation utility not yet consumed by a CLI projection.
#[allow(dead_code)]
pub fn snapshot_as_value(snapshot: &StorageCensusSnapshot) -> Value {
    serde_json::to_value(snapshot).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(used_bytes: u64) -> StorageCensusSnapshot {
        StorageCensusSnapshot {
            schema: "cynic.host.storage_census_snapshot.v1".into(),
            observed_at_unix_ms: 2,
            filesystems: vec![StorageFilesystemSnapshot {
                scope: "/".into(),
                source: "/dev/test".into(),
                total_bytes: 100,
                used_bytes,
                available_bytes: 100 - used_bytes,
            }],
        }
    }

    #[test]
    fn compares_usage_by_mount_scope() {
        let growth = compare_snapshots(&snapshot(40), &snapshot(70));
        assert_eq!(growth.len(), 1);
        assert_eq!(growth[0].scope, "/");
        assert_eq!(growth[0].previous_bytes, Some(40));
        assert_eq!(growth[0].current_bytes, Some(70));
        assert_eq!(growth[0].delta_bytes, Some(30));
        assert_eq!(growth[0].sample_count, 2);
    }

    #[test]
    fn leaves_new_filesystem_without_previous_value() {
        let previous = snapshot(40);
        let current = StorageCensusSnapshot {
            filesystems: vec![StorageFilesystemSnapshot {
                scope: "/new".into(),
                source: "/dev/new".into(),
                total_bytes: 100,
                used_bytes: 10,
                available_bytes: 90,
            }],
            ..snapshot(70)
        };
        let growth = compare_snapshots(&previous, &current);
        assert_eq!(growth[0].previous_bytes, None);
        assert_eq!(growth[0].delta_bytes, None);
    }
}
