//! In-memory adapter for anchor events (tests only).

use std::sync::Mutex;

use crate::domain::AnchorEvent;
use crate::port::AnchorEventStore;

pub struct InMemoryAnchorStore {
    events: Mutex<Vec<AnchorEvent>>,
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

impl AnchorEventStore for InMemoryAnchorStore {
    fn record(&self, event: &AnchorEvent) -> Result<u64, String> {
        let mut events = self.events.lock().map_err(|e| e.to_string())?;
        let mut next = self.next_id.lock().map_err(|e| e.to_string())?;
        let id = *next;
        *next += 1;
        let mut ev = event.clone();
        ev.id = Some(id);
        events.push(ev);
        Ok(id)
    }

    fn recent(&self, limit: usize) -> Result<Vec<AnchorEvent>, String> {
        let events = self.events.lock().map_err(|e| e.to_string())?;
        let mut sorted: Vec<_> = events.clone();
        sorted.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        Ok(sorted.into_iter().take(limit).collect())
    }

    fn pending_sync(&self, limit: usize) -> Result<Vec<AnchorEvent>, String> {
        let events = self.events.lock().map_err(|e| e.to_string())?;
        let pending: Vec<_> = events
            .iter()
            .filter(|e| e.sync_eligible)
            .cloned()
            .take(limit)
            .collect();
        Ok(pending)
    }

    fn mark_synced(&self, _ids: &[u64], _synced_at_ms: i64) -> Result<(), String> {
        Ok(())
    }
}