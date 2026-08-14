//! Concrete ModelPublisherPort implementations.
//!
//! Each adapter pushes a NormalisedModel to one specific target system.
//! All are independent — adding a new interface means adding a new struct here.

pub mod inference_toml;
pub mod null;
pub mod open_webui;
