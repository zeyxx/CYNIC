use crate::ports::storage_lease_port::StorageLeasePort;
use cynic_protocol::LeaseGuard;
use cynic_storage::{STORAGE_LEASE_SCHEMA, StorageLeaseGrant};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone)]
pub struct LocalStorageLeaseAdapter {
    root: PathBuf,
}
impl LocalStorageLeaseAdapter {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl StorageLeasePort for LocalStorageLeaseAdapter {
    fn acquire(
        &self,
        resource_id: &str,
        agent_id: &str,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<StorageLeaseGrant, String> {
        validate(resource_id, agent_id, ttl_ms)?;
        fs::create_dir_all(&self.root)
            .map_err(|_| "storage_lease_root_create_failed".to_string())?;
        let _lock = acquire_lock(&self.root.join(".leases.lock"))?;
        let path = self.root.join(format!("{resource_id}.json"));
        if let Some(existing) = read(&path)? {
            if existing.expires_at_ms > now_ms {
                return Err("storage_lease_already_held".to_string());
            }
        }
        let prior = read(&path)?
            .map(|grant| grant.guard.fencing_token)
            .unwrap_or(0);
        let grant = StorageLeaseGrant {
            schema: STORAGE_LEASE_SCHEMA.to_string(),
            guard: LeaseGuard {
                agent_id: agent_id.to_string(),
                target: format!("storage:{resource_id}"),
                lease_id: format!("storage-lease-{resource_id}-{now_ms}"),
                fencing_token: prior.saturating_add(1),
            },
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        };
        write(&path, &grant)?;
        Ok(grant)
    }
    fn renew(
        &self,
        guard: &LeaseGuard,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<StorageLeaseGrant, String> {
        if guard.fencing_token == 0 || guard.lease_id.is_empty() || ttl_ms == 0 {
            return Err("storage_lease_guard_invalid".to_string());
        }
        fs::create_dir_all(&self.root)
            .map_err(|_| "storage_lease_root_create_failed".to_string())?;
        let _lock = acquire_lock(&self.root.join(".leases.lock"))?;
        let path = path_for(&self.root, &guard.target)?;
        let current = read(&path)?.ok_or_else(|| "storage_lease_missing".to_string())?;
        if current.guard != *guard || current.expires_at_ms <= now_ms {
            return Err("storage_lease_stale_or_expired".to_string());
        }
        let grant = StorageLeaseGrant {
            expires_at_ms: now_ms.saturating_add(ttl_ms),
            ..current
        };
        write(&path, &grant)?;
        Ok(grant)
    }
    fn release(&self, guard: &LeaseGuard, now_ms: u64) -> Result<(), String> {
        fs::create_dir_all(&self.root)
            .map_err(|_| "storage_lease_root_create_failed".to_string())?;
        let _lock = acquire_lock(&self.root.join(".leases.lock"))?;
        let path = path_for(&self.root, &guard.target)?;
        let current = read(&path)?.ok_or_else(|| "storage_lease_missing".to_string())?;
        if current.guard != *guard || current.expires_at_ms <= now_ms {
            return Err("storage_lease_stale_or_expired".to_string());
        }
        fs::remove_file(path).map_err(|_| "storage_lease_release_failed".to_string())
    }
}
fn validate(resource_id: &str, agent_id: &str, ttl_ms: u64) -> Result<(), String> {
    if resource_id.is_empty() || resource_id.contains('/') || agent_id.is_empty() || ttl_ms == 0 {
        Err("storage_lease_request_invalid".to_string())
    } else {
        Ok(())
    }
}
fn path_for(root: &std::path::Path, target: &str) -> Result<PathBuf, String> {
    let id = target
        .strip_prefix("storage:")
        .ok_or_else(|| "storage_lease_target_invalid".to_string())?;
    if id.is_empty() || id.contains('/') {
        return Err("storage_lease_target_invalid".to_string());
    }
    Ok(root.join(format!("{id}.json")))
}
fn read(path: &std::path::Path) -> Result<Option<StorageLeaseGrant>, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "storage_lease_decode_failed".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("storage_lease_read_failed".to_string()),
    }
}
fn write(path: &std::path::Path, grant: &StorageLeaseGrant) -> Result<(), String> {
    let temp = path.with_extension("tmp");
    fs::write(
        &temp,
        serde_json::to_vec_pretty(grant).map_err(|_| "storage_lease_encode_failed".to_string())?,
    )
    .map_err(|_| "storage_lease_write_failed".to_string())?;
    fs::rename(temp, path).map_err(|_| "storage_lease_publish_failed".to_string())
}

fn acquire_lock(path: &std::path::Path) -> Result<std::fs::File, String> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| "storage_lease_lock_open_failed".to_string())?;
    #[cfg(unix)]
    if rustix::fs::flock(&file, rustix::fs::FlockOperation::LockExclusive).is_err() {
        return Err("storage_lease_lock_failed".to_string());
    }
    Ok(file)
}
