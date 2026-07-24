use cynic_protocol::{LeaseGuard, LeaseVerification};

pub trait SourceLeaseVerifier: Send + Sync {
    fn verify(&self, guard: &LeaseGuard) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct UnavailableSourceLeaseVerifier {
    blocker: String,
}

impl UnavailableSourceLeaseVerifier {
    pub fn new(blocker: impl Into<String>) -> Self {
        Self {
            blocker: blocker.into(),
        }
    }
}

impl Default for UnavailableSourceLeaseVerifier {
    fn default() -> Self {
        Self::new("coordination_lease_authority_unavailable")
    }
}

impl SourceLeaseVerifier for UnavailableSourceLeaseVerifier {
    fn verify(&self, _guard: &LeaseGuard) -> Result<(), String> {
        Err(self.blocker.clone())
    }
}

impl SourceLeaseVerifier for LeaseVerification {
    fn verify(&self, guard: &LeaseGuard) -> Result<(), String> {
        if self.guard != *guard {
            return Err("coordination_lease_verification_guard_mismatch".to_string());
        }
        if self.status != "proved" {
            return Err(self
                .blockers
                .first()
                .cloned()
                .unwrap_or_else(|| "coordination_lease_not_proved".to_string()));
        }
        Ok(())
    }
}
