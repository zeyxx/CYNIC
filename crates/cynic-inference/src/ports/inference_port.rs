use async_trait::async_trait;

use crate::domain::inference::BackendStatus;

#[async_trait]
pub trait BackendPort: Send + Sync {
    fn name(&self) -> &str;
    async fn health(&self) -> BackendStatus;
}
