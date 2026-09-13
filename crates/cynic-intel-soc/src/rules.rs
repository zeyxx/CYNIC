//! Pure rules and source classification for introspection.
//!
//! Canonical owner: `cynic-intel` (SoC foundation §481).

/// Whether a source name represents a permanent entity (organ/service) vs ephemeral session.
/// Permanent sources generate alerts on silence. Ephemeral sessions don't.
pub fn is_permanent_source(s: &str) -> bool {
    classify_source_tier(s) == "permanent"
}

/// Classify an agent_id into a source tier at write time (E1).
/// Single source of truth — called by POST /observe and by backfill.
/// Returns: "permanent" (organs, services), "cron" (scheduled jobs), "session" (ephemeral).
pub fn classify_source_tier(agent_id: &str) -> &'static str {
    if agent_id.is_empty() || agent_id == "unknown" {
        return "session";
    }
    if agent_id.starts_with("cron-") {
        return "cron";
    }
    if agent_id.starts_with("hermes")
        || agent_id.starts_with("x-")
        || agent_id == "kernel"
        || agent_id == "nightshift"
        || agent_id == "watchlist"
        || agent_id.starts_with("organ-")
    {
        return "permanent";
    }
    "session"
}

/// Retired sources that are explicitly suppressed from silence alarms.
pub const RETIRED_SOURCES: &[&str] = &[
    "organ-pump-portal",
    "organ-jupiter-swap",
    "organ-poh-witness",
    "organ-solana-ops",
    "organ-helius",
    "organ-pyth-oracle",
    "organ-dexscreener",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_source_tier() {
        assert_eq!(classify_source_tier("kernel"), "permanent");
        assert_eq!(classify_source_tier("hermes-main"), "permanent");
        assert_eq!(classify_source_tier("x-pulse"), "permanent");
        assert_eq!(classify_source_tier("organ-storage"), "permanent");
        assert_eq!(classify_source_tier("cron-clean"), "cron");
        assert_eq!(classify_source_tier("agent-codex-1"), "session");
        assert_eq!(classify_source_tier("unknown"), "session");
        assert_eq!(classify_source_tier(""), "session");
    }

    #[test]
    fn test_is_permanent_source() {
        assert!(is_permanent_source("kernel"));
        assert!(is_permanent_source("organ-dog"));
        assert!(!is_permanent_source("cron-backup"));
        assert!(!is_permanent_source("session-abc"));
    }
}
