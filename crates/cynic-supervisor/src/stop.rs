//! Bounded stop ladder for CYNIC-owned child processes.
//!
//! Sends SIGTERM (Unix, to the process group) or initiates a kill (Windows),
//! waits up to `timeout_secs`, then sends SIGKILL and waits unconditionally.
//! Callers supply the timeout from the supervisor contract (`15s` stop grace
//! by default) and own post-stop read-back proof.

use std::{io, time::Duration};

use process_wrap::tokio::ChildWrapper;

/// Gracefully stop a child process with a bounded wait and force kill.
pub async fn graceful_stop(child: &mut Box<dyn ChildWrapper>, timeout_secs: u64) -> io::Result<()> {
    #[cfg(unix)]
    {
        // SIGTERM the process group (ProcessGroup wrapper propagates to group).
        // Ignore send errors — process may already be dead.
        let _ = child.signal(libc::SIGTERM);
    }
    #[cfg(not(unix))]
    {
        // Windows: initiate kill, then give it a moment to flush.
        let _ = child.start_kill();
    }

    let timeout = Duration::from_secs(timeout_secs);
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(result) => result.map(|_| ()),
        Err(_elapsed) => {
            // Timed out — force kill.
            let _ = child.start_kill();
            child.wait().await.map(|_| ())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::spawn_backend;

    #[tokio::test]
    async fn graceful_stop_clean() {
        let env: HashMap<String, String> = HashMap::new();
        let mut child = spawn_backend(&["sleep", "60"], None, &env).unwrap();
        // Should SIGTERM immediately, well within the 5s bound.
        graceful_stop(&mut child, 5).await.unwrap();
        let exited = child.try_wait().unwrap();
        assert!(
            exited.is_some(),
            "process should have exited after graceful_stop"
        );
    }
}
