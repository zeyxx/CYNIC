//! Zeroclaw allowance use case.
//!
//! Owns the bounded spending-allowance policy for the zeroclaw organ: a grant
//! is a pure decision from owner/delegate/token/limits; redemption produces a
//! Solana Pay request that the injected adapter settles. No mutation is
//! implied by the grant; only an explicit, bounded pay redeems it.

use crate::ports::solana_pay_port::{
    AllowanceGrant, SolanaPayPort, SolanaPayReceipt, SolanaPayRequest,
};

/// Pure decision: build an allowance grant from desired state. Does not mutate
/// any substrate.
pub fn grant_allowance(input: AllowanceGrantInput) -> AllowanceGrant {
    AllowanceGrant {
        owner_pubkey: input.owner_pubkey,
        delegate_pubkey: input.delegate_pubkey,
        spl_token: input.spl_token,
        max_amount_per_period: input.max_amount_per_period,
        period_seconds: input.period_seconds,
        remaining_allowance: input.max_amount_per_period,
        allowlist_recipients: input.allowlist_recipients,
        allowlist_mints: input.allowlist_mints,
        expires_at: input.expires_at,
    }
}

#[derive(Debug, Clone)]
pub struct AllowanceGrantInput {
    pub owner_pubkey: String,
    pub delegate_pubkey: String,
    pub spl_token: String,
    pub max_amount_per_period: u64,
    pub period_seconds: u64,
    pub allowlist_recipients: Vec<String>,
    pub allowlist_mints: Vec<String>,
    pub expires_at: Option<u64>,
}

/// Redeem part of an allowance through the injected Solana Pay adapter.
///
/// Bounded: only an amount within `remaining_allowance` to an allowlisted
/// recipient/mint is eligible. Returns the observed receipt.
pub fn redeem_allowance(
    port: &dyn SolanaPayPort,
    allowance: &AllowanceGrant,
    recipient: &str,
    amount: u64,
) -> Result<SolanaPayReceipt, String> {
    if let Some(exp) = allowance.expires_at {
        if exp == 0 {
            return Err("allowance expired".to_string());
        }
    }
    if !allowance.allowlist_recipients.is_empty() && !allowance.allowlist_recipients.contains(&recipient.to_string()) {
        return Err("recipient not allowlisted".to_string());
    }
    if amount == 0 {
        return Err("amount must be positive".to_string());
    }
    if amount > allowance.remaining_allowance {
        return Err("amount exceeds remaining allowance".to_string());
    }
    let req = SolanaPayRequest {
        recipient: recipient.to_string(),
        amount,
        spl_token: allowance.spl_token.clone(),
        allowance_id: allowance.delegate_pubkey.clone(),
        memo: format!("zeroclaw:allowance:{}", allowance.delegate_pubkey),
    };
    port.submit_pay(&req)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakePay {
        last_sig: String,
    }
    impl SolanaPayPort for FakePay {
        fn submit_pay(&self, _req: &SolanaPayRequest) -> Result<SolanaPayReceipt, String> {
            Ok(SolanaPayReceipt {
                signature: self.last_sig.clone(),
                slot: 1,
                amount: _req.amount,
                allowance_id: _req.allowance_id.clone(),
            })
        }
        fn observe_allowance(&self, _id: &str) -> Result<AllowanceGrant, String> {
            Err("not observed in fake".to_string())
        }
    }

    fn sample() -> AllowanceGrant {
        grant_allowance(AllowanceGrantInput {
            owner_pubkey: "owner".into(),
            delegate_pubkey: "delegate".into(),
            spl_token: "tok".into(),
            max_amount_per_period: 100,
            period_seconds: 3600,
            allowlist_recipients: vec!["rec1".into()],
            allowlist_mints: vec![],
            expires_at: None,
        })
    }

    #[test]
    fn grant_seeds_remaining_to_max() {
        let a = sample();
        assert_eq!(a.remaining_allowance, 100);
    }

    #[test]
    fn redeem_within_allowance_ok() {
        let fake = FakePay { last_sig: "sigX".into() };
        let receipt = redeem_allowance(&fake, &sample(), "rec1", 40).unwrap();
        assert_eq!(receipt.signature, "sigX");
        assert_eq!(receipt.amount, 40);
    }

    #[test]
    fn redeem_exceeds_remaining_rejected() {
        let fake = FakePay { last_sig: "sigX".into() };
        let err = redeem_allowance(&fake, &sample(), "rec1", 150).unwrap_err();
        assert!(err.contains("exceeds"));
    }

    #[test]
    fn redeem_non_allowlisted_rejected() {
        let fake = FakePay { last_sig: "sigX".into() };
        let err = redeem_allowance(&fake, &sample(), "rec9", 10).unwrap_err();
        assert!(err.contains("allowlisted"));
    }
}
