use std::process::ExitCode;

use serde::Serialize;

#[allow(dead_code)]
pub(crate) fn emit_pretty_json(value: &impl Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => {
            println!("{payload}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("json error: {error}");
            ExitCode::FAILURE
        }
    }
}
