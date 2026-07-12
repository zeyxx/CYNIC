use std::process::ExitCode;

pub(crate) fn emit_json(
    value: &impl serde::Serialize,
    should_fail: bool,
) -> Result<ExitCode, String> {
    let payload = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    println!("{payload}");
    Ok(if should_fail {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

pub(crate) fn read_stdin_json() -> Result<serde_json::Value, String> {
    use std::io::Read;

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    serde_json::from_str(&input).map_err(|error| format!("failed to parse stdin json: {error}"))
}
