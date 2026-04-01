// transport/ — Wire-level adapters that implement the Dog transport trait.
// Each sub-module handles one protocol family (llama.cpp HTTP, cloud APIs).
// Phase 1 placeholder — trait and adapters added in Task 4.

pub mod cloud;
pub mod llama_server;
