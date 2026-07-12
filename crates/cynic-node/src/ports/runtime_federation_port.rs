use tokio::task::JoinHandle;

use crate::domain::runtime_federation::RuntimeFederationConfig;

pub trait RuntimeFederationPort {
    async fn load_persistent_queues(&self) -> Result<(), String>;
    fn spawn_connection(&self) -> JoinHandle<()>;
    async fn process_next_stimulus(&self) -> Result<(), String>;
}

pub trait RuntimeFederationFactory {
    type Federation: RuntimeFederationPort;

    fn build(&self, config: RuntimeFederationConfig) -> Self::Federation;
}
