use std::sync::Arc;

use crate::types::*;

/// Agent executor port — how agents are actually run.
///
/// The bus is **provider-agnostic**. It knows nothing about Claude Code,
/// Codex CLI, or Hermes-Agent. Those are concrete impls in adapters.
#[async_trait::async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Execute one turn for the given task. Returns the agent's output.
    ///
    /// This is where the LLM call happens. The bus provides `turn_input`
    /// (which includes prior context, task description, and any handoff data).
    /// The executor returns the agent's textual output.
    async fn execute_turn(&self, task: &Task, turn_input: &str) -> Result<String, AgentError>;

    /// Spawn a new agent session. Returns a session identifier.
    ///
    /// Idempotent — if a valid session already exists, returns it.
    async fn spawn(&self, agent_kind: AgentKind) -> Result<SessionId, AgentError>;

    /// Terminate an agent session. Graceful → force after timeout.
    async fn terminate(&self, session: &SessionId) -> Result<(), AgentError>;

    /// Heartbeat check — is the session still alive?
    async fn is_alive(&self, session: &SessionId) -> bool;
}

/// Identity provider port — cryptographic identity for agents.
#[async_trait::async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Generate a new keypair and derive the agent ID.
    async fn generate(&self) -> Result<(SigningKeyMaterial, AgentId), IdentityError>;

    /// Derive an agent ID from an existing public key.
    async fn derive_id(&self, public_key: &[u8]) -> AgentId;

    /// Sign a message with the agent's private key.
    async fn sign(&self, agent_id: &AgentId, message: &[u8]) -> Result<Vec<u8>, IdentityError>;

    /// Verify a signature against a public key.
    async fn verify(&self, public_key: &[u8], message: &[u8], signature: &[u8]) -> bool;
}

/// Reasoning provider port — LLM for the Coordinator (7B local).
///
/// This is distinct from AgentExecutor (which runs domain workers).
/// The Coordinator reasons *about* the plan, not *in* the domain.
#[async_trait::async_trait]
pub trait CoordinatorReasoner: Send + Sync {
    /// Decompose a high-level plan into a task DAG.
    async fn plan(&self, goal: &str) -> Result<Vec<Task>, ReasoningError>;

    /// Evaluate whether a task's output satisfies its acceptance criteria.
    async fn evaluate(&self, task: &Task, output: &str) -> Result<Evaluation, ReasoningError>;

    /// Detect stagnation: is the fleet stuck?
    async fn detect_stagnation(&self, recent_outputs: &[String]) -> Result<bool, ReasoningError>;
}

/// Supporting types for ports.

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(pub String);

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentKind {
    /// Free inference provider (e.g., meituan/longcat-2.0).
    Free,
    /// Sovereign local inference (cynic-inference runtime).
    Sovereign,
    /// Hermes-Agent CLI.
    Hermes,
    /// Claude Code CLI.
    ClaudeCode,
    /// Codex CLI.
    Codex,
    /// Custom agent (user-defined command).
    Custom,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("session {0} not found")]
    SessionNotFound(SessionId),
    #[error("spawn failed: {0}")]
    SpawnFailed(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("timeout after {0}ms")]
    Timeout(u64),
    #[error("agent exited with code {0}")]
    ExitCode(i32),
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("key generation failed: {0}")]
    KeygenFailed(String),
    #[error("signing failed: {0}")]
    SigningFailed(String),
    #[error("invalid public key")]
    InvalidPublicKey,
}

#[derive(Debug, thiserror::Error)]
pub enum ReasoningError {
    #[error("planning failed: {0}")]
    PlanningFailed(String),
    #[error("evaluation failed: {0}")]
    EvaluationFailed(String),
    #[error("model unavailable: {0}")]
    ModelUnavailable(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Evaluation {
    pub task_id: String,
    pub satisfied: bool,
    pub reason: String,
    pub suggested_action: SuggestedAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SuggestedAction {
    Accept,
    Reject,
    Retry,
    Escalate,
    Abort,
}

/// Placeholder for signing key material (private key bytes).
///
/// Concrete impls wrap ed25519-dalek::SigningKey or similar.
#[derive(Debug, Clone)]
pub struct SigningKeyMaterial(pub Vec<u8>);

// Re-export ports so bus consumers can use them.
pub type DynExecutor = Arc<dyn AgentExecutor>;
pub type DynIdentity = Arc<dyn IdentityProvider>;
pub type DynReasoner = Arc<dyn CoordinatorReasoner>;
