use std::process::ExitCode;

pub(crate) fn success_if_status(status: &str, success: &str, ok_code: ExitCode) -> ExitCode {
    success_if_status_in(status, &[success], ok_code)
}

pub(crate) fn success_if_status_in(status: &str, allowed: &[&str], ok_code: ExitCode) -> ExitCode {
    if allowed.contains(&status) {
        ok_code
    } else {
        ExitCode::FAILURE
    }
}
