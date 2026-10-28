mod apply;
mod decision;
mod filesystem;
mod observe;
mod rollback;
mod types;

#[cfg(test)]
mod tests;

pub use apply::{apply_dns_activation, root_authority_blocker};
pub use decision::{activation_argv, plan_dns_activation};
pub use filesystem::{
    Backup, atomic_install, capture_backups, missing_parent_directories, rollback_files_and_directories,
};
pub use observe::observe_dns_activation;
pub use rollback::{rollback_after_activation, rollback_matches};
pub use types::*;
