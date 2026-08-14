use crate::domain::homeostasis::{GarbageCollector, Waste};
use crate::ports::hypervisor_port::HypervisorPort;
use crate::ports::ssot_port::SSoTPort;
use tracing::{info, warn};

/// Use Case: Enforce homeostasis by running the Garbage Collector
pub struct EnforceHomeostasisUseCase<H: HypervisorPort, S: SSoTPort> {
    hypervisor: H,
    ssot: S,
}

impl<H: HypervisorPort, S: SSoTPort> EnforceHomeostasisUseCase<H, S> {
    pub fn new(hypervisor: H, ssot: S) -> Self {
        Self { hypervisor, ssot }
    }

    /// Run the GC cycle on L1 (Hypervisor)
    pub async fn execute_l1_hypervisor_gc(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Executing L1 Garbage Collector (Hypervisor vs SSoT)...");

        // 1. Fetch truth from SSoT
        let canonical_organs = self.ssot.fetch_canonical_organs().await?;

        // 2. Fetch reality from Hypervisor
        let active_instances = self.hypervisor.get_active_instances()?;

        // 3. Domain Logic: Evaluate waste
        let waste =
            GarbageCollector::evaluate_hypervisor_waste(&canonical_organs, &active_instances);

        if waste.is_empty() {
            info!("L1 Homeostasis: Perfect. No waste detected.");
            return Ok(());
        }

        // 4. Act upon the waste
        for w in waste {
            match w {
                Waste::GhostContainer(id) => {
                    warn!(
                        "L1 Homeostasis: Detected Ghost Container ID {}. Initiating termination...",
                        id
                    );
                    // In a dry-run we might just log. Here we assume we have mandate to kill.
                    // self.hypervisor.destroy_instance(id)?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
