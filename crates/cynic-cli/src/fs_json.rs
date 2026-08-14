use std::path::Path;

use serde::Serialize;

pub(crate) fn write_pretty_json_file(
    path: &Path,
    value: &impl Serialize,
    subject: &str,
) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "{subject} output path has no parent: {}",
            path.display()
        ));
    };

    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create {subject} directory {}: {error}",
            parent.display()
        )
    })?;

    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to serialize {subject}: {error}"))?;

    std::fs::write(path, payload)
        .map_err(|error| format!("failed to write {subject} {}: {error}", path.display()))
}
