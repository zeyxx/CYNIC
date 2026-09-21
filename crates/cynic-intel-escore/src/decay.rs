use cynic_intel::escore::types::{ModifierBps, RetentionBps, SCORE_SCALE, ScoreUnit};

/// Applique un decay exponentiel discret en arithmétique entière pure.
///
/// Le `retention_bps` représente le pourcentage du score conservé chaque jour.
/// Exemple : 9_277 bps ≈ exp(-0.075) par jour (≈ 7.25% de perte par jour).
///
/// L'intermédiaire u128 prévient tout overflow pour des scores jusqu'à ~1.8e19.
/// Déterministe : deux nœuds avec les mêmes inputs obtiennent le même résultat.
pub fn calculate_decay(
    base_value: ScoreUnit,
    days_inactive: u32,
    retention_bps: RetentionBps,
) -> ScoreUnit {
    if base_value == 0 || days_inactive == 0 {
        return base_value;
    }
    // Increment telemetry metric
    crate::metrics::ESCORE_METRICS
        .decay_operations_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Un taux de rétention > 10_000 bps serait une augmentation, ce qui est interdit ici.
    let retention = retention_bps.min(10_000) as u128;

    let mut value = base_value as u128;
    for _ in 0..days_inactive {
        value = value * retention / 10_000;
    }
    value as ScoreUnit
}

/// Modificateur quadratique anti-baleine.
///
/// Retourne un coefficient en bps (0 à 10_000) qui pénalise les grandes concentrations.
/// Un wallet qui contrôle 100% du score du réseau ne reçoit que 100% × (1/1)² = 100%
/// mais un wallet qui contrôle 10% du score total ne reçoit que (0.1)² = 1% du poids.
///
/// Note : c'est le fondement mathématique de l'équité du protocole.
pub fn calculate_quadratic_modifier(score: ScoreUnit, max_network_score: ScoreUnit) -> ModifierBps {
    if max_network_score == 0 || score == 0 {
        return 0;
    }
    // Normalisation en bps (0..=10_000) via u128 pour éviter l'overflow
    let normalized_bps = ((score as u128 * 10_000) / max_network_score as u128).min(10_000);
    // Modificateur quadratique : (normalized²) / 10_000 → résultat en bps
    ((normalized_bps * normalized_bps) / 10_000) as ModifierBps
}

/// Applique le poids du tier de preuve à un signal brut.
/// T1OnChain = 100%, T4Solo = 25%.
pub fn apply_tier_weight(raw_weight: ScoreUnit, tier_bps: u32) -> ScoreUnit {
    ((raw_weight as u128 * tier_bps as u128) / 10_000) as ScoreUnit
}

/// Convertit des "points CYNIC" (entiers humains) en ScoreUnits.
pub fn from_points(points: u64) -> ScoreUnit {
    points * SCORE_SCALE
}

/// Convertit des ScoreUnits en points CYNIC (arrondi à l'unité inférieure).
pub fn to_points(units: ScoreUnit) -> u64 {
    units / SCORE_SCALE
}

/// Applique un decay basé sur l'écart d'epochs entre le signal et l'epoch actuel.
///
/// Contrairement à `calculate_decay` qui utilise des jours, cette fonction
/// utilise la primitive `cynic_protocol::Epoch` pour un temps logique fiable.
/// Chaque epoch représente une unité de consensus déterministe.
///
/// Le `retention_bps` est interprété comme le taux de rétention PAR EPOCH.
/// Exemple : 9_277 bps ≈ exp(-0.075) par epoch (≈ 7.25% de perte par epoch).
///
/// # Arguments
/// - `base_value` : Le score de base à appliquer au decay.
/// - `signal_epoch` : L'epoch auquel le signal a été généré.
/// - `current_epoch` : L'epoch actuel du runtime.
/// - `retention_bps` : Le taux de rétention par epoch (en basis points).
///
/// # Returns
/// Le score après application du decay, ou `base_value` si l'écart est nul.
///
/// # Panics
/// Ne panic pas - si `current_epoch < signal_epoch`, retourne `base_value` sans decay.
pub fn calculate_epoch_decay(
    base_value: ScoreUnit,
    signal_epoch: u64,
    current_epoch: u64,
    retention_bps: RetentionBps,
) -> ScoreUnit {
    if base_value == 0 || current_epoch <= signal_epoch {
        // Pas de decay si le signal est à l'epoch actuel ou dans le futur.
        return base_value;
    }

    let epochs_elapsed = (current_epoch - signal_epoch) as u32;

    // Incrémente la métrique de decay
    crate::metrics::ESCORE_METRICS
        .decay_operations_total
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Un taux de rétention > 10_000 bps serait une augmentation, ce qui est interdit.
    let retention = retention_bps.min(10_000) as u128;

    let mut value = base_value as u128;
    for _ in 0..epochs_elapsed {
        value = value * retention / 10_000;
    }
    value as ScoreUnit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_is_strictly_monotonic() {
        // 9_277 bps ≈ exp(-0.075)/jour
        let d0 = calculate_decay(from_points(100), 0, 9_277);
        let d7 = calculate_decay(from_points(100), 7, 9_277);
        let d30 = calculate_decay(from_points(100), 30, 9_277);

        assert_eq!(d0, from_points(100), "Pas de decay à jour 0");
        assert!(d0 > d7, "Le score doit décroître après 7 jours");
        assert!(d7 > d30, "Le score doit décroître encore après 30 jours");
    }

    #[test]
    fn decay_is_fully_deterministic() {
        // Deux appels identiques → résultat identique (la raison d'être de ce module)
        let a = calculate_decay(from_points(500), 14, 9_500);
        let b = calculate_decay(from_points(500), 14, 9_500);
        assert_eq!(a, b, "Le calcul doit être déterministe");
    }

    #[test]
    fn quadratic_modifier_penalizes_whales() {
        // Un petit wallet (20% du réseau) obtient moins de poids qu'un grand (80%)
        let small = calculate_quadratic_modifier(from_points(20), from_points(100));
        let large = calculate_quadratic_modifier(from_points(80), from_points(100));

        assert!(small < large, "Les petits wallets sont moins pénalisés");

        // Un wallet représentant 100% du réseau doit obtenir 10_000 bps (100%)
        let max = calculate_quadratic_modifier(from_points(100), from_points(100));
        assert_eq!(max, 10_000);
    }

    #[test]
    fn tier_weight_reduces_signal_appropriately() {
        let raw = from_points(100);
        let t1 = apply_tier_weight(raw, 10_000); // 100%
        let t4 = apply_tier_weight(raw, 2_500); // 25%

        assert_eq!(t1, raw);
        assert_eq!(t4, from_points(25));
    }

    #[test]
    fn conversion_roundtrip() {
        let points = 42u64;
        assert_eq!(to_points(from_points(points)), points);
    }

    #[test]
    fn epoch_decay_is_strictly_monotonic() {
        // 9_277 bps ≈ exp(-0.075)/epoch
        let d0 = calculate_epoch_decay(from_points(100), 0, 0, 9_277);
        let d5 = calculate_epoch_decay(from_points(100), 0, 5, 9_277);
        let d20 = calculate_epoch_decay(from_points(100), 0, 20, 9_277);

        assert_eq!(d0, from_points(100), "Pas de decay à epoch identique");
        assert!(d0 > d5, "Le score doit décroître après 5 epochs");
        assert!(d5 > d20, "Le score doit décroître encore après 20 epochs");
    }

    #[test]
    fn epoch_decay_is_deterministic() {
        let a = calculate_epoch_decay(from_points(500), 10, 20, 9_500);
        let b = calculate_epoch_decay(from_points(500), 10, 20, 9_500);
        assert_eq!(a, b, "Le calcul doit être déterministe");
    }

    #[test]
    fn epoch_decay_no_decay_when_signal_in_future_or_same_epoch() {
        // Signal dans le futur par rapport à l'epoch actuel
        let future = calculate_epoch_decay(from_points(100), 5, 3, 9_000);
        assert_eq!(
            future,
            from_points(100),
            "Pas de decay si le signal est dans le futur"
        );

        // Signal au même epoch
        let same = calculate_epoch_decay(from_points(100), 7, 7, 9_000);
        assert_eq!(
            same,
            from_points(100),
            "Pas de decay si le signal est à l'epoch actuel"
        );
    }

    #[test]
    fn epoch_decay_zero_base_value() {
        let result = calculate_epoch_decay(0, 0, 50, 9_000);
        assert_eq!(result, 0, "Un score de base de 0 reste 0");
    }
}
