#[async_trait]
impl ActivityStorage for InMemoryStorage {
    async fn store_observation(&self, obs: &Observation) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        let obs_id = format!("observation:{}", s.observations.len());
        s.observations.push(RawObservation {
            id: obs_id,
            tool: obs.tool.clone(),
            target: obs.target.clone(),
            domain: obs.domain.clone(),
            status: obs.status.clone(),
            context: obs.context.clone(),
            created_at: obs.timestamp,
            project: obs.project.clone(),
            agent_id: obs.agent_id.clone(),
            session_id: obs.session_id.clone(),
            tags: obs.tags.clone(),
            value: obs.value.clone(),
            confidence: obs.confidence.clone(),
            consumer: obs.consumer.clone(),
            action: obs.action.clone(),
            depends_on: obs.depends_on.clone(),
            maturity: obs.maturity,
            hash: obs.hash.clone(),
            prev_hash: obs.prev_hash.clone(),
            observers: obs.observers.clone(),
            consensus_score: obs.consensus_score,
            source_tier: obs.source_tier.clone(),
        });
        Ok(())
    }

    async fn store_event(&self, event: &Event) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        let _id = format!("event:{}", s.events.len());
        s.events.push(event.clone());
        Ok(())
    }

    async fn fleet_stats(
        &self,
        _window_secs: u64,
        _limit: u32,
    ) -> Result<Vec<(String, u64, f64, u64, String)>, StorageError> {
        Ok(vec![])
    }

    async fn list_events(
        &self,
        _node: Option<&str>,
        _tool: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<RawEvent>, StorageError> {
        Ok(vec![])
    }

    async fn list_degraded_nodes(
        &self,
        _window_secs: u64,
        _fatal_threshold: f64,
    ) -> Result<Vec<(String, String, u64, u64)>, StorageError> {
        Ok(vec![])
    }

    async fn list_observations_raw(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations
            .iter()
            .rev()
            .filter(|o| domain.is_none_or(|d| o.domain == d))
            .filter(|o| agent_id.is_none_or(|a| o.agent_id == a))
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_observations_filtered(
        &self,
        domain: Option<&str>,
        agent_id: Option<&str>,
        project: Option<&str>,
        workstream: Option<&str>,
        thread_id: Option<&str>,
        target: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let rows = if let Some(target) = target {
            let rows = if let Some(domain) = domain {
                self.list_observations_by_target(domain, target, limit)
                    .await?
            } else {
                self.list_observations_raw(None, agent_id, limit).await?
            };
            rows.into_iter()
                .filter(|row| row.target == target)
                .collect()
        } else {
            self.list_observations_raw(domain, agent_id, limit).await?
        };

        let rows = rows
            .into_iter()
            .filter(|row| project.as_ref().is_none_or(|p| row.project == *p))
            .filter(|row| {
                if let Some(workstream) = workstream {
                    if workstream.is_empty() {
                        return true;
                    }
                    get_coordination_workstream(row).as_deref() == Some(workstream)
                } else {
                    true
                }
            })
            .filter(|row| {
                if let Some(thread_id) = thread_id {
                    if thread_id.is_empty() {
                        return true;
                    }
                    get_coordination_thread_id(row).as_deref() == Some(thread_id)
                } else {
                    true
                }
            })
            .take(limit as usize)
            .collect();
        Ok(rows)
    }

    async fn list_observations_by_target(
        &self,
        domain: &str,
        target: &str,
        limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations
            .iter()
            .rev()
            .filter(|o| o.domain == domain && o.target == target)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn list_observations_by_tag(
        &self,
        _domain: &str,
        _tag: &str,
        _limit: u32,
    ) -> Result<Vec<RawObservation>, StorageError> {
        Ok(vec![])
    }

    async fn last_observation_per_source(
        &self,
    ) -> Result<Vec<(String, String, u64)>, StorageError> {
        Ok(vec![])
    }

    async fn store_session_summary(&self, summary: &SessionSummary) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        s.summaries.push(summary.clone());
        Ok(())
    }

    async fn list_session_summaries(
        &self,
        limit: u32,
    ) -> Result<Vec<SessionSummary>, StorageError> {
        let s = self.state.lock().await;
        let mut v = s.summaries.clone();
        v.reverse();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn get_unsummarized_sessions(
        &self,
        min_observations: u32,
        limit: u32,
    ) -> Result<Vec<(String, String, u32)>, StorageError> {
        let s = self.state.lock().await;
        let summarized: std::collections::HashSet<_> =
            s.summaries.iter().map(|ss| ss.session_id.clone()).collect();
        let mut counts: HashMap<String, (String, u32)> = HashMap::new();
        for obs in &s.observations {
            let entry = counts
                .entry(obs.session_id.clone())
                .or_insert_with(|| (obs.agent_id.clone(), 0));
            entry.1 += 1;
        }
        let mut result: Vec<_> = counts
            .into_iter()
            .filter(|(sid, (_, count))| *count >= min_observations && !summarized.contains(sid))
            .map(|(sid, (aid, count))| (sid, aid, count))
            .collect();
        result.sort_by_key(|r| std::cmp::Reverse(r.2));
        result.truncate(limit as usize);
        Ok(result)
    }

    async fn get_session_observations(
        &self,
        session_id: &str,
    ) -> Result<Vec<RawObservation>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations
            .iter()
            .filter(|o| o.session_id == session_id)
            .cloned()
            .collect())
    }

    async fn store_session_compliance(
        &self,
        _compliance: &crate::domain::compliance::SessionCompliance,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn list_session_compliance(
        &self,
        _limit: u32,
    ) -> Result<Vec<crate::domain::compliance::SessionCompliance>, StorageError> {
        Ok(vec![])
    }

    async fn zone_activity(
        &self,
        _path_prefixes: &[String],
        _exclude_agent: &str,
        _project_root: &str,
    ) -> Result<Vec<crate::api::rest::dispatch::AgentActivity>, StorageError> {
        Ok(vec![])
    }

    async fn count_observations(&self) -> Result<u64, StorageError> {
        let s = self.state.lock().await;
        Ok(s.observations.len() as u64)
    }
}
