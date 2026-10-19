//! Hub provider family: Hugging Face Hub operations that are NOT model supply.
//!
//! Domain proof: `cynic-huggingface` owns the contract and the `hf` CLI
//! adapter. This surface consumes it directly — never through
//! `cynic-inference` (see `docs/design/inference-universe.md` §3 rule 1).
//! Model download/discovery stay under `cynic inference hf ...`.

use std::process::ExitCode;

use clap::Subcommand;

use crate::runtime_surface::hub_surface as exec;

#[derive(Subcommand)]
pub(crate) enum HubCommand {
    /// Publish a file or folder to the Hub (`hf upload`).
    Upload {
        /// Hub repo id, e.g. `org/dataset`.
        repo: String,
        /// Local file or folder to publish.
        #[arg(long)]
        path: String,
        /// Destination path inside the repo.
        #[arg(long = "path-in-repo")]
        path_in_repo: Option<String>,
        /// Repo type: model|dataset|space (default model, like `hf`).
        #[arg(long = "type", default_value = "model")]
        repo_type: String,
        /// Target revision (branch, tag, or commit).
        #[arg(long)]
        revision: Option<String>,
        /// Create a private repo when missing.
        #[arg(long)]
        private: bool,
        /// Create a public repo when missing.
        #[arg(long = "no-private")]
        no_private: bool,
        /// Glob include (repeatable).
        #[arg(long = "include")]
        include: Vec<String>,
        /// Glob exclude (repeatable).
        #[arg(long = "exclude")]
        exclude: Vec<String>,
        /// Commit message title.
        #[arg(long = "commit-message")]
        commit_message: Option<String>,
        /// Upload as a pull request.
        #[arg(long = "create-pr")]
        create_pr: bool,
        /// Explicit mutation admission.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
    /// One-shot Jobs on HF infrastructure (`hf jobs ...`).
    Job {
        #[command(subcommand)]
        command: HubJobCommand,
    },
    /// Scheduled Jobs on HF infrastructure (`hf jobs scheduled ...`).
    Sched {
        #[command(subcommand)]
        command: HubSchedCommand,
    },
}

#[derive(Subcommand)]
pub(crate) enum HubJobCommand {
    /// Submit a UV script or Docker command (`hf jobs uv run` / `hf jobs run`).
    Submit {
        /// UV script: local file or URL (exactly one of --script/--image).
        #[arg(long)]
        script: Option<String>,
        /// Docker image (exactly one of --script/--image).
        #[arg(long)]
        image: Option<String>,
        /// Script args or container command (repeatable, positional tail).
        #[arg(last = true)]
        args: Vec<String>,
        /// Job name (shows in the Hub UI).
        #[arg(long)]
        name: Option<String>,
        /// Hardware flavor, e.g. `cpu-basic`, `a10g-small`.
        #[arg(long)]
        flavor: Option<String>,
        /// Non-secret env entry K=V (repeatable).
        #[arg(long = "env")]
        env: Vec<String>,
        /// Secret NAME resolved from the process environment (repeatable;
        /// values never transit through argv or reports).
        #[arg(long = "secret")]
        secret: Vec<String>,
        /// Volume mount `hf://...` (repeatable).
        #[arg(long = "volume")]
        volume: Vec<String>,
        /// Max duration, e.g. `30m`, `2h`.
        #[arg(long)]
        timeout: Option<String>,
        /// Label K=V or bare label (repeatable).
        #[arg(long = "label")]
        label: Vec<String>,
        /// Namespace the job runs in.
        #[arg(long)]
        namespace: Option<String>,
        /// Explicit mutation admission.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
    /// Inspect one job (`hf jobs inspect ID --json`).
    Poll {
        /// Remote job id (or `namespace/id`).
        id: String,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// List jobs (`hf jobs ps --json`).
    List {
        /// Show all jobs, not just running.
        #[arg(long)]
        all: bool,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Fetch job logs (`hf jobs logs ID`).
    Logs {
        /// Remote job id.
        id: String,
        #[arg(long)]
        namespace: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Cancel a job (`hf jobs cancel ID`).
    Cancel {
        /// Remote job id.
        id: String,
        #[arg(long)]
        namespace: Option<String>,
        /// Explicit mutation admission.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum HubSchedCommand {
    /// Schedule a UV script or Docker command with a CRON / @alias schedule.
    Submit {
        /// `@hourly`, `@daily`, or CRON expression.
        #[arg(long)]
        schedule: String,
        /// UV script: local file or URL (exactly one of --script/--image).
        #[arg(long)]
        script: Option<String>,
        /// Docker image (exactly one of --script/--image).
        #[arg(long)]
        image: Option<String>,
        /// Script args or container command (repeatable, positional tail).
        #[arg(last = true)]
        args: Vec<String>,
        /// Job name (shows in the Hub UI).
        #[arg(long)]
        name: Option<String>,
        /// Hardware flavor, e.g. `cpu-basic`, `a10g-small`.
        #[arg(long)]
        flavor: Option<String>,
        /// Non-secret env entry K=V (repeatable).
        #[arg(long = "env")]
        env: Vec<String>,
        /// Secret NAME resolved from the process environment (repeatable).
        #[arg(long = "secret")]
        secret: Vec<String>,
        /// Volume mount `hf://...` (repeatable).
        #[arg(long = "volume")]
        volume: Vec<String>,
        /// Max duration, e.g. `30m`, `2h`.
        #[arg(long)]
        timeout: Option<String>,
        /// Label K=V or bare label (repeatable).
        #[arg(long = "label")]
        label: Vec<String>,
        /// Namespace the job runs in.
        #[arg(long)]
        namespace: Option<String>,
        /// Explicit mutation admission.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
    /// List scheduled jobs (`hf jobs scheduled ps --json`).
    List {
        #[arg(long)]
        json: bool,
    },
    /// Delete a scheduled job (`hf jobs scheduled delete ID`).
    Delete {
        /// Scheduled job id.
        id: String,
        /// Explicit mutation admission.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_hub_command(command: HubCommand) -> ExitCode {
    match command {
        HubCommand::Upload {
            repo,
            path,
            path_in_repo,
            repo_type,
            revision,
            private,
            no_private,
            include,
            exclude,
            commit_message,
            create_pr,
            yes,
            json,
        } => exec::execute_hub_upload(&exec::HubUploadArgs {
            repo: &repo,
            path: &path,
            path_in_repo: path_in_repo.as_deref(),
            repo_type: &repo_type,
            revision: revision.as_deref(),
            private,
            no_private,
            include: &include,
            exclude: &exclude,
            commit_message: commit_message.as_deref(),
            create_pr,
            yes,
            json,
        }),
        HubCommand::Job { command } => match command {
            HubJobCommand::Submit {
                script,
                image,
                args,
                name,
                flavor,
                env,
                secret,
                volume,
                timeout,
                label,
                namespace,
                yes,
                json,
            } => exec::execute_hub_job_submit(&exec::HubJobSubmitArgs {
                script: script.as_deref(),
                image: image.as_deref(),
                args: &args,
                name: name.as_deref(),
                flavor: flavor.as_deref(),
                env: &env,
                secret: &secret,
                volume: &volume,
                timeout: timeout.as_deref(),
                label: &label,
                namespace: namespace.as_deref(),
                schedule: None,
                yes,
                json,
            }),
            HubJobCommand::Poll { id, namespace, json } => {
                exec::execute_hub_job_poll(&id, namespace.as_deref(), json)
            }
            HubJobCommand::List { all, namespace, json } => {
                exec::execute_hub_job_list(all, namespace.as_deref(), json)
            }
            HubJobCommand::Logs { id, namespace, json } => {
                exec::execute_hub_job_logs(&id, namespace.as_deref(), json)
            }
            HubJobCommand::Cancel { id, namespace, yes, json } => {
                exec::execute_hub_job_cancel(&id, namespace.as_deref(), yes, json)
            }
        },
        HubCommand::Sched { command } => match command {
            HubSchedCommand::Submit {
                schedule,
                script,
                image,
                args,
                name,
                flavor,
                env,
                secret,
                volume,
                timeout,
                label,
                namespace,
                yes,
                json,
            } => exec::execute_hub_job_submit(&exec::HubJobSubmitArgs {
                script: script.as_deref(),
                image: image.as_deref(),
                args: &args,
                name: name.as_deref(),
                flavor: flavor.as_deref(),
                env: &env,
                secret: &secret,
                volume: &volume,
                timeout: timeout.as_deref(),
                label: &label,
                namespace: namespace.as_deref(),
                schedule: Some(&schedule),
                yes,
                json,
            }),
            HubSchedCommand::List { json } => exec::execute_hub_sched_list(json),
            HubSchedCommand::Delete { id, yes, json } => {
                exec::execute_hub_sched_delete(&id, yes, json)
            }
        },
    }
}
