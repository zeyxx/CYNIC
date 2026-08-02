//! Observed identity attached by the Unix-domain listener.
//!
//! This is transport evidence, not authorization. A session admission policy
//! must still map the numeric peer to an enrolled principal and delegation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalPeerCredentials {
    pub process_id: Option<u32>,
    pub process_start_time_ticks: Option<u64>,
    pub user_id: u32,
    pub group_id: u32,
}

impl LocalPeerCredentials {
    pub fn observed(credential: &tokio::net::unix::UCred) -> Self {
        let process_id = credential
            .pid()
            .and_then(|process_id| u32::try_from(process_id).ok());
        Self {
            process_id,
            process_start_time_ticks: process_id.and_then(observe_process_start_time_ticks),
            user_id: credential.uid(),
            group_id: credential.gid(),
        }
    }
}

fn observe_process_start_time_ticks(process_id: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{process_id}/stat")).ok()?;
    let command_end = stat.rfind(')')?;
    stat.get(command_end + 1..)?
        .split_whitespace()
        .nth(19)
        .and_then(|value| value.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::observe_process_start_time_ticks;

    #[test]
    fn current_process_start_time_is_observable() {
        assert!(observe_process_start_time_ticks(std::process::id()).is_some());
    }
}
