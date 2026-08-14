#[async_trait]
impl VerdictStorage for InMemoryStorage {
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.verdicts.push(verdict.clone());
        Ok(())
    }

    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts.iter().find(|v| v.id == id).cloned())
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s.verdicts.iter().rev().cloned().collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn recent_max_disagreements(&self, limit: usize) -> Result<Vec<f64>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s.verdicts.iter().map(|v| v.max_disagreement).collect();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(limit);
        Ok(v)
    }

    async fn list_verdicts_by_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Verdict>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .verdicts
            .iter()
            .filter(|v| v.domain == domain)
            .cloned()
            .collect();
        v.sort_by(|a, b| {
            a.q_score
                .sovereignty
                .partial_cmp(&b.q_score.sovereignty)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn last_integrity_hash(&self) -> Result<Option<String>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts
            .iter()
            .max_by_key(|v| &v.timestamp)
            .and_then(|v| v.integrity_hash.clone()))
    }

    async fn count_verdicts(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.verdicts.len() as u64)
    }

    async fn count_verdicts_by_kind(
        &self,
    ) -> Result<std::collections::HashMap<String, u64>, StorageError> {
        let s = self.state.lock().await;
        let mut map = HashMap::new();
        for v in &s.verdicts {
            *map.entry(format!("{:?}", v.kind).to_lowercase())
                .or_insert(0) += 1;
        }
        Ok(map)
    }
}
