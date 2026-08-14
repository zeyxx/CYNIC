use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn path_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

pub fn path_mode_equals(path: impl AsRef<Path>, expected: u32) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o777 == expected)
        .unwrap_or(false)
}

pub fn path_executable(path: impl AsRef<Path>) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o100 != 0)
        .unwrap_or(false)
}
