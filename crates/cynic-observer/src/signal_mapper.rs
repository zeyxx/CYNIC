//! Signal Mapper — Connecteur Observer → Kernel
//!
//! Ce module est le pont entre les couches. Il traduit les actions brutes
//! captées par le Watcher (domaine Observer) en signaux E-Score sémantiques
//! (domaine Kernel), sans jamais faire d'I/O.
//!
//! Architecture hexagonale : entrée = `PumpFunAction`, sortie = `Vec<EScoreSignal>`.
//! Le mapping est pur, déterministe, et testable sans réseau.

use cynic_intel::escore::types::{EScoreSignal, EvidenceTier, ScoreUnit, StatType, SCORE_SCALE};

use crate::parser::PumpFunAction;

/// Résultat d'un mapping : zéro, un ou plusieurs signaux pour un événement.
#[derive(Debug)]
pub struct MappedSignals {
    /// Signaux pour le wallet qui a effectué l'action.
    pub wallet_signals: Vec<EScoreSignal>,
    /// Signaux pour le token concerné (son mint address = wallet_id du signal).
    pub token_signals: Vec<EScoreSignal>,
}

/// Traduit une `PumpFunAction` en signaux E-Score pour le Kernel.
///
/// Philosophie du mapping :
/// - `TokenCreated`  → `Poiesis` (création de valeur) + `Parrhesia` (intention déclarée)
/// - `Trade(Buy)`    → `Karteria` (engagement long-terme) + `Philanthropia` (on fait circuler les fonds)
///                     Si sol_amount > seuil → signal `Anaideia` inversé (anti-baleine flag)
/// - `Trade(Sell)`   → `Autarkeia` (gestion du risque) — neutre à modéré
///                     Si dump massif → pénalité `Parrhesia` (signal de malhonnêteté)
/// - `CurveCompleted` → `Karteria` boost pour les holders restants (preuve d'endurance)
/// - `Unknown`       → aucun signal
pub fn map_action(action: &PumpFunAction, now_unix: i64) -> MappedSignals {
    match action {
        // ── Création de token ─────────────────────────────────────────────
        PumpFunAction::TokenCreated {
            token_address,
            user,
        } => {
            let wallet_signals = vec![
                // Créer un token = acte de création (Poiesis)
                signal(
                    user,
                    StatType::Poiesis,
                    EvidenceTier::T1OnChain,
                    5 * SCORE_SCALE,
                    now_unix,
                ),
                // Intention déclarée : le créateur s'engage publiquement (Parrhesia)
                signal(
                    user,
                    StatType::Parrhesia,
                    EvidenceTier::T1OnChain,
                    2 * SCORE_SCALE,
                    now_unix,
                ),
            ];
            let token_signals = vec![
                // Le token naît avec un score Poiesis de son créateur
                signal(
                    token_address,
                    StatType::Poiesis,
                    EvidenceTier::T1OnChain,
                    5 * SCORE_SCALE,
                    now_unix,
                ),
            ];
            MappedSignals {
                wallet_signals,
                token_signals,
            }
        }

        // ── Trade ─────────────────────────────────────────────────────────
        PumpFunAction::Trade {
            token_address,
            sol_amount,
            is_buy,
            ..
        } => {
            // Seuil anti-wash trade : ignorer les transactions < 0.1 SOL (100_000_000 lamports)
            if *sol_amount < 100_000_000 {
                return MappedSignals {
                    wallet_signals: vec![],
                    token_signals: vec![],
                };
            }

            // Seuil baleine : > 10 SOL (en lamports : 10 * 10^9)
            let is_whale_move = *sol_amount > 10_000_000_000;

            // Score proportionnel au volume (plafonné à 100 points pour éviter les abus)
            let volume_score = volume_to_score(*sol_amount).min(100 * SCORE_SCALE);

            if *is_buy {
                buy_signals(token_address, volume_score, is_whale_move, now_unix)
            } else {
                sell_signals(token_address, *sol_amount, volume_score, now_unix)
            }
        }

        // ── Bonding curve complète → Migration Raydium ────────────────────
        PumpFunAction::CurveCompleted { token_address } => {
            let token_signals = vec![
                // Le token a prouvé son endurance jusqu'à la graduation
                signal(
                    token_address,
                    StatType::Karteria,
                    EvidenceTier::T1OnChain,
                    50 * SCORE_SCALE,
                    now_unix,
                ),
                // La complétion = honnêteté prouvée (pas de rug avant graduation)
                signal(
                    token_address,
                    StatType::Parrhesia,
                    EvidenceTier::T1OnChain,
                    30 * SCORE_SCALE,
                    now_unix,
                ),
                // Koinonia : une vraie communauté s'est formée
                signal(
                    token_address,
                    StatType::Koinonia,
                    EvidenceTier::T1OnChain,
                    20 * SCORE_SCALE,
                    now_unix,
                ),
            ];
            MappedSignals {
                wallet_signals: vec![],
                token_signals,
            }
        }

        PumpFunAction::Unknown => MappedSignals {
            wallet_signals: vec![],
            token_signals: vec![],
        },
    }
}

// ── Helpers privés ────────────────────────────────────────────────────────────

fn buy_signals(
    token_address: &str,
    volume_score: ScoreUnit,
    is_whale: bool,
    now_unix: i64,
) -> MappedSignals {
    let mut token_signals = vec![
        // Achat = endurance (commitment Karteria)
        signal(
            token_address,
            StatType::Karteria,
            EvidenceTier::T1OnChain,
            volume_score,
            now_unix,
        ),
        // Achat = participation à l'écosystème (Philanthropia)
        signal(
            token_address,
            StatType::Philanthropia,
            EvidenceTier::T1OnChain,
            volume_score / 2,
            now_unix,
        ),
    ];

    if is_whale {
        // Une baleine qui achète massivement : signal négatif d'humilité (Anaideia)
        // On encode la pénalité comme un poids faible (1 unité = quasi zéro contribution)
        // Le vrai système de pénalité sera géré par le modificateur quadratique de l'agrégateur.
        token_signals.push(signal(
            token_address,
            StatType::Anaideia,
            EvidenceTier::T1OnChain,
            SCORE_SCALE / 10,
            now_unix,
        ));
    } else {
        // Petit achat : signal Anaideia positif (humilité = bien)
        token_signals.push(signal(
            token_address,
            StatType::Anaideia,
            EvidenceTier::T1OnChain,
            volume_score,
            now_unix,
        ));
    }

    MappedSignals {
        wallet_signals: vec![],
        token_signals,
    }
}

fn sell_signals(
    token_address: &str,
    sol_amount: u64,
    volume_score: ScoreUnit,
    now_unix: i64,
) -> MappedSignals {
    // Seuil dump : vente > 50 SOL = potentiel rug/exit scam
    let is_dump = sol_amount > 50_000_000_000;

    let mut token_signals = vec![
        // Vente = gestion du risque (Autarkeia)
        signal(
            token_address,
            StatType::Autarkeia,
            EvidenceTier::T1OnChain,
            volume_score / 3,
            now_unix,
        ),
    ];

    if is_dump {
        // Gros dump → pénalité Parrhesia (malhonnêteté possible)
        token_signals.push(signal(
            token_address,
            StatType::Parrhesia,
            EvidenceTier::T1OnChain,
            SCORE_SCALE / 100,
            now_unix,
        ));
    }

    MappedSignals {
        wallet_signals: vec![],
        token_signals,
    }
}

/// Convertit un montant en lamports en ScoreUnits (relation logarithmique).
/// 1 SOL (1e9 lamports) → ~1 point. 10 SOL → ~2.3 points. Plafond à 100 points.
fn volume_to_score(lamports: u64) -> ScoreUnit {
    if lamports == 0 {
        return 0;
    }
    // Approximation entière de ln(lamports / 1e9 + 1) × SCORE_SCALE
    // On évite les floats avec une approximation par paliers logarithmiques.
    let sol_units = lamports / 1_000_000; // en micro-SOL pour rester en u64
                                          // Paliers : chaque doublement de volume = +0.7 point (~ln(2))
    let bits = 64 - sol_units.leading_zeros() as u64;
    bits * SCORE_SCALE * 7 / 10 // ≈ ln(sol_units) × SCORE_SCALE
}

fn signal(
    wallet_id: &str,
    stat_type: StatType,
    evidence_tier: EvidenceTier,
    raw_weight: ScoreUnit,
    timestamp: i64,
) -> EScoreSignal {
    EScoreSignal {
        wallet_id: wallet_id.to_owned(),
        stat_type,
        evidence_tier,
        raw_weight,
        timestamp,
        epoch: None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_720_000_000;

    #[test]
    fn token_creation_emits_poiesis_and_parrhesia() {
        let action = PumpFunAction::TokenCreated {
            token_address: "MINT_XYZ".to_string(),
            user: "CREATOR_WALLET".to_string(),
        };
        let mapped = map_action(&action, NOW);

        assert_eq!(mapped.wallet_signals.len(), 2);
        assert_eq!(mapped.token_signals.len(), 1);
        assert!(mapped
            .wallet_signals
            .iter()
            .any(|s| s.stat_type == StatType::Poiesis));
        assert!(mapped
            .wallet_signals
            .iter()
            .any(|s| s.stat_type == StatType::Parrhesia));
    }

    #[test]
    fn small_buy_emits_karteria_philanthropia_anaideia() {
        let action = PumpFunAction::Trade {
            token_address: "MINT_XYZ".to_string(),
            sol_amount: 1_000_000_000, // 1 SOL
            token_amount: 100_000,
            is_buy: true,
        };
        let mapped = map_action(&action, NOW);
        let stats: Vec<_> = mapped.token_signals.iter().map(|s| &s.stat_type).collect();

        assert!(stats.contains(&&StatType::Karteria));
        assert!(stats.contains(&&StatType::Philanthropia));
        assert!(stats.contains(&&StatType::Anaideia));
    }

    #[test]
    fn whale_buy_generates_weak_anaideia() {
        let small = PumpFunAction::Trade {
            token_address: "T".to_string(),
            sol_amount: 1_000_000_000, // 1 SOL
            token_amount: 0,
            is_buy: true,
        };
        let whale = PumpFunAction::Trade {
            token_address: "T".to_string(),
            sol_amount: 50_000_000_000, // 50 SOL
            token_amount: 0,
            is_buy: true,
        };

        let small_map = map_action(&small, NOW);
        let whale_map = map_action(&whale, NOW);

        let small_anaideia = small_map
            .token_signals
            .iter()
            .find(|s| s.stat_type == StatType::Anaideia)
            .unwrap()
            .raw_weight;
        let whale_anaideia = whale_map
            .token_signals
            .iter()
            .find(|s| s.stat_type == StatType::Anaideia)
            .unwrap()
            .raw_weight;

        assert!(
            whale_anaideia < small_anaideia,
            "Une baleine doit avoir un Anaideia plus faible qu'un petit acheteur"
        );
    }

    #[test]
    fn curve_completed_boosts_karteria_and_parrhesia() {
        let action = PumpFunAction::CurveCompleted {
            token_address: "GRADUATED_TOKEN".to_string(),
        };
        let mapped = map_action(&action, NOW);

        assert!(mapped.wallet_signals.is_empty());
        assert_eq!(mapped.token_signals.len(), 3);
        let stats: Vec<_> = mapped.token_signals.iter().map(|s| &s.stat_type).collect();
        assert!(stats.contains(&&StatType::Karteria));
        assert!(stats.contains(&&StatType::Parrhesia));
        assert!(stats.contains(&&StatType::Koinonia));
    }

    #[test]
    fn dump_sell_penalizes_parrhesia() {
        let dump = PumpFunAction::Trade {
            token_address: "RUG_TOKEN".to_string(),
            sol_amount: 100_000_000_000, // 100 SOL dump
            token_amount: 0,
            is_buy: false,
        };
        let mapped = map_action(&dump, NOW);
        assert!(mapped
            .token_signals
            .iter()
            .any(|s| s.stat_type == StatType::Parrhesia));
    }

    #[test]
    fn mapping_is_deterministic() {
        let action = PumpFunAction::Trade {
            token_address: "T".to_string(),
            sol_amount: 5_000_000_000,
            token_amount: 500_000,
            is_buy: true,
        };
        let a = map_action(&action, NOW);
        let b = map_action(&action, NOW);

        assert_eq!(a.token_signals.len(), b.token_signals.len());
        for (sa, sb) in a.token_signals.iter().zip(b.token_signals.iter()) {
            assert_eq!(sa.raw_weight, sb.raw_weight);
        }
    }
}
