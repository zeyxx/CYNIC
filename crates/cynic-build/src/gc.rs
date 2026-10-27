//! Garbage collection decision for build targets.
//!
//! Pure function: decides which build-target profiles to remove based on
//! disk pressure and profile age. I/O (listing, deleting) is the caller's job.

/// A build-target profile on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetProfile {
    pub name: String,
    pub size_bytes: u64,
    pub age_days: u64,
}

/// Decide which build-target profiles to remove.
///
/// Pure function: no I/O. Returns the names of profiles to remove.
///
/// Rules:
/// - If disk pressure < 0.8: remove nothing.
/// - If 0.8 <= pressure < 0.9: remove profiles older than 7 days.
/// - If pressure >= 0.9: remove all profiles except the newest one.
pub fn decide_gc(pressure: f64, targets: &[TargetProfile]) -> Vec<String> {
    if pressure < 0.8 {
        return Vec::new();
    }

    if pressure >= 0.9 {
        // Keep the newest (lowest age_days), remove the rest.
        if targets.is_empty() {
            return Vec::new();
        }
        let newest = targets
            .iter()
            .min_by_key(|t| t.age_days)
            .map(|t| t.name.clone())
            .unwrap_or_default();
        return targets
            .iter()
            .filter(|t| t.name != newest)
            .map(|t| t.name.clone())
            .collect();
    }

    // 0.8 <= pressure < 0.9: remove profiles older than 7 days.
    targets
        .iter()
        .filter(|t| t.age_days > 7)
        .map(|t| t.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gc_under_pressure() {
        let targets = vec![TargetProfile {
            name: "debug".into(),
            size_bytes: 8_000_000_000,
            age_days: 30,
        }];
        assert!(decide_gc(0.5, &targets).is_empty());
    }

    #[test]
    fn gc_old_targets_at_high_pressure() {
        let targets = vec![
            TargetProfile {
                name: "debug".into(),
                size_bytes: 8_000_000_000,
                age_days: 30,
            },
            TargetProfile {
                name: "default".into(),
                size_bytes: 2_500_000_000,
                age_days: 3,
            },
        ];
        let to_remove = decide_gc(0.85, &targets);
        assert_eq!(to_remove, vec!["debug"]);
    }

    #[test]
    fn gc_all_but_newest_at_critical() {
        let targets = vec![
            TargetProfile {
                name: "debug".into(),
                size_bytes: 8_000_000_000,
                age_days: 30,
            },
            TargetProfile {
                name: "default".into(),
                size_bytes: 2_500_000_000,
                age_days: 3,
            },
        ];
        let to_remove = decide_gc(0.95, &targets);
        assert_eq!(to_remove, vec!["debug"]);
    }

    #[test]
    fn empty_targets() {
        assert!(decide_gc(0.95, &[]).is_empty());
    }
}
