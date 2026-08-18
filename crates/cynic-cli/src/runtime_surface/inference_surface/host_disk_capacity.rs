//! CLI wiring from the canonical host disk observer into inference import.

use std::path::Path;

use cynic_host::adapters::local_disk::LocalDiskAdapter;
use cynic_host::domain::disk::DiskObservationPort;
use cynic_inference::ports::filesystem_capacity_port::FilesystemCapacityPort;

pub(crate) struct HostDiskCapacity;

impl FilesystemCapacityPort for HostDiskCapacity {
    fn available_bytes(&self, path: &Path) -> Option<u64> {
        let report = LocalDiskAdapter::new().observe_disk().ok()?;
        let target = path.to_string_lossy();
        report
            .partitions
            .iter()
            .filter(|partition| path_is_within_mount(&target, &partition.mount_point))
            .max_by_key(|partition| partition.mount_point.len())
            .map(|partition| partition.available_bytes)
    }
}

fn path_is_within_mount(path: &str, mount: &str) -> bool {
    mount == "/"
        || path == mount
        || path
            .strip_prefix(mount)
            .is_some_and(|tail| tail.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_boundary_does_not_match_a_prefix_only() {
        assert!(path_is_within_mount("/data/models", "/data"));
        assert!(!path_is_within_mount("/database", "/data"));
    }
}
