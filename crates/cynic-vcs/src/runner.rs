//! Command execution abstractions for VCS operations.
//!
//! Layer-pure ports: defines runner traits without concrete OS process bindings.
//! Concrete implementations (LocalVcsRunner) live in `cynic-host`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VcsCommandOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub spawn_error: Option<String>,
}

pub trait VcsRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> VcsCommandOutput;
}

/// String-oriented argv runner output matching CYNIC ArgvOutput.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgvOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
}

impl ArgvOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }

    pub fn summary(&self) -> String {
        if let Some(err) = &self.spawn_error {
            return format!("spawn error: {err}");
        }
        let code = self
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".into());
        let msg = if !self.stdout.trim().is_empty() {
            self.stdout.trim()
        } else if !self.stderr.trim().is_empty() {
            self.stderr.trim()
        } else {
            "empty output"
        };
        format!("exit {code}: {msg}")
    }
}

pub trait ArgvRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> ArgvOutput;
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub struct MapArgvRunner {
    pub responses: Vec<(Vec<String>, ArgvOutput)>,
}

#[cfg(test)]
impl ArgvRunner for MapArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        for (pattern, resp) in &self.responses {
            if pattern.iter().map(|s| s.as_str()).eq(argv.iter().copied()) {
                return resp.clone();
            }
        }
        ArgvOutput {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: format!("unmatched argv: {:?}", argv),
            spawn_error: None,
        }
    }
}
