//! Ephemeral fleet sweep — pure planner plus lease-file helpers.
//!
//! The sweeper never destroys what it cannot prove ephemeral: templates
//! and static reservations are skipped, unleased guests are reported
//! (fail closed, siblings share the cluster), only expired leases destroy.

use serde::{Deserialize, Serialize};

/// A guest as observed from the hypervisor (REST list or config read-back).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedGuest {
    pub vmid: u32,
    pub name: String,
    pub status: String,
    pub template: bool,
}

/// A TTL lease written at clone time. Lives in the runtime lease file
/// (not git); reconciled against observed state at sweep time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetLease {
    pub vmid: u32,
    pub expires_at: u64,
    pub owner: String,
    pub purpose: String,
}

/// Sweep decision per guest. `Destroy` only for expired leases;
/// `Report` for anything the sweeper must not touch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SweepAction {
    Destroy { vmid: u32, reason: String },
    Report { vmid: u32, reason: String },
}

/// Pure sweep planner: no I/O, fully unit-tested.
///
/// - template guests and static reservations are skipped silently,
/// - expired leases (now >= expires_at) destroy,
/// - valid leases keep (any power state),
/// - unleased, non-static, non-template guests report (never destroy).
pub fn plan_sweep(
    observed: &[ObservedGuest],
    leases: &[FleetLease],
    static_vmids: &[u32],
    now: u64,
) -> Vec<SweepAction> {
    let mut actions = Vec::new();
    for guest in observed {
        if guest.template {
            continue;
        }
        if static_vmids.contains(&guest.vmid) {
            continue;
        }
        match leases.iter().find(|lease| lease.vmid == guest.vmid) {
            Some(lease) if now >= lease.expires_at => actions.push(SweepAction::Destroy {
                vmid: guest.vmid,
                reason: format!("expired_lease:owner={}", lease.owner),
            }),
            Some(_) => {}
            None => actions.push(SweepAction::Report {
                vmid: guest.vmid,
                reason: "unleased_guest:fail_closed".to_string(),
            }),
        }
    }
    actions
}

/// Read the runtime lease file. Missing file means no leases (not an
/// error); corrupt JSON is an error (never sweep blind).
pub fn read_leases(path: &std::path::Path) -> Result<Vec<FleetLease>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("fleet_leases_unreadable:{}:{error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("fleet_leases_corrupt:{}:{error}", path.display()))
}

/// Record (or refresh) a lease: read-modify-write, same vmid replaced.
/// Parent directories are created on demand.
pub fn write_lease(path: &std::path::Path, lease: &FleetLease) -> Result<(), String> {
    let mut leases = read_leases(path)?;
    leases.retain(|existing| existing.vmid != lease.vmid);
    leases.push(lease.clone());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("fleet_leases_mkdir:{}:{error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&leases)
        .map_err(|error| format!("fleet_leases_encode:{error}"))?;
    std::fs::write(path, text)
        .map_err(|error| format!("fleet_leases_unwritable:{}:{error}", path.display()))
}

/// Drop a lease after a successful destroy (keeps the file truthful).
pub fn drop_lease(path: &std::path::Path, vmid: u32) -> Result<(), String> {
    let mut leases = read_leases(path)?;
    leases.retain(|existing| existing.vmid != vmid);
    let text = serde_json::to_string_pretty(&leases)
        .map_err(|error| format!("fleet_leases_encode:{error}"))?;
    std::fs::write(path, text)
        .map_err(|error| format!("fleet_leases_unwritable:{}:{error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guest(vmid: u32, template: bool) -> ObservedGuest {
        ObservedGuest {
            vmid,
            name: format!("agent-{vmid}"),
            status: "running".to_string(),
            template,
        }
    }

    fn lease(vmid: u32, expires_at: u64) -> FleetLease {
        FleetLease {
            vmid,
            expires_at,
            owner: "cynic-agent".to_string(),
            purpose: format!("agent-{vmid}"),
        }
    }

    #[test]
    fn expired_leases_destroy_everything_else_survives() {
        let observed = vec![
            guest(120, false),
            guest(121, false),
            guest(122, false),
            guest(113, true),
            guest(107, false),
        ];
        let leases = vec![lease(120, 100), lease(121, 9_999)];

        let actions = plan_sweep(&observed, &leases, &[107], 200);

        assert!(actions.contains(&SweepAction::Destroy {
            vmid: 120,
            reason: "expired_lease:owner=cynic-agent".to_string(),
        }));
        assert!(!actions.iter().any(|action| match action {
            SweepAction::Destroy { vmid, .. } => *vmid == 121,
            SweepAction::Report { vmid, .. } => *vmid == 121,
        }));
        assert!(!actions.iter().any(|action| match action {
            SweepAction::Destroy { vmid, .. } => *vmid == 113,
            SweepAction::Report { vmid, .. } => *vmid == 113,
        }));
        assert!(!actions.iter().any(|action| match action {
            SweepAction::Destroy { vmid, .. } => *vmid == 107,
            SweepAction::Report { vmid, .. } => *vmid == 107,
        }));
        assert!(actions.contains(&SweepAction::Report {
            vmid: 122,
            reason: "unleased_guest:fail_closed".to_string(),
        }));
    }

    #[test]
    fn lease_roundtrip_replaces_same_vmid() {
        let dir = std::env::temp_dir().join("cynic-fleet-test");
        let path = dir.join("leases.json");
        let _ = std::fs::remove_file(&path);

        write_lease(&path, &lease(120, 100)).expect("write");
        write_lease(&path, &lease(120, 200)).expect("rewrite");
        let leases = read_leases(&path).expect("read");
        assert_eq!(leases.len(), 1);
        assert_eq!(leases[0].expires_at, 200);

        drop_lease(&path, 120).expect("drop");
        assert!(read_leases(&path).expect("read").is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_lease_file_is_empty_not_error() {
        let path = std::env::temp_dir().join("cynic-fleet-test-absent.json");
        let _ = std::fs::remove_file(&path);
        assert!(read_leases(&path).expect("read").is_empty());
    }
}
