//! POSIX `df` adapter for read-only filesystem capacity observation.

use std::path::Path;
use std::process::Command;

use crate::ports::filesystem_capacity_port::FilesystemCapacityPort;

#[derive(Debug, Default)]
pub struct DfFilesystemCapacity;

impl FilesystemCapacityPort for DfFilesystemCapacity {
    fn available_bytes(&self, path: &Path) -> Option<u64> {
        let output = Command::new("df")
            .args(["-Pk", path.to_str()?])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8(output.stdout).ok()?;
        let line = text.lines().filter(|line| !line.trim().is_empty()).last()?;
        let available_blocks = line.split_whitespace().nth(3)?.parse::<u64>().ok()?;
        available_blocks.checked_mul(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observes_existing_temp_directory_capacity() {
        assert!(
            DfFilesystemCapacity
                .available_bytes(std::env::temp_dir().as_path())
                .is_some()
        );
    }
}
