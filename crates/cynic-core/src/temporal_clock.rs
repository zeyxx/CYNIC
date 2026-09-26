//! TemporalClock — Source de vérité pour l'epoch logique du runtime.
//!
//! Le kernel expose un `TemporalClock` thread-safe qui sert de source de vérité
//! unique pour l'epoch logique. L'observer et le node s'y synchronisent via
//! des EpochCertificates reçus via le gossip P2P, ou via un advance local
//! contrôlé par le planificateur kernel.

use std::sync::atomic::{AtomicU64, Ordering};
use cynic_protocol::Epoch;
use serde::{Deserialize, Serialize};

/// Probe en lecture seule de l'état temporel du kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalObservation {
    pub current_epoch: Epoch,
    pub last_mark_timestamp_unix: i64,
}

/// TemporalClock : source de vérité atomique pour l'epoch kernel.
pub struct TemporalClock {
    /// L'epoch logique courant (atomique pour la concurrence).
    current_epoch: AtomicU64,
    /// Timestamp Unix du dernier mark d'epoch.
    last_mark_timestamp: AtomicU64,
}

impl TemporalClock {
    pub fn new() -> Self {
        Self {
            current_epoch: AtomicU64::new(0),
            last_mark_timestamp: AtomicU64::new(0),
        }
    }

    /// Retourne l'epoch courant (lecture lock-free).
    pub fn current_epoch(&self) -> Epoch {
        Epoch::new(self.current_epoch.load(Ordering::Relaxed))
    }

    /// Progresse l'epoch de 1 (avance locale).
    /// Cette opération est bornée et contrôlée par le scheduler kernel.
    pub fn advance(&self) -> Epoch {
        let new_epoch = self.current_epoch.fetch_add(1, Ordering::SeqCst) + 1;
        self.last_mark_timestamp.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            Ordering::Relaxed,
        );
        Epoch::new(new_epoch)
    }

    /// Tente de synchroniser à un epoch certifié (P2P).
    /// Ne recule jamais l'epoch (monotonie garantie).
    /// Retourne l'epoch après tentative de synchronisation.
    pub fn sync_to_certificate(&self, cert_epoch: Epoch) -> Option<Epoch> {
        let current = self.current_epoch();
        if cert_epoch <= current {
            // Pas de recul possible — on ignore
            return None;
        }
        // CAS (compare-and-swap) pour monter à l'epoch certifié
        let expected = current.id();
        let desired = cert_epoch.id();
        match self.current_epoch.compare_exchange(
            expected,
            desired,
            Ordering::SeqCst,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                self.last_mark_timestamp.store(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    Ordering::Relaxed,
                );
                Some(Epoch::new(desired))
            }
            Err(_) => {
                // Un autre thread a déjà avancé — on réessaie une fois
                let new_current = self.current_epoch();
                if cert_epoch <= new_current {
                    None
                } else {
                    // Retry CAS
                    if self.current_epoch.compare_exchange(
                        new_current.id(),
                        cert_epoch.id(),
                        Ordering::SeqCst,
                        Ordering::Relaxed,
                    ).is_ok() {
                        Some(cert_epoch)
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Produit une observation atomique (read-back proof).
    pub fn observe(&self) -> TemporalObservation {
        TemporalObservation {
            current_epoch: self.current_epoch(),
            last_mark_timestamp_unix: self.last_mark_timestamp.load(Ordering::Relaxed) as i64,
        }
    }
}

impl Default for TemporalClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait pour exposer le TemporalClock via les backends.
pub trait TemporalClockProvider {
    fn current_epoch(&self) -> Epoch;
    fn observe(&self) -> TemporalObservation;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_starts_at_zero() {
        let clock = TemporalClock::new();
        assert_eq!(clock.current_epoch(), Epoch::ZERO);
    }

    #[test]
    fn advance_increments_monotonically() {
        let clock = TemporalClock::new();
        let e0 = clock.current_epoch();
        let e1 = clock.advance();
        let e2 = clock.advance();
        assert_eq!(e0, Epoch::ZERO);
        assert_eq!(e1, Epoch::new(1));
        assert_eq!(e2, Epoch::new(2));
    }

    #[test]
    fn sync_to_certificate_advances_only_forward() {
        let clock = TemporalClock::new();
        clock.advance(); // epoch = 1
        // Certificat pour un epoch PLUS AVANCÉ doit être accepté
        assert_eq!(clock.sync_to_certificate(Epoch::new(5)), Some(Epoch::new(5)));
        // Certificat pour un epoch PLUS PETIT doit être ignoré
        assert_eq!(clock.sync_to_certificate(Epoch::new(2)), None);
    }

    #[test]
    fn observation_reflects_current_state() {
        let clock = TemporalClock::new();
        let obs = clock.observe();
        assert_eq!(obs.current_epoch, Epoch::ZERO);
    }
}
