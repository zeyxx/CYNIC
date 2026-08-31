//! Phase 4 Operator Decision Store.
//!
//! Provides local persistence and read-back for operator decision records,
//! linking human decision lineage to opportunity digests.

use std::path::{Path, PathBuf};

use cynic_protocol::organic_opportunity::{
    OpportunityDecisionRecord, OrganicOpportunity, apply_decision_record,
};

/// Local filesystem store for operator decisions.
#[derive(Debug, Clone)]
pub struct OpportunityDecisionStore {
    storage_dir: PathBuf,
}

impl OpportunityDecisionStore {
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            storage_dir: storage_dir.into(),
        }
    }

    /// Load all decision records from the storage directory.
    pub fn load_decisions(&self) -> Vec<OpportunityDecisionRecord> {
        if !self.storage_dir.exists() {
            return Vec::new();
        }

        let mut records = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(record) =
                            serde_json::from_str::<OpportunityDecisionRecord>(&content)
                        {
                            records.push(record);
                        }
                    }
                }
            }
        }

        records.sort_by(|a, b| a.decided_at_unix_s.cmp(&b.decided_at_unix_s));
        records
    }

    /// Persist one operator decision record.
    pub fn record_decision(&self, record: &OpportunityDecisionRecord) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.storage_dir).map_err(|e| {
            format!(
                "failed to create decision dir {}: {}",
                self.storage_dir.display(),
                e
            )
        })?;

        let filename = format!("{}.json", sanitize_id(&record.decision_id));
        let dest = self.storage_dir.join(filename);

        let json = serde_json::to_string_pretty(record)
            .map_err(|e| format!("failed to serialize decision record: {}", e))?;

        std::fs::write(&dest, json).map_err(|e| {
            format!(
                "failed to write decision record to {}: {}",
                dest.display(),
                e
            )
        })?;

        Ok(dest)
    }

    /// Apply all recorded decisions onto a slice of opportunities.
    pub fn apply_to_opportunities(&self, opportunities: &mut [OrganicOpportunity]) {
        let decisions = self.load_decisions();
        for opp in opportunities.iter_mut() {
            for decision in &decisions {
                apply_decision_record(opp, decision);
            }
        }
    }
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::organic_opportunity::{
        EnergyEstimate, OPPORTUNITY_DECISION_SCHEMA, ORGANIC_OPPORTUNITY_SCHEMA,
        OperatorDisposition, OpportunityClass, SmallestNextStep,
    };

    fn sample_opp(id: &str) -> OrganicOpportunity {
        OrganicOpportunity {
            schema: ORGANIC_OPPORTUNITY_SCHEMA.to_string(),
            opportunity_id: id.to_string(),
            scope: format!("scope/{}", id),
            sources: vec![],
            class: OpportunityClass::Emerging,
            expected_unlock: "test".to_string(),
            estimated_energy: EnergyEstimate::Unknown {
                reason: "test".to_string(),
            },
            freshness_seconds: 0,
            confidence: 0.8,
            smallest_next_step: SmallestNextStep::Observe,
            residual_unknowns: vec![],
            operator_disposition: OperatorDisposition::Unreviewed,
        }
    }

    #[test]
    fn persists_and_loads_decisions() {
        let tmp = tempfile::tempdir().unwrap();
        let store = OpportunityDecisionStore::new(tmp.path());

        let dec = OpportunityDecisionRecord {
            schema: OPPORTUNITY_DECISION_SCHEMA.to_string(),
            decision_id: "dec:opp-1:defer".to_string(),
            opportunity_digest: "opp-1".to_string(),
            operator_id: "human:operator".to_string(),
            decided_at_unix_s: 1724000000,
            disposition: OperatorDisposition::Deferred {
                at_unix_s: 1724000000,
                reason: "deferred during debt purge".to_string(),
            },
            reason: "deferred during debt purge".to_string(),
            action_intent: None,
        };

        store.record_decision(&dec).unwrap();

        let loaded = store.load_decisions();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].decision_id, "dec:opp-1:defer");

        let mut opps = vec![sample_opp("opp-1"), sample_opp("opp-2")];
        store.apply_to_opportunities(&mut opps);

        assert!(matches!(
            opps[0].operator_disposition,
            OperatorDisposition::Deferred { .. }
        ));
        assert!(matches!(
            opps[1].operator_disposition,
            OperatorDisposition::Unreviewed
        ));
    }
}
