//! In-memory adapter for anchor observations (tests only).

use std::sync::Mutex;

use cynic_protocol::observation_record::Observation;
use crate::port::AnchorEventStore;

pub struct InMemoryAnchorStore {
    events: Mutex<Vec<Observation>>,
    next_id: Mutex<u64>,
}

impl InMemoryAnchorStore {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }
}

impl Default for InMemoryAnchorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for InMemoryAnchorStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryAnchorStore").finish()
    }
}

impl AnchorEventStore for InMemoryAnchorStore {
    fn record(&self, obs: &Observation) -> Result<u64, String> {
        let mut events = self.events.lock().map_err(|e| e.to_string())?;
        let mut next = self.next_id.lock().map_err(|e| e.to_string())?;
        let id = *next;
        *next += 1;
        events.push(obs.clone());
        Ok(id)
    }

    fn recent(&self, limit: usize) -> Result<Vec<Observation>, String> {
        let events = self.events.lock().map_err(|e| e.to_string())?;
        let mut sorted: Vec<_> = events.clone();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(sorted.into_iter().take(limit).collect())
    }

    fn pending_sync(&self, limit: usize) -> Result<Vec<Observation>, String> {
        let events = self.events.lock().map_err(|e| e.to_string())?;
        Ok(events.iter().cloned().take(limit).collect())
    }

    fn mark_synced(&self, _ids: &[u64], _synced_at_ms: i64) -> Result<(), String> {
        Ok(())
    }
}