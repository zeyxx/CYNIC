//! Port definitions for VCS runners.

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
