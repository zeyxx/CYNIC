//! Infrastructure layer — contains async primitives and filesystem access.
//!
//! This is the boundary where `std::fs`, `tokio::sync`, and other I/O
//! primitives are permitted. The `domain/` layer is 100% pure.

pub mod domain_task_queue;
pub mod inference_queue;
pub mod model_layout;
pub mod principal_paths;
pub mod slot_semaphore;
pub mod slot_tracker;
