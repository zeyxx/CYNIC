//! Output and input ports for QUIC transport operations.

use super::domain::{ConnectionMigrationEvent, QuicTransportParameters, StreamChannel};

/// Port defining asynchronous QUIC stream handling capabilities.
#[async_trait::async_trait]
pub trait QuicTransportPort: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Negotiate and establish a QUIC session with peer.
    async fn connect(
        &self,
        peer_addr: &str,
        server_name: &str,
        params: &QuicTransportParameters,
    ) -> Result<String, Self::Error>;

    /// Open an outbound multiplexed stream of the specified channel type.
    async fn open_stream(
        &self,
        connection_id: &str,
        channel: StreamChannel,
    ) -> Result<u64, Self::Error>;

    /// Send bytes over an open stream without head-of-line blocking on sibling streams.
    async fn send_data(
        &self,
        connection_id: &str,
        stream_id: u64,
        data: &[u8],
    ) -> Result<(), Self::Error>;

    /// Read incoming bytes from an open stream.
    async fn receive_data(
        &self,
        connection_id: &str,
        stream_id: u64,
        max_bytes: usize,
    ) -> Result<Vec<u8>, Self::Error>;

    /// Notify or observe a connection migration event.
    async fn observe_migration(
        &self,
        connection_id: &str,
    ) -> Result<Option<ConnectionMigrationEvent>, Self::Error>;
}
