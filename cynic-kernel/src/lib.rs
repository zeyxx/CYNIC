pub mod probe;
pub mod supervisor;
pub mod hal;
pub mod pulse;
pub mod storage;
pub mod backend;
pub mod backend_llamacpp;
pub mod router;
pub mod dog;
pub mod gemini_dog;
pub mod deterministic_dog;
pub mod judge;
pub mod rest;

pub mod cynic_v2 {
    tonic::include_proto!("cynic.v2");
}
