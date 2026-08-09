//! Sovereign Solana-native plugin and threat firewall for ZeroClaw agents.

pub mod plugin;
pub mod qr;

pub use plugin::{ZeroClawSolanaPayOutput, ZeroClawSolanaPlugin, ZeroClawToolDefinition};
pub use qr::render_qr_ascii;
