use super::assemble;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const O_NOFOLLOW: i32 = 0o400000;
const O_NONBLOCK: i32 = 0o4000;

fn read_json(path: &Path) -> Result<Value, String> {
    let mut options = OpenOptions::new();
    options.read(true).custom_flags(O_NOFOLLOW | O_NONBLOCK);
    let mut file = options
        .open(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

pub(crate) fn canonical_json(value: &Value) -> String {
    fn sort(value: &Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), sort(v)))
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            Value::Array(a) => Value::Array(a.iter().map(sort).collect()),
            other => other.clone(),
        }
    }
    serde_json::to_string_pretty(&sort(value)).expect("JSON serialization") + "\n"
}

pub(crate) fn check(manifest: &Path, output: &Path) -> Result<(), String> {
    let expected = assemble(manifest)?;
    let actual = read_json(output)?;
    if canonical_json(&expected) != canonical_json(&actual) {
        return Err("assembled output does not match manifest fragments".into());
    }
    Ok(())
}

pub(crate) fn write(manifest: &Path, output: &Path) -> Result<(), String> {
    let expected = assemble(manifest)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(output, canonical_json(&expected))
        .map_err(|e| format!("cannot write {}: {e}", output.display()))
}
