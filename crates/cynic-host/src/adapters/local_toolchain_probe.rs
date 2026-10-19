//! Toolchain probe adapter — observes which tools a host actually provides.
//!
//! The probe is the evidence side of the pool-eligibility rule: a tool that
//! cannot be resolved in `PATH` (or reports a version below the declared
//! minimum) is absent, whatever the registry claims.

use crate::domain::pool_eligibility::{ToolKind, ToolObservation};
use cynic_lifecycle::ArgvRunner;

/// Probe the local host for the given tool names.
pub struct LocalToolchainProbe<R: ArgvRunner> {
    runner: R,
}

impl<R: ArgvRunner> LocalToolchainProbe<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Resolve one tool in `PATH`.
    pub fn resolve_path(&self, name: &str) -> Option<String> {
        let argv = ["sh", "-c", &format!("command -v {name}")];
        let out = self.runner.run(&argv);
        if !out.success() {
            return None;
        }
        let candidate = out.stdout.trim().to_string();
        if candidate.is_empty() {
            None
        } else {
            Some(candidate)
        }
    }

    /// Read the version banner of a resolved tool.
    pub fn read_version(&self, name: &str) -> Option<String> {
        let out = self.runner.run(&[name, "--version"]);
        let text = if out.stdout.trim().is_empty() {
            out.stderr.clone()
        } else {
            out.stdout.clone()
        };
        text.lines()
            .next()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
    }

    /// Probe one tool: path plus version banner.
    pub fn probe_tool(&self, name: &str) -> ToolObservation {
        let path = self.resolve_path(name);
        let version = path.as_ref().and_then(|_| self.read_version(name));
        ToolObservation {
            name: name.to_string(),
            present: path.is_some(),
            path,
            version,
            kind: ToolKind::Binary,
        }
    }

    /// Probe a pkg-config module (e.g. `openssl` for the `libssl-dev` headers).
    ///
    /// System libraries have no executable: they are proven by pkg-config
    /// reporting the module and its version.
    pub fn probe_pkg_config(&self, module: &str) -> ToolObservation {
        let argv = ["pkg-config", "--modversion", module];
        let out = self.runner.run(&argv);
        let version = if out.success() {
            let value = out.stdout.trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        } else {
            None
        };
        ToolObservation {
            name: module.to_string(),
            present: version.is_some(),
            path: None,
            version,
            kind: ToolKind::PkgConfig,
        }
    }

    /// Probe every tool in `names`, honouring each requirement's kind.
    pub fn probe_tools(&self, requirements: &[(String, ToolKind)]) -> Vec<ToolObservation> {
        requirements
            .iter()
            .map(|(name, kind)| match kind {
                ToolKind::Binary => self.probe_tool(name),
                ToolKind::PkgConfig => self.probe_pkg_config(name),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, MapArgvRunner};

    fn runner(responses: Vec<(Vec<String>, ArgvOutput)>) -> MapArgvRunner {
        MapArgvRunner { responses }
    }

    fn ok(stdout: &str) -> ArgvOutput {
        ArgvOutput {
            stdout: stdout.to_string(),
            exit_code: Some(0),
            stderr: String::new(),
            spawn_error: None,
        }
    }

    #[test]
    fn missing_tool_is_observed_as_absent() {
        let probe = LocalToolchainProbe::new(runner(vec![(
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "command -v nope".to_string(),
            ],
            ok(""),
        )]));
        let observation = probe.probe_tool("nope");
        assert!(!observation.present);
        assert!(observation.path.is_none());
    }

    #[test]
    fn present_tool_reports_path_and_version() {
        let probe = LocalToolchainProbe::new(runner(vec![
            (
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "command -v cargo".to_string(),
                ],
                ok("/usr/bin/cargo\n"),
            ),
            (
                vec!["cargo".to_string(), "--version".to_string()],
                ok("cargo 1.95.0 (abc 2026-01-01)\n"),
            ),
        ]));
        let observation = probe.probe_tool("cargo");
        assert!(observation.present);
        assert_eq!(observation.path.as_deref(), Some("/usr/bin/cargo"));
        assert_eq!(
            observation.version.as_deref(),
            Some("cargo 1.95.0 (abc 2026-01-01)")
        );
    }
}
