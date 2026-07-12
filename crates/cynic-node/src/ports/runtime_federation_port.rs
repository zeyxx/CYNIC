use tokio::task::JoinHandle;

pub trait RuntimeFederationPort {
    async fn load_persistent_queues(&self) -> Result<(), String>;
    fn spawn_connection(&self) -> JoinHandle<()>;
    async fn process_next_stimulus(&self) -> Result<(), String>;
}

pub trait RuntimeFederationFactory {
    type Federation: RuntimeFederationPort;

    fn build(&self, kernel_http_url: &str, node_identity: &str) -> Self::Federation;
}
