//! GASdf pure decision functions — no I/O, no clock reads.
//!
//! All timestamps are inputs. Adapters persist the produced decisions.

use super::types::*;

/// True iff the session status is terminal (cannot transition further).
pub fn is_terminal(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Settled
            | SessionStatus::Expired
            | SessionStatus::Rejected
            | SessionStatus::Refunded
    )
}

/// Validate a token against the registry for a given amount.
pub fn decide_token_allowed(
    registry: &PaymentTokenRegistry,
    mint: &str,
    amount: u64,
) -> TokenDecision {
    let token = registry.tokens.iter().find(|t| t.mint == mint);
    match token {
        None => TokenDecision::Unknown,
        Some(t) if !t.enabled => TokenDecision::Disabled,
        Some(t) if amount < t.min_amount => TokenDecision::BelowMinimum,
        Some(_) => TokenDecision::Allowed,
    }
}

/// Decide whether a session may be quoted for a given plan/token now.
pub fn decide_subscription_quote(
    session: &SubscriptionSession,
    plan: &SubscriptionPlan,
    token: &PaymentToken,
    input_amount: u64,
    estimated_asdf: u64,
    slippage_bps: Option<u16>,
    now: u64,
    gas_sponsored: bool,
) -> SubscriptionQuoteDecision {
    if !matches!(session.status, SessionStatus::Open | SessionStatus::Quoted) {
        return SubscriptionQuoteDecision::Reject {
            reason: "session not quotable".into(),
        };
    }
    if now >= session.expires_at {
        return SubscriptionQuoteDecision::Reject {
            reason: "session expired".into(),
        };
    }
    if !plan.enabled {
        return SubscriptionQuoteDecision::Reject {
            reason: "plan disabled".into(),
        };
    }
    if plan.app_id != session.app_id || plan.plan_id != session.plan_id {
        return SubscriptionQuoteDecision::Reject {
            reason: "plan does not match session".into(),
        };
    }
    if session.plan_id.is_empty() {
        return SubscriptionQuoteDecision::Reject {
            reason: "session has no plan".into(),
        };
    }
    if !token.enabled {
        return SubscriptionQuoteDecision::Reject {
            reason: "token disabled".into(),
        };
    }
    if token.mint != session.input_token_mint.clone().unwrap_or_default()
        && session.input_token_mint.is_some()
    {
        // allow re-quote with the same or a newly chosen token; mismatch only
        // when session already locked a different mint
        if session.input_token_mint.as_deref() != Some(token.mint.as_str()) {
            return SubscriptionQuoteDecision::Reject {
                reason: "session locked to a different input token".into(),
            };
        }
    }
    if input_amount < token.min_amount {
        return SubscriptionQuoteDecision::Reject {
            reason: "input amount below token minimum".into(),
        };
    }
    let effective_slippage = slippage_bps.unwrap_or(token.slippage_bps_default);
    // max slippage enforced by caller via registry; here we guard obviously huge values
    if effective_slippage > 10_000 {
        return SubscriptionQuoteDecision::Reject {
            reason: "slippage exceeds hard ceiling".into(),
        };
    }
    if estimated_asdf < plan.price_asdf_base_units {
        return SubscriptionQuoteDecision::Reject {
            reason: "conversion shortfall against plan price".into(),
        };
    }
    SubscriptionQuoteDecision::Accept {
        input_amount,
        estimated_asdf,
        slippage_bps: effective_slippage,
        gas_sponsored,
    }
}

/// Decide a session state-machine transition.
pub fn decide_session_transition(
    session: &SubscriptionSession,
    event: SessionEvent,
    now: u64,
) -> SessionTransition {
    if is_terminal(session.status) {
        return SessionTransition::NoOp {
            reason: "session already terminal".into(),
        };
    }
    if now >= session.expires_at {
        match session.status {
            SessionStatus::Settled | SessionStatus::Refunded => {}
            _ => return SessionTransition::ToExpired,
        }
    }
    match (session.status, event) {
        (SessionStatus::Open, SessionEvent::QuoteRequested) => SessionTransition::ToQuoted,
        (SessionStatus::Quoted, SessionEvent::PaymentSubmitted) => SessionTransition::ToPending,
        (SessionStatus::Pending, SessionEvent::PaymentConfirmed { at }) => {
            if at > now {
                SessionTransition::NoOp {
                    reason: "confirmation timestamp in the future".into(),
                }
            } else {
                SessionTransition::ToSettled { settled_at: at }
            }
        }
        (SessionStatus::Settled, SessionEvent::RefundRequested { at }) => {
            SessionTransition::ToRefunded { refunded_at: at }
        }
        _ => SessionTransition::NoOp {
            reason: "illegal transition".into(),
        },
    }
}

/// Decide whether a pending payment may be settled.
pub fn decide_settlement(
    session: &SubscriptionSession,
    received_asdf: u64,
    required_asdf: u64,
    confirmed: bool,
    now: u64,
) -> SettlementDecision {
    if session.status != SessionStatus::Pending {
        return SettlementDecision::Rejected {
            reason: "session not pending".into(),
        };
    }
    if now >= session.expires_at {
        return SettlementDecision::Rejected {
            reason: "session expired".into(),
        };
    }
    if !confirmed {
        return SettlementDecision::PendingConfirmation;
    }
    if received_asdf < required_asdf {
        return SettlementDecision::Shortfall {
            received_asdf,
            required_asdf,
        };
    }
    SettlementDecision::Settle { settled_at: now }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> PaymentTokenRegistry {
        PaymentTokenRegistry {
            schema: "cynic.gasdf.payment_tokens.v1".into(),
            tokens: vec![
                PaymentToken {
                    mint: "SOL".into(),
                    symbol: "SOL".into(),
                    decimals: 9,
                    kind: PaymentTokenKind::Native,
                    enabled: true,
                    min_amount: 1_000_000,
                    slippage_bps_default: 50,
                },
                PaymentToken {
                    mint: "USDC_MINT".into(),
                    symbol: "USDC".into(),
                    decimals: 6,
                    kind: PaymentTokenKind::Stable,
                    enabled: false,
                    min_amount: 10_000,
                    slippage_bps_default: 10,
                },
            ],
            limits: PaymentTokenLimits {
                max_slippage_bps: 300,
                quote_ttl_seconds: 300,
                min_quote_amount_asdf: 100_000,
            },
        }
    }

    fn open_session() -> SubscriptionSession {
        SubscriptionSession {
            schema: GASDF_SESSION_SCHEMA.into(),
            session_id: "s1".into(),
            app_id: "cultscreener".into(),
            plan_id: "pro-monthly".into(),
            buyer_pubkey: "buyer".into(),
            input_token_mint: None,
            status: SessionStatus::Open,
            created_at: 1000,
            expires_at: 2000,
            settled_at: None,
        }
    }

    fn plan() -> SubscriptionPlan {
        SubscriptionPlan {
            app_id: "cultscreener".into(),
            plan_id: "pro-monthly".into(),
            name: "Pro monthly".into(),
            price_asdf_base_units: 100_000_000,
            duration_seconds: 2592000,
            enabled: true,
        }
    }

    #[test]
    fn token_allowed_variants() {
        let reg = registry();
        assert_eq!(decide_token_allowed(&reg, "SOL", 2_000_000), TokenDecision::Allowed);
        assert_eq!(decide_token_allowed(&reg, "USDC_MINT", 20_000), TokenDecision::Disabled);
        assert_eq!(decide_token_allowed(&reg, "NOPE", 1), TokenDecision::Unknown);
        assert_eq!(decide_token_allowed(&reg, "SOL", 10), TokenDecision::BelowMinimum);
    }

    #[test]
    fn quote_accepts_valid() {
        let sess = open_session();
        let pl = plan();
        let tok = registry().tokens[0].clone();
        let d = decide_subscription_quote(&sess, &pl, &tok, 2_000_000, 100_000_000, Some(25), 1100, true);
        match d {
            SubscriptionQuoteDecision::Accept {
                slippage_bps,
                gas_sponsored,
                ..
            } => {
                assert_eq!(slippage_bps, 25);
                assert!(gas_sponsored);
            }
            other => panic!("expected accept, got {:?}", other),
        }
    }

    #[test]
    fn quote_rejects_disabled_plan_and_token_and_shortfall() {
        let sess = open_session();
        let mut pl = plan();
        pl.enabled = false;
        let tok = registry().tokens[0].clone();
        assert!(matches!(
            decide_subscription_quote(&sess, &pl, &tok, 2_000_000, 100_000_000, None, 1100, false),
            SubscriptionQuoteDecision::Reject { .. }
        ));

        let pl = plan();
        let mut tok2 = registry().tokens[0].clone();
        tok2.enabled = false;
        assert!(matches!(
            decide_subscription_quote(&sess, &pl, &tok2, 2_000_000, 100_000_000, None, 1100, false),
            SubscriptionQuoteDecision::Reject { .. }
        ));

        let tok = registry().tokens[0].clone();
        assert!(matches!(
            decide_subscription_quote(&sess, &pl, &tok, 2_000_000, 50_000_000, None, 1100, false),
            SubscriptionQuoteDecision::Reject { .. }
        ));
    }

    #[test]
    fn quote_rejects_locked_token_mismatch() {
        let mut sess = open_session();
        sess.input_token_mint = Some("OTHER".into());
        let pl = plan();
        let tok = registry().tokens[0].clone();
        assert!(matches!(
            decide_subscription_quote(&sess, &pl, &tok, 2_000_000, 100_000_000, None, 1100, false),
            SubscriptionQuoteDecision::Reject { .. }
        ));
    }

    #[test]
    fn session_lifecycle_transitions() {
        let mut sess = open_session();
        assert_eq!(
            decide_session_transition(&sess, SessionEvent::QuoteRequested, 1100),
            SessionTransition::ToQuoted
        );
        sess.status = SessionStatus::Quoted;
        assert_eq!(
            decide_session_transition(&sess, SessionEvent::PaymentSubmitted, 1100),
            SessionTransition::ToPending
        );
        sess.status = SessionStatus::Pending;
        assert_eq!(
            decide_session_transition(&sess, SessionEvent::PaymentConfirmed { at: 1200 }, 1200),
            SessionTransition::ToSettled { settled_at: 1200 }
        );
    }

    #[test]
    fn expired_session_transitions_to_expired() {
        let mut sess = open_session();
        sess.expires_at = 1050;
        assert_eq!(
            decide_session_transition(&sess, SessionEvent::QuoteRequested, 1100),
            SessionTransition::ToExpired
        );
    }

    #[test]
    fn terminal_session_is_noop() {
        let mut sess = open_session();
        sess.status = SessionStatus::Settled;
        assert!(matches!(
            decide_session_transition(&sess, SessionEvent::PaymentSubmitted, 1100),
            SessionTransition::NoOp { .. }
        ));
    }

    #[test]
    fn settlement_variants() {
        let mut sess = open_session();
        sess.status = SessionStatus::Pending;
        sess.expires_at = 2000;
        assert_eq!(
            decide_settlement(&sess, 100_000_000, 100_000_000, true, 1100),
            SettlementDecision::Settle { settled_at: 1100 }
        );
        assert!(matches!(
            decide_settlement(&sess, 50_000_000, 100_000_000, true, 1100),
            SettlementDecision::Shortfall { .. }
        ));
        assert_eq!(
            decide_settlement(&sess, 100_000_000, 100_000_000, false, 1100),
            SettlementDecision::PendingConfirmation
        );
        // not pending -> rejected
        let mut sess2 = open_session();
        assert!(matches!(
            decide_settlement(&sess2, 100_000_000, 100_000_000, true, 1100),
            SettlementDecision::Rejected { .. }
        ));
    }
}