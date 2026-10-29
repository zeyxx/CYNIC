//! Local append-only evidence spool.
//!
//! The spool is a durability membrane, not a second source of truth. It keeps
//! machine-readable observations while the central store is unavailable and
//! exposes an explicit drain operation for a later reconciler.

use serde::{Serialize, de::DeserializeOwned};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_SPOOL_BYTES: u64 = 64 * 1024 * 1024;
const CLAIM_STALE_AFTER_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct LocalEvidenceSpool {
    path: PathBuf,
    operation_lock: Arc<Mutex<()>>,
}

impl LocalEvidenceSpool {
    pub fn from_env(name: &str) -> Option<Self> {
        let root = std::env::var("CYNIC_LOCAL_SPOOL_DIR")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        Some(Self {
            path: PathBuf::from(root).join(format!("{name}.jsonl")),
            operation_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            operation_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn ready(&self) -> bool {
        // Observation must not create directories or otherwise mutate the host.
        // Provisioning owns directory creation; append is the bounded mutation.
        self.path.parent().is_some_and(Path::is_dir)
    }

    pub fn append<T: Serialize>(&self, value: &T) -> Result<(), String> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| "spool operation lock poisoned".to_string())?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "spool has no parent".to_string())?;
        fs::create_dir_all(parent).map_err(|error| format!("create spool directory: {error}"))?;
        let encoded =
            serde_json::to_vec(value).map_err(|error| format!("encode spool record: {error}"))?;
        let current_size = self.spool_bytes()?;
        if current_size.saturating_add(encoded.len() as u64 + 1) > MAX_SPOOL_BYTES {
            return Err(format!("spool capacity exceeded ({MAX_SPOOL_BYTES} bytes)"));
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("open spool: {error}"))?;
        file.write_all(&encoded)
            .map_err(|error| format!("write spool: {error}"))?;
        file.write_all(b"\n")
            .map_err(|error| format!("write spool newline: {error}"))?;
        file.sync_data()
            .map_err(|error| format!("sync spool: {error}"))
    }

    pub fn pending(&self) -> Result<usize, String> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| "spool operation lock poisoned".to_string())?;
        let mut count = 0;
        for path in self.segment_paths()? {
            let file = fs::File::open(&path)
                .map_err(|error| format!("open spool segment {}: {error}", path.display()))?;
            count += BufReader::new(file)
                .lines()
                .filter(|line| line.as_ref().is_ok_and(|line| !line.trim().is_empty()))
                .count();
        }
        Ok(count)
    }

    /// Read all records without changing the spool. This is intentionally
    /// separate from `drain`: observation must never acknowledge evidence.
    pub fn read<T: DeserializeOwned>(&self) -> Result<Vec<T>, String> {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| "spool operation lock poisoned".to_string())?;
        let mut records = Vec::new();
        for path in self.segment_paths()? {
            let file = fs::File::open(&path)
                .map_err(|error| format!("open spool segment {}: {error}", path.display()))?;
            records.extend(BufReader::new(file).lines().filter_map(|line| match line {
                Ok(line) if line.trim().is_empty() => None,
                Ok(line) => Some(
                    serde_json::from_str(&line).map_err(|error| format!("decode spool: {error}")),
                ),
                Err(error) => Some(Err(format!("read spool: {error}"))),
            }));
        }
        records.into_iter().collect()
    }

    /// Drain only after the consumer has durably accepted every record.
    /// The active file is first rotated atomically. Each rotated segment is
    /// then claimed with another atomic rename, so concurrent processes cannot
    /// acknowledge the same active segment or delete a concurrent append.
    /// A claim left by a crashed process is eligible for replay after a bounded
    /// stale interval; callback failures immediately restore the drain segment.
    pub fn drain<T: DeserializeOwned, F>(&self, mut consume: F) -> Result<usize, String>
    where
        F: FnMut(T) -> Result<(), String>,
    {
        let _guard = self
            .operation_lock
            .lock()
            .map_err(|_| "spool operation lock poisoned".to_string())?;
        self.rotate_active_segment()?;
        let mut consumed = 0;
        for segment in self
            .reclaimable_segment_paths()?
            .into_iter()
            .filter(|path| path != &self.path)
        {
            let claimed = self.claim_path(&segment);
            match fs::rename(&segment, &claimed) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(format!("claim spool segment: {error}")),
            }
            let file = match fs::File::open(&claimed) {
                Ok(file) => file,
                Err(error) => {
                    let _ = fs::rename(&claimed, &segment);
                    return Err(format!("open claimed spool segment: {error}"));
                }
            };
            let mut segment_consumed = 0;
            for line in BufReader::new(file).lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        let _ = fs::rename(&claimed, &segment);
                        return Err(format!("read spool segment: {error}"));
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                let record = match serde_json::from_str(&line) {
                    Ok(record) => record,
                    Err(error) => {
                        let _ = fs::rename(&claimed, &segment);
                        return Err(format!("decode spool segment: {error}"));
                    }
                };
                if let Err(error) = consume(record) {
                    let _ = fs::rename(&claimed, &segment);
                    return Err(error);
                }
                segment_consumed += 1;
            }
            fs::remove_file(&claimed)
                .map_err(|error| format!("remove drained spool segment: {error}"))?;
            consumed += segment_consumed;
        }
        Ok(consumed)
    }

    fn spool_bytes(&self) -> Result<u64, String> {
        Ok(self
            .segment_paths()?
            .into_iter()
            .filter_map(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum())
    }

    fn segment_paths(&self) -> Result<Vec<PathBuf>, String> {
        let Some(parent) = self.path.parent() else {
            return Ok(if self.path.exists() {
                vec![self.path.clone()]
            } else {
                Vec::new()
            });
        };
        if !parent.is_dir() {
            return Ok(Vec::new());
        }
        let active_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "spool filename is not valid UTF-8".to_string())?;
        let mut paths = fs::read_dir(parent)
            .map_err(|error| format!("list spool directory: {error}"))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path == &self.path
                    || path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with(&format!("{active_name}.drain."))
                                || name.starts_with(&format!("{active_name}.claimed."))
                        })
            })
            .collect::<Vec<_>>();
        paths.sort();
        Ok(paths)
    }

    fn reclaimable_segment_paths(&self) -> Result<Vec<PathBuf>, String> {
        let now = SystemTime::now();
        Ok(self
            .segment_paths()?
            .into_iter()
            .filter(|path| {
                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    return false;
                };
                if name.contains(".drain.") {
                    return true;
                }
                if !name.contains(".claimed.") {
                    return false;
                }
                fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|modified| now.duration_since(modified).ok())
                    .is_some_and(|age| age.as_secs() >= CLAIM_STALE_AFTER_SECS)
            })
            .collect())
    }

    fn rotate_active_segment(&self) -> Result<(), String> {
        if !self.path.exists() {
            return Ok(());
        }
        let rotated = self.segment_path("drain");
        match fs::rename(&self.path, &rotated) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("rotate active spool segment: {error}")),
        }
    }

    fn segment_path(&self, kind: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let pid = std::process::id();
        PathBuf::from(format!(
            "{}.{}.{}.{}",
            self.path.display(),
            kind,
            pid,
            nonce
        ))
    }

    fn claim_path(&self, segment: &Path) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let _ = segment;
        PathBuf::from(format!(
            "{}.claimed.{}.{}",
            self.path.display(),
            std::process::id(),
            nonce
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::LocalEvidenceSpool;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Record {
        id: String,
        value: u32,
    }

    #[test]
    fn append_pending_and_drain_are_durable_steps() {
        let root = tempfile::tempdir().expect("tempdir");
        let spool = LocalEvidenceSpool::at(root.path().join("evidence.jsonl"));
        spool
            .append(&Record {
                id: "a".into(),
                value: 1,
            })
            .expect("append");
        spool
            .append(&Record {
                id: "b".into(),
                value: 2,
            })
            .expect("append");
        assert_eq!(spool.pending().expect("pending"), 2);
        let mut records = Vec::new();
        let drained = spool
            .drain::<Record, _>(|record| {
                records.push(record);
                Ok(())
            })
            .expect("drain");
        assert_eq!(drained, 2);
        assert_eq!(records.len(), 2);
        assert_eq!(spool.pending().expect("pending after drain"), 0);
    }

    #[test]
    fn cloned_spool_serializes_concurrent_appends() {
        let root = tempfile::tempdir().expect("tempdir");
        let spool = LocalEvidenceSpool::at(root.path().join("concurrent.jsonl"));
        let first = spool.clone();
        let second = spool.clone();
        let left = std::thread::spawn(move || {
            first
                .append(&Record {
                    id: "left".into(),
                    value: 1,
                })
                .expect("left append");
        });
        let right = std::thread::spawn(move || {
            second
                .append(&Record {
                    id: "right".into(),
                    value: 2,
                })
                .expect("right append");
        });
        left.join().expect("left join");
        right.join().expect("right join");
        assert_eq!(spool.pending().expect("pending"), 2);
        assert_eq!(spool.read::<Record>().expect("read").len(), 2);
    }

    #[test]
    fn failed_consumer_leaves_claimed_segment_for_replay() {
        let root = tempfile::tempdir().expect("tempdir");
        let spool = LocalEvidenceSpool::at(root.path().join("replay.jsonl"));
        spool
            .append(&Record {
                id: "replay".into(),
                value: 7,
            })
            .expect("append");
        let failed = spool.drain::<Record, _>(|_| Err("store unavailable".to_string()));
        assert!(failed.is_err());
        assert_eq!(spool.pending().expect("pending after failed drain"), 1);

        let mut replayed = Vec::new();
        let drained = spool
            .drain::<Record, _>(|record| {
                replayed.push(record);
                Ok(())
            })
            .expect("replay");
        assert_eq!(drained, 1);
        assert_eq!(replayed[0].id, "replay");
        assert_eq!(spool.pending().expect("pending after replay"), 0);
    }

    #[test]
    fn append_during_drain_survives_in_a_new_active_segment() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("rotation.jsonl");
        let spool = LocalEvidenceSpool::at(path.clone());
        let concurrent_writer = LocalEvidenceSpool::at(path);
        spool
            .append(&Record {
                id: "before".into(),
                value: 1,
            })
            .expect("append");
        spool
            .drain::<Record, _>(|record| {
                assert_eq!(record.id, "before");
                concurrent_writer
                    .append(&Record {
                        id: "during".into(),
                        value: 2,
                    })
                    .expect("concurrent append");
                Ok(())
            })
            .expect("drain");
        assert_eq!(spool.pending().expect("pending after rotation"), 1);
        assert_eq!(spool.read::<Record>().expect("read")[0].id, "during");
    }
}
