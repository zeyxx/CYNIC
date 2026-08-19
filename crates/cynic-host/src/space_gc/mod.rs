pub mod observe;
pub mod types;

pub use observe::observe_space_usage;
pub use types::{GcScope, GcSummary, SpaceGcObservation};
