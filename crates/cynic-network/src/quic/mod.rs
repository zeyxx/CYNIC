//! QUIC and HTTP/3 multiplexed communication subsystem (RFC 9000 & RFC 9114).

pub mod domain;
pub mod port;

pub use domain::*;
pub use port::*;
