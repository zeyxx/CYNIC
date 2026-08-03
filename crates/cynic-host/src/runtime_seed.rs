use std::{fs, path::Path};

use crate::update_lifecycle::ArgvRunner;
use cynic_protocol::{
    RUNTIME_SEED_OBSERVATION_SCHEMA, RuntimeSeedObservation, RuntimeSeedRequest,
    SecurityEvidenceState,
};
use serde::{Deserialize, Serialize};

mod apply;
mod filesystem;
mod registry;
mod signature;

pub use apply::apply_runtime_seed;
use filesystem::{ensure_root, observed_hostname, read_link_marker, regular_file, sha256_file};
use registry::observe_installed_digest;
pub use signature::{RuntimeSeedSignatureVerifier, SshRuntimeSeedSignatureVerifier};

pub const RUNTIME_SEED_CONTRACT_REVISION: u64 = 4;
const RELEASE_MARKER_SCHEMA: &str = "cynic.runtime_seed.release_marker.v3";
const RELEASE_MARKER: &str = ".cynic-runtime-release.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSeedReleaseMarker {
    #[serde(default)]
    pub schema: Option<String>,
    pub release_id: String,
    pub release_epoch: u64,
    #[serde(default)]
    pub source_commit: Option<String>,
    #[serde(default)]
    pub build_proof_digest: Option<String>,
    pub basket_digest: String,
    #[serde(default)]
    pub registry_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSeedApplyReport {
    pub schema: &'static str,
    pub status: String,
    pub before: RuntimeSeedObservation,
    pub after: Option<RuntimeSeedObservation>,
    pub activated: bool,
    pub registry_activated: bool,
    pub rolled_back: bool,
    pub error: Option<String>,
}

pub trait RuntimeSeedActivationPort {
    fn observe(&self, request: &RuntimeSeedRequest) -> RuntimeSeedObservation;
    fn apply(&self, request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport;
    fn persist(
        &self,
        request: &RuntimeSeedRequest,
        report: &RuntimeSeedApplyReport,
    ) -> Result<std::path::PathBuf, String>;
}

pub struct LocalRuntimeSeedActivation<'a, R> {
    verifier: &'a dyn RuntimeSeedSignatureVerifier,
    runner: &'a R,
}

impl<'a, R> LocalRuntimeSeedActivation<'a, R> {
    pub fn new(verifier: &'a dyn RuntimeSeedSignatureVerifier, runner: &'a R) -> Self {
        Self { verifier, runner }
    }
}

impl<R: ArgvRunner> RuntimeSeedActivationPort for LocalRuntimeSeedActivation<'_, R> {
    fn observe(&self, request: &RuntimeSeedRequest) -> RuntimeSeedObservation {
        observe_runtime_seed(request, self.verifier)
    }

    fn apply(&self, request: &RuntimeSeedRequest) -> RuntimeSeedApplyReport {
        apply_runtime_seed(request, self.verifier, self.runner)
    }

    fn persist(
        &self,
        request: &RuntimeSeedRequest,
        report: &RuntimeSeedApplyReport,
    ) -> Result<std::path::PathBuf, String> {
        persist_runtime_seed_evidence(Path::new(&request.install_root), report)
    }
}

pub fn observe_runtime_seed(
    request: &RuntimeSeedRequest,
    verifier: &dyn RuntimeSeedSignatureVerifier,
) -> RuntimeSeedObservation {
    let mut unknowns = Vec::new();
    let observed_node = observed_hostname();
    let basket_present = regular_file(&request.basket_path, "basket").is_ok();
    let detached_signature_present =
        regular_file(&request.detached_signature_path, "signature").is_ok();
    let basket_digest_read_back = if basket_present {
        sha256_file(Path::new(&request.basket_path))
            .map_err(|error| unknowns.push(error))
            .ok()
    } else {
        None
    };
    let signature = if basket_present && detached_signature_present {
        match verifier.verify(request) {
            Ok(()) => SecurityEvidenceState::Proven,
            Err(error) => {
                unknowns.push(error);
                SecurityEvidenceState::Absent
            }
        }
    } else {
        SecurityEvidenceState::Unknown
    };
    let install_root = Path::new(&request.install_root);
    let mut slot_layout = match fs::symlink_metadata(install_root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            SecurityEvidenceState::Proven
        }
        Ok(_) => SecurityEvidenceState::Absent,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SecurityEvidenceState::Absent,
        Err(error) => {
            unknowns.push(format!("runtime_seed_install_root_unobservable:{error}"));
            SecurityEvidenceState::Unknown
        }
    };
    let current = match observe_link_marker(install_root, &request.current_link) {
        Ok(marker) => marker,
        Err(error) => {
            slot_layout = SecurityEvidenceState::Absent;
            unknowns.push(error);
            None
        }
    };
    let previous = match observe_link_marker(install_root, &request.previous_link) {
        Ok(marker) => marker,
        Err(error) => {
            slot_layout = SecurityEvidenceState::Absent;
            unknowns.push(error);
            None
        }
    };

    RuntimeSeedObservation {
        schema: RUNTIME_SEED_OBSERVATION_SCHEMA.to_string(),
        observed_at_unix_ms: now_unix_ms(),
        node_id: observed_node,
        seed_contract_revision: Some(RUNTIME_SEED_CONTRACT_REVISION),
        trust_root: if regular_file(&request.allowed_signers_path, "allowed_signers").is_ok() {
            SecurityEvidenceState::Proven
        } else {
            SecurityEvidenceState::Absent
        },
        signature,
        basket_present,
        detached_signature_present,
        basket_digest_read_back,
        current_release_id: current.as_ref().map(|marker| marker.release_id.clone()),
        current_release_epoch: current.as_ref().map(|marker| marker.release_epoch),
        current_source_commit: current
            .as_ref()
            .and_then(|marker| marker.source_commit.clone()),
        current_build_proof_digest: current
            .as_ref()
            .and_then(|marker| marker.build_proof_digest.clone()),
        current_basket_digest: current.as_ref().map(|marker| marker.basket_digest.clone()),
        current_registry_digest: current
            .as_ref()
            .and_then(|marker| marker.registry_digest.clone()),
        installed_registry_digest: match observe_installed_digest(request) {
            Ok(digest) => digest,
            Err(error) => {
                unknowns.push(error);
                None
            }
        },
        previous_release_id: previous.map(|marker| marker.release_id),
        slot_layout,
        postflight: SecurityEvidenceState::Unknown,
        evidence_refs: vec![
            format!("seed-contract-revision:{RUNTIME_SEED_CONTRACT_REVISION}"),
            format!("node:{request_node}", request_node = request.node_id),
        ],
        unknowns,
    }
}

fn observe_link_marker(
    root: &Path,
    relative_link: &str,
) -> Result<Option<RuntimeSeedReleaseMarker>, String> {
    match fs::symlink_metadata(root.join(relative_link)) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            read_link_marker(root, relative_link).map(Some)
        }
        Ok(_) => Err(format!("runtime_seed_{relative_link}_link_unsafe")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!(
            "runtime_seed_{relative_link}_link_unobservable:{error}"
        )),
    }
}

pub fn persist_runtime_seed_evidence(
    install_root: &Path,
    report: &RuntimeSeedApplyReport,
) -> Result<std::path::PathBuf, String> {
    ensure_root(install_root)?;
    let evidence_root = install_root.join("evidence");
    match fs::symlink_metadata(&evidence_root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err("runtime_seed_evidence_root_unsafe".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&evidence_root)
                .map_err(|error| format!("runtime_seed_evidence_root_create_failed:{error}"))?;
        }
        Err(error) => {
            return Err(format!("runtime_seed_evidence_root_unobservable:{error}"));
        }
    }
    let release_id = report
        .after
        .as_ref()
        .and_then(|observation| observation.current_release_id.as_deref())
        .unwrap_or("unactivated");
    let path = evidence_root.join(format!("{}-{release_id}.json", now_unix_ms()));
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("runtime_seed_evidence_encode_failed:{error}"))?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("runtime_seed_evidence_create_failed:{error}"))?;
    use std::io::Write as _;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("runtime_seed_evidence_write_failed:{error}"))?;
    fs::File::open(&evidence_root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("runtime_seed_evidence_sync_failed:{error}"))?;
    Ok(path)
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}
