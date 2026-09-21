use std::sync::atomic::{AtomicU64, Ordering};

/// Métriques et Primitives de Mesure de l'E-Score.
///
/// Permet de mesurer l'activité, la qualité et la dérive (decay)
/// du système de réputation en temps réel de manière thread-safe.
#[derive(Debug)]
pub struct EScoreMetrics {
    /// Nombre total de signaux élémentaires traités par le Kernel.
    pub signals_processed: AtomicU64,
    /// Nombre total de snapshots de portefeuilles agrégés.
    pub wallets_aggregated: AtomicU64,
    /// Nombre total de scores de tokens agrégés.
    pub tokens_aggregated: AtomicU64,
    /// Somme cumulée des ScoreUnits appliquées sur le réseau.
    pub total_score_units_accumulated: AtomicU64,
    /// Nombre de fois où le modificateur anti-baleine a dilué un score (>0 et < 10_000 bps).
    pub anti_whale_dilutions_total: AtomicU64,
    /// Nombre d'opérations de decay (érosion temporelle) appliquées.
    pub decay_operations_total: AtomicU64,
}

impl Default for EScoreMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// Instance globale statique pour simplifier l'accès depuis l'Observer ou le reste du Kernel
pub static ESCORE_METRICS: EScoreMetrics = EScoreMetrics::new();

impl EScoreMetrics {
    pub const fn new() -> Self {
        Self {
            signals_processed: AtomicU64::new(0),
            wallets_aggregated: AtomicU64::new(0),
            tokens_aggregated: AtomicU64::new(0),
            total_score_units_accumulated: AtomicU64::new(0),
            anti_whale_dilutions_total: AtomicU64::new(0),
            decay_operations_total: AtomicU64::new(0),
        }
    }

    /// Réinitialise tous les compteurs (utile pour les benchmarks/tests).
    pub fn reset(&self) {
        self.signals_processed.store(0, Ordering::Relaxed);
        self.wallets_aggregated.store(0, Ordering::Relaxed);
        self.tokens_aggregated.store(0, Ordering::Relaxed);
        self.total_score_units_accumulated
            .store(0, Ordering::Relaxed);
        self.anti_whale_dilutions_total.store(0, Ordering::Relaxed);
        self.decay_operations_total.store(0, Ordering::Relaxed);
    }

    /// Expose les métriques sous format d'exposition Prometheus (text-based).
    pub fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(1024);

        Self::prom_counter(
            &mut out,
            "cynic_escore_signals_processed_total",
            "Nombre total de signaux E-Score traités",
            self.signals_processed.load(Ordering::Relaxed),
        );
        Self::prom_counter(
            &mut out,
            "cynic_escore_wallets_aggregated_total",
            "Nombre total d'agrégations de portefeuilles",
            self.wallets_aggregated.load(Ordering::Relaxed),
        );
        Self::prom_counter(
            &mut out,
            "cynic_escore_tokens_aggregated_total",
            "Nombre total d'agrégations de tokens",
            self.tokens_aggregated.load(Ordering::Relaxed),
        );
        Self::prom_counter(
            &mut out,
            "cynic_escore_units_accumulated_total",
            "Score global cumulé en ScoreUnits",
            self.total_score_units_accumulated.load(Ordering::Relaxed),
        );
        Self::prom_counter(
            &mut out,
            "cynic_escore_anti_whale_dilutions_total",
            "Nombre de dilutions anti-baleine appliquées",
            self.anti_whale_dilutions_total.load(Ordering::Relaxed),
        );
        Self::prom_counter(
            &mut out,
            "cynic_escore_decay_operations_total",
            "Nombre de calculs de decay effectués",
            self.decay_operations_total.load(Ordering::Relaxed),
        );

        out
    }

    fn prom_counter(out: &mut String, name: &str, help: &str, value: u64) {
        out.push_str(&format!("# HELP {help}\n"));
        out.push_str(&format!("# TYPE {name} counter\n"));
        out.push_str(&format!("{name} {value}\n"));
    }
}
