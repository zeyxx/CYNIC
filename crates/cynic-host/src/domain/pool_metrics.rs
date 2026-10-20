//! Pool job cost, latency, and execution inefficiency measurement.
//!
//! Pure domain: it takes observed job records and derives the statistics the
//! pool needs to reason about placement — how long a job waits, how long it
//! runs, how often it fails, what each runner actually costs, and where the
//! execution effort is wasted.
//!
//! No I/O: the adapter collects the observations, this module decides.

use serde::{Deserialize, Serialize};

/// One observed job execution on a pool runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct JobObservation {
    /// Workflow run id (Forgejo task id).
    pub run_id: i64,
    /// Runner that executed the job.
    pub runner: String,
    /// Job label the runner was matched on (e.g. `cynic/build`).
    pub label: String,
    /// Terminal state: `success`, `failure`, `cancelled`.
    pub status: String,
    /// Commit the job executed.
    #[serde(default)]
    pub head_sha: String,
    /// Execution identifier: the workflow execution the job belongs to.
    ///
    /// Forgejo exposes one run per job; the jobs of a single gate execution
    /// share this id. It is the unit of concurrency, not the commit.
    #[serde(default)]
    pub execution_id: String,
    /// Job name inside the workflow (e.g. `Gate 3 Core Types`).
    #[serde(default)]
    pub job_name: String,
    /// Seconds between the job being queued and the runner picking it up.
    #[serde(default)]
    pub queue_seconds: Option<u64>,
    /// Seconds the job actually ran.
    #[serde(default)]
    pub run_seconds: Option<u64>,
    /// Wall-clock seconds of the whole run (queue + execution).
    #[serde(default)]
    pub wall_seconds: Option<u64>,
    /// Epoch seconds when the job started (unknown stays unknown).
    #[serde(default)]
    pub started_epoch: Option<i64>,
    /// Epoch seconds when the job finished (unknown stays unknown).
    #[serde(default)]
    pub ended_epoch: Option<i64>,
}

/// Aggregated cost and latency of one runner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct RunnerCost {
    pub runner: String,
    pub label: String,
    /// Jobs observed for this runner.
    pub jobs: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
    /// Failure rate over the observed jobs (0.0 when no job ran).
    pub failure_rate: f64,
    /// Mean wall-clock seconds, when measured.
    pub mean_wall_seconds: Option<f64>,
    /// Median wall-clock seconds, when measured.
    pub median_wall_seconds: Option<f64>,
    /// P95 wall-clock seconds, when measured.
    pub p95_wall_seconds: Option<f64>,
    /// Mean queue seconds (wait before a runner picked the job up).
    pub mean_queue_seconds: Option<f64>,
    /// Mean execution seconds.
    pub mean_run_seconds: Option<f64>,
    /// Longest observed wall-clock seconds.
    pub max_wall_seconds: Option<u64>,
    /// Total wall seconds burned by this runner.
    pub total_wall_seconds: u64,
}

/// Aggregated cost and latency of one pool label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LabelCost {
    pub label: String,
    pub jobs: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub failure_rate: f64,
    pub mean_wall_seconds: Option<f64>,
    pub median_wall_seconds: Option<f64>,
    pub p95_wall_seconds: Option<f64>,
    pub mean_queue_seconds: Option<f64>,
    /// Runners that served this label.
    pub runners: Vec<String>,
}

/// One execution group: every job the pool ran for one commit on one label.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionGroup {
    /// Execution the group belongs to (jobs sharing it ran concurrently).
    pub execution_id: String,
    /// Commit the group executed.
    pub head_sha: String,
    pub label: String,
    /// Jobs in the group.
    pub jobs: usize,
    /// Distinct job names in the group.
    pub unique_jobs: usize,
    /// Jobs that re-ran a job name already seen in this group.
    pub redundant_jobs: usize,
    /// Wall seconds burned by the redundant jobs.
    pub redundant_wall_seconds: u64,
    /// Sum of the wall seconds of every job in the group.
    pub total_wall_seconds: u64,
    /// Wall-clock span between the first start and the last finish.
    pub span_seconds: Option<u64>,
    /// Parallel speedup: total work divided by the span it took.
    ///
    /// 1.0 means the group was strictly serial; 7.0 means seven jobs ran
    /// concurrently over the same span.
    pub parallel_speedup: Option<f64>,
    /// Wall seconds burned by failed jobs (work that must be redone).
    pub failed_wall_seconds: u64,
}

/// Where the pool execution effort is wasted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct InefficiencyReport {
    /// Schema identifier.
    pub schema: &'static str,
    /// Execution groups observed (one per commit and label).
    pub groups: usize,
    /// Jobs that re-ran an already-executed job name within their group.
    pub redundant_jobs: usize,
    /// Wall seconds burned by redundant jobs.
    pub redundant_wall_seconds: u64,
    /// Wall seconds burned by failed jobs (rework).
    pub failed_wall_seconds: u64,
    /// Total wall seconds observed.
    pub total_wall_seconds: u64,
    /// Share of the total wall time wasted on redundancy and rework.
    pub wasted_ratio: f64,
    /// Mean parallel speedup across the measured groups.
    pub mean_parallel_speedup: Option<f64>,
    /// Groups that ran strictly serially (speedup at most 1.5).
    pub serial_groups: usize,
    /// The groups that wasted the most wall seconds.
    pub worst_groups: Vec<ExecutionGroup>,
}

/// The full pool measurement report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PoolMetrics {
    pub schema: &'static str,
    /// Total observed jobs.
    pub jobs: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cancelled: usize,
    /// Failure rate over the observed jobs.
    pub failure_rate: f64,
    /// Mean wall-clock seconds across every observed job.
    pub mean_wall_seconds: Option<f64>,
    pub median_wall_seconds: Option<f64>,
    pub p95_wall_seconds: Option<f64>,
    /// Mean queue seconds across every observed job.
    pub mean_queue_seconds: Option<f64>,
    /// Per-runner breakdown.
    pub runners: Vec<RunnerCost>,
    /// Per-label breakdown.
    pub labels: Vec<LabelCost>,
    /// Execution inefficiency breakdown.
    pub inefficiencies: InefficiencyReport,
}

/// Schema identifier of the pool metrics report.
pub const POOL_METRICS_SCHEMA: &str = "cynic.pool.metrics.v1";
/// Schema identifier of the inefficiency report.
pub const POOL_INEFFICIENCY_SCHEMA: &str = "cynic.pool.inefficiency.v1";

fn mean(values: &[u64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: u128 = values.iter().map(|v| *v as u128).sum();
    Some(sum as f64 / values.len() as f64)
}

fn median(sorted: &[u64]) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        let low = sorted[mid - 1] as f64;
        let high = sorted[mid] as f64;
        Some((low + high) / 2.0)
    } else {
        Some(sorted[mid] as f64)
    }
}

/// Percentile over a sorted slice (nearest-rank).
fn percentile(sorted: &[u64], pct: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = (pct / 100.0 * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[index] as f64)
}

fn wall_values(observations: &[JobObservation]) -> Vec<u64> {
    observations.iter().filter_map(|o| o.wall_seconds).collect()
}

fn queue_values(observations: &[JobObservation]) -> Vec<u64> {
    observations
        .iter()
        .filter_map(|o| o.queue_seconds)
        .collect()
}

fn run_values(observations: &[JobObservation]) -> Vec<u64> {
    observations.iter().filter_map(|o| o.run_seconds).collect()
}

fn tally(observations: &[JobObservation]) -> (usize, usize, usize) {
    let succeeded = observations
        .iter()
        .filter(|o| o.status.eq_ignore_ascii_case("success"))
        .count();
    let failed = observations
        .iter()
        .filter(|o| o.status.eq_ignore_ascii_case("failure"))
        .count();
    let cancelled = observations
        .iter()
        .filter(|o| o.status.eq_ignore_ascii_case("cancelled"))
        .count();
    (succeeded, failed, cancelled)
}

fn failure_rate(jobs: usize, failed: usize) -> f64 {
    if jobs == 0 {
        0.0
    } else {
        failed as f64 / jobs as f64
    }
}

/// Group the observations by execution id: one group per gate execution.
///
/// The execution id is the unit of concurrency: the jobs of a single gate
/// execution share it. Grouping by commit instead would merge every
/// re-execution of the same commit into one group and hide the real
/// parallelism.
fn execution_groups(observations: &[JobObservation]) -> Vec<ExecutionGroup> {
    let mut keys: Vec<(String, String, String)> = Vec::new();
    for observation in observations {
        let key = (
            observation.execution_id.clone(),
            observation.head_sha.clone(),
            observation.label.clone(),
        );
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.sort();

    let mut groups: Vec<ExecutionGroup> = Vec::new();
    for (execution_id, head_sha, label) in keys {
        let subset: Vec<&JobObservation> = observations
            .iter()
            .filter(|o| o.execution_id == execution_id && o.label == label)
            .collect();
        if subset.is_empty() {
            continue;
        }
        let mut names: Vec<String> = Vec::new();
        let mut redundant_jobs = 0usize;
        let mut redundant_wall_seconds = 0u64;
        let mut total_wall_seconds = 0u64;
        let mut failed_wall_seconds = 0u64;
        let mut starts: Vec<i64> = Vec::new();
        let mut ends: Vec<i64> = Vec::new();
        for observation in &subset {
            let wall = observation.wall_seconds.unwrap_or(0);
            total_wall_seconds += wall;
            if observation.status.eq_ignore_ascii_case("failure") {
                failed_wall_seconds += wall;
            }
            if names.contains(&observation.job_name) {
                redundant_jobs += 1;
                redundant_wall_seconds += wall;
            } else {
                names.push(observation.job_name.clone());
            }
            if let Some(start) = observation.started_epoch {
                starts.push(start);
            }
            if let Some(end) = observation.ended_epoch {
                ends.push(end);
            }
        }
        let span_seconds = match (starts.iter().min(), ends.iter().max()) {
            (Some(first), Some(last)) if last >= first => Some((last - first) as u64),
            _ => None,
        };
        let parallel_speedup = span_seconds
            .filter(|span| *span > 0)
            .map(|span| total_wall_seconds as f64 / span as f64);
        groups.push(ExecutionGroup {
            execution_id,
            head_sha,
            label,
            jobs: subset.len(),
            unique_jobs: names.len(),
            redundant_jobs,
            redundant_wall_seconds,
            total_wall_seconds,
            span_seconds,
            parallel_speedup,
            failed_wall_seconds,
        });
    }
    groups
}

/// Derive the execution inefficiency report.
pub fn measure_inefficiencies(observations: &[JobObservation]) -> InefficiencyReport {
    let groups = execution_groups(observations);
    let redundant_jobs: usize = groups.iter().map(|g| g.redundant_jobs).sum();
    let redundant_wall_seconds: u64 = groups.iter().map(|g| g.redundant_wall_seconds).sum();
    let failed_wall_seconds: u64 = groups.iter().map(|g| g.failed_wall_seconds).sum();
    let total_wall_seconds: u64 = groups.iter().map(|g| g.total_wall_seconds).sum();
    let wasted = redundant_wall_seconds.saturating_add(failed_wall_seconds);

    let speedups: Vec<u64> = groups
        .iter()
        .filter_map(|g| g.parallel_speedup.map(|s| (s * 100.0) as u64))
        .collect();
    let mean_speedup = if speedups.is_empty() {
        None
    } else {
        let sum: u128 = speedups.iter().map(|v| *v as u128).sum();
        Some(sum as f64 / speedups.len() as f64 / 100.0)
    };

    let serial_groups = groups
        .iter()
        .filter(|g| g.parallel_speedup.map(|s| s <= 1.5).unwrap_or(false))
        .count();

    let mut worst_groups = groups.clone();
    worst_groups.sort_by(|a, b| {
        b.redundant_wall_seconds
            .saturating_add(b.failed_wall_seconds)
            .cmp(
                &a.redundant_wall_seconds
                    .saturating_add(a.failed_wall_seconds),
            )
    });
    worst_groups.truncate(5);

    InefficiencyReport {
        schema: POOL_INEFFICIENCY_SCHEMA,
        groups: groups.len(),
        redundant_jobs,
        redundant_wall_seconds,
        failed_wall_seconds,
        total_wall_seconds,
        wasted_ratio: if total_wall_seconds == 0 {
            0.0
        } else {
            wasted as f64 / total_wall_seconds as f64
        },
        mean_parallel_speedup: mean_speedup,
        serial_groups,
        worst_groups,
    }
}

/// Derive the pool cost and latency report from observed jobs.
pub fn measure_pool(observations: &[JobObservation]) -> PoolMetrics {
    let (succeeded, failed, cancelled) = tally(observations);
    let mut wall = wall_values(observations);
    wall.sort_unstable();
    let mut queue = queue_values(observations);
    queue.sort_unstable();

    let mut runners: Vec<RunnerCost> = Vec::new();
    let mut runner_names: Vec<String> = Vec::new();
    for observation in observations {
        if !runner_names.contains(&observation.runner) {
            runner_names.push(observation.runner.clone());
        }
    }
    runner_names.sort();
    for runner in runner_names {
        let subset: Vec<JobObservation> = observations
            .iter()
            .filter(|o| o.runner == runner)
            .cloned()
            .collect();
        let (ok, ko, cancelled) = tally(&subset);
        let mut wall = wall_values(&subset);
        wall.sort_unstable();
        let mut queue = queue_values(&subset);
        queue.sort_unstable();
        let label = subset
            .first()
            .map(|o| o.label.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let total_wall_seconds: u64 = subset.iter().filter_map(|o| o.wall_seconds).sum();
        runners.push(RunnerCost {
            runner: runner.clone(),
            label,
            jobs: subset.len(),
            succeeded: ok,
            failed: ko,
            cancelled,
            failure_rate: failure_rate(subset.len(), ko),
            mean_wall_seconds: mean(&wall),
            median_wall_seconds: median(&wall),
            p95_wall_seconds: percentile(&wall, 95.0),
            mean_queue_seconds: mean(&queue),
            mean_run_seconds: mean(&run_values(&subset)),
            max_wall_seconds: wall.last().copied(),
            total_wall_seconds,
        });
    }

    let mut labels: Vec<LabelCost> = Vec::new();
    let mut label_names: Vec<String> = Vec::new();
    for observation in observations {
        if !label_names.contains(&observation.label) {
            label_names.push(observation.label.clone());
        }
    }
    label_names.sort();
    for label in label_names {
        let subset: Vec<JobObservation> = observations
            .iter()
            .filter(|o| o.label == label)
            .cloned()
            .collect();
        let (ok, ko, cancelled) = tally(&subset);
        let mut wall = wall_values(&subset);
        wall.sort_unstable();
        let mut queue = queue_values(&subset);
        queue.sort_unstable();
        let mut serving: Vec<String> = subset
            .iter()
            .map(|o| o.runner.clone())
            .collect::<std::collections::BTreeSet<String>>()
            .into_iter()
            .collect();
        serving.sort();
        labels.push(LabelCost {
            label: label.clone(),
            jobs: subset.len(),
            succeeded: ok,
            failed: ko,
            cancelled,
            failure_rate: failure_rate(subset.len(), ko),
            mean_wall_seconds: mean(&wall),
            median_wall_seconds: median(&wall),
            p95_wall_seconds: percentile(&wall, 95.0),
            mean_queue_seconds: mean(&queue),
            runners: serving,
        });
    }

    PoolMetrics {
        schema: POOL_METRICS_SCHEMA,
        jobs: observations.len(),
        succeeded,
        failed,
        cancelled,
        failure_rate: failure_rate(observations.len(), failed),
        mean_wall_seconds: mean(&wall),
        median_wall_seconds: median(&wall),
        p95_wall_seconds: percentile(&wall, 95.0),
        mean_queue_seconds: mean(&queue),
        runners,
        labels,
        inefficiencies: measure_inefficiencies(observations),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(runner: &str, status: &str, wall: u64, queue: u64) -> JobObservation {
        JobObservation {
            run_id: 1,
            runner: runner.to_string(),
            label: "cynic/build".to_string(),
            status: status.to_string(),
            head_sha: "sha1".to_string(),
            execution_id: "exec1".to_string(),
            job_name: "Gate 3 Core Types".to_string(),
            queue_seconds: Some(queue),
            run_seconds: Some(wall.saturating_sub(queue)),
            wall_seconds: Some(wall),
            started_epoch: Some(1_000),
            ended_epoch: Some(1_000 + wall as i64),
        }
    }

    fn job_at(
        runner: &str,
        status: &str,
        name: &str,
        sha: &str,
        start: i64,
        wall: u64,
    ) -> JobObservation {
        job_at_exec(runner, status, name, sha, "exec1", start, wall)
    }

    fn job_at_exec(
        runner: &str,
        status: &str,
        name: &str,
        sha: &str,
        execution: &str,
        start: i64,
        wall: u64,
    ) -> JobObservation {
        JobObservation {
            run_id: 1,
            runner: runner.to_string(),
            label: "cynic/build".to_string(),
            status: status.to_string(),
            head_sha: sha.to_string(),
            execution_id: execution.to_string(),
            job_name: name.to_string(),
            queue_seconds: Some(0),
            run_seconds: Some(wall),
            wall_seconds: Some(wall),
            started_epoch: Some(start),
            ended_epoch: Some(start + wall as i64),
        }
    }

    #[test]
    fn empty_observations_produce_a_zeroed_report() {
        let report = measure_pool(&[]);
        assert_eq!(report.jobs, 0);
        assert_eq!(report.failure_rate, 0.0);
        assert!(report.mean_wall_seconds.is_none());
        assert!(report.runners.is_empty());
        assert_eq!(report.inefficiencies.groups, 0);
        assert_eq!(report.inefficiencies.wasted_ratio, 0.0);
    }

    #[test]
    fn failure_rate_counts_failures_over_jobs() {
        let report = measure_pool(&[
            job("a", "success", 100, 10),
            job("a", "failure", 200, 20),
            job("b", "success", 300, 30),
        ]);
        assert_eq!(report.jobs, 3);
        assert_eq!(report.failed, 1);
        assert!((report.failure_rate - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn per_runner_cost_splits_the_observations() {
        let report = measure_pool(&[
            job("builder", "success", 100, 10),
            job("builder", "success", 200, 20),
            job("compute", "failure", 300, 30),
        ]);
        let builder = report
            .runners
            .iter()
            .find(|r| r.runner == "builder")
            .expect("builder row");
        assert_eq!(builder.jobs, 2);
        assert_eq!(builder.succeeded, 2);
        assert_eq!(builder.failure_rate, 0.0);
        assert!((builder.mean_wall_seconds.unwrap() - 150.0).abs() < 1e-9);
        assert_eq!(builder.total_wall_seconds, 300);
    }

    #[test]
    fn queue_latency_is_reported_per_runner() {
        let report = measure_pool(&[job("builder", "success", 100, 40)]);
        let builder = &report.runners[0];
        assert!((builder.mean_queue_seconds.unwrap() - 40.0).abs() < 1e-9);
        assert!((builder.mean_run_seconds.unwrap() - 60.0).abs() < 1e-9);
    }

    #[test]
    fn median_and_p95_are_computed_over_wall_seconds() {
        let report = measure_pool(&[
            job("a", "success", 10, 0),
            job("a", "success", 20, 0),
            job("a", "success", 30, 0),
            job("a", "success", 40, 0),
        ]);
        assert!((report.median_wall_seconds.unwrap() - 25.0).abs() < 1e-9);
        assert_eq!(report.p95_wall_seconds.unwrap(), 40.0);
    }

    #[test]
    fn labels_list_the_runners_that_served_them() {
        let report = measure_pool(&[
            job("builder", "success", 10, 0),
            job("compute", "success", 20, 0),
            job("compute", "failure", 30, 0),
        ]);
        let label = &report.labels[0];
        assert_eq!(label.label, "cynic/build");
        assert_eq!(label.runners, vec!["builder", "compute"]);
        assert_eq!(label.jobs, 3);
        assert!((label.failure_rate - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn a_repeated_job_name_counts_as_redundant() {
        let report = measure_pool(&[
            job_at("builder", "success", "Gate 3 Core Types", "sha1", 0, 100),
            job_at("builder", "success", "Gate 3 Core Types", "sha1", 200, 100),
            job_at("builder", "success", "Gate 3 Knowledge", "sha1", 200, 100),
        ]);
        let inefficiencies = &report.inefficiencies;
        assert_eq!(inefficiencies.groups, 1);
        assert_eq!(inefficiencies.redundant_jobs, 1);
        assert_eq!(inefficiencies.redundant_wall_seconds, 100);
        assert!(inefficiencies.wasted_ratio > 0.0);
    }

    #[test]
    fn failed_jobs_count_as_rework() {
        let report = measure_pool(&[job_at(
            "builder",
            "failure",
            "Gate 2 Workspace Compilation",
            "sha1",
            0,
            200,
        )]);
        assert_eq!(report.inefficiencies.failed_wall_seconds, 200);
        assert!((report.inefficiencies.wasted_ratio - 1.0).abs() < 1e-9);
    }

    #[test]
    fn concurrent_jobs_report_a_parallel_speedup() {
        let report = measure_pool(&[
            job_at("builder", "success", "Gate 3 Core Types", "sha1", 0, 100),
            job_at("compute", "success", "Gate 3 Knowledge", "sha1", 0, 100),
        ]);
        let group = &report.inefficiencies.worst_groups[0];
        assert_eq!(group.jobs, 2);
        assert_eq!(group.unique_jobs, 2);
        assert_eq!(group.redundant_jobs, 0);
        assert_eq!(group.span_seconds, Some(100));
        let speedup = group.parallel_speedup.expect("speedup");
        assert!((speedup - 2.0).abs() < 1e-9);
        assert!((report.inefficiencies.mean_parallel_speedup.unwrap() - 2.0).abs() < 1e-9);
        assert_eq!(report.inefficiencies.serial_groups, 0);
    }

    #[test]
    fn serial_jobs_are_reported_as_serial() {
        let report = measure_pool(&[
            job_at("builder", "success", "Gate 3 Core Types", "sha1", 0, 100),
            job_at("builder", "success", "Gate 3 Knowledge", "sha1", 100, 100),
        ]);
        assert_eq!(report.inefficiencies.serial_groups, 1);
        let speedup = report.inefficiencies.worst_groups[0]
            .parallel_speedup
            .expect("speedup");
        assert!((speedup - 1.0).abs() < 1e-9);
    }

    #[test]
    fn different_commits_are_different_groups() {
        let report = measure_pool(&[
            job_at("builder", "success", "Gate 3 Core Types", "sha1", 0, 100),
            job_at("builder", "success", "Gate 3 Core Types", "sha2", 0, 100),
        ]);
        assert_eq!(report.inefficiencies.groups, 2);
        assert_eq!(report.inefficiencies.redundant_jobs, 0);
    }

    #[test]
    fn the_same_commit_rerun_is_a_distinct_group() {
        // Two executions of the same commit are two groups, not one merged
        // group: the second execution must not look like a redundant job.
        let report = measure_pool(&[
            job_at_exec(
                "builder",
                "success",
                "Gate 3 Core Types",
                "sha1",
                "exec1",
                0,
                100,
            ),
            job_at_exec(
                "builder",
                "success",
                "Gate 3 Core Types",
                "sha1",
                "exec2",
                0,
                100,
            ),
        ]);
        assert_eq!(report.inefficiencies.groups, 2);
        assert_eq!(report.inefficiencies.redundant_jobs, 0);
    }

    #[test]
    fn a_job_repeated_inside_one_execution_is_redundant() {
        let report = measure_pool(&[
            job_at_exec(
                "builder",
                "success",
                "Gate 3 Core Types",
                "sha1",
                "exec1",
                0,
                100,
            ),
            job_at_exec(
                "builder",
                "success",
                "Gate 3 Core Types",
                "sha1",
                "exec1",
                200,
                100,
            ),
        ]);
        assert_eq!(report.inefficiencies.groups, 1);
        assert_eq!(report.inefficiencies.redundant_jobs, 1);
        assert_eq!(report.inefficiencies.redundant_wall_seconds, 100);
    }
}
