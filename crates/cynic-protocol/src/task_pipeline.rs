//! Universal tick-window contract — CYNIC fractal time primitive.
//!
//! Implémente `.hermes/plans/2026-08-24_universal-time-energy-task-roi-plan.md`
//! §9-§10 (itération V1 « une fenêtre, un hôte, mesuré »).
//!
//! Contrats portés par ce module (un seul concept par nom, cf. glossaire) :
//! - `TickWindow` : la fenêtre budgétée universelle, liée à un `temporal::Epoch`
//!   logique (temps logique > temps physique).
//! - `EnergyBudget` / `EnergyConsumed` : allocation et mesure par dimension ;
//!   absent de mesure = `None`, jamais zéro.
//! - `CapacityObservation` + `FreshnessPolicy` : fraîcheur PAR CHAMP
//!   (profil stable = longue ; headroom dynamique = courte, défaut 60 s,
//!   portée par la policy nœud — décision Q3).
//! - `WindowJournalRecord` : le journal JSONL append-only EST le ledger
//!   (gap 4). Chaque record porte `parent_window_id` (assurance schéma, gap 3).
//!   Confinement temporaire : la levée (compaction des records settlés) est
//!   hors périmètre V1 mais interdite d'oubli (ambition `replayable_memory`).
//!
//! Horloges (gap 2) : ce module ne lit JAMAIS l'horloge murelle. Les durées et
//! fraîcheurs sont exprimées en millisecondes monotones (`std::time::Instant`)
//! que l'adaptateur mesure ; `unix_ms` n'est qu'une annotation non
//! authoritative laissée aux couches de projection. Les fonctions pures
//! reçoivent `now_elapsed_ms` en paramètre — déterministes et testables.

use serde::{Deserialize, Serialize};

use crate::temporal::Epoch;

pub const TICK_WINDOW_SCHEMA: &str = "cynic.tick_window.v1";
pub const WINDOW_JOURNAL_RECORD_SCHEMA: &str = "cynic.window_journal_record.v1";
pub const CAPACITY_OBSERVATION_SCHEMA: &str = "cynic.capacity_observation.v1";
pub const FRESHNESS_POLICY_SCHEMA: &str = "cynic.capacity_freshness_policy.v1";

/// Défaut prudent du gap 5 : headroom dynamique. Choix INITIAL arbitraire mais
/// serré ; à calibrer par la dérive réelle mesurée par le micro-bench V1.
/// Jamais codé en dur côté adaptateur : la policy nœud override.
pub const DEFAULT_DYNAMIC_FRESHNESS_MS: u64 = 60_000;

// ---------------------------------------------------------------------------
// Budget énergie — allocation typée par dimension
// ---------------------------------------------------------------------------

/// Budget d'énergie alloué à une fenêtre. `wall_time_ms` est obligatoire
/// (toujours mesurable via horloge monotone) ; les autres dimensions sont
/// opportunistes et restent `None` quand elles ne sont pas mesurées.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyBudget {
    pub wall_time_ms: u64,
    #[serde(default)]
    pub inference_tokens: Option<u64>,
    #[serde(default)]
    pub cpu_ms: Option<u64>,
    #[serde(default)]
    /// byte-seconds accumulés (occupation mémoire × durée) si mesurés.
    pub ram_bytes_s: Option<u64>,
}

impl EnergyBudget {
    /// Prédicat pur de confinement par dimension (réutilisé pour le nesting
    /// profondeur 1 et l'admission contre l'enveloppe). Sémantique :
    /// - demande `None` → ne consomme rien → passe toujours ;
    /// - limite `None` → dimension non plafonnée par la policy → passe ;
    /// - sinon demande ≤ limite.
    pub fn fits_within(&self, limit: &EnergyBudget) -> bool {
        self.wall_time_ms <= limit.wall_time_ms
            && dim_fits(self.inference_tokens, limit.inference_tokens)
            && dim_fits(self.cpu_ms, limit.cpu_ms)
            && dim_fits(self.ram_bytes_s, limit.ram_bytes_s)
    }
}

fn dim_fits(request: Option<u64>, limit: Option<u64>) -> bool {
    match (request, limit) {
        (None, _) => true,
        (_, None) => true,
        (Some(r), Some(l)) => r <= l,
    }
}

/// Somme dimensionnelle additive (tokens, cpu_ms, ram_bytes_s). `wall_time_ms`
/// n'est PAS additif entre fenêtres concurrentes (le temps ne se cumule pas) :
/// il est borné par l'enveloppe via `fits_within`, pas sommé ici.
fn sum_dims(acc: &mut [Option<u64>; 3], b: &EnergyBudget) {
    acc[0] = add_dim(acc[0], b.inference_tokens);
    acc[1] = add_dim(acc[1], b.cpu_ms);
    acc[2] = add_dim(acc[2], b.ram_bytes_s);
}

fn add_dim(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (None, None) => None,
        (Some(x), None) | (None, Some(x)) => Some(x),
        (Some(x), Some(y)) => Some(x.saturating_add(y)),
    }
}

/// Énergie réellement consommée, mesurée par l'adaptateur (delta `Instant`
/// pour wall_time ; échantillonnage sysinfo avant/après pour cpu/ram ;
/// compteurs de streaming pour tokens). Absent de mesure = `None`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EnergyConsumed {
    pub wall_time_ms: u64,
    #[serde(default)]
    pub inference_tokens: Option<u64>,
    #[serde(default)]
    pub cpu_ms: Option<u64>,
    #[serde(default)]
    pub ram_bytes_s: Option<u64>,
}

impl EnergyConsumed {
    /// La consommation respecte-t-elle le budget réservé ? Une dimension non
    /// mesurée (`None`) n'est ni conforme ni violante — c'est un unknown du
    /// receipt, pas un échec.
    pub fn within(&self, budget: &EnergyBudget) -> bool {
        self.wall_time_ms <= budget.wall_time_ms
            && consumed_dim_within(self.inference_tokens, budget.inference_tokens)
            && consumed_dim_within(self.cpu_ms, budget.cpu_ms)
            && consumed_dim_within(self.ram_bytes_s, budget.ram_bytes_s)
    }
}

fn consumed_dim_within(consumed: Option<u64>, budgeted: Option<u64>) -> bool {
    match (consumed, budgeted) {
        (None, _) => true,
        (Some(_), None) => false, // consommé sans budget réservé = dépassement
        (Some(c), Some(b)) => c <= b,
    }
}

// ---------------------------------------------------------------------------
// Fenêtre universelle
// ---------------------------------------------------------------------------

/// La fenêtre budgétée universelle — primitive temporelle fractale instanciée
/// par couche (kernel tick, host reconciliation, node workload, fleet rollout,
/// lab benchmark). Liée à un epoch LOGIQUE local ; aucune prétention
/// d'alignement inter-hôtes (gap 2 : digests uniquement aux frontières).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickWindow {
    pub schema: String,
    pub window_id: String,
    pub host_id: String,
    /// Assurance schéma pour le nesting (gap 3, profondeur 1 en V1).
    #[serde(default)]
    pub parent_window_id: Option<String>,
    /// Temps logique local (jamais une horloge physique).
    pub epoch: Epoch,
    pub budget: EnergyBudget,
}

impl TickWindow {
    pub fn new(
        window_id: impl Into<String>,
        host_id: impl Into<String>,
        epoch: Epoch,
        budget: EnergyBudget,
    ) -> Self {
        Self {
            schema: TICK_WINDOW_SCHEMA.to_string(),
            window_id: window_id.into(),
            host_id: host_id.into(),
            parent_window_id: None,
            epoch,
            budget,
        }
    }
}

// ---------------------------------------------------------------------------
// Capacité observée + fraîcheur par champ
// ---------------------------------------------------------------------------

/// Faits de profil STABLES (ne changent pas au reboot près) — fraîcheur longue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileFacts {
    pub cpu_threads: u32,
    pub total_ram_bytes: u64,
}

/// Headroom DYNAMIQUE — bascule en secondes sous burst ; fraîcheur courte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicHeadroom {
    pub available_ram_bytes: u64,
    /// Lecture d'horloge monotone locale (ms depuis le démarrage du process
    /// observateur, via `Instant`). Jamais unix_ms.
    pub observed_at_elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityObservation {
    pub schema: String,
    pub profile: ProfileFacts,
    pub profile_observed_at_elapsed_ms: u64,
    pub dynamic: DynamicHeadroom,
}

/// Policy de fraîcheur déclarée par nœud (décision Q3 : seuils = données).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshnessPolicy {
    pub schema: String,
    pub profile_max_age_ms: u64,
    pub dynamic_max_age_ms: u64,
}

impl FreshnessPolicy {
    pub fn with_defaults() -> Self {
        Self {
            schema: FRESHNESS_POLICY_SCHEMA.to_string(),
            // Valeur initiale arbitraire généreuse pour des faits stables ;
            // la policy nœud doit la déclarer explicitement en production.
            profile_max_age_ms: 24 * 60 * 60 * 1000,
            dynamic_max_age_ms: DEFAULT_DYNAMIC_FRESHNESS_MS,
        }
    }
}

fn field_is_fresh(observed_at_elapsed_ms: u64, now_elapsed_ms: u64, max_age_ms: u64) -> bool {
    now_elapsed_ms.saturating_sub(observed_at_elapsed_ms) <= max_age_ms
}

// ---------------------------------------------------------------------------
// Admission — le journal EST le ledger (gap 4), fraîcheur par champ (gap 5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum WindowAdmission {
    Admitted,
    /// Refus définitif : la demande viole une règle déclarée.
    Blocked {
        reasons: Vec<String>,
    },
    /// On ne sait PAS décider (capacité périmée…) — jamais un succès fabriqué.
    Unknown {
        reasons: Vec<String>,
    },
}

/// Admission d'une fenêtre contre l'enveloppe concurrente déclarée par le nœud.
///
/// - `open_reservations` : budgets des fenêtres actuellement ouvertes sur
///   l'hôte (lus depuis le journal — le journal EST le ledger, aucun compteur
///   séparé).
/// - `envelope` : plafonds simultanés déclarés (policy nœud). Dimension
///   `None` = non plafonnée.
///
/// Ordre de décision : Unknown (fraîcheur) AVANT Blocked (dépassement) — on ne
/// bloque pas sur des chiffres périmés, on dit qu'on ne sait pas.
pub fn admit_window(
    request: &EnergyBudget,
    open_reservations: &[EnergyBudget],
    envelope: &EnergyBudget,
    capacity: &CapacityObservation,
    policy: &FreshnessPolicy,
    now_elapsed_ms: u64,
) -> WindowAdmission {
    let mut unknowns = Vec::new();
    if !field_is_fresh(
        capacity.dynamic.observed_at_elapsed_ms,
        now_elapsed_ms,
        policy.dynamic_max_age_ms,
    ) {
        unknowns.push("capacity_dynamic_stale".to_string());
    }
    if !field_is_fresh(
        capacity.profile_observed_at_elapsed_ms,
        now_elapsed_ms,
        policy.profile_max_age_ms,
    ) {
        unknowns.push("capacity_profile_stale".to_string());
    }
    if !unknowns.is_empty() {
        return WindowAdmission::Unknown { reasons: unknowns };
    }

    let mut blocked = Vec::new();
    if request.wall_time_ms == 0 {
        blocked.push("wall_time_budget_zero".to_string());
    }

    let mut reserved = [None; 3];
    for open in open_reservations {
        sum_dims(&mut reserved, open);
    }
    let mut total = reserved;
    // total[i] = somme(réservations ouvertes) + demande courante, par dimension.
    sum_dims(&mut total, request);

    let request_dims = [
        request.inference_tokens,
        request.cpu_ms,
        request.ram_bytes_s,
    ];
    for (i, name) in DIMENSION_NAMES.iter().enumerate() {
        check_dimension(
            name,
            request_dims[i],
            total[i],
            envelope_dim(envelope, i),
            &mut blocked,
        );
    }

    if !blocked.is_empty() {
        return WindowAdmission::Blocked { reasons: blocked };
    }
    WindowAdmission::Admitted
}

pub const DIMENSION_NAMES: [&str; 3] = ["inference_tokens", "cpu_ms", "ram_bytes_s"];

/// Une demande `None` ne consomme rien ; une enveloppe sans plafond (`None`)
/// n'est pas bornée par la policy. Sinon : dépassement si
/// réservations ouvertes + demande > plafond.
fn check_dimension(
    name: &str,
    requested: Option<u64>,
    reserved_plus_requested: Option<u64>,
    limit: Option<u64>,
    blocked: &mut Vec<String>,
) {
    let (Some(_), Some(total_value), Some(limit_value)) =
        (requested, reserved_plus_requested, limit)
    else {
        return;
    };
    if total_value > limit_value {
        blocked.push(format!("envelope_exceeded:{name}"));
    }
}

fn envelope_dim(envelope: &EnergyBudget, i: usize) -> Option<u64> {
    match i {
        0 => envelope.inference_tokens,
        1 => envelope.cpu_ms,
        _ => envelope.ram_bytes_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(wall_ms: u64, tokens: Option<u64>) -> EnergyBudget {
        EnergyBudget {
            wall_time_ms: wall_ms,
            inference_tokens: tokens,
            cpu_ms: None,
            ram_bytes_s: None,
        }
    }

    fn fresh_capacity(now: u64) -> CapacityObservation {
        CapacityObservation {
            schema: CAPACITY_OBSERVATION_SCHEMA.to_string(),
            profile: ProfileFacts {
                cpu_threads: 16,
                total_ram_bytes: 28_000_000_000,
            },
            profile_observed_at_elapsed_ms: now,
            dynamic: DynamicHeadroom {
                available_ram_bytes: 19_000_000_000,
                observed_at_elapsed_ms: now,
            },
        }
    }

    #[test]
    fn admitted_when_envelope_has_room_and_capacity_fresh() {
        let req = budget(5_000, Some(1_000));
        let env = budget(u64::MAX, Some(10_000));
        let cap = fresh_capacity(1_000);
        let pol = FreshnessPolicy::with_defaults();
        assert_eq!(
            admit_window(&req, &[], &env, &cap, &pol, 1_500),
            WindowAdmission::Admitted
        );
    }

    #[test]
    fn stale_dynamic_capacity_is_unknown_not_admitted() {
        let req = budget(5_000, Some(1));
        let env = budget(u64::MAX, Some(10_000));
        let mut cap = fresh_capacity(1_000);
        cap.dynamic.observed_at_elapsed_ms = 0; // très vieux headroom
        let pol = FreshnessPolicy::with_defaults(); // dynamic 60s
        assert_eq!(
            admit_window(&req, &[], &env, &cap, &pol, 61_000),
            WindowAdmission::Unknown {
                reasons: vec!["capacity_dynamic_stale".to_string()]
            }
        );
    }

    #[test]
    fn stale_profile_capacity_is_unknown() {
        let req = budget(5_000, None);
        let env = budget(u64::MAX, None);
        let mut cap = fresh_capacity(0);
        cap.profile_observed_at_elapsed_ms = 0;
        let pol = FreshnessPolicy {
            profile_max_age_ms: 100,
            ..FreshnessPolicy::with_defaults()
        };
        assert_eq!(
            admit_window(&req, &[], &env, &cap, &pol, 200),
            WindowAdmission::Unknown {
                reasons: vec!["capacity_profile_stale".to_string()]
            }
        );
    }

    #[test]
    fn envelope_exceeded_blocks_with_dimension_name() {
        let open = [budget(9_000, Some(9_000))];
        let req = budget(1_000, Some(2_000)); // 9000+2000 > 10000
        let env = budget(u64::MAX, Some(10_000));
        let cap = fresh_capacity(1_000);
        let pol = FreshnessPolicy::with_defaults();
        assert_eq!(
            admit_window(&req, &open.as_slice(), &env, &cap, &pol, 1_001),
            WindowAdmission::Blocked {
                reasons: vec!["envelope_exceeded:inference_tokens".to_string()]
            }
        );
    }

    #[test]
    fn zero_wall_time_budget_is_blocked() {
        let req = budget(0, None);
        let env = budget(u64::MAX, None);
        let cap = fresh_capacity(1_000);
        let pol = FreshnessPolicy::with_defaults();
        assert_eq!(
            admit_window(&req, &[], &env, &cap, &pol, 1_001),
            WindowAdmission::Blocked {
                reasons: vec!["wall_time_budget_zero".to_string()]
            }
        );
    }

    #[test]
    fn nesting_depth_one_child_fits_parent_remaining() {
        // Règle gap 3 : budget fils ⊆ reste du père (même prédicat).
        let parent = budget(10_000, Some(8_000));
        let child_ok = budget(4_000, Some(3_000));
        assert!(child_ok.fits_within(&parent));

        let mut parent_after_first = budget(6_000, Some(5_000)); // reste simulé
        parent_after_first.cpu_ms = None;
        let child_too_big = budget(7_000, None);
        assert!(!child_too_big.fits_within(&parent_after_first));
    }

    #[test]
    fn consumption_over_reserved_dimension_is_violation_but_unknown_dim_is_not() {
        let b = budget(1_000, Some(100));
        let over = EnergyConsumed {
            wall_time_ms: 999,
            inference_tokens: Some(101),
            cpu_ms: None,
            ram_bytes_s: None,
        };
        assert!(!over.within(&b));

        let unmeasured = EnergyConsumed {
            wall_time_ms: 500,
            inference_tokens: None,
            cpu_ms: None,
            ram_bytes_s: None,
        };
        assert!(unmeasured.within(&b)); // None = unknown du receipt, pas échec
    }

    #[test]
    fn consumed_without_reservation_is_a_breach() {
        let b = EnergyBudget {
            wall_time_ms: 1_000,
            inference_tokens: None,
            cpu_ms: Some(50),
            ram_bytes_s: None,
        };
        let c = EnergyConsumed {
            wall_time_ms: 100,
            inference_tokens: None,
            cpu_ms: Some(10),
            ram_bytes_s: None,
        };
        assert!(c.within(&b)); // cpu mesuré sous budget ok
        let rogue = EnergyConsumed {
            wall_time_ms: 100,
            inference_tokens: Some(5),
            cpu_ms: None,
            ram_bytes_s: None,
        };
        assert!(!rogue.within(&b)); // tokens consommés sans réservation
    }
}
