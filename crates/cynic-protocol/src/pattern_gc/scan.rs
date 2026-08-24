//! Pure windowing of timestamped pattern events into [`PatternWindows`].
//!
//! This is the pure half of the future scan surface: it turns ordered event
//! timestamps plus an explicit boundary into the windowed counts consumed by
//! `policy::classify`. File access, JSONL parsing, and key extraction stay
//! adapter-owned; nothing here touches the filesystem.

use super::types::PatternWindows;

/// Windowed counts plus an explicit residue for events stamped after the
/// declared boundary.
///
/// Future-stamped events are never silently folded into a window (unknown
/// is data): the caller sees `out_of_order` and decides — fix the clock,
/// drop the source, or re-run with a later boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowOutcome {
    pub windows: PatternWindows,
    /// Events whose timestamp lies beyond `boundary_s`.
    pub out_of_order: u64,
}

/// Split event unix-seconds into a cold (historical) and hot (recent)
/// window against `boundary_s`.
///
/// The hot window covers `[boundary - recent_span, boundary]`; every earlier
/// event falls in the cold window. `historical_span` therefore spans from
/// the oldest observed event to the hot start (0 when the cold side is
/// empty), keeping rate comparisons honest for young keys. Returns `None`
/// when `recent_span_s` is zero — an explicit unknown, not a guess.
pub fn window_events(
    timestamps: &[i64],
    boundary_s: i64,
    recent_span_s: u64,
) -> Option<WindowOutcome> {
    if recent_span_s == 0 {
        return None;
    }
    let hot_start = boundary_s - recent_span_s as i64;
    let mut historical: u64 = 0;
    let mut recent: u64 = 0;
    let mut out_of_order: u64 = 0;
    let mut oldest: Option<i64> = None;

    for &ts in timestamps {
        if ts > boundary_s {
            out_of_order += 1;
        } else if ts >= hot_start {
            recent += 1;
        } else {
            historical += 1;
            oldest = Some(oldest.map_or(ts, |min: i64| min.min(ts)));
        }
    }

    let historical_span = oldest.map_or(0, |oldest| (hot_start - oldest).max(0) as u64);

    Some(WindowOutcome {
        windows: PatternWindows {
            historical,
            recent,
            historical_span,
            recent_span: recent_span_s,
        },
        out_of_order,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_span_is_explicit_unknown() {
        assert!(window_events(&[1, 2, 3], 100, 0).is_none());
    }

    #[test]
    fn empty_input_yields_zero_windows() {
        let o = window_events(&[], 1000, 100).unwrap();
        assert_eq!(o.windows.historical, 0);
        assert_eq!(o.windows.recent, 0);
        assert_eq!(o.windows.historical_span, 0);
        assert_eq!(o.out_of_order, 0);
    }

    #[test]
    fn mixed_events_split_at_hot_start() {
        // boundary 1000, hot [900, 1000]: 950 hot; 100/500 cold; 1100 future.
        let ts = [100, 500, 950, 1100];
        let o = window_events(&ts, 1000, 100).unwrap();
        assert_eq!(o.windows.historical, 2);
        assert_eq!(o.windows.recent, 1);
        assert_eq!(o.windows.recent_span, 100);
        // Cold spans 900 - 100 = 800 seconds.
        assert_eq!(o.windows.historical_span, 800);
        assert_eq!(o.out_of_order, 1);
    }

    #[test]
    fn young_key_with_only_recent_events_has_zero_cold_span() {
        let o = window_events(&[950, 990], 1000, 100).unwrap();
        assert_eq!(o.windows.historical, 0);
        assert_eq!(o.windows.recent, 2);
        assert_eq!(o.windows.historical_span, 0);
    }

    #[test]
    fn boundary_inclusive_edges() {
        // Exactly at hot_start and exactly at boundary both land hot.
        let o = window_events(&[900, 1000], 1000, 100).unwrap();
        assert_eq!(o.windows.recent, 2);
        assert_eq!(o.windows.historical, 0);
    }

    #[test]
    fn unsorted_input_is_accepted() {
        let o = window_events(&[990, 10, 950], 1000, 100).unwrap();
        assert_eq!(o.windows.historical, 1);
        assert_eq!(o.windows.recent, 2);
        // Oldest cold event is 10 regardless of input order.
        assert_eq!(o.windows.historical_span, 890);
    }
}
