//! Spawn-path regression for tmux harness observation.
//!
//! The pure parser is covered by unit tests. These tests prove the adapter
//! surfaces sessions through the real `tmux list-sessions` spawn path (via a
//! fake `tmux` earlier on `PATH`) and never reports a silent empty harness on
//! success: a successful but unparseable answer stays an explicit unknown.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use cynic_host::adapters::local_operator_transport::LocalOperatorTransportObserver;
use cynic_host::transport::OperatorTransportObservationPort;

/// A fake `tmux` executable earlier on `PATH`, removed and uninstalled on drop.
struct FakeTmux {
    dir: PathBuf,
    previous_path: Option<OsString>,
}

impl FakeTmux {
    fn install(payload: &str) -> Self {
        let dir = env::temp_dir().join(format!("cynic-fake-tmux-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear stale fake tmux dir");
        }
        fs::create_dir_all(&dir).expect("create fake tmux dir");
        fs::write(dir.join("payload"), payload).expect("write fake tmux payload");
        let script = dir.join("tmux");
        fs::write(
            &script,
            format!("#!/bin/sh\ncat \"{}\"\n", dir.join("payload").display()),
        )
        .expect("write fake tmux script");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))
            .expect("make fake tmux executable");
        let previous_path = env::var_os("PATH");
        let mut paths = vec![dir.clone().into_os_string()];
        if let Some(ref path) = previous_path {
            paths.push(path.clone());
        }
        // PATH mutation is process-local to this integration-test binary.
        unsafe { env::set_var("PATH", env::join_paths(paths).expect("join fake tmux PATH")) };
        Self { dir, previous_path }
    }
}

impl Drop for FakeTmux {
    fn drop(&mut self) {
        match self.previous_path.take() {
            // PATH restore is process-local to this integration-test binary.
            Some(path) => unsafe { env::set_var("PATH", path) },
            None => unsafe { env::remove_var("PATH") },
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn spawn_path_surfaces_live_session() {
    let _fake = FakeTmux::install("mcp-design\t1\t1\n");
    let observation = LocalOperatorTransportObserver.observe_tmux();

    assert_eq!(observation.status, "active");
    assert_eq!(observation.sessions.len(), 1);
    assert_eq!(observation.sessions[0].name, "mcp-design");
    assert_eq!(observation.sessions[0].windows, 1);
    assert_eq!(observation.sessions[0].attached_clients, 1);
    assert_eq!(observation.detail, None);
}

#[test]
fn successful_empty_answer_stays_explicit_unknown() {
    let _fake = FakeTmux::install("");
    let observation = LocalOperatorTransportObserver.observe_tmux();

    assert_eq!(observation.status, "active");
    assert!(observation.sessions.is_empty());
    assert!(
        observation.detail.is_some(),
        "a successful zero-session answer must name the socket ambiguity"
    );
}
