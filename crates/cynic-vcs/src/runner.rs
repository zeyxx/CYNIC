//! Command execution abstractions for VCS operations.

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

pub trait ArgvRunner: Send + Sync {
    fn run(&self, argv: &[&str]) -> ArgvOutput;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalVcsRunner;

impl VcsRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> VcsCommandOutput {
        if argv.is_empty() {
            return VcsCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some("empty argv".to_string()),
            };
        }
        match std::process::Command::new(argv[0])
            .args(&argv[1..])
            .output()
        {
            Ok(output) => VcsCommandOutput {
                exit_code: output.status.code(),
                stdout: output.stdout,
                stderr: output.stderr,
                spawn_error: None,
            },
            Err(error) => VcsCommandOutput {
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                spawn_error: Some(format!("spawn {}: {error}", argv[0])),
            },
        }
    }
}

impl ArgvRunner for LocalVcsRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        let out = <Self as VcsRunner>::run(self, argv);
        ArgvOutput {
            exit_code: out.exit_code,
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            spawn_error: out.spawn_error,
        }
    }
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
