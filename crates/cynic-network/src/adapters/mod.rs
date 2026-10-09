//! Imperative Shell — Concrete I/O Adapters for CYNIC Network.
//!
//! Conforms to Functional Core / Imperative Shell (FCIS) architecture:
//! - All direct side-effects, external HTTP client calls, and raw OS filesystem
//!   reads are strictly isolated here.
//! - Core domain modules remain 100% pure Zero-I/O.

pub mod dns_resolver;
pub mod finance;
pub mod headscale;
pub mod host_probe;

pub use dns_resolver::RegistryDnsResolver;
pub use finance::NetworkFinanceMeter;
pub use headscale::{HeadscaleClient, HeadscaleConfig};
pub use host_probe::{LocalNetworkAdapter, LocalNetworkDiscoveryProbeAdapter};

