//! Exponential backoff with a min-uptime stability reset.
//!
//! Ported from `cynic-node/src/supervise.rs`. Pure-ish state machine over
//! configured retry policy; callers own clocks and read-back proof.

use std::time::{Duration, Instant};

use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use cynic_protocol::supervision::RestartPolicy;

/// Configuration consumed by [`Backoff::from_config`].
#[derive(Debug, Clone, Copy)]
pub struct BackoffConfig {
    pub max_attempts: u32,
    pub initial_delay_secs: u64,
    pub max_delay_secs: u64,
    pub min_uptime_secs: u64,
}

impl BackoffConfig {
    pub fn to_restart_policy(&self) -> RestartPolicy {
        RestartPolicy {
            max_attempts: self.max_attempts,
            initial_delay_secs: self.initial_delay_secs,
            max_delay_secs: self.max_delay_secs,
            min_uptime_secs: self.min_uptime_secs,
        }
    }
}

/// Trait allowing [`Backoff::from_config`] to consume any config-like value.
pub trait RestartConfigLike {
    fn max_attempts(&self) -> u32;
    fn initial_delay_secs(&self) -> u64;
    fn max_delay_secs(&self) -> u64;
    fn min_uptime_secs(&self) -> u64;
}

impl RestartConfigLike for BackoffConfig {
    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
    fn initial_delay_secs(&self) -> u64 {
        self.initial_delay_secs
    }
    fn max_delay_secs(&self) -> u64 {
        self.max_delay_secs
    }
    fn min_uptime_secs(&self) -> u64 {
        self.min_uptime_secs
    }
}

/// Exponential backoff with a min-uptime stability reset.
#[derive(Debug)]
pub struct Backoff {
    pub attempt: u32,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    min_uptime: Duration,
    last_start: Instant,
}

impl Backoff {
    pub fn new(
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
        min_uptime: Duration,
    ) -> Self {
        Self {
            attempt: 0,
            max_attempts,
            initial_delay,
            max_delay,
            min_uptime,
            last_start: Instant::now(),
        }
    }

    pub fn from_config<T: RestartConfigLike>(cfg: &T) -> Self {
        Self::new(
            cfg.max_attempts(),
            Duration::from_secs(cfg.initial_delay_secs()),
            Duration::from_secs(cfg.max_delay_secs()),
            Duration::from_secs(cfg.min_uptime_secs()),
        )
    }

    pub fn record_start(&mut self) {
        self.last_start = Instant::now();
    }

    pub fn reset_if_stable(&mut self) {
        if self.last_start.elapsed() >= self.min_uptime {
            self.attempt = 0;
        }
    }

    pub fn reset(&mut self) {
        self.attempt = 0;
    }

    pub fn exhausted(&self) -> bool {
        self.attempt >= self.max_attempts
    }

    pub fn can_retry(&self) -> bool {
        !self.exhausted()
    }

    pub fn next_delay(&mut self) -> Duration {
        self.attempt = self.attempt.saturating_add(1);
        let policy = RestartPolicy {
            max_attempts: self.max_attempts,
            initial_delay_secs: self.initial_delay.as_secs(),
            max_delay_secs: self.max_delay.as_secs(),
            min_uptime_secs: self.min_uptime.as_secs(),
        };
        Duration::from_secs(cynic_protocol::supervision::delay_for_attempt(
            &policy,
            self.attempt,
        ))
    }

    /// Alias for [`Self::next_delay`].
    pub fn delay(&mut self) -> Duration {
        self.next_delay()
    }

    pub async fn wait_or_shutdown(&mut self, shutdown: &CancellationToken) {
        let delay = self.next_delay();
        tokio::select! {
            _ = sleep(delay) => {}
            _ = shutdown.cancelled() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_progression() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            Duration::from_secs(10),
        );
        assert_eq!(b.next_delay(), Duration::from_secs(2));
        assert_eq!(b.next_delay(), Duration::from_secs(4));
        assert_eq!(b.next_delay(), Duration::from_secs(8));
        assert_eq!(b.next_delay(), Duration::from_secs(16));
        assert_eq!(b.next_delay(), Duration::from_secs(32));
        assert_eq!(b.attempt, 5);
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut b = Backoff::new(
            10,
            Duration::from_secs(2),
            Duration::from_secs(30),
            Duration::from_secs(10),
        );
        for _ in 0..10 {
            let delay = b.next_delay();
            assert!(delay <= Duration::from_secs(30), "delay {delay:?} exceeded max");
        }
        let mut b2 = Backoff::new(
            10,
            Duration::from_secs(2),
            Duration::from_secs(30),
            Duration::from_secs(10),
        );
        for _ in 0..4 {
            b2.next_delay();
        }
        assert_eq!(b2.next_delay(), Duration::from_secs(30));
    }

    #[test]
    fn backoff_reset_if_stable() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            Duration::from_secs(0),
        );
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt, 2);
        b.record_start();
        b.reset_if_stable();
        assert_eq!(b.attempt, 0);
    }

    #[test]
    fn backoff_reset_if_stable_not_yet() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            Duration::from_secs(3600),
        );
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt, 2);
        b.record_start();
        b.reset_if_stable();
        assert_eq!(b.attempt, 2, "should NOT reset — not stable yet");
    }

    #[test]
    fn backoff_reset_unconditional() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            Duration::from_secs(3600),
        );
        b.next_delay();
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt, 3);
        b.reset();
        assert_eq!(b.attempt, 0);
    }
}
