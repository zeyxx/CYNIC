//! Node CLI surfaces - moved from cynic-node (the daemon).
//!
//! cynic-node is a daemon. Its systemd unit runs `cynic-node control`. The
//! other 16 subcommands are CLI surfaces that belong in `cynic`, not in a
//! daemon. They were moved here in the composition-root work.

pub mod cli;
pub mod dispatch;
pub mod network_surfaces_uc;
