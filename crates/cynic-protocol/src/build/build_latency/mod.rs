//! Build-latency policy predicates derived from measured winning numbers.
//!
//! Each predicate encodes one threshold measured in Tasks 1-5 of the
//! build-latency science plan. Zero I/O — orchestration stays in cynic-node.
//!
//! # Winning numbers (measured, verbatim)
//!
//! - **H1** (Task 2): persistent stable checkout + shared `CARGO_TARGET_DIR`
//!   gives **-51.6 %** execute on same SHA (92 929 ms → 44 985 ms).
//!   Source: settlement sidecar `build-latency-science-task2-h1`,
//!   builder 10.10.10.119, 2026-10-14.
//!
//! - **H2** (Task 3): `jobs=2` is the sweet spot on 4-CPU/8GB
//!   (79 691 ms → 49 962 ms → 46 222 ms for J1/J2/J4).
//!   RSS peak ~1.2-1.3 GB flat across JOBS; cap set at **6144 MB**.
//!   Source: settlement sidecar `build-latency-science-task3-h2`,
//!   builder 10.10.10.119, 2026-10-14.
//!
//! - **H3** (Task 4): sccache cross-SHA Rust hit rate **~90 %**
//!   (adopt threshold was **20 %**).
//!   Source: sccache stats, builder 10.10.10.119, 2026-10-14.

pub mod predicates;

pub use predicates::*;
