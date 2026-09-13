//! Network observation port (4-layer report assembly).
//!
//! `observe_network` is implemented by `LocalNetworkAdapter` (reads `/proc`
//! and `ip -j`) and `RemoteNetworkAdapter<R: ArgvRunner>` (SSH, injected
//! runner). The port keeps the host-side decision boundary: it decides
//! which layer each parser result feeds, never the raw substrate.

use crate::types::NetworkReport;

/// Read-only account of the network substrate in 4 layers.
///
/// Implementors MUST populate every layer they can observe; a missing lower
/// layer stays an explicit residual unknown (`report.unknowns`), never a guess.
pub trait NetworkObservationPort {
    /// Assemble a `NetworkReport` from the local or remote substrate.
    fn observe_network(&self) -> NetworkReport;
}
