//! Universal capacity contract — CYNIC metabolic state + freshness policy.
//!
//! Implémente `.hermes/plans/2026-08-24_universal-time-energy-task-roi-plan.md`
//! §6 étape 2 : le métabolisme (`MetabolicState`, jusqu'ici port host-only dans
//! `cynic-host/src/ports/metabolism.rs`) devient un CONTRAT protocol, et les
//! seuils deviennent une policy déclarée par nœud (décision HITL Q3 :
//! « policy déclarée par nœud, seuils = données »).
//!
//! Ce module ne lit AUCUN substrat. Il assemble :
//! - les observations canoniques existantes `CpuReport`/`MemoryReport`
//!   (`cynic_protocol::cpu`, `cynic_protocol::memory`) en une
//!   `CapacityObservationV1` référençable ;
//! - la décision pure `assess_metabolic_state(observation, policy)` qui
//!   remplace les constantes 85/90/40/60 codées en dur de l'ancien port.
//!
//! Horloge (gap 2) : les timestamps des reports restent des annotations ; la
//! fraîcheur est jugée par l'appelant via horloge monotone locale.

use serde::{Deserialize, Serialize};

use crate::cpu::CpuReport;
use crate::memory::MemoryReport;

pub const METABOLIC_POLICY_SCHEMA: &str = "cynic.metabolic_policy.v1";
pub const CAPACITY_ASSESSMENT_SCHEMA: &str = "cynic.capacity_assessment.v1";

/// État métabolique de l'hôte — vocabulaire canonique (déplacé du port host).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetabolicState {
    /// Ressources abondantes : tâches lourdes admises.
    HighPerformance,
    /// Charge normale : participation limitée.
    Standard,
    /// Souffrance (CPU/RAM saturés) : mode passif, refuse d'être Leader.
    Survival,
}

/// Seuils déclarés par nœud (décision Q3). Toutes les valeurs sont des données
/// observées/déclarées — plus aucune constante dans le code d'exécution.
/// Sémantique identique à l'ancien port host (85/90 → Survival,
/// 40/60 → Standard), désormais explicite et overridable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetabolicPolicy {
    pub schema: String,
    pub policy_id: String,
    /// CPU % au-delà duquel l'hôte passe en Survival.
    pub survival_cpu_pct: f64,
    /// RAM % utilisée au-delà duquel l'hôte passe en Survival.
    pub survival_mem_pct: f64,
    /// CPU % au-delà duquel l'hôte passe en Standard.
    pub standard_cpu_pct: f64,
    /// RAM % utilisée au-delà duquel l'hôte passe en Standard.
    pub standard_mem_pct: f64,
}

impl MetabolicPolicy {
    /// Valeurs initiales = celles observées dans l'ancien port host
    /// (continuité sémantique), portées comme DONNÉES par la policy nœud.
    pub fn with_legacy_defaults(policy_id: impl Into<String>) -> Self {
        Self {
            schema: METABOLIC_POLICY_SCHEMA.to_string(),
            policy_id: policy_id.into(),
            survival_cpu_pct: 85.0,
            survival_mem_pct: 90.0,
            standard_cpu_pct: 40.0,
            standard_mem_pct: 60.0,
        }
    }

    /// Validation pure : Survival doit être strictement au-dessus de Standard.
    pub fn violations(&self) -> Vec<String> {
        let mut v = Vec::new();
        if self.survival_cpu_pct <= self.standard_cpu_pct {
            v.push("survival_cpu_must_exceed_standard_cpu".to_string());
        }
        if self.survival_mem_pct <= self.standard_mem_pct {
            v.push("survival_mem_must_exceed_standard_mem".to_string());
        }
        for (name, value) in [
            ("survival_cpu_pct", self.survival_cpu_pct),
            ("survival_mem_pct", self.survival_mem_pct),
            ("standard_cpu_pct", self.standard_cpu_pct),
            ("standard_mem_pct", self.standard_mem_pct),
        ] {
            if !(0.0..=100.0).contains(&value) || !value.is_finite() {
                v.push(format!("{name}_out_of_range"));
            }
        }
        if self.policy_id.trim().is_empty() {
            v.push("policy_id_missing".to_string());
        }
        v
    }
}

/// Observation de capacité assemblée : référence les reports canoniques déjà
/// produits par les adapters CPU/mémoire (aucune re-mesure ici).
#[derive(Debug, Clone, Serialize)]
pub struct CapacityAssessmentV1 {
    pub schema: &'static str,
    pub state: MetabolicState,
    /// CPU % utilisé (depuis CpuUsage.util_pct si mesuré).
    pub cpu_util_pct: Option<f64>,
    /// RAM % utilisée (calculée depuis MemoryReport.usage physical).
    pub mem_used_pct: Option<f64>,
    /// RAM disponible en octets (headroom dynamique pour EnergyBudget).
    pub available_ram_bytes: u64,
    /// Policy appliquée (digest déclaré par l'appelant).
    pub policy_id: String,
    /// Inconnus hérités des reports sources + de l'évaluation.
    pub residual_unknowns: Vec<String>,
}

/// Décision pure : état métabolique depuis observations + policy nœud.
///
/// Unknown-first : sans mesure CPU ou RAM exploitable, l'état est `Standard`
/// avec unknowns explicites ? NON — un état non mesurable ne peut pas être
/// affirmé : on retourne `Survival` ? NON PLUS. On retourne l'état le plus
/// PRUDENT justifiable : sans aucune donnée, l'hôte ne peut pas prétendre à
/// HighPerformance ; mais Survival déclencherait des refus non fondés.
/// Décision : `Standard` + residual_unknowns listant ce qui manque (l'état
/// intermédiaire est celui qui ne promet ni capacité ni panne).
pub fn assess_metabolic_state(
    cpu: Option<&CpuReport>,
    memory: Option<&MemoryReport>,
    policy: &MetabolicPolicy,
) -> CapacityAssessmentV1 {
    let mut residual_unknowns = Vec::new();

    let cpu_util_pct = cpu.and_then(|report| report.usage.util_pct);
    if cpu.is_none() {
        residual_unknowns.push("cpu_report_missing".to_string());
    } else if cpu_util_pct.is_none() {
        residual_unknowns.push("cpu_utilization_not_measured".to_string());
    }

    let mem_used_pct = memory.map(|report| {
        let total = report.physical.total_bytes;
        let used = report.physical.used_bytes;
        if total == 0 {
            residual_unknowns.push("mem_total_zero".to_string());
            None
        } else {
            Some((used as f64 / total as f64) * 100.0)
        }
    });
    let mem_used_pct = mem_used_pct.unwrap_or_else(|| {
        residual_unknowns.push("memory_report_missing".to_string());
        None
    });
    let available_ram_bytes = memory
        .map(|report| report.physical.available_bytes)
        .unwrap_or(0);
    if memory.is_none() {
        // available_ram_bytes = 0 sans mesure : signalé, jamais fabriqué.
        residual_unknowns.push("available_ram_unmeasured".to_string());
    }

    let state = match (cpu_util_pct, mem_used_pct) {
        (Some(cpu_pct), Some(mem_pct)) => {
            if cpu_pct > policy.survival_cpu_pct || mem_pct > policy.survival_mem_pct {
                MetabolicState::Survival
            } else if cpu_pct > policy.standard_cpu_pct || mem_pct > policy.standard_mem_pct {
                MetabolicState::Standard
            } else {
                MetabolicState::HighPerformance
            }
        }
        _ => {
            // Données insuffisantes : état intermédiaire prudent + unknowns
            // explicites (jamais HighPerformance sans preuve).
            MetabolicState::Standard
        }
    };

    CapacityAssessmentV1 {
        schema: CAPACITY_ASSESSMENT_SCHEMA,
        state,
        cpu_util_pct,
        mem_used_pct,
        available_ram_bytes,
        policy_id: policy.policy_id.clone(),
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{CpuPhysical, CpuUsage, CpuVolume};

    fn cpu_report(util_pct: f64) -> CpuReport {
        CpuReport {
            schema: "cynic.host_cpu_report.v1".to_string(),
            timestamp: "annotation-only".to_string(),
            physical: CpuPhysical {
                model_name: "test".to_string(),
                logical_cpus: 16,
                cores: Some(8),
                sockets: Some(1),
                max_bogomips_or_hz: None,
            },
            volume: CpuVolume {
                governor: None,
                isolated_mask: None,
                online_cpus: 16,
                possible_cpus: 16,
            },
            usage: CpuUsage {
                user: 0,
                system: 0,
                idle: 0,
                iowait: 0,
                softirq: 0,
                steal: 0,
                util_pct: Some(util_pct),
            },
            pressure: None,
            residual_unknowns: Vec::new(),
        }
    }

    fn mem_report(total_gb: u64, used_gb: u64) -> MemoryReport {
        MemoryReport {
            schema: "cynic.host_memory_report.v1".to_string(),
            timestamp: "annotation-only".to_string(),
            physical: crate::memory::MemoryUsage {
                total_bytes: total_gb * 1024 * 1024 * 1024,
                free_bytes: (total_gb - used_gb) * 1024 * 1024 * 1024,
                available_bytes: (total_gb - used_gb) * 1024 * 1024 * 1024,
                used_bytes: used_gb * 1024 * 1024 * 1024,
                reclaimable_bytes: 0,
            },
            volume: crate::memory::MemoryVolume {
                page_size_bytes: 4096,
                swappiness: None,
                hugepages_reserved: None,
                hugepage_size_bytes: None,
            },
            swap: crate::memory::MemoryUsage {
                total_bytes: 0,
                free_bytes: 0,
                available_bytes: 0,
                used_bytes: 0,
                reclaimable_bytes: 0,
            },
            pressure: None,
            meminfo_headline: None,
            residual_unknowns: Vec::new(),
        }
    }

    #[test]
    fn legacy_defaults_match_old_host_port_semantics() {
        let p = MetabolicPolicy::with_legacy_defaults("node-a");
        assert!(p.violations().is_empty());
        assert_eq!(p.survival_cpu_pct, 85.0);
        assert_eq!(p.standard_mem_pct, 60.0);
    }

    #[test]
    fn inverted_thresholds_are_violations() {
        let p = MetabolicPolicy {
            schema: METABOLIC_POLICY_SCHEMA.to_string(),
            policy_id: "broken".to_string(),
            survival_cpu_pct: 30.0,
            survival_mem_pct: 90.0,
            standard_cpu_pct: 40.0,
            standard_mem_pct: 60.0,
        };
        assert!(
            p.violations()
                .contains(&"survival_cpu_must_exceed_standard_cpu".to_string())
        );
    }

    #[test]
    fn high_performance_under_standard_thresholds() {
        let p = MetabolicPolicy::with_legacy_defaults("n");
        let a = assess_metabolic_state(Some(&cpu_report(10.0)), Some(&mem_report(28, 8)), &p);
        assert_eq!(a.state, MetabolicState::HighPerformance);
        assert!(a.residual_unknowns.is_empty());
        assert_eq!(a.available_ram_bytes, 20 * 1024 * 1024 * 1024);
    }

    #[test]
    fn standard_between_thresholds() {
        let p = MetabolicPolicy::with_legacy_defaults("n");
        let a = assess_metabolic_state(Some(&cpu_report(50.0)), Some(&mem_report(28, 10)), &p);
        assert_eq!(a.state, MetabolicState::Standard);
    }

    #[test]
    fn survival_above_survival_thresholds() {
        let p = MetabolicPolicy::with_legacy_defaults("n");
        let a = assess_metabolic_state(Some(&cpu_report(86.0)), Some(&mem_report(28, 10)), &p);
        assert_eq!(a.state, MetabolicState::Survival);

        // Mémoire seule peut suffoquer aussi.
        let a2 = assess_metabolic_state(Some(&cpu_report(10.0)), Some(&mem_report(28, 26)), &p);
        assert_eq!(a2.state, MetabolicState::Survival);
    }

    #[test]
    fn missing_reports_yield_prudent_standard_with_unknowns_never_high_performance() {
        let p = MetabolicPolicy::with_legacy_defaults("n");
        let a = assess_metabolic_state(None, None, &p);
        assert_eq!(a.state, MetabolicState::Standard);
        assert!(
            a.residual_unknowns
                .contains(&"cpu_report_missing".to_string())
        );
        assert!(
            a.residual_unknowns
                .contains(&"memory_report_missing".to_string())
        );
        assert!(
            a.residual_unknowns
                .contains(&"available_ram_unmeasured".to_string())
        );

        // CPU présent mais utilisation non mesurée → toujours prudent.
        let mut cpu_no_util = cpu_report(0.0);
        cpu_no_util.usage.util_pct = None;
        let b = assess_metabolic_state(Some(&cpu_no_util), Some(&mem_report(28, 8)), &p);
        assert_eq!(b.state, MetabolicState::Standard);
        assert!(
            b.residual_unknowns
                .contains(&"cpu_utilization_not_measured".to_string())
        );
    }

    #[test]
    fn custom_policy_is_honored_data_not_constants() {
        // Un nœud contraint (ex: lab partagé) déclare des seuils serrés.
        let p = MetabolicPolicy {
            schema: METABOLIC_POLICY_SCHEMA.to_string(),
            policy_id: "lab-strict".to_string(),
            survival_cpu_pct: 70.0,
            survival_mem_pct: 80.0,
            standard_cpu_pct: 20.0,
            standard_mem_pct: 40.0,
        };
        assert!(p.violations().is_empty());
        // CPU 50% dépasse le seuil standard strict (20%) mais pas survival (70%)
        // ; RAM 15/28 = 53.6% > standard 40%, < survival 80%. État attendu :
        // Standard (au-dessus de Standard, sous Survival).
        let a = assess_metabolic_state(Some(&cpu_report(50.0)), Some(&mem_report(28, 15)), &p);
        assert_eq!(a.state, MetabolicState::Standard);
        // Une seule dimension au-delà du seuil Survival suffit.
        let b = assess_metabolic_state(Some(&cpu_report(75.0)), Some(&mem_report(28, 15)), &p);
        assert_eq!(b.state, MetabolicState::Survival);
    }
}
