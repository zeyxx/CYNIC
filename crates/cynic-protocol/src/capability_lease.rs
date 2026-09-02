//! Capability lease — temporary privilege escalation for bounded commands.
//!
//! Inspired by Solana's transaction model:
//! - Request: ask for a specific command to run with a budget
//! - Sign: authority CYNIC signs the capability (like a program signing an instruction)
//! - Execute: command runs in a sandbox with resource limits (like a Solana runtime)
//! - Receipt: proof of execution (like a Solana transaction receipt)
//!
//! Pattern:
//!   LeaseRequest → sign_lease() → CapabilityLease → execute_lease() → LeaseReceipt
//!
//! The lease is:
//! - Time-bounded (TTL, like Solana's blockhash expiry)
//! - Scope-bounded (exact command, like Solana's instruction data)
//! - Budget-bounded (energy, like Solana's compute units)
//! - Single-use (consumed after execution, like Solana's transaction nonce)

use serde::{Deserialize, Serialize};

pub const CAPABILITY_LEASE_SCHEMA: &str = "cynic.capability_lease.v1";
pub const LEASE_REQUEST_SCHEMA: &str = "cynic.lease_request.v1";
pub const LEASE_RECEIPT_SCHEMA: &str = "cynic.lease_receipt.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaseRequest {
    pub schema: String,
    pub request_id: String,
    pub requester_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub justification: String,
    pub energy_budget_joules: f64,
    pub wall_time_budget_ms: u64,
    pub ttl_seconds: u64,
    pub requested_at_unix_s: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityLease {
    pub schema: String,
    pub request: LeaseRequest,
    pub lease_id: String,
    pub authority_id: String,
    pub signature_hex: String,
    pub granted_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaseReceipt {
    pub schema: String,
    pub lease_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub energy_consumed_joules: f64,
    pub wall_time_consumed_ms: u64,
    pub executed_at_unix_s: u64,
    pub within_budget: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseError {
    Expired,
    AlreadyConsumed,
    CommandNotAllowed,
    EnergyBudgetExceeded,
    TimeBudgetExceeded,
    ExecutionFailed { exit_code: i32, stderr: String },
    InvalidSignature,
    UnknownAuthority,
}

impl std::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expired => write!(f, "lease expired"),
            Self::AlreadyConsumed => write!(f, "lease already consumed"),
            Self::CommandNotAllowed => write!(f, "command not in allowed list"),
            Self::EnergyBudgetExceeded => write!(f, "energy budget exceeded"),
            Self::TimeBudgetExceeded => write!(f, "wall time budget exceeded"),
            Self::ExecutionFailed { exit_code, stderr } => {
                write!(f, "execution failed (exit {}): {}", exit_code, stderr)
            }
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::UnknownAuthority => write!(f, "unknown authority"),
        }
    }
}

pub fn validate_lease_request(
    request: &LeaseRequest,
    allowed_commands: &[&str],
    max_energy_joules: f64,
) -> Result<(), LeaseError> {
    if !allowed_commands.contains(&request.command.as_str()) {
        return Err(LeaseError::CommandNotAllowed);
    }
    if request.energy_budget_joules > max_energy_joules {
        return Err(LeaseError::EnergyBudgetExceeded);
    }
    if request.wall_time_budget_ms > 300_000 {
        return Err(LeaseError::TimeBudgetExceeded);
    }
    if request.ttl_seconds > 3600 {
        return Err(LeaseError::Expired);
    }
    Ok(())
}

pub fn is_lease_valid(lease: &CapabilityLease, now_unix_s: u64) -> bool {
    !lease.consumed && now_unix_s >= lease.granted_at_unix_s && now_unix_s < lease.expires_at_unix_s
}

pub fn estimate_command_energy(command: &str, args: &[String]) -> f64 {
    match command {
        "rm" => 100.0 + (args.len() as f64 * 10.0),
        "pvesh" => 200.0,
        "systemctl" => 150.0,
        "mkdir" => 50.0,
        "chown" => 75.0,
        _ => 500.0,
    }
}

pub fn is_command_allowed(command: &str, allowed: &[&str]) -> bool {
    allowed.contains(&command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_lease_request_accepts_allowed_command() {
        let request = LeaseRequest {
            schema: LEASE_REQUEST_SCHEMA.to_string(),
            request_id: "req_001".to_string(),
            requester_id: "hermes:main".to_string(),
            command: "rm".to_string(),
            args: vec!["-rf".to_string(), "/tmp/test".to_string()],
            working_dir: None,
            justification: "CYNIC-RT-001: cleanup orphaned checkout".to_string(),
            energy_budget_joules: 200.0,
            wall_time_budget_ms: 5000,
            ttl_seconds: 300,
            requested_at_unix_s: 1724000000,
        };
        let allowed = &["rm", "pvesh", "systemctl", "mkdir"];
        assert!(validate_lease_request(&request, allowed, 1000.0).is_ok());
    }

    #[test]
    fn validate_lease_request_rejects_disallowed_command() {
        let request = LeaseRequest {
            schema: LEASE_REQUEST_SCHEMA.to_string(),
            request_id: "req_002".to_string(),
            requester_id: "hermes:main".to_string(),
            command: "chmod".to_string(),
            args: vec!["777".to_string()],
            working_dir: None,
            justification: "should fail".to_string(),
            energy_budget_joules: 100.0,
            wall_time_budget_ms: 1000,
            ttl_seconds: 60,
            requested_at_unix_s: 1724000000,
        };
        let allowed = &["rm", "pvesh"];
        let result = validate_lease_request(&request, allowed, 1000.0);
        assert!(result.is_err());
    }

    #[test]
    fn validate_lease_request_rejects_excessive_energy() {
        let request = LeaseRequest {
            schema: LEASE_REQUEST_SCHEMA.to_string(),
            request_id: "req_003".to_string(),
            requester_id: "hermes:main".to_string(),
            command: "rm".to_string(),
            args: vec![],
            working_dir: None,
            justification: "too much energy".to_string(),
            energy_budget_joules: 10000.0,
            wall_time_budget_ms: 1000,
            ttl_seconds: 60,
            requested_at_unix_s: 1724000000,
        };
        let allowed = &["rm"];
        let result = validate_lease_request(&request, allowed, 1000.0);
        assert!(result.is_err());
    }

    #[test]
    fn is_lease_valid_checks_expiry() {
        let lease = CapabilityLease {
            schema: CAPABILITY_LEASE_SCHEMA.to_string(),
            request: LeaseRequest {
                schema: LEASE_REQUEST_SCHEMA.to_string(),
                request_id: "req".to_string(),
                requester_id: "agent".to_string(),
                command: "rm".to_string(),
                args: vec![],
                working_dir: None,
                justification: "test".to_string(),
                energy_budget_joules: 100.0,
                wall_time_budget_ms: 1000,
                ttl_seconds: 60,
                requested_at_unix_s: 1000,
            },
            lease_id: "lease_001".to_string(),
            authority_id: "cynic".to_string(),
            signature_hex: "abcd".to_string(),
            granted_at_unix_s: 1000,
            expires_at_unix_s: 1060,
            consumed: false,
        };
        assert!(is_lease_valid(&lease, 1030));
        assert!(!is_lease_valid(&lease, 1070));
    }

    #[test]
    fn is_lease_valid_checks_consumed() {
        let mut lease = CapabilityLease {
            schema: CAPABILITY_LEASE_SCHEMA.to_string(),
            request: LeaseRequest {
                schema: LEASE_REQUEST_SCHEMA.to_string(),
                request_id: "req".to_string(),
                requester_id: "agent".to_string(),
                command: "rm".to_string(),
                args: vec![],
                working_dir: None,
                justification: "test".to_string(),
                energy_budget_joules: 100.0,
                wall_time_budget_ms: 1000,
                ttl_seconds: 60,
                requested_at_unix_s: 1000,
            },
            lease_id: "lease_001".to_string(),
            authority_id: "cynic".to_string(),
            signature_hex: "abcd".to_string(),
            granted_at_unix_s: 1000,
            expires_at_unix_s: 1060,
            consumed: true,
        };
        assert!(!is_lease_valid(&lease, 1030));
        lease.consumed = false;
        assert!(is_lease_valid(&lease, 1030));
    }

    #[test]
    fn estimate_command_energy_matches_known_commands() {
        let energy = estimate_command_energy("rm", &["-rf".to_string(), "/tmp".to_string()]);
        assert!((energy - 120.0).abs() < 0.01);
        let energy = estimate_command_energy("pvesh", &[]);
        assert!((energy - 200.0).abs() < 0.01);
        let energy = estimate_command_energy("unknown_cmd", &[]);
        assert!((energy - 500.0).abs() < 0.01);
    }

    #[test]
    fn is_command_allowed_checks_list() {
        let allowed = &["rm", "pvesh", "systemctl"];
        assert!(is_command_allowed("rm", allowed));
        assert!(!is_command_allowed("chmod", allowed));
    }
}
