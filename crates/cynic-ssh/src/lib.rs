//! Pure SSH transport helpers: options, argv construction, validation,
//! and remote command/outcome types.
//!
//! This crate holds no I/O. Validation and argv construction are pure
//! functions; callers decide how the resulting argv is executed.

pub mod argv;
pub mod command;
pub mod executor;
pub mod config;
pub mod inspection;
pub mod options;
pub mod trust;

pub use argv::{
    scp_argv, ssh_argv, ssh_script_argv, validate_remote_script, validate_remote_segment,
    validate_target,
};
pub use command::{RemoteCommand, RemoteExecutorPort, RemoteOutcome, TransportForm};
pub use executor::SshRemoteExecutor;
pub use options::{SshOptions, SSH_BATCH_OPTIONS};


pub use config::{
    SshConfigEntry, SshConfigParse, find_entry, parse_ssh_config, parse_ssh_config_file,
};
pub use inspection::{ExecutableObservation, SshHostKeyObservation, SshTargetInspectionReport};
pub use trust::{
    SSH_TRUST_VERIFICATION_MAX_AGE_MS, SshTrustAdmissionReport, SshTrustAnchor,
    SshTrustInspectionReport, SshTrustStore, SshTrustVerification, admit_ssh_trust_anchor,
    default_ssh_trust_store_path, load_ssh_trust_store, plan_ssh_trust_admission,
    project_trust_verification, verify_ssh_target_trust, verify_ssh_target_trust_at,
};
