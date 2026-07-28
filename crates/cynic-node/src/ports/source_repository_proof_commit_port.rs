use cynic_protocol::{LeaseGuard, LeaseVerification, RuntimeProofBundle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRepositoryProofCommitError {
    pub code: String,
    pub message: String,
}

impl SourceRepositoryProofCommitError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait SourceRepositoryProofCommitPort {
    async fn claim(
        &self,
        agent_id: &str,
        target: &str,
    ) -> Result<LeaseGuard, SourceRepositoryProofCommitError>;

    async fn verify(
        &self,
        guard: &LeaseGuard,
    ) -> Result<LeaseVerification, SourceRepositoryProofCommitError>;

    async fn finalize(
        &self,
        guard: &LeaseGuard,
        bundle: &RuntimeProofBundle,
    ) -> Result<String, SourceRepositoryProofCommitError>;

    async fn release(&self, guard: &LeaseGuard) -> Result<(), SourceRepositoryProofCommitError>;
}
