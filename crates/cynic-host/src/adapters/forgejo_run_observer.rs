//! Forgejo Actions run adapter for the pool metrics brick.
//!
//! Read-only: GETs the sovereign Forgejo REST API. The runner identity is not
//! exposed by Forgejo 11, so the observation carries the job label and the
//! observed latency; the runner is attributed by the caller from the registry
//! or the runner logs.

use serde::Deserialize;

use crate::domain::pool_metrics::JobObservation;

/// One workflow run as returned by the Forgejo tasks endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct RunRecord {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub head_branch: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub workflow_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub run_started_at: String,
    /// Workflow execution number: the jobs of one gate execution share it.
    #[serde(default)]
    pub run_number: i64,
}

/// Parse an RFC3339 timestamp into seconds since the epoch.
///
/// Returns `None` on an unparsable timestamp: an unknown latency stays
/// unknown rather than becoming a zero.
pub fn parse_rfc3339(value: &str) -> Option<i64> {
    if value.trim().is_empty() {
        return None;
    }
    // Minimal RFC3339 parser: YYYY-MM-DDTHH:MM:SS[.frac][Z|+HH:MM]
    let bytes = value.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let year: i64 = value.get(0..4)?.parse().ok()?;
    let month: i64 = value.get(5..7)?.parse().ok()?;
    let day: i64 = value.get(8..10)?.parse().ok()?;
    let hour: i64 = value.get(11..13)?.parse().ok()?;
    let minute: i64 = value.get(14..16)?.parse().ok()?;
    let second: i64 = value.get(17..19)?.parse().ok()?;
    let days = days_from_civil(year, month, day);
    let mut epoch = days * 86_400 + hour * 3_600 + minute * 60 + second;

    // Offset suffix, if any.
    let tail = &value[19..];
    let offset_part = tail
        .trim_start_matches(|c: char| c == '.' || c.is_ascii_digit())
        .trim();
    if let Some(sign) = offset_part.chars().next() {
        if sign == '+' || sign == '-' {
            let digits: String = offset_part[1..]
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect();
            if digits.len() >= 4 {
                let oh: i64 = digits.get(0..2)?.parse().ok()?;
                let om: i64 = digits.get(2..4)?.parse().ok()?;
                let offset = oh * 3_600 + om * 60;
                if sign == '+' {
                    epoch -= offset;
                } else {
                    epoch += offset;
                }
            }
        }
    }
    Some(epoch)
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Derive the job observations from raw Forgejo runs.
///
/// `runner_of` attributes a run to a runner: Forgejo 11 does not expose the
/// executor, so the caller supplies the attribution (registry label, runner
/// logs, or the runner name when known).
pub fn observations_from_runs(
    runs: &[RunRecord],
    label: &str,
    runner_of: &dyn Fn(&RunRecord) -> String,
) -> Vec<JobObservation> {
    runs.iter()
        .map(|run| {
            let started =
                parse_rfc3339(&run.run_started_at).or_else(|| parse_rfc3339(&run.created_at));
            let ended = parse_rfc3339(&run.updated_at);
            let wall = match (started, ended) {
                (Some(start), Some(end)) if end >= start => Some((end - start) as u64),
                _ => None,
            };
            JobObservation {
                run_id: run.id,
                runner: runner_of(run),
                label: label.to_string(),
                status: run.status.clone(),
                head_sha: run.head_sha.clone(),
                execution_id: run.run_number.to_string(),
                job_name: run.name.clone(),
                queue_seconds: None,
                run_seconds: wall,
                wall_seconds: wall,
                started_epoch: started,
                ended_epoch: ended,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_utc_is_parsed() {
        assert_eq!(parse_rfc3339("2026-10-20T10:07:45Z"), Some(1_792_490_865));
    }

    #[test]
    fn rfc3339_offset_is_applied() {
        let utc = parse_rfc3339("2026-10-20T10:07:45Z").unwrap();
        let offset = parse_rfc3339("2026-10-20T12:07:45+02:00").unwrap();
        assert_eq!(utc, offset);
    }

    #[test]
    fn empty_or_broken_timestamps_stay_unknown() {
        assert_eq!(parse_rfc3339(""), None);
        assert_eq!(parse_rfc3339("not-a-date"), None);
    }

    #[test]
    fn wall_seconds_come_from_started_and_updated() {
        let runs = vec![RunRecord {
            id: 7,
            name: "Gate 3 Core Types".to_string(),
            head_branch: "#1".to_string(),
            head_sha: "abc".to_string(),
            event: "pull_request".to_string(),
            status: "success".to_string(),
            workflow_id: "fgate.yml".to_string(),
            created_at: "2026-10-20T10:00:00Z".to_string(),
            updated_at: "2026-10-20T10:02:30Z".to_string(),
            run_started_at: "2026-10-20T10:00:00Z".to_string(),
            run_number: 1048,
        }];
        let observations = observations_from_runs(&runs, "cynic/build", &|_| "builder".to_string());
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].wall_seconds, Some(150));
        assert_eq!(observations[0].runner, "builder");
        assert_eq!(observations[0].label, "cynic/build");
        assert_eq!(observations[0].execution_id, "1048");
        assert_eq!(observations[0].head_sha, "abc");
        assert_eq!(observations[0].job_name, "Gate 3 Core Types");
    }

    #[test]
    fn a_run_without_timestamps_yields_no_latency() {
        let runs = vec![RunRecord {
            id: 8,
            name: String::new(),
            head_branch: String::new(),
            head_sha: String::new(),
            event: String::new(),
            status: "running".to_string(),
            workflow_id: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            run_started_at: String::new(),
            run_number: 1049,
        }];
        let observations = observations_from_runs(&runs, "cynic/build", &|_| "builder".to_string());
        assert_eq!(observations[0].wall_seconds, None);
    }
}
