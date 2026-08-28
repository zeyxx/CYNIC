use serde::{Deserialize, Serialize};

pub const TMUX_SESSION_CONFIG_SCHEMA: &str = "cynic.tmux_session_config.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxSessionConfig {
    pub schema: String,
    pub session_name: String,
    pub ssh_target: String,
}

impl TmuxSessionConfig {
    pub fn builder() -> Self {
        Self {
            schema: TMUX_SESSION_CONFIG_SCHEMA.to_string(),
            session_name: "cynic-builder".to_string(),
            ssh_target: "cynic-builder-build".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_config_has_correct_target() {
        let c = TmuxSessionConfig::builder();
        assert_eq!(c.session_name, "cynic-builder");
        assert_eq!(c.ssh_target, "cynic-builder-build");
    }
}
