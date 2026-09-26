use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use cynic_node_protocol::digest_json;

use crate::hardware_baseline_apply::{
    HardwareBaselineBundle, HardwareBaselineBundleProof, HardwareBaselineBundleStorePort,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalHardwareBaselineBundleStore;

impl HardwareBaselineBundleStorePort for LocalHardwareBaselineBundleStore {
    fn persist_absent_or_matching(
        &self,
        path: &Path,
        bundle: &HardwareBaselineBundle,
    ) -> Result<HardwareBaselineBundleProof, String> {
        let parent = path
            .parent()
            .ok_or_else(|| "hardware_baseline_bundle_parent_missing".to_string())?;
        if !parent.is_dir() {
            return Err("hardware_baseline_bundle_parent_absent".to_string());
        }
        let bundle_digest = digest_json(bundle).map_err(|error| error.to_string())?;
        if path.exists() {
            let existing = fs::read(path)
                .map_err(|error| format!("hardware_baseline_bundle_read_failed:{error}"))?;
            let existing_bundle = serde_json::from_slice::<HardwareBaselineBundle>(&existing)
                .map_err(|error| format!("hardware_baseline_bundle_existing_invalid:{error}"))?;
            if digest_json(&existing_bundle).ok().as_deref() != Some(bundle_digest.as_str()) {
                return Err("hardware_baseline_bundle_conflict".to_string());
            }
            return Ok(HardwareBaselineBundleProof {
                path: path.display().to_string(),
                bundle_digest,
                created: false,
            });
        }
        let mut payload = serde_json::to_vec_pretty(bundle).map_err(|error| error.to_string())?;
        payload.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| format!("hardware_baseline_bundle_create_failed:{error}"))?;
        file.write_all(&payload)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("hardware_baseline_bundle_write_failed:{error}"))?;
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("hardware_baseline_bundle_parent_sync_failed:{error}"))?;
        Ok(HardwareBaselineBundleProof {
            path: path.display().to_string(),
            bundle_digest,
            created: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware_baseline::plan_hardware_baseline;
    use crate::hardware_baseline_apply::build_hardware_baseline_bundle;
    use crate::hardware_conformance::{
        EvidenceState, HardwareDevice, HardwareInventory, HardwareObservation, assess_hardware,
    };

    fn fixture() -> HardwareBaselineBundle {
        let report = assess_hardware(HardwareObservation {
            inventory: HardwareInventory {
                board_name: Some("fixture".into()),
                cpu_model: Some("fixture".into()),
                devices: vec![HardwareDevice {
                    bus: "fixture".into(),
                    address: "0".into(),
                    class: None,
                    vendor_id: None,
                    device_id: None,
                    model: None,
                    firmware: None,
                    driver: None,
                }],
                ..Default::default()
            },
            baseline_digest: None,
            firmware_version: Some("1".into()),
            max_temperature_millicelsius: Some(40_000),
            corrected_memory_errors: Some(0),
            uncorrected_memory_errors: Some(0),
            storage_device_count: 1,
            offline_memory_test: EvidenceState::Unknown,
            memory_tuning: EvidenceState::Unknown,
            runtime_tuning_overrides: Vec::new(),
        });
        let plan = plan_hardware_baseline(
            "fixture",
            "operator",
            Path::new("/injected/hardware-baseline.json"),
            &report,
        );
        build_hardware_baseline_bundle(&plan).expect("bundle")
    }

    #[test]
    fn staging_is_idempotent_but_refuses_a_different_bundle() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("bundle.json");
        let store = LocalHardwareBaselineBundleStore;
        let bundle = fixture();
        assert!(
            store
                .persist_absent_or_matching(&path, &bundle)
                .expect("first")
                .created
        );
        assert!(
            !store
                .persist_absent_or_matching(&path, &bundle)
                .expect("second")
                .created
        );
        let mut other = bundle;
        other.target_path.push_str(".other");
        assert_eq!(
            store.persist_absent_or_matching(&path, &other),
            Err("hardware_baseline_bundle_conflict".to_string())
        );
    }
}
