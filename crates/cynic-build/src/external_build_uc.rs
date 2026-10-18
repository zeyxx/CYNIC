//! Pure orchestration usecase for external build operations.
//!
//! WHY: composes the pure admission validator, the `ExternalBuildPort` trait,
//! and the pure proof builder into callable usecase functions. This module is
//! the single entry point for running external builds/tests — callers inject
//! any port implementation (HTTP, local, mock) and receive typed proofs.
//! HOW: thin orchestration layer. `run_external_build` delegates to the port's
//! `run_build_proof`; `run_external_command` delegates to the port's
//! `run_command` for the first command in the request.
//! NEGATIVE BOUNDARY: no std::process::Command, no SSH, no HTTP, no filesystem
//! I/O. No direct calls to `build_external_proof` — the port owns proof
//! construction from live results.

use crate::external_build::{
    ExternalBuildPort, ExternalBuildProof, ExternalBuildRequest, ExternalCommandResult,
};

// ── Usecase: run full build proof ─────────────────────────────────────────────

/// Run a full external build/test proof via the given port.
///
/// Pure orchestration: delegates all I/O to the port, returns the proof.
/// The builder node ID is taken from the request.
pub fn run_external_build(
    port: &dyn ExternalBuildPort,
    req: ExternalBuildRequest,
) -> ExternalBuildProof {
    port.run_build_proof(&req, &req.builder_node_id)
}

// ── Usecase: run single command ───────────────────────────────────────────────

/// Run the first command in the request via the given port.
///
/// Pure orchestration: delegates I/O to the port, wraps the output as
/// [`ExternalCommandResult`]. If the request has no commands, returns a
/// synthetic failure result (no port call).
pub fn run_external_command(
    port: &dyn ExternalBuildPort,
    req: &ExternalBuildRequest,
) -> ExternalCommandResult {
    match req.commands.first() {
        Some(cmd) => {
            let output = port.run_command(cmd, &req.work_dir);
            ExternalCommandResult::from_output(cmd.clone(), &output)
        }
        None => ExternalCommandResult {
            command: String::new(),
            exit_code: None,
            stdout: String::new(),
            stderr: "no commands in request".to_string(),
            spawn_error: Some("no commands in request".to_string()),
            success: false,
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_build::{ExternalBuildKind, build_external_proof};
    use crate::ports::BuildCommandOutput;

    // ── Mock port ──────────────────────────────────────────────────────────────

    struct MockPort {
        proof: ExternalBuildProof,
        command_result: BuildCommandOutput,
        last_command: std::cell::RefCell<Option<(String, String)>>,
    }

    impl ExternalBuildPort for MockPort {
        fn run_command(&self, command: &str, work_dir: &str) -> BuildCommandOutput {
            *self.last_command.borrow_mut() = Some((command.to_string(), work_dir.to_string()));
            self.command_result.clone()
        }

        fn run_build_proof(
            &self,
            _request: &ExternalBuildRequest,
            _builder_node_id: &str,
        ) -> ExternalBuildProof {
            self.proof.clone()
        }
    }

    fn success_output() -> BuildCommandOutput {
        BuildCommandOutput {
            exit_code: Some(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    fn fail_output() -> BuildCommandOutput {
        BuildCommandOutput {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error".to_string(),
            spawn_error: None,
        }
    }

    fn sample_request() -> ExternalBuildRequest {
        ExternalBuildRequest {
            project_id: "test-proj".to_string(),
            commands: vec!["make build".to_string(), "make test".to_string()],
            work_dir: "/builder/work".to_string(),
            kind: ExternalBuildKind::Build,
            builder_node_id: "builder-1".to_string(),
        }
    }

    // ── run_external_build ─────────────────────────────────────────────────────

    #[test]
    fn run_external_build_delegates_to_port() {
        let proof = build_external_proof(
            "test-proj",
            ExternalBuildKind::Build,
            vec![],
            "builder-1",
        );
        let port = MockPort {
            proof: proof.clone(),
            command_result: success_output(),
            last_command: std::cell::RefCell::new(None),
        };
        let req = sample_request();
        let result = run_external_build(&port, req);
        assert_eq!(result.schema, "cynic.build.external_proof.v1");
        assert_eq!(result.project_id, "test-proj");
        assert_eq!(result.builder_node_id, "builder-1");
    }

    #[test]
    fn run_external_build_returns_proof_from_port() {
        let inner_proof = build_external_proof(
            "test-proj",
            ExternalBuildKind::Test,
            vec![],
            "builder-9",
        );
        let port = MockPort {
            proof: inner_proof.clone(),
            command_result: success_output(),
            last_command: std::cell::RefCell::new(None),
        };
        let mut req = sample_request();
        req.builder_node_id = "builder-9".to_string();
        let result = run_external_build(&port, req);
        assert_eq!(result.builder_node_id, "builder-9");
        assert_eq!(result.kind, ExternalBuildKind::Test);
    }

    // ── run_external_command ───────────────────────────────────────────────────

    #[test]
    fn run_external_command_success() {
        let port = MockPort {
            proof: build_external_proof(
                "x",
                ExternalBuildKind::Build,
                vec![],
                "x",
            ),
            command_result: success_output(),
            last_command: std::cell::RefCell::new(None),
        };
        let req = sample_request();
        let result = run_external_command(&port, &req);
        assert!(result.success);
        assert_eq!(result.command, "make build");
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "ok");
    }

    #[test]
    fn run_external_command_failure() {
        let port = MockPort {
            proof: build_external_proof(
                "x",
                ExternalBuildKind::Build,
                vec![],
                "x",
            ),
            command_result: fail_output(),
            last_command: std::cell::RefCell::new(None),
        };
        let req = sample_request();
        let result = run_external_command(&port, &req);
        assert!(!result.success);
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(result.stderr, "error");
    }

    #[test]
    fn run_external_command_empty_commands_returns_failure() {
        let port = MockPort {
            proof: build_external_proof(
                "x",
                ExternalBuildKind::Build,
                vec![],
                "x",
            ),
            command_result: success_output(),
            last_command: std::cell::RefCell::new(None),
        };
        let mut req = sample_request();
        req.commands = vec![];
        let result = run_external_command(&port, &req);
        assert!(!result.success);
        assert!(result.command.is_empty());
        assert!(result.spawn_error.is_some());
    }

    #[test]
    fn run_external_command_calls_port_with_correct_args() {
        let port = MockPort {
            proof: build_external_proof(
                "x",
                ExternalBuildKind::Build,
                vec![],
                "x",
            ),
            command_result: success_output(),
            last_command: std::cell::RefCell::new(None),
        };
        let req = sample_request();
        let _ = run_external_command(&port, &req);
        let borrowed = port.last_command.borrow();
        let (cmd, dir) = borrowed.as_ref().expect("port should have been called");
        assert_eq!(cmd, "make build");
        assert_eq!(dir, "/builder/work");
    }
}
