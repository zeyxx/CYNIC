//! Provider HTTP adapters live in submodules:
//! - `cloudflare` — CloudflareDnsPort + EdgeProbePort impl
//! - `porkbun` — RegistrarPort impl
//! - `memory` — in-memory test adapters (cfg(test))

pub mod cloudflare;
pub mod porkbun;

#[cfg(test)]
pub mod memory;
