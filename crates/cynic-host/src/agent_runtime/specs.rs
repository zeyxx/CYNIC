use super::types::{AgentRuntimeMode, AgentRuntimeSpec, StatusProjection};

pub(super) const RUNTIME_SPECS: &[AgentRuntimeSpec] = &[
    AgentRuntimeSpec::new(
        "codex-cli",
        AgentRuntimeMode::Interactive,
        &["codex", "--version"],
        None,
        StatusProjection::None,
    ),
    AgentRuntimeSpec::new(
        "antigravity-cli",
        AgentRuntimeMode::Interactive,
        &["agy", "--version"],
        None,
        StatusProjection::None,
    ),
    AgentRuntimeSpec::new(
        "hermes-agent",
        AgentRuntimeMode::Autonomous,
        &["hermes", "--version"],
        Some(&["hermes", "status"]),
        StatusProjection::Hermes,
    ),
    AgentRuntimeSpec::new(
        "mercury-agent",
        AgentRuntimeMode::Autonomous,
        &["mercury", "--version"],
        Some(&["mercury", "status"]),
        StatusProjection::None,
    ),
];
