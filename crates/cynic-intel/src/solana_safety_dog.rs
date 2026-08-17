//! Deterministic Solana Safety Dog for evaluating transfers, Solana Pay requests, and spend allowances.
//!
//! Enforces mint allowlists, recipient allowlists, spend caps, base58 format validation,
//! and produces deterministic risk verdicts for ZeroClaw / CYNIC.

use cynic_protocol::solana_ops::{AllowanceGrant, SolanaPayRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolanaSafetyVerdict {
    Approved,
    HumanApprovalRequired(String),
    Rejected(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolanaSafetyReport {
    pub verdict: SolanaSafetyVerdict,
    pub risk_score: f64,
    pub checks_passed: Vec<String>,
    pub checks_failed: Vec<String>,
}

/// Validates base58 string characters (Solana address / signature format check).
pub fn is_valid_base58(input: &str) -> bool {
    if input.is_empty() || input.len() < 32 || input.len() > 88 {
        return false;
    }
    input
        .chars()
        .all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c))
}

/// Evaluates a `SolanaPayRequest` for security, address validity, and spend thresholds.
pub fn evaluate_solana_pay_request(
    request: &SolanaPayRequest,
    allowance: Option<&AllowanceGrant>,
) -> SolanaSafetyReport {
    let mut checks_passed = Vec::new();
    let mut checks_failed = Vec::new();
    let mut risk_score: f64 = 0.0;

    // 1. Recipient address base58 validation
    if is_valid_base58(&request.recipient) {
        checks_passed.push("recipient_base58_valid".into());
    } else {
        checks_failed.push("recipient_base58_invalid".into());
        risk_score += 0.80;
    }

    // 2. SPL Token mint base58 validation if present
    if let Some(ref mint) = request.spl_token {
        if is_valid_base58(mint) {
            checks_passed.push("spl_token_mint_base58_valid".into());
        } else {
            checks_failed.push("spl_token_mint_base58_invalid".into());
            risk_score += 0.50;
        }
    }

    // 3. Amount checks
    if let Some(amt) = request.amount {
        if amt > 0.0 {
            checks_passed.push("amount_positive".into());
            if amt > 100.0 {
                checks_failed.push("high_value_transfer".into());
                risk_score += 0.30;
            }
        } else {
            checks_failed.push("amount_non_positive".into());
            risk_score += 0.90;
        }
    } else {
        checks_passed.push("unspecified_amount_request".into());
    }

    // 4. Allowance Grant validation if provided
    if let Some(grant) = allowance {
        let amount_val = request.amount.unwrap_or(0.0);
        let mint_ref = request.spl_token.as_deref();

        match grant.validate_transfer(&request.recipient, amount_val, mint_ref) {
            Ok(()) => {
                checks_passed.push("allowance_grant_passed".into());
            }
            Err(reason) => {
                checks_failed.push(format!("allowance_grant_violation:{reason}"));
                risk_score += 0.60;
            }
        }
    }

    let final_risk_score = risk_score.min(1.0);
    let verdict = if !checks_failed.is_empty() {
        if final_risk_score >= 0.70 {
            SolanaSafetyVerdict::Rejected(format!(
                "Failed safety checks: {}",
                checks_failed.join(", ")
            ))
        } else {
            SolanaSafetyVerdict::HumanApprovalRequired(format!(
                "Risk score ({final_risk_score:.2}) requires human approval: {}",
                checks_failed.join(", ")
            ))
        }
    } else {
        SolanaSafetyVerdict::Approved
    };

    SolanaSafetyReport {
        verdict,
        risk_score: final_risk_score,
        checks_passed,
        checks_failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_solana_pay_request_approved() {
        let req = SolanaPayRequest::new("7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c")
            .with_amount(5.0)
            .with_spl_token("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

        let report = evaluate_solana_pay_request(&req, None);
        assert_eq!(report.verdict, SolanaSafetyVerdict::Approved);
        assert_eq!(report.risk_score, 0.0);
        assert!(report.checks_failed.is_empty());
    }

    #[test]
    fn invalid_base58_recipient_rejected() {
        let req = SolanaPayRequest::new("invalid_0x_recipient_address!").with_amount(1.0);

        let report = evaluate_solana_pay_request(&req, None);
        assert!(matches!(report.verdict, SolanaSafetyVerdict::Rejected(_)));
        assert!(report.risk_score >= 0.70);
    }
}
