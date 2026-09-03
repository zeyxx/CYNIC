//! Host wall-clock — single canonical millisecond source.
//!
//! Every host module that stamps `observed_at_unix_ms` / `proved_at_unix_ms`
//! uses this function. No private `now_unix_ms` copies elsewhere.

/// Current wall-clock in milliseconds since the Unix epoch, saturating at
/// `u64::MAX`. Returns `0` when the system clock is before the epoch.
pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
