#[async_trait]
impl CrystalStorage for InMemoryStorage {
    async fn store_crystal(&self, crystal: &Crystal) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.crystals.insert(crystal.id.clone(), crystal.clone());
        Ok(())
    }

    async fn get_crystal(&self, id: &str) -> Result<Option<Crystal>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals.get(id).cloned())
    }

    async fn list_crystals(&self, limit: u32) -> Result<Vec<Crystal>, StorageError> {
        self.list_crystals_filtered(limit, None, None).await
    }

    async fn list_crystals_filtered(
        &self,
        limit: u32,
        domain: Option<&str>,
        state: Option<&str>,
    ) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .crystals
            .values()
            .filter(|c| domain.is_none_or(|d| c.domain == d))
            .filter(|c| {
                state.is_none_or(|st| format!("{:?}", c.state).to_lowercase() == st.to_lowercase())
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| {
            state_rank(&a.state).cmp(&state_rank(&b.state)).then(
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn delete_crystal(&self, id: &str) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.crystals.remove(id);
        Ok(())
    }

    async fn update_crystal_relations(
        &self,
        id: &str,
        relations: std::collections::BTreeMap<String, String>,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        if let Some(c) = s.crystals.get_mut(id) {
            c.relations = relations;
        }
        Ok(())
    }

    async fn list_crystals_for_domain(
        &self,
        domain: &str,
        limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .crystals
            .values()
            .filter(|c| {
                (c.domain == domain || c.domain == "general")
                    && (c.state == CrystalState::Crystallized || c.state == CrystalState::Canonical)
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn count_crystal_observations(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals.values().map(|c| c.observations as u64).sum())
    }

    async fn observe_crystal(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: i64,
        voter_count: usize,
        verdict_id: &str,
        verdict_kind: &str,
    ) -> Result<(), StorageError> {
        if voter_count < MIN_QUORUM {
            return Err(StorageError::QueryFailed(format!(
                "quorum gate: voter_count {voter_count} < MIN_QUORUM {MIN_QUORUM}"
            )));
        }

        let sanitized = sanitize_crystal_content(content);
        let mut s = self.state.lock().await;

        let crystal = s.crystals.entry(id.to_string()).or_insert_with(|| Crystal {
            id: id.to_string(),
            content: sanitized.clone(),
            domain: domain.to_string(),
            confidence: 0.0,
            observations: 0,
            state: CrystalState::Forming,
            created_at: timestamp,
            updated_at: timestamp,
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
            source_diversity: 0,
            relations: std::collections::BTreeMap::new(),
            embedding: None,
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
        });

        if score > crystal.confidence {
            crystal.content = sanitized;
        }

        let old_mean = crystal.confidence;
        let old_obs = crystal.observations as f64;
        let new_conf = if crystal.observations > 0 {
            (old_mean * old_obs + score) / (old_obs + 1.0)
        } else {
            score
        };
        let delta = score - old_mean;
        let delta2 = score - new_conf;
        crystal.variance_m2 = if crystal.observations > 0 {
            crystal.variance_m2 + delta * delta2
        } else {
            0.0
        };
        crystal.confidence = new_conf;
        crystal.observations += 1;

        crystal.mean_quorum = if crystal.observations > 1 {
            (crystal.mean_quorum * old_obs + voter_count as f64) / crystal.observations as f64
        } else {
            voter_count as f64
        };

        match verdict_kind {
            "howl" => crystal.howl_count += 1,
            "wag" => crystal.wag_count += 1,
            "growl" => crystal.growl_count += 1,
            "bark" => crystal.bark_count += 1,
            _ => crystal.howl_count += 1,
        }

        crystal.updated_at = timestamp;
        crystal.source_diversity = crystal.contributing_sources.len() as u32;
        crystal.certainty = crate::domain::ccm::compute_certainty(
            crystal.variance_m2,
            crystal.observations,
            crystal.source_diversity,
            1.0,
        );
        crystal.state = crate::domain::ccm::classify(crystal.certainty, crystal.observations);

        if !verdict_id.is_empty()
            && crystal.contributing_verdicts.len() < crate::domain::ccm::MAX_CONTRIBUTING_VERDICTS
            && !crystal
                .contributing_verdicts
                .contains(&verdict_id.to_string())
        {
            crystal.contributing_verdicts.push(verdict_id.to_string());
        }

        Ok(())
    }

    async fn observe_crystal_hypha(
        &self,
        id: &str,
        content: &str,
        domain: &str,
        score: f64,
        timestamp: i64,
        source: &str,
        sentiment: Option<&str>,
    ) -> Result<(), StorageError> {
        let sanitized = sanitize_crystal_content(content);
        let mut s = self.state.lock().await;

        if s.crystals
            .get(id)
            .is_some_and(|c| c.state == CrystalState::Dissolved)
        {
            return Err(StorageError::QueryFailed(format!(
                "crystal {id} is dissolved — hypha observation rejected"
            )));
        }

        let crystal = s.crystals.entry(id.to_string()).or_insert_with(|| Crystal {
            id: id.to_string(),
            content: sanitized.clone(),
            domain: domain.to_string(),
            confidence: 0.0,
            observations: 0,
            state: CrystalState::Forming,
            created_at: timestamp,
            updated_at: timestamp,
            contributing_verdicts: vec![],
            certainty: 0.0,
            variance_m2: 0.0,
            mean_quorum: 0.0,
            howl_count: 0,
            wag_count: 0,
            growl_count: 0,
            bark_count: 0,
            source_diversity: 0,
            relations: std::collections::BTreeMap::new(),
            embedding: None,
            contributing_sources: std::collections::BTreeMap::new(),
            shattered_at: None,
            shatter_reason: None,
            shatter_source: None,
        });

        let old_mean = crystal.confidence;
        if score > old_mean && !content.is_empty() {
            crystal.content = sanitized;
        }

        let old_obs = crystal.observations as f64;
        let new_conf = if crystal.observations > 0 {
            (old_mean * old_obs + score) / (old_obs + 1.0)
        } else {
            score
        };
        let delta = score - old_mean;
        let delta2 = score - new_conf;
        crystal.variance_m2 = if crystal.observations > 0 {
            crystal.variance_m2 + delta * delta2
        } else {
            0.0
        };
        crystal.confidence = new_conf;
        crystal.observations += 1;

        match sentiment {
            Some("positive") => crystal.wag_count += 1,
            Some("negative") => crystal.growl_count += 1,
            _ => {}
        }

        *crystal
            .contributing_sources
            .entry(source.to_string())
            .or_insert(0) += 1;
        crystal.updated_at = timestamp;
        crystal.source_diversity = crystal.contributing_sources.len() as u32;
        crystal.certainty = crate::domain::ccm::compute_certainty(
            crystal.variance_m2,
            crystal.observations,
            crystal.source_diversity,
            1.0,
        );
        crystal.state = crate::domain::ccm::classify(crystal.certainty, crystal.observations);

        Ok(())
    }

    async fn shatter_crystal(
        &self,
        id: &str,
        reason: &str,
        source: &str,
        timestamp: i64,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        match s.crystals.get_mut(id) {
            Some(crystal) => {
                if crystal.state == CrystalState::Dissolved {
                    return Ok(());
                }
                crystal.state = CrystalState::Dissolved;
                crystal.shattered_at = Some(timestamp.to_string());
                crystal.shatter_reason = Some(reason.to_string());
                crystal.shatter_source = Some(source.to_string());
                crystal.updated_at = timestamp;
                Ok(())
            }
            None => Err(StorageError::QueryFailed(format!(
                "shatter_crystal: crystal {id} not found"
            ))),
        }
    }

    async fn store_crystal_embedding(
        &self,
        id: &str,
        embedding: &[f32],
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        match s.crystals.get_mut(id) {
            Some(crystal) => {
                crystal.embedding = Some(embedding.to_vec());
                Ok(())
            }
            None => Err(StorageError::QueryFailed(format!(
                "store_crystal_embedding: crystal {id} not found"
            ))),
        }
    }

    async fn search_crystals_semantic(
        &self,
        _query_embedding: &[f32],
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        Ok(vec![])
    }

    async fn find_similar_crystal(
        &self,
        _embedding: &[f32],
        _domain: &str,
        _threshold: f64,
    ) -> Result<Option<(String, f64)>, StorageError> {
        Ok(None)
    }

    async fn list_crystals_missing_embedding(
        &self,
        _limit: u32,
    ) -> Result<Vec<Crystal>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.crystals
            .values()
            .filter(|c| c.embedding.is_none())
            .cloned()
            .collect())
    }

    async fn consolidate_duplicate_crystals(&self) -> Result<u64, StorageError> {
        Ok(0)
    }
}
