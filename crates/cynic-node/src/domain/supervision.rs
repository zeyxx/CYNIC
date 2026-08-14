#[derive(Debug)]
pub enum ExitReason {
    /// SIGTERM / SIGINT received; clean shutdown requested.
    Shutdown,
    /// Backend process exited unexpectedly or failed health checks.
    Crashed,
    /// Kernel evicted this node heartbeat identity.
    Expired,
    /// Wrong model loaded; backend identity check failed.
    Mismatch,
    /// Config file changed on disk; proactive reload required.
    ConfigChanged,
    /// Permanent error that cannot be recovered from.
    Fatal(String),
}
