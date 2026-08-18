//! Read-only capacity observation for a filesystem target.

use std::path::Path;

/// Portable boundary for free-space observation. `None` is an explicit unknown,
/// never permission to begin a cross-filesystem copy.
pub trait FilesystemCapacityPort: Send + Sync {
    fn available_bytes(&self, path: &Path) -> Option<u64>;
}
