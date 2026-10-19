//! Generic fullscreen dashboard kit for CYNIC domains.
//!
//! Domains project their own data onto [`Dashboard`] (tables plus text
//! panels) and hand the kit a refresh closure. The kit owns the terminal:
//! fullscreen rendering, `q` quit, `r` refresh, plain-text fallback when
//! stdout is not a TTY. The kit never reads domain files — the closure does.

pub mod app;
pub mod conversation;
pub mod types;

pub use app::run;
pub use conversation::{Message, run_conversation};
pub use types::{Dashboard, Panel, TablePanel, TextPanel};
