use serde::{Deserialize, Serialize};

/// L'unité atomique du E-Score. 1 point = 1_000_000 ScoreUnits.
/// Utiliser des entiers évite toute non-associativité des floats IEEE 754.
/// Deux nœuds Argo calculant la même séquence obtiennent EXACTEMENT le même résultat.
pub type ScoreUnit = u64;

/// Taux de rétention journalier en points de base (basis points).
/// 10_000 bps = 100 % (aucun decay). 9_277 bps ≈ exp(-0.075)/jour.
pub type RetentionBps = u32;

/// Multiplicateur anti-baleine en bps (0 à 10_000).
pub type ModifierBps = u32;

/// 1 point standard dans le système CYNIC.
pub const SCORE_SCALE: ScoreUnit = 1_000_000;

/// Tier de preuve de chaque signal d'entrée.
/// Plus le tier est bas, plus le signal est objectif et résistant à la manipulation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum EvidenceTier {
    /// Vérité absolue : ancré dans la SVM Solana (on-chain).
    T1OnChain,
    /// Attestation cryptographique hors-chaîne mais vérifiable.
    T2Attestation,
    /// Signal d'un Oracle externe (ex: Twitter via Helius webhook).
    T3Oracle,
    /// Signal auto-déclaré, le moins fiable.
    T4Solo,
}

impl EvidenceTier {
    /// Pondération du tier. T1 = 100%, T4 = 25%.
    /// Retourné en bps pour rester en arithmétique entière.
    pub fn weight_bps(&self) -> u32 {
        match self {
            EvidenceTier::T1OnChain => 10_000,    // 100%
            EvidenceTier::T2Attestation => 7_500, //  75%
            EvidenceTier::T3Oracle => 5_000,      //  50%
            EvidenceTier::T4Solo => 2_500,        //  25%
        }
    }
}

/// Les 7 vertus grecques dimensionnant le E-Score.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum StatType {
    /// Endurance, consistance dans le temps (réputation long-terme).
    Karteria,
    /// Franchise, honnêteté : pas de wash-trading, pas de rugpull.
    Parrhesia,
    /// Humilité : ne pas se positionner en whale dominante.
    Anaideia,
    /// Générosité, contributions au bien commun (LP, grants).
    Philanthropia,
    /// Autosuffisance, indépendance vis-à-vis du levier et des protocoles risqués.
    Autarkeia,
    /// Création de valeur : deploy de contrats, d'outils, de bots publics.
    Poiesis,
    /// Vie communautaire : gouvernance, vote, coordination on-chain.
    Koinonia,
}

/// Un signal élémentaire contribuant au E-Score d'un wallet.
/// `raw_weight` est en `ScoreUnit` (entier) — jamais en float.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EScoreSignal {
    pub wallet_id: String,
    pub stat_type: StatType,
    pub evidence_tier: EvidenceTier,
    /// Poids brut en ScoreUnits (1 point = 1_000_000 unités).
    pub raw_weight: ScoreUnit,
    /// Unix timestamp (secondes) de l'événement qui a généré ce signal.
    pub timestamp: i64,
}
