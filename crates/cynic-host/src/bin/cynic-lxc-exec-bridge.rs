use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().nth(1).as_deref() == Some("--status") {
        let status = serde_json::json!({
            "schema": "cynic.lxc_exec_bridge.helper_status.v1",
            "status": "active",
            "runtime": "rust_native",
            "shell_execution": false,
            "secret_exposed": false
        });
        println!("{}", serde_json::to_string(&status).unwrap_or_default());
        return ExitCode::SUCCESS;
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
        match serde_json::from_str::<cynic_host::lxc_exec_bridge::LxcBridgeRequest>(&input) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("request parse failed: {error}");
                return ExitCode::FAILURE;
            }
        };
    let pct_bin = std::env::var("CYNIC_PCT_BIN").unwrap_or_else(|_| "/usr/sbin/pct".to_string());
    let report = cynic_host::lxc_exec_bridge::execute(&request, &pct_bin);
    let success = report.status == "completed";
    println!("{}", serde_json::to_string(&report).unwrap_or_default());
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
