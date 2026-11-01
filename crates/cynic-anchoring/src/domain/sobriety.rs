//! Sobriety domain logic: substances and milestones.
//!
//! Pure — no I/O.

use serde::{Deserialize, Serialize};

/// Substances tracked for sobriety.
///
/// `Other` carries a free-form label so that divergent profiles or new
/// substances can be tracked without modifying this enum. Known substances
/// are listed explicitly; `HardDrug` is NOT a catch-all — it refers
/// specifically to illicit hard drugs (opioids, stimulants, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SobrietySubstance {
    Alcohol,
    Tobacco,
    Cannabis,
    /// Opioids, stimulants, hallucinogens, etc. — NOT a catch-all.
    Opioid,
    Stimulant,
    Hallucinogen,
    /// Free-form label for anything not listed above.
    Other(String),
}

/// Sobriety milestones in days since last use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SobrietyMilestone {
    Days1,
    Days3,
    Days7,
    Month1,
    Month3,
    Month6,
}

impl SobrietyMilestone {
    /// Returns the number of days for this milestone.
    pub fn days(&self) -> u32 {
        match self {
            SobrietyMilestone::Days1 => 1,
            SobrietyMilestone::Days3 => 3,
            SobrietyMilestone::Days7 => 7,
            SobrietyMilestone::Month1 => 30,
            SobrietyMilestone::Month3 => 90,
            SobrietyMilestone::Month6 => 180,
        }
    }

    /// Compute the highest milestone reached given days sober.
    pub fn highest_reached(days_sober: u32) -> Option<Self> {
        let milestones = [
            SobrietyMilestone::Days1,
            SobrietyMilestone::Days3,
            SobrietyMilestone::Days7,
            SobrietyMilestone::Month1,
            SobrietyMilestone::Month3,
            SobrietyMilestone::Month6,
        ];
        milestones
            .iter()
            .rev()
            .find(|m| days_sober >= m.days())
            .copied()
    }

    /// Returns all milestones reached up to the given days sober.
    pub fn all_reached(days_sober: u32) -> Vec<Self> {
        let milestones = [
            SobrietyMilestone::Days1,
            SobrietyMilestone::Days3,
            SobrietyMilestone::Days7,
            SobrietyMilestone::Month1,
            SobrietyMilestone::Month3,
            SobrietyMilestone::Month6,
        ];
        milestones
            .iter()
            .filter(|m| days_sober >= m.days())
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestone_highest_reached() {
        assert_eq!(SobrietyMilestone::highest_reached(0), None);
        assert_eq!(
            SobrietyMilestone::highest_reached(1),
            Some(SobrietyMilestone::Days1)
        );
        assert_eq!(
            SobrietyMilestone::highest_reached(90),
            Some(SobrietyMilestone::Month3)
        );
        assert_eq!(
            SobrietyMilestone::highest_reached(200),
            Some(SobrietyMilestone::Month6)
        );
    }

    #[test]
    fn milestone_all_reached() {
        let reached = SobrietyMilestone::all_reached(7);
        assert_eq!(reached.len(), 3);
    }
}