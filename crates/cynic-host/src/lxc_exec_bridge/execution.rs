use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cynic_protocol::CynicErrorCode;

use super::{
    decision::{admit, admit_after_incident_authority},
    incident_authority::{IncidentAuthorityPort, UnavailableIncidentAuthority},
    types::{
        LxcBridgeAuthority, LxcBridgeDecision, LxcBridgeRequest, LxcBridgeResponse, RESPONSE_SCHEMA,
    },
};

pub fn execute(request: &LxcBridgeRequest, pct_bin: &str) -> LxcBridgeResponse {
    let authority = UnavailableIncidentAuthority;
    execute_with_incident_authority(request, pct_bin, &authority)
}

pub fn execute_with_incident_authority(
    request: &LxcBridgeRequest,
    pct_bin: &str,
    authority: &dyn IncidentAuthorityPort,
) -> LxcBridgeResponse {
    let now_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(u64::MAX);
    let decision = if request.authority == LxcBridgeAuthority::RecoveryBreakglass {
        match request.breakglass_admission.as_ref() {
            Some(admission) => {
                // Reconcile every non-authority condition before consuming a
                // one-shot receipt. The public decision deliberately reports
                // BG-006 here; that is the sole blocker the injected port may
                // discharge.
                if let LxcBridgeDecision::Blocked { mut blockers } = admit(request, pct_bin) {
                    blockers.retain(|blocker| {
                        blocker != CynicErrorCode::BreakglassAuthorityVerifierUnavailable.code()
                    });
                    if !blockers.is_empty() {
                        return response(
                            request,
                            "blocked",
                            false,
                            None,
                            String::new(),
                            String::new(),
                            false,
                            blockers,
                        );
                    }
                }
                let action = canonical_action(&request.argv).unwrap_or_default();
                match authority.verify_and_consume(
                    admission,
                    &format!("lxc:{}", request.vmid),
                    &action,
                    now_unix_s,
                ) {
                    Ok(_) => admit_after_incident_authority(request, pct_bin, now_unix_s),
                    Err(error) => {
                        return response(
                            request,
                            "blocked",
                            false,
                            None,
                            String::new(),
                            String::new(),
                            false,
                            vec![error.code().to_string()],
                        );
                    }
                }
            }
            None => admit(request, pct_bin),
        }
    } else {
        admit(request, pct_bin)
    };
    let (argv, mutates) = match decision {
        LxcBridgeDecision::Observe { pct_argv } => (pct_argv, false),
        LxcBridgeDecision::Apply { pct_argv } => (pct_argv, true),
        LxcBridgeDecision::Blocked { blockers } => {
            return response(
                request,
                "blocked",
                false,
                None,
                String::new(),
                String::new(),
                false,
                blockers,
            );
        }
    };
    let mut child = match Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return response(
                request,
                "adapter_failed",
                mutates,
                None,
                String::new(),
                error.to_string(),
                false,
                vec!["pct_spawn_failed".to_string()],
            );
        }
    };
    let deadline = Instant::now() + Duration::from_secs(request.timeout_seconds);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let output = child.wait_with_output().ok();
                return response(
                    request,
                    "timeout",
                    mutates,
                    output.as_ref().and_then(|value| value.status.code()),
                    output
                        .as_ref()
                        .map(|value| String::from_utf8_lossy(&value.stdout).into_owned())
                        .unwrap_or_default(),
                    output
                        .as_ref()
                        .map(|value| String::from_utf8_lossy(&value.stderr).into_owned())
                        .unwrap_or_default(),
                    true,
                    vec!["post_timeout_guest_state_unknown".to_string()],
                );
            }
            Err(error) => {
                let _ = child.kill();
                return response(
                    request,
                    "adapter_failed",
                    mutates,
                    None,
                    String::new(),
                    error.to_string(),
                    false,
                    vec!["pct_wait_failed".to_string()],
                );
            }
        }
    }
    match child.wait_with_output() {
        Ok(output) => response(
            request,
            if output.status.success() {
                "completed"
            } else {
                "failed"
            },
            mutates,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            false,
            Vec::new(),
        ),
        Err(error) => response(
            request,
            "adapter_failed",
            mutates,
            None,
            String::new(),
            error.to_string(),
            false,
            vec!["pct_output_read_failed".to_string()],
        ),
    }
}

fn canonical_action(argv: &[String]) -> Option<String> {
    match argv {
        [program, operation, unit]
            if program == "/usr/bin/systemctl"
                && matches!(
                    operation.as_str(),
                    "start" | "stop" | "restart" | "enable" | "disable"
                ) =>
        {
            Some(format!("systemctl:{operation}:{unit}"))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn response(
    request: &LxcBridgeRequest,
    status: &str,
    mutates: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    timed_out: bool,
    unknowns: Vec<String>,
) -> LxcBridgeResponse {
    LxcBridgeResponse {
        schema: RESPONSE_SCHEMA,
        status: status.to_string(),
        vmid: request.vmid,
        purpose: request.purpose.clone(),
        authority: request.authority,
        mutates,
        requested_apply: request.apply,
        exit_code,
        stdout,
        stderr,
        timed_out,
        output_secret_classification: "untrusted_operational_output",
        unknowns,
    }
}
