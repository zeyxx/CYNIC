//! Admitted window execution — binds WindowRunner (cynic-host) to
//! BuildExecutionPort so a bootstrap-seed build runs inside a budgeted,
//! journaled tick window (plan §9 gap 1 → §10 composition bridge).
//!
//! Layer note: this lives in `cynic-node` because BuildExecutionPort lives
//! here and node already depends on host. Host never sees this file.
//!
//! Flow: admit (queue gate, upstream) → WindowRunner.run opens the ledger
//! reservation → build executes → close record carries measured
//! EnergyConsumed (wall + cpu_ms) and the build read-back as receipt. A
//! build failure still closes the window with status Failed — no dangling
//! reservations.

use std::path::PathBuf;
use std::time::Duration;

use cynic_host::adapters::window_runner::{WindowRunError, WindowRunner};
use cynic_protocol::task_pipeline::EnergyBudget;
use cynic_protocol::temporal::Epoch;

use crate::ports::build_scheduler_port::{BuildExecutionPort, BuildExecutionRequest};

/// One admitted build window: everything needed to open, execute, close.
#[derive(Debug, Clone)]
pub struct AdmittedBuildWindow {
    pub window_id: String,
    pub parent_window_id: Option<String>,
    pub epoch: Epoch,
    pub budget: EnergyBudget,
    pub capacity_ref: String,
    /// Canonical argv from DevelopmentGenesis (protocol-owned derivation).
    pub argv: Vec<String>,
    pub working_directory: PathBuf,
    pub timeout: Duration,
}

/// Measured receipt of one admitted build window.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdmittedBuildReceipt {
    pub window_id: String,
    pub consumed_wall_time_ms: u64,
    pub consumed_cpu_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

/// Execute one admitted build inside its journaled window.
///
/// The meter is created per call — one metering scope per window. Nested
/// windows would re-borrow it through `run`'s closure argument.
///
/// Sync/async bridge: WindowRunner is sync and receives a closure that
/// blocks on the build future via the ambient tokio handle; callers invoke
/// this from within a tokio runtime.
pub async fn run_admitted_build(
    runner: &WindowRunner,
    window: &AdmittedBuildWindow,
    cancellation: &tokio_util::sync::CancellationToken,
    execution: &dyn BuildExecutionPort,
) -> Result<AdmittedBuildReceipt, WindowRunError> {
    let mut meter = cynic_host::adapters::cpu_usage_meter::CpuUsageMeter::new();
    let request = BuildExecutionRequest {
        argv: window.argv.clone(),
        working_directory: window.working_directory.clone(),
        environment: Default::default(),
        timeout: window.timeout,
    };

    let outcome = runner.run(
        &window.window_id,
        window.parent_window_id.as_deref(),
        window.epoch,
        window.budget.clone(),
        &window.capacity_ref,
        &mut meter,
        |_meter| {
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { execution.execute(&request, cancellation).await })
            });
            match result {
                Ok(read_back) => Ok(read_back),
                Err(error) => Err(format!("build_execution_failed:{error:?}")),
            }
        },
    )?;

    Ok(AdmittedBuildReceipt {
        window_id: outcome.window_id.clone(),
        consumed_wall_time_ms: outcome.consumed.wall_time_ms,
        consumed_cpu_ms: outcome.consumed.cpu_ms,
        exit_code: outcome.result.exit_code,
        stdout_tail: truncate(&outcome.result.stdout, 2048),
        stderr_tail: truncate(&outcome.result.stderr, 2048),
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("…{}", &s[s.len() - max..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::build_scheduler_port::{
        BuildExecutionError, BuildExecutionPort, BuildExecutionReadBack, BuildExecutionRequest,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Stub execution port that records it was called and succeeds.
    struct CountingExecution {
        calls: AtomicU32,
    }

    impl BuildExecutionPort for CountingExecution {
        fn execute<'a>(
            &'a self,
            _request: &'a BuildExecutionRequest,
            _cancellation: &'a tokio_util::sync::CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<BuildExecutionReadBack, BuildExecutionError>,
                    > + Send
                    + 'a,
            >,
        > {
            let calls = &self.calls;
            Box::pin(async move {
                calls.fetch_add(1, Ordering::Relaxed);
                Ok(BuildExecutionReadBack {
                    exit_code: Some(0),
                    stdout: "build ok".into(),
                    stderr: String::new(),
                })
            })
        }
    }

    #[tokio::main]
    async fn test_main() {
        // Required so block_in_place/block_on inside run_admitted_build has
        // a runtime.
    }

    fn window() -> AdmittedBuildWindow {
        AdmittedBuildWindow {
            window_id: "w-build".into(),
            parent_window_id: None,
            epoch: cynic_protocol::temporal::Epoch::new(1),
            budget: EnergyBudget {
                wall_time_ms: 60_000,
                inference_tokens: None,
                cpu_ms: None,
                ram_bytes_s: None,
            },
            capacity_ref: "cap-1".into(),
            argv: vec!["true".into()],
            working_directory: std::env::temp_dir(),
            timeout: Duration::from_secs(30),
        }
    }

    fn journal() -> (
        tempfile::NamedTempFile,
        cynic_host::adapters::fs_window_journal::FsWindowJournal,
    ) {
        let temp = tempfile::NamedTempFile::new().unwrap();
        let journal = cynic_host::adapters::fs_window_journal::FsWindowJournal::open(temp.path());
        (temp, journal)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn admitted_build_opens_executes_and_closes_coherently() {
        let (_t, journal) = journal();
        let runner = WindowRunner::new(std::sync::Arc::new(journal.clone()), "test-host");
        let exec = CountingExecution {
            calls: AtomicU32::new(0),
        };

        let receipt = run_admitted_build(
            &runner,
            &window(),
            &tokio_util::sync::CancellationToken::new(),
            &exec,
        )
        .await
        .unwrap();

        assert_eq!(receipt.exit_code, Some(0));
        assert_eq!(exec.calls.load(Ordering::Relaxed), 1);

        // Ledger coherence after the full lifecycle.
        let state = journal.replay_state().unwrap();
        assert!(state.violations.is_empty());
        assert!(state.open.is_empty());
        assert!(cynic_protocol::window_journal::journal_is_coherent(&state));
    }
}
