//! CYNIC inference runtime — application services and use cases.
//!
//! Thin composition layer over cynic-inference domain contracts. Adapters
//! (LLM providers, embeddings, tool routing) plug in behind ports; this crate
//! owns the orchestration use-cases that bind them into a runnable inference
//! surface for the kernel host.
