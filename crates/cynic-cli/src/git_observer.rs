//! Local Git observer adapter implementing `cynic_audit_engine::ports::GitObserverPort`.

use std::path::{Path, PathBuf};
use cynic_audit_engine::ports::GitObserverPort;
use cynic_process::ArgvRunner;
use cynic_protocol::argv::ArgvOutput;

pub struct LocalGitObserver {
    pub root: PathBuf,
}

impl LocalGitObserver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl GitObserverPort for LocalGitObserver {
    fn run_git(&self, args: &[&str]) -> ArgvOutput {
        let mut argv = vec!["git", "-C", self.root.to_str().unwrap_or(".")];
        argv.extend_from_slice(args);
        cynic_process::LocalArgvRunner.run(&argv)
    }

    fn patch_id(&self, commit: &str) -> Option<String> {
        let show = self.run_git(&["show", "--pretty=format:", commit]);
        if !show.success() {
            return None;
        }
        let mut piped = std::process::Command::new("git")
            .current_dir(&self.root)
            .args(["patch-id", "--stable"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;
        use std::io::Write;
        if let Some(stdin) = piped.stdin.as_mut() {
            let _ = stdin.write_all(show.stdout.as_bytes());
        }
        let out = piped.wait_with_output().ok()?;
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .map(str::to_string)
    }
}
