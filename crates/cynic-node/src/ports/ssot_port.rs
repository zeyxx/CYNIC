use std::error::Error;

/// Port: Interface to the Single Source of Truth (SurrealDB)
pub trait SSoTPort {
    /// Fetch the canonical list of permitted LXC/VM IDs
    async fn fetch_canonical_organs(&self) -> Result<Vec<u32>, Box<dyn Error>>;
}
