//! Hugging Face Hub CLI adapter (`hf`).
//!
//! Canonical download path into the CYNIC model vault. Does not replace ODS
//! model management; complements native vault lifecycle.

pub mod cli;

pub use cli::HfCliAdapter;
