//! Reproducible agent evaluation with typed policies and machine-readable proof.

pub mod dataset;
pub mod openai;
pub mod oracle;
pub mod policy;
pub mod port;
pub mod runner;
pub mod types;

pub use types::{AgentEvalDataset, AgentEvalReport, EvalCase, GenerationPolicy};
