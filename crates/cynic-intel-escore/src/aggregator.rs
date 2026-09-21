use crate::decay::{apply_tier_weight, calculate_decay, calculate_quadratic_modifier};
use crate::merkle::{Blake3Hash, MerkleError, merkle_root};
use cynic_intel::escore::types::{EScoreSignal, Epoch, SCORE_SCALE, ScoreUnit, StatType};
use serde::{Deserialize, Serialize};

// ── Poids de chaque vertu dans le score composite d'un wallet ────────────────
// Ces poids sont en bps (somme = 10_000 = 100 %).
// Modifiables via gouvernance (Agora) sans changer le code.
// Karteria (endurance) et Parrhesia (honnêteté) sont fondamentales.
const STAT_WEIGHTS_BPS: [(StatType, u32); 7] = [
    (StatType::Karteria, 2_500),      // 25% — endurance, historique long
    (StatType::Parrhesia, 2_000),     // 20% — honnêteté, pas de rug
    (StatType::Anaideia, 1_500),      // 15% — humilité anti-baleine
    (StatType::Philanthropia, 1_500), // 15% — générosité (LP, grants)
    (StatType::Autarkeia, 1_000),     // 10% — indépendance du levier
    (StatType::Poiesis, 500),         //  5% — création de valeur
    (StatType::Koinonia, 1_000),      // 10% — vie communautaire
];

// Taux de rétention standard par dimension (bps/jour).
// Karteria décroît lentement (haute inertie), Koinonia décroît vite (engagement volatile).
const STAT_RETENTION_BPS: [(StatType, u32); 7] = [
    (StatType::Karteria, 9_800),      // 98%/j — très stable
    (StatType::Parrhesia, 9_700),     // 97%/j
    (StatType::Anaideia, 9_600),      // 96%/j
    (StatType::Philanthropia, 9_500), // 95%/j
    (StatType::Autarkeia, 9_600),     // 96%/j
    (StatType::Poiesis, 9_400),       // 94%/j
    (StatType::Koinonia, 9_200),      // 92%/j — le plus volatile
];

/// Snapshot agrégé d'un wallet sur les 7 dimensions, à un instant T.
/// C'est l'unité de base de la fractale E-Score côté utilisateur.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WalletSnapshot {
    pub pubkey: String,
    /// Score agrégé par dimension (index = position dans `StatType`).
    pub dim_scores: [ScoreUnit; 7],
    /// Score composite pondéré (somme des 7 dimensions × leurs poids).
    pub composite: ScoreUnit,
    /// Racine de Merkle de tous les signaux ayant contribué à ce snapshot.
    /// Sert de preuve cryptographique déterministe pour la fédération.
    pub signal_root: Blake3Hash,
}

/// E-Score d'un Token : agrégation fractale des wallets qui l'ont tradé/holdé.
/// C'est la "réputation de marché" du token, calculée sans aucun float.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TokenEScore {
    pub mint: String,
    /// Score composite final du token (en ScoreUnits).
    pub composite: ScoreUnit,
    /// Décomposition par dimension pour l'UI.
    pub dim_scores: [ScoreUnit; 7],
    /// Nombre de wallets ayant contribué.
    pub holder_count: u32,
    /// Score maximal d'un wallet parmi les contributeurs.
    /// Exposé pour permettre le calcul du modificateur anti-baleine relatif.
    pub max_holder_score: ScoreUnit,
}

/// Erreur d'agrégation.
#[derive(Debug, thiserror::Error)]
pub enum AggregatorError {
    #[error("no signals provided for wallet {0}")]
    EmptySignals(String),
    #[error("merkle error: {0}")]
    Merkle(#[from] MerkleError),
}

// ── Agrégation d'un Wallet ────────────────────────────────────────────────────

/// Agrège les signaux bruts d'un wallet en un `WalletSnapshot` déterministe.
///
/// Étapes :
/// 1. Pour chaque signal → applique decay (selon StatType) + tier weight.
/// 2. Somme les signaux par dimension.
/// 3. Calcule le composite pondéré (STAT_WEIGHTS_BPS).
/// 4. Calcule la racine de Merkle de l'ensemble des signaux.
///
/// Le paramètre `current_epoch` permet de calculer un decay temporel via des epochs
/// logiques au lieu des jours Unix, garantissant une convergence déterministe entre nœuds.
pub fn aggregate_wallet(
    pubkey: &str,
    signals: &[EScoreSignal],
    now_unix: i64,
    current_epoch: Epoch,
) -> Result<WalletSnapshot, AggregatorError> {
    if signals.is_empty() {
        return Err(AggregatorError::EmptySignals(pubkey.to_owned()));
    }

    let mut dim_scores = [0u64; 7];

    for signal in signals {
        // Priorité au decay par epoch si disponible, sinon decay par jours Unix.
        let decayed = if let Some(signal_epoch) = signal.epoch {
            // Decay basé sur les epochs logiques
            let retention_bps = retention_for(&signal.stat_type);
            crate::decay::calculate_epoch_decay(
                signal.raw_weight,
                signal_epoch,
                current_epoch.id(),
                retention_bps,
            )
        } else {
            // Fallback: decay basé sur les jours Unix (comportement legacy)
            let days_inactive = days_elapsed(signal.timestamp, now_unix);
            let retention_bps = retention_for(&signal.stat_type);
            calculate_decay(signal.raw_weight, days_inactive, retention_bps)
        };

        // Pondération par le tier de preuve (T1=100%, T4=25%)
        let weighted = apply_tier_weight(decayed, signal.evidence_tier.weight_bps());

        // Accumulation dans la dimension correspondante
        let idx = stat_index(&signal.stat_type);
        dim_scores[idx] = dim_scores[idx].saturating_add(weighted);
    }

    // Score composite pondéré par les poids des vertus
    let composite = weighted_composite(&dim_scores);

    // Preuve cryptographique (Merkle sur les signaux bruts)
    let signal_root = merkle_root(signals)?;

    // Increment telemetry metrics
    let m = &crate::metrics::ESCORE_METRICS;
    m.signals_processed
        .fetch_add(signals.len() as u64, std::sync::atomic::Ordering::Relaxed);
    m.wallets_aggregated
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    m.total_score_units_accumulated
        .fetch_add(composite, std::sync::atomic::Ordering::Relaxed);

    Ok(WalletSnapshot {
        pubkey: pubkey.to_owned(),
        dim_scores,
        composite,
        signal_root,
    })
}

// ── Agrégation d'un Token (fractale) ─────────────────────────────────────────

/// Agrège les snapshots de wallets en un `TokenEScore`.
///
/// La fractale E-Score fonctionne ainsi :
/// - Chaque wallet contribue à la réputation du token qu'il trade/hold.
/// - Mais les grosses baleine ne dominent pas : un modificateur quadratique
///   anti-baleine est appliqué à chaque wallet, relatif au wallet le plus fort.
/// - Le résultat est déterministe : deux nœuds Argo avec les mêmes snapshots
///   calculent exactement le même TokenEScore.
pub fn aggregate_token(
    mint: &str,
    snapshots: &[WalletSnapshot],
) -> Result<TokenEScore, AggregatorError> {
    if snapshots.is_empty() {
        return Err(AggregatorError::EmptySignals(mint.to_owned()));
    }

    // 1. Trouver le score maximum parmi tous les wallets (référence anti-baleine)
    let max_holder_score = snapshots
        .iter()
        .map(|s| s.composite)
        .max()
        .unwrap_or(SCORE_SCALE);

    let mut token_dims = [0u64; 7];
    let mut total_composite: u128 = 0;

    let m = &crate::metrics::ESCORE_METRICS;
    m.tokens_aggregated
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    for snapshot in snapshots {
        // 2. Modificateur quadratique : les baleines sont réduites proportionnellement
        let anti_whale_bps = calculate_quadratic_modifier(snapshot.composite, max_holder_score);
        if anti_whale_bps > 0 && anti_whale_bps < 10_000 {
            m.anti_whale_dilutions_total
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // 3. Contribution de ce wallet : composite × anti_whale / 10_000
        let contribution =
            (snapshot.composite as u128 * anti_whale_bps as u128 / 10_000) as ScoreUnit;

        total_composite = total_composite.saturating_add(contribution as u128);

        // 4. Accumulation par dimension avec le même modificateur
        for (i, &dim) in snapshot.dim_scores.iter().enumerate() {
            let dim_contribution = (dim as u128 * anti_whale_bps as u128 / 10_000) as ScoreUnit;
            token_dims[i] = token_dims[i].saturating_add(dim_contribution);
        }
    }

    // Score composite final : moyenne pondérée sur le nombre de holders
    let holder_count = snapshots.len() as u32;
    let composite = (total_composite / holder_count as u128) as ScoreUnit;

    Ok(TokenEScore {
        mint: mint.to_owned(),
        composite,
        dim_scores: token_dims,
        holder_count,
        max_holder_score,
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn days_elapsed(signal_unix: i64, now_unix: i64) -> u32 {
    let diff = now_unix.saturating_sub(signal_unix).max(0);
    (diff / 86_400) as u32
}

fn stat_index(stat: &StatType) -> usize {
    match stat {
        StatType::Karteria => 0,
        StatType::Parrhesia => 1,
        StatType::Anaideia => 2,
        StatType::Philanthropia => 3,
        StatType::Autarkeia => 4,
        StatType::Poiesis => 5,
        StatType::Koinonia => 6,
    }
}

fn retention_for(stat: &StatType) -> u32 {
    STAT_RETENTION_BPS
        .iter()
        .find(|(s, _)| std::mem::discriminant(s) == std::mem::discriminant(stat))
        .map(|(_, r)| *r)
        .unwrap_or(9_500)
}

fn weighted_composite(dim_scores: &[ScoreUnit; 7]) -> ScoreUnit {
    let mut total: u128 = 0;
    for (i, &(_, weight_bps)) in STAT_WEIGHTS_BPS.iter().enumerate() {
        total += dim_scores[i] as u128 * weight_bps as u128;
    }
    // total est en ScoreUnits × bps, on divise par 10_000 pour revenir en ScoreUnits
    (total / 10_000) as ScoreUnit
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_intel::escore::types::{EvidenceTier, SCORE_SCALE};
    use std::sync::atomic::Ordering;

    const NOW: i64 = 1_720_100_000;

    fn signal(stat: StatType, tier: EvidenceTier, points: u64, days_ago: i64) -> EScoreSignal {
        EScoreSignal {
            wallet_id: "test".to_string(),
            stat_type: stat,
            evidence_tier: tier,
            raw_weight: points * SCORE_SCALE,
            timestamp: NOW - days_ago * 86_400,
            epoch: None,
        }
    }

    #[test]
    fn wallet_aggregate_is_deterministic() {
        let signals = vec![
            signal(StatType::Karteria, EvidenceTier::T1OnChain, 100, 7),
            signal(StatType::Parrhesia, EvidenceTier::T2Attestation, 50, 3),
            signal(StatType::Koinonia, EvidenceTier::T3Oracle, 30, 1),
        ];

        let current_epoch = Epoch::new(100);
        let a = aggregate_wallet("wallet-x", &signals, NOW, current_epoch).unwrap();
        let b = aggregate_wallet("wallet-x", &signals, NOW, current_epoch).unwrap();
        assert_eq!(a, b, "L'agrégation doit être 100% déterministe");
        assert!(a.composite > 0);
    }

    #[test]
    fn recent_signals_score_higher_than_old() {
        let fresh = vec![signal(StatType::Karteria, EvidenceTier::T1OnChain, 100, 1)];
        let stale = vec![signal(
            StatType::Karteria,
            EvidenceTier::T1OnChain,
            100,
            365,
        )];

        let current_epoch = Epoch::new(100);
        let s_fresh = aggregate_wallet("w", &fresh, NOW, current_epoch).unwrap();
        let s_stale = aggregate_wallet("w", &stale, NOW, current_epoch).unwrap();

        assert!(
            s_fresh.composite > s_stale.composite,
            "Un signal récent doit scorer plus haut qu'un signal vieux d'un an"
        );
    }

    #[test]
    fn t1_tier_scores_higher_than_t4() {
        let on_chain = vec![signal(StatType::Karteria, EvidenceTier::T1OnChain, 100, 0)];
        let self_decl = vec![signal(StatType::Karteria, EvidenceTier::T4Solo, 100, 0)];

        let current_epoch = Epoch::new(100);
        let t1 = aggregate_wallet("w", &on_chain, NOW, current_epoch).unwrap();
        let t4 = aggregate_wallet("w", &self_decl, NOW, current_epoch).unwrap();

        assert!(t1.composite > t4.composite, "T1OnChain doit dominer T4Solo");
    }

    #[test]
    fn anti_whale_reduces_dominant_wallet_contribution() {
        // Whale : 1000 points, Minnow : 10 points
        let current_epoch = Epoch::new(100);
        let whale = {
            let s = vec![signal(
                StatType::Karteria,
                EvidenceTier::T1OnChain,
                1_000,
                0,
            )];
            aggregate_wallet("whale", &s, NOW, current_epoch).unwrap()
        };
        let minnow = {
            let s = vec![signal(StatType::Karteria, EvidenceTier::T1OnChain, 10, 0)];
            aggregate_wallet("minnow", &s, NOW, current_epoch).unwrap()
        };

        let snapshots = vec![whale.clone(), minnow.clone()];
        let token = aggregate_token("MINT_XYZ", &snapshots).unwrap();

        // La contribution brute de la whale est 100× celle du minnow.
        // Après modificateur quadratique, ce ratio doit être bien plus faible.
        let raw_ratio = whale.composite / minnow.composite.max(1);
        let token_max = token.max_holder_score;

        // Le token score doit être ancré entre les deux valeurs, pas dominé par la whale.
        assert!(token.composite < whale.composite);
        assert!(token.composite > 0);
        assert!(raw_ratio > 10, "Le ratio brut initial doit être > 10×");
        assert!(
            token_max == whale.composite,
            "La whale est bien le max référence"
        );
    }

    #[test]
    fn fractal_scales_correctly_at_token_level() {
        // 5 wallets identiques → le token score doit être identique au score individuel
        let current_epoch = Epoch::new(100);
        let s = vec![signal(StatType::Karteria, EvidenceTier::T1OnChain, 100, 10)];
        let snap = aggregate_wallet("w", &s, NOW, current_epoch).unwrap();

        let all_equal: Vec<WalletSnapshot> = (0..5)
            .map(|i| WalletSnapshot {
                pubkey: format!("w{i}"),
                ..snap.clone()
            })
            .collect();

        let token = aggregate_token("EQUAL_TOKEN", &all_equal).unwrap();
        // Quand tous les wallets sont identiques, le modificateur quadratique est max (10_000)
        // → contribution = composite × 10_000 / 10_000 = composite pour chacun
        // → moyenne = composite (pas de dilution)
        assert_eq!(
            token.composite, snap.composite,
            "Wallets identiques → token score = wallet score"
        );
    }

    #[test]
    fn test_escore_metrics_tracking() {
        let m = &crate::metrics::ESCORE_METRICS;

        // On prend les valeurs initiales pour gérer l'interpénétration des tests en parallèle
        let init_signals = m.signals_processed.load(Ordering::Relaxed);
        let init_wallets = m.wallets_aggregated.load(Ordering::Relaxed);
        let init_tokens = m.tokens_aggregated.load(Ordering::Relaxed);
        let init_decay = m.decay_operations_total.load(Ordering::Relaxed);
        let init_anti_whale = m.anti_whale_dilutions_total.load(Ordering::Relaxed);

        let current_epoch = Epoch::new(100);

        // 1. On agrège un wallet
        let s = vec![
            signal(StatType::Karteria, EvidenceTier::T1OnChain, 100, 5), // decay appliqué car days > 0
            signal(StatType::Parrhesia, EvidenceTier::T1OnChain, 50, 0),
        ];
        let _snap = aggregate_wallet("w-metrics", &s, NOW, current_epoch).unwrap();

        assert!(m.signals_processed.load(Ordering::Relaxed) >= init_signals + 2);
        assert!(m.wallets_aggregated.load(Ordering::Relaxed) > init_wallets);
        assert!(m.decay_operations_total.load(Ordering::Relaxed) > init_decay);

        // 2. On agrège un token avec 1 whale et 1 minnow pour tester les dilutions
        let whale = WalletSnapshot {
            pubkey: "whale".to_string(),
            dim_scores: [1000 * SCORE_SCALE; 7],
            composite: 1000 * SCORE_SCALE,
            signal_root: [0; 32],
        };
        let minnow = WalletSnapshot {
            pubkey: "minnow".to_string(),
            dim_scores: [10 * SCORE_SCALE; 7],
            composite: 10 * SCORE_SCALE,
            signal_root: [0; 32],
        };

        let _token = aggregate_token("token-metrics", &[whale, minnow]).unwrap();
        assert!(m.tokens_aggregated.load(Ordering::Relaxed) > init_tokens);
        // Le minnow a un score < max_holder_score (1000) donc subit une dilution (<10_000 bps)
        assert!(m.anti_whale_dilutions_total.load(Ordering::Relaxed) > init_anti_whale);

        // 3. Rendu Prometheus (on vérifie la présence des compteurs)
        let prometheus_out = m.render_prometheus();
        assert!(prometheus_out.contains("cynic_escore_signals_processed_total"));
        assert!(prometheus_out.contains("cynic_escore_wallets_aggregated_total"));
        assert!(prometheus_out.contains("cynic_escore_anti_whale_dilutions_total"));
    }
}
