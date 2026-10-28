//! Remote command, outcome, transport form, and executor port.

use serde::{Deserialize, Serialize};

/// How a command was carried to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportForm {
    /// The target is the machine this process runs on.
    Local,
    /// The target is reached over SSH.
    Ssh,
}

/// A command to run on a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCommand {
    /// The program and its arguments, exactly, with no shell involved.
    pub argv: Vec<String>,
}

/// The result of running a command on a target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteOutcome {
    /// Exit code of the command (or 255 when the transport itself failed).
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// True when the command could not be started at all.
    pub transport_failed: bool,
}

impl RemoteOutcome {
    pub fn success(&self) -> bool {
        !self.transport_failed && self.exit_code == 0
    }
}

/// A transport that can run a command on a target.
pub trait RemoteExecutorPort {
    /// Run a command on the target, returning the observed outcome.
    fn run(&self, target: &str, command: &RemoteCommand) -> RemoteOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_outcome_success() {
        let ok = RemoteOutcome {
            exit_code: 0,
            stdout: "hi".into(),
            stderr: String::new(),
            transport_failed: false,
        };
        assert!(ok.success());

        let fail = RemoteOutcome {
            exit_code: 1,
            stdout: String::new(),
            stderr: "err".into(),
            transport_failed: false,
        };
        assert!(!fail.success());

        let transport = RemoteOutcome {
            exit_code: 255,
            stdout: String::new(),
            stderr: "no connect".into(),
            transport_failed: true,
        };
        assert!(!transport.success());
    }

    #[test]
    fn transport_form_serde_roundtrip() {
        let local = serde_json::to_string(&TransportForm::Local).unwrap();
        assert_eq!(local, "\"local\"");
        let ssh: TransportForm = serde_json::from_str("\"ssh\"").unwrap();
        assert_eq!(ssh, TransportForm::Ssh);
    }

    #[test]
    fn remote_command_roundtrip() {
        let cmd = RemoteCommand {
            argv: vec!["sh".into(), "-c".into(), "echo hi".into()],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let back: RemoteCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cmd);
    }
}
