use serde::{Deserialize, Serialize};

use cynic_protocol::{SecurityEvidenceState, digest_json};

pub const HARDWARE_CONFORMANCE_SCHEMA: &str = "cynic.host.hardware_conformance.v1";
pub const HARDWARE_BASELINE_SCHEMA: &str = "cynic.host.hardware_baseline.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareObservation {
    pub inventory: HardwareInventory,
    pub baseline_digest: Option<String>,
    pub firmware_version: Option<String>,
    pub max_temperature_millicelsius: Option<i64>,
    pub corrected_memory_errors: Option<u64>,
    pub uncorrected_memory_errors: Option<u64>,
    pub storage_device_count: usize,
    pub offline_memory_test: EvidenceState,
    pub memory_tuning: EvidenceState,
    pub runtime_tuning_overrides: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareBaseline {
    pub schema: String,
    pub subject: String,
    pub inventory_digest: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct HardwareInventory {
    pub system_vendor: Option<String>,
    pub system_product: Option<String>,
    pub board_vendor: Option<String>,
    pub board_name: Option<String>,
    pub board_version: Option<String>,
    pub firmware_vendor: Option<String>,
    pub firmware_date: Option<String>,
    pub cpu_model: Option<String>,
    pub cpu_microcode: Option<String>,
    pub logical_cpu_count: usize,
    pub kernel_release: Option<String>,
    pub devices: Vec<HardwareDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareDevice {
    pub bus: String,
    pub address: String,
    pub class: Option<String>,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub driver: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Satisfied,
    Refuted,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareConformanceReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub observation: HardwareObservation,
    pub inventory_digest: Option<String>,
    pub identity_state: SecurityEvidenceState,
    pub gates: Vec<HardwareGate>,
    pub residual_unknowns: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HardwareGate {
    pub id: &'static str,
    pub status: &'static str,
    pub detail: String,
}

pub fn assess_hardware(observation: HardwareObservation) -> HardwareConformanceReport {
    let inventory_digest = digest_json(&observation.inventory).ok();
    let identity_state = match (
        observation.baseline_digest.as_deref(),
        inventory_digest.as_deref(),
    ) {
        (Some(expected), Some(observed)) if expected == observed => SecurityEvidenceState::Proven,
        (Some(_), Some(_)) => SecurityEvidenceState::Refuted,
        _ => SecurityEvidenceState::Unknown,
    };
    let gates = vec![
        HardwareGate {
            id: "hardware_identity_stable",
            status: match identity_state {
                SecurityEvidenceState::Proven => "satisfied",
                SecurityEvidenceState::Refuted => "refuted",
                _ => "unknown",
            },
            detail: format!(
                "baseline_digest={:?} inventory_digest={:?}",
                observation.baseline_digest, inventory_digest
            ),
        },
        evidence_gate(
            "offline_memory_test_passed",
            observation.offline_memory_test,
            "Memtest86+ or equivalent must run outside the active OS",
        ),
        evidence_gate(
            "memory_tuning_declared_stable",
            observation.memory_tuning,
            &if observation.runtime_tuning_overrides.is_empty() {
                "XMP, overclock and undervolt state requires firmware/operator attestation"
                    .to_string()
            } else {
                format!(
                    "firmware attestation required; active runtime overrides={}",
                    observation.runtime_tuning_overrides.join(",")
                )
            },
        ),
        HardwareGate {
            id: "no_uncorrected_memory_errors",
            status: match observation.uncorrected_memory_errors {
                Some(0) => "satisfied",
                Some(_) => "refuted",
                None => "unknown",
            },
            detail: format!(
                "uncorrected_memory_errors={:?} corrected_memory_errors={:?}",
                observation.uncorrected_memory_errors, observation.corrected_memory_errors
            ),
        },
        presence_gate(
            "firmware_observed",
            observation.firmware_version.is_some(),
            format!("firmware_version={:?}", observation.firmware_version),
        ),
        presence_gate(
            "hardware_inventory_observed",
            observation.inventory.board_name.is_some()
                && observation.inventory.cpu_model.is_some()
                && !observation.inventory.devices.is_empty(),
            format!(
                "board={:?} cpu={:?} device_count={}",
                observation.inventory.board_name,
                observation.inventory.cpu_model,
                observation.inventory.devices.len()
            ),
        ),
        presence_gate(
            "thermal_sensor_observed",
            observation.max_temperature_millicelsius.is_some(),
            format!(
                "max_temperature_millicelsius={:?}",
                observation.max_temperature_millicelsius
            ),
        ),
        presence_gate(
            "storage_devices_observed",
            observation.storage_device_count > 0,
            format!("storage_device_count={}", observation.storage_device_count),
        ),
    ];
    let residual_unknowns = gates
        .iter()
        .filter(|gate| gate.status == "unknown")
        .map(|gate| gate.id)
        .collect::<Vec<_>>();
    let status = if gates.iter().all(|gate| gate.status == "satisfied") {
        "qualified"
    } else {
        "blocked"
    };
    HardwareConformanceReport {
        schema: HARDWARE_CONFORMANCE_SCHEMA,
        status,
        observation,
        inventory_digest,
        identity_state,
        gates,
        residual_unknowns,
    }
}

pub fn observe_hardware_conformance(
    observer: &impl crate::ports::HostHardwareObservationPort,
) -> HardwareConformanceReport {
    assess_hardware(observer.observe_hardware())
}

fn evidence_gate(id: &'static str, state: EvidenceState, detail: &str) -> HardwareGate {
    HardwareGate {
        id,
        status: match state {
            EvidenceState::Satisfied => "satisfied",
            EvidenceState::Refuted => "refuted",
            EvidenceState::Unknown => "unknown",
        },
        detail: detail.to_string(),
    }
}

fn presence_gate(id: &'static str, present: bool, detail: String) -> HardwareGate {
    HardwareGate {
        id,
        status: if present { "satisfied" } else { "unknown" },
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_offline_evidence_blocks_promotion() {
        let report = assess_hardware(HardwareObservation {
            inventory: fixture_inventory(),
            baseline_digest: None,
            firmware_version: Some("1.0".into()),
            max_temperature_millicelsius: Some(42_000),
            corrected_memory_errors: Some(0),
            uncorrected_memory_errors: Some(0),
            storage_device_count: 1,
            offline_memory_test: EvidenceState::Unknown,
            memory_tuning: EvidenceState::Unknown,
            runtime_tuning_overrides: Vec::new(),
        });

        assert_eq!(report.status, "blocked");
        assert_eq!(report.residual_unknowns.len(), 3);
    }

    #[test]
    fn complete_stable_evidence_qualifies_hardware() {
        let mut observation = HardwareObservation {
            inventory: fixture_inventory(),
            baseline_digest: None,
            firmware_version: Some("1.0".into()),
            max_temperature_millicelsius: Some(42_000),
            corrected_memory_errors: Some(0),
            uncorrected_memory_errors: Some(0),
            storage_device_count: 1,
            offline_memory_test: EvidenceState::Satisfied,
            memory_tuning: EvidenceState::Satisfied,
            runtime_tuning_overrides: Vec::new(),
        };
        observation.baseline_digest = digest_json(&observation.inventory).ok();
        let report = assess_hardware(observation);

        assert_eq!(report.status, "qualified");
        assert!(report.residual_unknowns.is_empty());
    }

    fn fixture_inventory() -> HardwareInventory {
        HardwareInventory {
            system_vendor: Some("fixture".into()),
            system_product: Some("fixture".into()),
            board_vendor: Some("fixture".into()),
            board_name: Some("fixture".into()),
            board_version: Some("1".into()),
            firmware_vendor: Some("fixture".into()),
            firmware_date: Some("2026-01-01".into()),
            cpu_model: Some("fixture".into()),
            cpu_microcode: Some("1".into()),
            logical_cpu_count: 1,
            kernel_release: Some("fixture".into()),
            devices: vec![HardwareDevice {
                bus: "fixture".into(),
                address: "0".into(),
                class: None,
                vendor_id: None,
                device_id: None,
                model: Some("fixture".into()),
                firmware: None,
                driver: None,
            }],
        }
    }
}
