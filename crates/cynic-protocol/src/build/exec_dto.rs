use serde::{Deserialize, Serialize};

/// Request body for `POST /exec` on a remote builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub command: String,
    #[serde(default)]
    pub work_dir: String,
}

/// Response body returned by `POST /exec` on a remote builder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_request_round_trips() {
        let req = ExecRequest {
            command: "cargo check".to_string(),
            work_dir: "/tmp/repo".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: ExecRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.command, "cargo check");
        assert_eq!(back.work_dir, "/tmp/repo");
    }

    #[test]
    fn exec_request_work_dir_defaults() {
        let req: ExecRequest = serde_json::from_str(r#"{"command":"ls"}"#).unwrap();
        assert_eq!(req.command, "ls");
        assert!(req.work_dir.is_empty());
    }

    #[test]
    fn exec_response_round_trips() {
        let resp = ExecResponse {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration_ms: 42,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: ExecResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.exit_code, 0);
        assert_eq!(back.stdout, "ok");
        assert_eq!(back.duration_ms, 42);
    }
}
