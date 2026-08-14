//! Token rug-risk heuristics.
//!
//! This crate owns interpreted token intelligence. Pure token data may later
//! move to `cynic-data`; until then, callers adapt their token record through
//! `RugRiskTokenView`.

/// Read-only view required by rug-risk scoring.
pub trait RugRiskTokenView {
    fn holder_data_available(&self) -> bool;
    fn age_hours(&self) -> u64;
    fn holder_count(&self) -> u64;
    fn trajectory_class(&self) -> Option<&str>;
    fn trajectory_decay(&self) -> Option<f64>;
    fn mint_authority_active(&self) -> bool;
    fn freeze_authority_active(&self) -> bool;
    fn lp_status(&self) -> &str;
    fn supply_burned_pct(&self) -> Option<f64>;
    fn origin(&self) -> Option<&str>;
    fn top1_pct(&self) -> f64;
    fn herfindahl(&self) -> Option<f64>;
    fn buy_sell_ratio(&self) -> Option<f64>;
    fn divergence_class(&self) -> Option<&str>;
    fn kscore_diamond_hands(&self) -> Option<f64>;
    fn volume_24h_usd(&self) -> Option<f64>;
    fn liquidity_usd(&self) -> Option<f64>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RugRiskAssessment {
    pub score: f64,
    pub signals: Vec<RugSignal>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RugSignal {
    pub name: &'static str,
    pub weight: f64,
    pub evidence: String,
}

pub fn assess_rug_risk(token: &impl RugRiskTokenView) -> RugRiskAssessment {
    let mut signals = Vec::new();
    let holder_ok = token.holder_data_available();
    let age_hours = token.age_hours();

    if age_hours < 6 {
        signals.push(RugSignal {
            name: "very_young",
            weight: 0.25,
            evidence: format!("age={age_hours}h"),
        });
    }

    if holder_ok && age_hours < 24 && token.holder_count() < 50 {
        signals.push(RugSignal {
            name: "young_low_holders",
            weight: 0.20,
            evidence: format!("age={}h, holders={}", age_hours, token.holder_count()),
        });
    }

    match token.trajectory_class() {
        Some("DYING") => signals.push(RugSignal {
            name: "dying_trajectory",
            weight: 0.15,
            evidence: format!(
                "trajectory=DYING, decay={:.2}",
                token.trajectory_decay().unwrap_or(0.0)
            ),
        }),
        Some("DECLINING") => signals.push(RugSignal {
            name: "declining_trajectory",
            weight: 0.10,
            evidence: format!(
                "trajectory=DECLINING, decay={:.2}",
                token.trajectory_decay().unwrap_or(0.0)
            ),
        }),
        _ => {}
    }

    if token.mint_authority_active() {
        signals.push(RugSignal {
            name: "mint_authority_active",
            weight: 0.20,
            evidence: "mint authority can inflate supply".into(),
        });
    }

    if token.freeze_authority_active() {
        signals.push(RugSignal {
            name: "freeze_authority_active",
            weight: 0.15,
            evidence: "freeze authority can lock wallets".into(),
        });
    }

    if token.lp_status() != "burned" && age_hours > 24 {
        signals.push(RugSignal {
            name: "no_lp_commitment",
            weight: 0.15,
            evidence: format!("lp={}, age={}h", token.lp_status(), age_hours),
        });
    }

    if token.supply_burned_pct().is_none_or(|pct| pct == 0.0) && age_hours > 24 {
        signals.push(RugSignal {
            name: "no_supply_burn",
            weight: 0.10,
            evidence: format!(
                "supply_burned={}%, age={}h",
                token.supply_burned_pct().unwrap_or(0.0),
                age_hours
            ),
        });
    }

    if token.origin() == Some("pump.fun") {
        signals.push(RugSignal {
            name: "pump_fun_origin",
            weight: 0.10,
            evidence: "origin=pump.fun (98.6% baseline rug rate)".into(),
        });
    }

    if holder_ok && token.top1_pct() > 50.0 {
        signals.push(RugSignal {
            name: "extreme_concentration",
            weight: 0.20,
            evidence: format!("top1={:.1}%", token.top1_pct()),
        });
    }

    if holder_ok
        && let Some(herfindahl) = token.herfindahl()
        && herfindahl > 0.3
    {
        signals.push(RugSignal {
            name: "unhealthy_distribution",
            weight: 0.15,
            evidence: format!("herfindahl={herfindahl:.3}"),
        });
    }

    if let Some(buy_sell_ratio) = token.buy_sell_ratio()
        && buy_sell_ratio < 0.5
    {
        signals.push(RugSignal {
            name: "active_distribution",
            weight: 0.20,
            evidence: format!("buy_sell_ratio={buy_sell_ratio:.2}"),
        });
    }

    if token.divergence_class() == Some("DISTRIBUTION") {
        signals.push(RugSignal {
            name: "distribution_class",
            weight: 0.15,
            evidence: "divergence=DISTRIBUTION".into(),
        });
    }

    if let Some(diamond_hands) = token.kscore_diamond_hands()
        && diamond_hands < 0.2
    {
        signals.push(RugSignal {
            name: "no_diamond_hands",
            weight: 0.10,
            evidence: format!("diamond_hands={diamond_hands:.3}"),
        });
    }

    if let (Some(volume), Some(liquidity)) = (token.volume_24h_usd(), token.liquidity_usd())
        && volume > 0.0
        && liquidity < 1000.0
    {
        signals.push(RugSignal {
            name: "thin_liquidity",
            weight: 0.15,
            evidence: format!("volume=${volume:.0}, liquidity=${liquidity:.0}"),
        });
    }

    signals.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let score = signals
        .iter()
        .map(|signal| signal.weight)
        .sum::<f64>()
        .clamp(0.0, 1.0);

    RugRiskAssessment { score, signals }
}

pub fn format_rug_risk_section(assessment: &RugRiskAssessment) -> String {
    if assessment.score == 0.0 && assessment.signals.is_empty() {
        return String::new();
    }

    let mut section = format!("\n[RUG RISK]\nrug_risk_score: {:.2}\n", assessment.score);
    for signal in assessment.signals.iter().take(5) {
        section.push_str(&format!(
            "  - {} ({:.2}): {}\n",
            signal.name, signal.weight, signal.evidence
        ));
    }
    section
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Default)]
    struct FixtureToken {
        holder_data_available: bool,
        age_hours: u64,
        holder_count: u64,
        trajectory_class: Option<String>,
        trajectory_decay: Option<f64>,
        mint_authority_active: bool,
        freeze_authority_active: bool,
        lp_status: String,
        supply_burned_pct: Option<f64>,
        origin: Option<String>,
        top1_pct: f64,
        herfindahl: Option<f64>,
        buy_sell_ratio: Option<f64>,
        divergence_class: Option<String>,
        kscore_diamond_hands: Option<f64>,
        volume_24h_usd: Option<f64>,
        liquidity_usd: Option<f64>,
    }

    impl RugRiskTokenView for FixtureToken {
        fn holder_data_available(&self) -> bool {
            self.holder_data_available
        }

        fn age_hours(&self) -> u64 {
            self.age_hours
        }

        fn holder_count(&self) -> u64 {
            self.holder_count
        }

        fn trajectory_class(&self) -> Option<&str> {
            self.trajectory_class.as_deref()
        }

        fn trajectory_decay(&self) -> Option<f64> {
            self.trajectory_decay
        }

        fn mint_authority_active(&self) -> bool {
            self.mint_authority_active
        }

        fn freeze_authority_active(&self) -> bool {
            self.freeze_authority_active
        }

        fn lp_status(&self) -> &str {
            &self.lp_status
        }

        fn supply_burned_pct(&self) -> Option<f64> {
            self.supply_burned_pct
        }

        fn origin(&self) -> Option<&str> {
            self.origin.as_deref()
        }

        fn top1_pct(&self) -> f64 {
            self.top1_pct
        }

        fn herfindahl(&self) -> Option<f64> {
            self.herfindahl
        }

        fn buy_sell_ratio(&self) -> Option<f64> {
            self.buy_sell_ratio
        }

        fn divergence_class(&self) -> Option<&str> {
            self.divergence_class.as_deref()
        }

        fn kscore_diamond_hands(&self) -> Option<f64> {
            self.kscore_diamond_hands
        }

        fn volume_24h_usd(&self) -> Option<f64> {
            self.volume_24h_usd
        }

        fn liquidity_usd(&self) -> Option<f64> {
            self.liquidity_usd
        }
    }

    fn healthy() -> FixtureToken {
        FixtureToken {
            holder_data_available: true,
            holder_count: 250_000,
            top1_pct: 12.5,
            age_hours: 1200,
            lp_status: "burned".into(),
            supply_burned_pct: Some(10.0),
            liquidity_usd: Some(5_000_000.0),
            volume_24h_usd: Some(500_000.0),
            trajectory_class: Some("STABLE".into()),
            buy_sell_ratio: Some(1.2),
            divergence_class: Some("STRONG_HOLD".into()),
            herfindahl: Some(0.05),
            kscore_diamond_hands: Some(0.7),
            ..FixtureToken::default()
        }
    }

    fn has_signal(assessment: &RugRiskAssessment, name: &str) -> bool {
        assessment.signals.iter().any(|signal| signal.name == name)
    }

    #[test]
    fn healthy_token_score_zero() {
        let assessment = assess_rug_risk(&healthy());
        assert_eq!(assessment.score, 0.0);
        assert!(assessment.signals.is_empty());
    }

    #[test]
    fn young_pump_fun_rug_collects_core_signals() {
        let token = FixtureToken {
            holder_data_available: true,
            holder_count: 5,
            top1_pct: 80.0,
            age_hours: 2,
            mint_authority_active: true,
            freeze_authority_active: true,
            lp_status: "unsecured".into(),
            origin: Some("pump.fun".into()),
            ..FixtureToken::default()
        };

        let assessment = assess_rug_risk(&token);

        assert!(assessment.score > 0.7);
        assert!(has_signal(&assessment, "very_young"));
        assert!(has_signal(&assessment, "young_low_holders"));
        assert!(has_signal(&assessment, "extreme_concentration"));
        assert!(has_signal(&assessment, "mint_authority_active"));
        assert!(has_signal(&assessment, "freeze_authority_active"));
        assert!(has_signal(&assessment, "pump_fun_origin"));
    }

    #[test]
    fn formats_top_risk_signals() {
        let assessment = RugRiskAssessment {
            score: 0.25,
            signals: vec![RugSignal {
                name: "very_young",
                weight: 0.25,
                evidence: "age=1h".into(),
            }],
        };

        let section = format_rug_risk_section(&assessment);
        assert!(section.contains("[RUG RISK]"));
        assert!(section.contains("very_young"));
    }
}
