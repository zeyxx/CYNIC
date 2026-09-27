impl Default for InMemoryStorage {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
        }
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl StoragePort for InMemoryStorage {
    async fn ping(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn flush_usage(&self, snapshot: &[(String, DogUsage)]) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        for (dog_id, usage) in snapshot {
            let entry = s.usage.entry(dog_id.clone()).or_insert_with(|| UsageRow {
                dog_id: dog_id.clone(),
                prompt_tokens: 0,
                completion_tokens: 0,
                requests: 0,
                failures: 0,
                total_latency_ms: 0,
            });
            entry.prompt_tokens += usage.prompt_tokens;
            entry.completion_tokens += usage.completion_tokens;
            entry.requests += usage.requests;
            entry.failures += usage.failures;
            entry.total_latency_ms += usage.total_latency_ms;
        }
        Ok(())
    }

    async fn load_usage_history(&self) -> Result<Vec<UsageRow>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.usage.values().cloned().collect())
    }

    async fn flush_dog_stats(&self, stats: &[(String, DogStats)]) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        for (dog_id, stat) in stats {
            s.dog_stats.insert(dog_id.clone(), stat.clone());
        }
        Ok(())
    }

    async fn load_dog_stats(
        &self,
    ) -> Result<Vec<(String, cynic_eval::eval::dog_health::DogStats)>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.dog_stats
            .iter()
            .map(|(id, stat)| (id.clone(), stat.clone()))
            .collect())
    }

    async fn load_dog_stat(
        &self,
        dog_id: &str,
    ) -> Result<Option<cynic_eval::eval::dog_health::DogStats>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.dog_stats.get(dog_id).cloned())
    }
}
