//! Fleet projection: one responsibility — turn the node registry and the
//! coordination log into an observable fleet picture as pure data (`assemble`)
//! or a single-file HTML dashboard (`render_html`). Zero I/O, zero terminal
//! handling. Interactive rendering belongs to `cynic-tui`, fed by mapping a
//! [`FleetReport`] onto its generic dashboard model.

pub mod report;
pub mod types;

pub use report::{assemble, render_html, short_hash};
pub use types::{FleetLogRow, FleetNodeRow, FleetReport, LogEntry, NodeEntry};
