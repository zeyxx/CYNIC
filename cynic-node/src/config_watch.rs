// Config watch concern: detect TOML changes on disk, signal reload.
//
// L3: proactive config reload. Instead of waiting for a model mismatch
// (L2, ~60s verify interval), detect the file change within seconds.

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tokio::sync::watch;

/// Spawn a filesystem watcher on `config_path`.
///
/// Returns a `watch::Receiver<()>` that fires whenever the config file is
/// modified or created. The caller polls `changed()` and re-reads the file.
///
/// The returned `RecommendedWatcher` must be kept alive — dropping it stops
/// the watcher. The `mpsc` bridge is internal (notify uses sync channels).
pub(crate) fn spawn_config_watcher(
    config_path: &str,
) -> Result<(watch::Receiver<()>, RecommendedWatcher), String> {
    let path = PathBuf::from(config_path);
    let watch_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("cannot extract filename from {config_path}"))?
        .to_os_string();

    let (tx, rx) = watch::channel(());

    // notify uses std::sync::mpsc internally — bridge to tokio via a dedicated thread.
    let (sync_tx, sync_rx) = mpsc::channel::<()>();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        let fname = &file_name; // moved into closure, borrow here is free
        if let Ok(event) = res {
            let dominated = matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_));
            if !dominated {
                return;
            }
            // Only fire for our specific file (not other files in the same dir).
            let our_file = event
                .paths
                .iter()
                .any(|p| p.file_name().is_some_and(|f| f == fname));
            if our_file {
                let _ = sync_tx.send(());
            }
        }
    })
    .map_err(|e| format!("watcher init failed: {e}"))?;

    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("watch {}: {e}", watch_dir.display()))?;

    // Bridge thread: sync_rx → tokio watch::Sender.
    // Debounce: coalesce rapid writes (editor save = truncate+write+rename).
    std::thread::spawn(move || {
        while sync_rx.recv().is_ok() {
            // Drain any queued events within 500ms (debounce).
            let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
            while let Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) =
                sync_rx.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            {
                if std::time::Instant::now() >= deadline {
                    break;
                }
            }
            let _ = tx.send(());
        }
    });

    tracing::info!(
        config = config_path,
        "L3: file watcher active on config directory"
    );

    Ok((rx, watcher))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn watcher_detects_file_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-node.toml");
        std::fs::write(&path, b"[dog]\nname = \"old\"").unwrap();

        let (mut rx, _watcher) = spawn_config_watcher(path.to_str().unwrap()).unwrap();

        // Modify the file after a short delay
        let p = path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let mut f = std::fs::File::create(&p).unwrap();
            f.write_all(b"[dog]\nname = \"new\"").unwrap();
        });

        // Should receive notification within 2s (500ms debounce + margin)
        let changed = tokio::time::timeout(std::time::Duration::from_secs(2), rx.changed()).await;

        assert!(changed.is_ok(), "watcher should fire within 2s");
    }
}
