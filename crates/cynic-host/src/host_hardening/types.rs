use cynic_protocol::hardening::*;
pub use cynic_protocol::hardening::HOST_HARDENING_REPORT_SCHEMA_V1;

pub const SSHD_CYNIC_DROP_IN_PATH: &str = "/etc/ssh/sshd_config.d/99-cynic-hardening.conf";
pub const NFTABLES_CYNIC_PATH: &str = "/etc/nftables.conf";
