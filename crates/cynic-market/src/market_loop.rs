//! Market loop — proposal → prediction → resolution → scoring.

use crate::registry::MarketRegistry;

/// Market loop engine.
#[derive(Debug)]
pub struct MarketLoop;

impl MarketLoop {
    pub fn new() -> Self {
        Self
    }

    /// Process a proposal through the market loop.
    pub fn process_proposal(
        &self,
        _registry: &mut MarketRegistry,
        _proposal_id: &str,
    ) -> Result<(), String> {
        // TODO: implement full loop
        Ok(())
    }
}

impl Default for MarketLoop {
    fn default() -> Self {
        Self::new()
    }
}
