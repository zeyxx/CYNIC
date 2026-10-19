use super::types::*;

pub fn decide_fee_payer_health(fp: &FeePayer) -> FeePayerHealth {
    if fp.balance_lamports == 0 {
        FeePayerHealth::Depleted
    } else if fp.balance_lamports < fp.low_balance_threshold_lamports {
        FeePayerHealth::LowBalance
    } else {
        FeePayerHealth::Healthy
    }
}

pub fn decide_gas_quote(
    session: &GasSession,
    fee_payer: &FeePayer,
    fee_lamports: u64,
    now: u64,
    _quote_ttl_seconds: u64,
) -> GasQuoteDecision {
    if !matches!(session.status, GasSessionStatus::Open | GasSessionStatus::Quoted) {
        return GasQuoteDecision::Reject {
            reason: "session not quotable".into(),
        };
    }
    if now >= session.expires_at {
        return GasQuoteDecision::Reject {
            reason: "session expired".into(),
        };
    }
    if fee_lamports == 0 {
        return GasQuoteDecision::Reject {
            reason: "fee must be positive".into(),
        };
    }
    match decide_fee_payer_health(fee_payer) {
        FeePayerHealth::Depleted => {
            return GasQuoteDecision::Reject {
                reason: "fee payer depleted".into(),
            };
        }
        FeePayerHealth::LowBalance => {
            if fee_payer.balance_lamports < fee_lamports {
                return GasQuoteDecision::Reject {
                    reason: "fee payer cannot cover quote".into(),
                };
            }
        }
        FeePayerHealth::Healthy => {}
    }
    let gas_sponsored = fee_payer.balance_lamports >= fee_lamports;
    GasQuoteDecision::Accept {
        fee_lamports,
        gas_sponsored,
    }
}

pub fn decide_submit(
    session: &GasSession,
    quote_fee_lamports: u64,
    validated: bool,
    signature_present: bool,
    now: u64,
) -> SubmitDecision {
    if session.status != GasSessionStatus::Quoted && session.status != GasSessionStatus::Pending {
        return SubmitDecision::Reject {
            reason: "session not submit-ready".into(),
        };
    }
    if now >= session.expires_at {
        return SubmitDecision::Reject {
            reason: "session expired".into(),
        };
    }
    if !validated {
        return SubmitDecision::Reject {
            reason: "transaction failed validation".into(),
        };
    }
    if !signature_present {
        return SubmitDecision::PendingConfirmation;
    }
    SubmitDecision::Accept {
        fee_lamports: quote_fee_lamports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> GasSession {
        GasSession {
            schema: GASDF_GASLESS_SESSION_SCHEMA.into(),
            session_id: "g1".into(),
            buyer_pubkey: "buyer".into(),
            status: GasSessionStatus::Open,
            created_at: 1000,
            expires_at: 2000,
            quote_id: None,
            signature: None,
            settled_at: None,
        }
    }

    fn payer(bal: u64) -> FeePayer {
        FeePayer {
            pubkey: "payer".into(),
            balance_lamports: bal,
            low_balance_threshold_lamports: 10_000,
        }
    }

    #[test]
    fn fee_payer_health_variants() {
        assert_eq!(decide_fee_payer_health(&payer(50_000)), FeePayerHealth::Healthy);
        assert_eq!(decide_fee_payer_health(&payer(5_000)), FeePayerHealth::LowBalance);
        assert_eq!(decide_fee_payer_health(&payer(0)), FeePayerHealth::Depleted);
    }

    #[test]
    fn quote_accepts_when_payer_can_cover() {
        let sess = session();
        let d = decide_gas_quote(&sess, &payer(50_000), 4_000, 1100, 300);
        assert!(matches!(d, GasQuoteDecision::Accept { .. }));
    }

    #[test]
    fn quote_rejects_depleted_or_expired_or_zero() {
        let mut sess = session();
        assert!(matches!(
            decide_gas_quote(&sess, &payer(0), 4_000, 1100, 300),
            GasQuoteDecision::Reject { .. }
        ));
        assert!(matches!(
            decide_gas_quote(&sess, &payer(50_000), 0, 1100, 300),
            GasQuoteDecision::Reject { .. }
        ));
        sess.expires_at = 1050;
        assert!(matches!(
            decide_gas_quote(&sess, &payer(50_000), 4_000, 1100, 300),
            GasQuoteDecision::Reject { .. }
        ));
    }

    #[test]
    fn submit_pending_then_accept() {
        let mut sess = session();
        sess.status = GasSessionStatus::Quoted;
        sess.expires_at = 2000;
        match decide_submit(&sess, 4_000, true, false, 1100) {
            SubmitDecision::PendingConfirmation => {}
            other => panic!("expected pending, got {:?}", other),
        }
        match decide_submit(&sess, 4_000, true, true, 1100) {
            SubmitDecision::Accept { fee_lamports } => assert_eq!(fee_lamports, 4_000),
            other => panic!("expected accept, got {:?}", other),
        }
    }

    #[test]
    fn submit_rejects_invalid_or_expired() {
        let mut sess = session();
        sess.status = GasSessionStatus::Quoted;
        sess.expires_at = 2000;
        assert!(matches!(
            decide_submit(&sess, 4_000, false, true, 1100),
            SubmitDecision::Reject { .. }
        ));
        sess.expires_at = 1050;
        assert!(matches!(
            decide_submit(&sess, 4_000, true, true, 1100),
            SubmitDecision::Reject { .. }
        ));
    }
}