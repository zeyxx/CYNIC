use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--status") {
        let is_root = unsafe { libc::geteuid() } == 0;
        let pct_bin =
            std::env::var("CYNIC_PCT_BIN").unwrap_or_else(|_| "/usr/sbin/pct".to_string());
        let pct_available = Path::new(&pct_bin).is_file();
        let authority_config = std::env::var_os("CYNIC_BREAKGLASS_AUTHORITY_CONFIG")
            .unwrap_or_else(|| "/etc/cynic/lxc-breakglass-authority.toml".into());
        let authority_config_path = Path::new(&authority_config);
        let authority_config_present = authority_config_path.is_file();

        let mut blockers = Vec::new();
        if !is_root {
            blockers.push("euid_not_root".to_string());
        }
        if !pct_available {
            blockers.push(format!("pct_binary_missing:{pct_bin}"));
        }

        let status = if blockers.is_empty() {
            "active"
        } else {
            "blocked"
        };
        let payload = serde_json::json!({
            "schema": "cynic.lxc_exec_bridge.helper_status.v1",
            "status": status,
            "runtime": "rust_native",
            "is_root": is_root,
            "pct_bin": pct_bin,
            "pct_available": pct_available,
            "authority_config_path": authority_config.to_string_lossy(),
            "authority_config_present": authority_config_present,
            "shell_execution": false,
            "secret_exposed": false,
            "blockers": blockers
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
        return if blockers.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    if unsafe { libc::geteuid() } != 0 {
        eprintln!("cynic-lxc-exec-bridge requires its admitted root boundary");
        return ExitCode::FAILURE;
    }
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("request read failed");
        return ExitCode::FAILURE;
    }
    let request =
        match serde_json::from_str::<cynic_lxc::LxcBridgeRequest>(&input) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("request parse failed: {error}");
                return ExitCode::FAILURE;
            }
        };
    let pct_bin = std::env::var("CYNIC_PCT_BIN").unwrap_or_else(|_| "/usr/sbin/pct".to_string());
    let authority_config = std::env::var_os("CYNIC_BREAKGLASS_AUTHORITY_CONFIG")
        .unwrap_or_else(|| "/etc/cynic/lxc-breakglass-authority.toml".into());
    let authority_config_path = Path::new(&authority_config);
    let report = match cynic_lxc::LocalIncidentAuthority::from_config_file(
        authority_config_path,
    ) {
        Ok(authority) => cynic_lxc::execute_with_incident_authority(
            &request, &pct_bin, &authority,
        ),
        Err(error) => {
            if authority_config_path.exists() {
                eprintln!(
                    "warning: failed to parse incident authority config at {}: {:?}",
                    authority_config_path.display(),
                    error
                );
            }
            cynic_lxc::execute(&request, &pct_bin)
        }
    };
    let success = report.status == "completed";
    println!("{}", serde_json::to_string(&report).unwrap_or_default());
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
