use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckoutEntry {
    pub path: String,
    pub sha: String,
    pub branch: String,
    pub age_secs: Option<u64>,
    pub size_bytes: u64,
    pub size_human: String,
    pub owner_uid: u32,
    pub owner_gid: u32,
    pub is_orphan: bool,
    pub is_root_owned: bool,
    pub contains_foreign_owned: bool,
    pub remote_exists: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskPressure {
    pub avail_bytes: u64,
    pub avail_human: String,
    pub use_pct: u8,
    pub is_pressured: bool,
    pub root_owned_target_count: usize,
}
