use std::collections::{HashMap, VecDeque};

use crate::websocket::{PendingStimulus, PendingVerdict, WebSocketClient};

impl WebSocketClient {
    pub async fn load_persistent_queues(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (stimuli, verdicts) = self.queue_store.load_queues()?;
        let mut stimuli_queue = self.pending_stimuli.lock().await;
        *stimuli_queue = stimuli;
        drop(stimuli_queue);
        let mut verdicts_queue = self.pending_verdicts.lock().await;
        *verdicts_queue = verdicts;
        Ok(())
    }

    pub async fn save_persistent_queues(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (stimuli, verdicts) = self.queue_snapshots().await;
        self.queue_store.save_queues(&stimuli, &verdicts)
    }

    pub async fn should_skip_stimulus(&self, stimulus_id: &str) -> bool {
        let now = chrono::Utc::now().timestamp();
        let cache = self.dedup_cache.lock().await;

        if let Some(timestamp) = cache.get(stimulus_id) {
            now - *timestamp < 3600
        } else {
            false
        }
    }

    pub async fn mark_stimulus_processed(&self, stimulus_id: String) {
        let now = chrono::Utc::now().timestamp();
        let mut cache = self.dedup_cache.lock().await;
        cache.insert(stimulus_id, now);
        cache.retain(|_, &mut ts| now - ts < 3600);
    }

    pub async fn queue_stimulus(&self, stimulus: PendingStimulus) {
        {
            let mut queue = self.pending_stimuli.lock().await;
            queue.push_back(stimulus);
        }
        let _ = self.save_persistent_queues().await;
    }

    pub async fn queue_verdict(&self, verdict: PendingVerdict) {
        {
            let mut queue = self.pending_verdicts.lock().await;
            queue.push_back(verdict);
        }
        let _ = self.save_persistent_queues().await;
    }

    pub async fn dequeue_stimulus(&self) -> Option<PendingStimulus> {
        let stimulus = {
            let mut queue = self.pending_stimuli.lock().await;
            queue.pop_front()
        };
        if stimulus.is_some() {
            let _ = self.save_persistent_queues().await;
        }
        stimulus
    }

    pub async fn dequeue_verdict(&self) -> Option<PendingVerdict> {
        let verdict = {
            let mut queue = self.pending_verdicts.lock().await;
            queue.pop_front()
        };
        if verdict.is_some() {
            let _ = self.save_persistent_queues().await;
        }
        verdict
    }

    pub async fn get_node_id(&self) -> Option<String> {
        self.node_id.lock().await.clone()
    }

    pub async fn set_node_id(&self, id: String) {
        *self.node_id.lock().await = Some(id);
    }

    pub async fn pending_stimulus_count(&self) -> usize {
        self.pending_stimuli.lock().await.len()
    }

    pub async fn pending_verdict_count(&self) -> usize {
        self.pending_verdicts.lock().await.len()
    }

    pub(super) async fn queue_snapshots(
        &self,
    ) -> (VecDeque<PendingStimulus>, VecDeque<PendingVerdict>) {
        let stimuli = self.pending_stimuli.lock().await.clone();
        let verdicts = self.pending_verdicts.lock().await.clone();
        (stimuli, verdicts)
    }
}

#[allow(dead_code)]
fn _type_anchor(_: HashMap<String, i64>) {}
