use std::collections::VecDeque;
use std::path::Path;

use tracing::info;

use super::{PendingStimulus, PendingVerdict};

pub(super) fn load_queues(
    queue_dir: &Path,
) -> Result<(VecDeque<PendingStimulus>, VecDeque<PendingVerdict>), Box<dyn std::error::Error>> {
    let stimuli_file = queue_dir.join("pending_stimuli.json");
    let verdicts_file = queue_dir.join("pending_verdicts.json");

    let mut stimuli_queue = VecDeque::new();
    let mut verdicts_queue = VecDeque::new();

    if stimuli_file.exists() {
        let data = std::fs::read_to_string(&stimuli_file)?;
        let stimuli: Vec<PendingStimulus> = serde_json::from_str(&data)?;
        stimuli_queue.extend(stimuli);
        info!(
            count = stimuli_queue.len(),
            "Loaded pending stimuli from disk"
        );
    }

    if verdicts_file.exists() {
        let data = std::fs::read_to_string(&verdicts_file)?;
        let verdicts: Vec<PendingVerdict> = serde_json::from_str(&data)?;
        verdicts_queue.extend(verdicts);
        info!(
            count = verdicts_queue.len(),
            "Loaded pending verdicts from disk"
        );
    }

    Ok((stimuli_queue, verdicts_queue))
}

pub(super) fn save_queues(
    queue_dir: &Path,
    stimuli: &VecDeque<PendingStimulus>,
    verdicts: &VecDeque<PendingVerdict>,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(queue_dir)?;

    let stimuli_file = queue_dir.join("pending_stimuli.json");
    let stimuli_vec: Vec<_> = stimuli.iter().cloned().collect();
    std::fs::write(&stimuli_file, serde_json::to_string(&stimuli_vec)?)?;

    let verdicts_file = queue_dir.join("pending_verdicts.json");
    let verdicts_vec: Vec<_> = verdicts.iter().cloned().collect();
    std::fs::write(&verdicts_file, serde_json::to_string(&verdicts_vec)?)?;

    info!("Saved persistent queues to disk");
    Ok(())
}
