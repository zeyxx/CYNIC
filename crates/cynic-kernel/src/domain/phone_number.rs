//! Phone number domain types for CallShield.
//!
//! A phone number is judged by community reports (human Dog verdicts),
//! call metadata, and temporal patterns. The score represents spam
//! likelihood: 0.0 = safe, 1.0 = confirmed scam.

use serde::{Deserialize, Serialize};

/// Core data structure for a phone number under judgment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneData {
    pub number: String,
    pub country_code: String,
    pub total_events: u64,
    pub label_distribution: LabelDistribution,
    pub reporter_count: u32,
    pub mean_reporter_trust: f32,
    pub age_days: u32,
    pub days_since_last_report: u32,
    pub challenge_pass_rate: Option<f32>,
    pub contestation_count: u32,
    pub owner_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LabelDistribution {
    pub legitimate: u32,
    pub nuisance: u32,
    pub scam: u32,
    pub unknown: u32,
}

impl LabelDistribution {
    pub fn total(&self) -> u32 {
        self.legitimate + self.nuisance + self.scam + self.unknown
    }

    pub fn spam_score(&self) -> f32 {
        let total = self.total() as f32;
        if total == 0.0 {
            return 0.5;
        }
        let weighted =
            (self.nuisance as f32 * 0.75) + (self.scam as f32 * 1.0) + (self.unknown as f32 * 0.5);
        weighted / total
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhoneLabel {
    Legitimate,
    Nuisance,
    Scam,
    Unknown,
}

impl PhoneLabel {
    pub fn numeric_value(&self) -> f32 {
        match self {
            Self::Legitimate => 0.0,
            Self::Unknown => 0.5,
            Self::Nuisance => 0.75,
            Self::Scam => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_distribution_spam_score_all_legit() {
        let dist = LabelDistribution {
            legitimate: 10,
            nuisance: 0,
            scam: 0,
            unknown: 0,
        };
        assert!((dist.spam_score() - 0.0).abs() < 0.001);
    }

    #[test]
    fn label_distribution_spam_score_all_scam() {
        let dist = LabelDistribution {
            legitimate: 0,
            nuisance: 0,
            scam: 10,
            unknown: 0,
        };
        assert!((dist.spam_score() - 1.0).abs() < 0.001);
    }

    #[test]
    fn label_distribution_spam_score_mixed() {
        let dist = LabelDistribution {
            legitimate: 5,
            nuisance: 3,
            scam: 2,
            unknown: 0,
        };
        assert!((dist.spam_score() - 0.425).abs() < 0.001);
    }

    #[test]
    fn label_distribution_spam_score_empty() {
        let dist = LabelDistribution::default();
        assert!((dist.spam_score() - 0.5).abs() < 0.001);
    }

    #[test]
    fn phone_label_numeric_values() {
        assert!((PhoneLabel::Legitimate.numeric_value() - 0.0).abs() < 0.001);
        assert!((PhoneLabel::Nuisance.numeric_value() - 0.75).abs() < 0.001);
        assert!((PhoneLabel::Scam.numeric_value() - 1.0).abs() < 0.001);
    }
}
