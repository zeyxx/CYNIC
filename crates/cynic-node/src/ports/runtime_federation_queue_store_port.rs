use std::collections::VecDeque;

use crate::websocket::{PendingStimulus, PendingVerdict};

pub trait RuntimeFederationQueueStorePort: Send + Sync {
    fn load_queues(
        &self,
    ) -> Result<(VecDeque<PendingStimulus>, VecDeque<PendingVerdict>), Box<dyn std::error::Error>>;

    fn save_queues(
        &self,
        stimuli: &VecDeque<PendingStimulus>,
        verdicts: &VecDeque<PendingVerdict>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
