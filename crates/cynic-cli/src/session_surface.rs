use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::tmux_session_supervision::{
    LocalTmuxSessionSupervisor, TmuxSessionSupervisionPort,
};

use crate::emit_json;

#[derive(Subcommand)]
pub(crate) enum SessionCommand {
    /// Observe tmux sessions with per-pane metrics and activity signals
    Observe {
        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_session(command: &SessionCommand) -> ExitCode {
    match command {
        SessionCommand::Observe { json } => {
            let supervisor = LocalTmuxSessionSupervisor::new();
            let report = supervisor.observe_sessions();
            if *json {
                emit_json(&report, false)
            } else {
                println!("{report}");
                if report.detail.is_some() && report.session_count == 0 {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
    }
}
